//! Upload settings panel and its background worker (T-104d). The worker owns
//! the upload engine; window commands only send it messages and return at
//! once, and the worker tells the window what changed ("upload-changed").
//! It only starts in the process that holds the history lock, so two copies
//! of the app never upload at the same time.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uploader::config::{self, ConfigError};
use uploader::http::{Http, NetError, Request, Response};
use uploader::secrets::{credential_name, CredentialManager};
use uploader::signin::SignIn;
use uploader::{Engine, Status, Wake};

const UPLOAD_CHANGED: &str = "upload-changed";
/// Games sent before the worker looks at its messages again (a switch-off
/// or sign-out waits at most this many uploads).
const BUDGET: usize = 5;
/// How long a sign-in link is waited for.
const SIGN_IN_WAIT: Duration = Duration::from_secs(15 * 60);

enum Msg {
    Kick,
    SetEnabled(bool),
    SignIn(String),
    CancelSignIn,
    LinkSent,
    SignedIn(Result<uploader::auth::Session, String>),
    SignOut,
}

/// What the panel shows.
#[derive(Clone, Debug, Default, Serialize)]
pub struct View {
    #[serde(flatten)]
    status: Status,
    /// The worker runs in this window's process.
    running: bool,
    /// Email a sign-in link was asked for, while waiting for it to be opened.
    signing_in: Option<String>,
    /// True once the link request went out.
    link_sent: bool,
    /// A one-off message (a refused switch, a failed sign-in).
    notice: Option<String>,
}

#[derive(Default)]
pub struct Upload {
    tx: Mutex<Option<Sender<Msg>>>,
    view: Mutex<View>,
}

impl Upload {
    fn send(&self, msg: Msg) -> bool {
        let tx = self.tx.lock().unwrap_or_else(|e| e.into_inner());
        tx.as_ref().is_some_and(|tx| tx.send(msg).is_ok())
    }
}

fn not_running(upload: &Upload) -> Result<(), String> {
    Err(if upload.tx.lock().map(|t| t.is_none()).unwrap_or(true) {
        "Upload is not running in this window: it starts once the app follows your games (if another Tavern Ledger is open, use that one).".into()
    } else {
        "The upload worker stopped; restart the app.".into()
    })
}

#[tauri::command]
pub fn upload_status(upload: State<'_, Upload>) -> View {
    upload
        .view
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

#[tauri::command]
pub fn upload_set_enabled(upload: State<'_, Upload>, on: bool) -> Result<(), String> {
    if upload.send(Msg::SetEnabled(on)) {
        Ok(())
    } else {
        not_running(&upload)
    }
}

#[tauri::command]
pub fn upload_sign_in(upload: State<'_, Upload>, email: String) -> Result<(), String> {
    if email.len() > 254 {
        return Err("That email is too long.".into());
    }
    if upload.send(Msg::SignIn(email)) {
        Ok(())
    } else {
        not_running(&upload)
    }
}

#[tauri::command]
pub fn upload_cancel_sign_in(upload: State<'_, Upload>) -> Result<(), String> {
    if upload.send(Msg::CancelSignIn) {
        Ok(())
    } else {
        not_running(&upload)
    }
}

#[tauri::command]
pub fn upload_sign_out(upload: State<'_, Upload>) -> Result<(), String> {
    if upload.send(Msg::SignOut) {
        Ok(())
    } else {
        not_running(&upload)
    }
}

/// New games were saved: look for games to send.
pub fn kick(app: &AppHandle) {
    app.state::<Upload>().send(Msg::Kick);
}

/// Starts the worker for this data folder (once the history lock is held).
pub fn start(app: &AppHandle, data_dir: PathBuf) {
    let (tx, rx) = mpsc::channel();
    let upload = app.state::<Upload>();
    *upload.tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx.clone());
    let app = app.clone();
    thread::spawn(move || run(app, data_dir, tx, rx));
}

/// Stands in for the HTTPS client if it could not start: every request fails.
struct NoHttp;
impl Http for NoHttp {
    fn send(&self, _: Request) -> Result<Response, NetError> {
        Err(NetError::Connect("no client".into()))
    }
}

fn engine_for(data_dir: PathBuf) -> Engine {
    let tokens = Box::new(CredentialManager::new(&credential_name(&data_dir)));
    let mut server = config::load(&data_dir);
    let http: Arc<dyn Http> = match uploader::http::ReqwestHttp::new() {
        Ok(client) => Arc::new(client),
        Err(_) => {
            server = Err(ConfigError::Invalid("the HTTPS client could not start"));
            Arc::new(NoHttp)
        }
    };
    Engine::new(data_dir, server, http, tokens)
}

fn publish(app: &AppHandle, update: impl FnOnce(&mut View)) {
    let upload = app.state::<Upload>();
    let snapshot = {
        let mut view = upload.view.lock().unwrap_or_else(|e| e.into_inner());
        update(&mut view);
        view.clone()
    };
    let _ = app.emit(UPLOAD_CHANGED, snapshot);
}

/// One sign-in attempt in its own thread: it waits up to 15 minutes for the
/// link, and the worker must stay free meanwhile.
fn sign_in_thread(engine: &Engine, email: String, tx: Sender<Msg>, cancel: Arc<AtomicBool>) {
    let Some(server) = engine.server().cloned() else {
        let _ = tx.send(Msg::SignedIn(Err("No upload server is set up yet.".into())));
        return;
    };
    let http = engine.http();
    thread::spawn(move || {
        // A bug here must not leave the panel on "signing in" for ever.
        let attempt = catch_unwind(AssertUnwindSafe(|| {
            SignIn::start(&server, http.as_ref(), &email).and_then(|flow| {
                let _ = tx.send(Msg::LinkSent);
                flow.finish(
                    &server,
                    http.as_ref(),
                    Instant::now() + SIGN_IN_WAIT,
                    &cancel,
                )
            })
        }));
        let result =
            attempt.unwrap_or_else(|_| Err("The sign-in stopped after an internal error.".into()));
        // A cancelled attempt says nothing: the user started another one or gave up.
        if !cancel.load(Ordering::SeqCst) {
            let _ = tx.send(Msg::SignedIn(result));
        }
    });
}

fn run(app: AppHandle, data_dir: PathBuf, tx: Sender<Msg>, rx: Receiver<Msg>) {
    let mut engine = engine_for(data_dir);
    let mut cancel: Option<Arc<AtomicBool>> = None;
    let mut wake = Wake::Now;
    publish(&app, |v| {
        v.status = engine.status().clone();
        v.running = true;
    });
    loop {
        let msg = match wake {
            Wake::Idle => rx.recv().ok(),
            Wake::Now => Some(rx.try_recv().unwrap_or(Msg::Kick)),
            Wake::In(d) => match rx.recv_timeout(d) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => Some(Msg::Kick),
                Err(RecvTimeoutError::Disconnected) => None,
            },
        };
        let Some(msg) = msg else { return };
        let mut notice: Option<Option<String>> = None;
        let mut signing_in: Option<Option<String>> = None;
        let mut link_sent = None;
        match msg {
            Msg::Kick => {}
            Msg::SetEnabled(on) => notice = Some(engine.set_enabled(on).err()),
            Msg::SignIn(email) => {
                if let Some(old) = cancel.take() {
                    old.store(true, Ordering::SeqCst);
                }
                let flag = Arc::new(AtomicBool::new(false));
                cancel = Some(flag.clone());
                signing_in = Some(Some(email.trim().to_string()));
                link_sent = Some(false);
                notice = Some(None);
                sign_in_thread(&engine, email, tx.clone(), flag);
            }
            Msg::CancelSignIn => {
                if let Some(old) = cancel.take() {
                    old.store(true, Ordering::SeqCst);
                }
                signing_in = Some(None);
            }
            Msg::LinkSent => link_sent = Some(true),
            Msg::SignedIn(result) => {
                cancel = None;
                signing_in = Some(None);
                notice = Some(match result {
                    Ok(session) => engine.signed_in_with(session).err(),
                    Err(why) => Some(why),
                });
            }
            Msg::SignOut => {
                // The engine reports it as the last result.
                engine.sign_out();
                notice = Some(None);
            }
        }
        wake = engine.run(Instant::now(), BUDGET);
        publish(&app, |v| {
            v.status = engine.status().clone();
            if let Some(n) = notice {
                v.notice = n;
            }
            if let Some(s) = signing_in {
                v.signing_in = s;
            }
            if let Some(l) = link_sent {
                v.link_sent = l;
            }
        });
    }
}
