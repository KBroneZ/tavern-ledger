//! Self-update from the project's signed GitHub releases (T-105b, D-135).
//!
//! Off in a build whose `plugins.updater.pubkey` (tauri.conf.json) is not a
//! real minisign public key: the updater plugin is then never registered and
//! nothing here touches the network. On, the app checks a minute after it
//! starts and every six hours (unless the user turned automatic checks off),
//! downloads a newer release in the background, and the plugin verifies its
//! signature and the version it was signed for before anything is kept. The
//! main window then says the version is ready; it is installed only when the
//! user asks, never during a game, and the app never restarts by itself.
//! The choice to check automatically is kept in `updates.json` in the data
//! folder; a missing file means on.

use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_updater::{Error as PluginError, Update, UpdaterExt};

use crate::applog::{self, Logged};
use crate::overlay::lock;
use crate::AppState;

pub const FILE: &str = "updates.json";
/// Sent to the main window with the new [`View`].
pub const CHANGED: &str = "update-changed";
/// The only window that shows updates.
const WINDOW: &str = "main";
/// Where every release's installer is, by tag: the only download address
/// the app accepts.
pub const RELEASES: &str = "https://github.com/KBroneZ/tavern-ledger/releases/download/";
const FIRST_CHECK: Duration = Duration::from_secs(60);
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Redirects followed on the way to a file (GitHub sends one or two).
const MAX_REDIRECTS: usize = 5;
const SAVE_FAILED: &str = "The update setting could not be saved; it will be lost after a restart.";
const READ_FAILED: &str = "The update setting could not be read; automatic checks are on.";

/// The release public key, when the config's updater key is a real minisign
/// public key (base64 of its text, as `tauri signer generate` writes it).
/// Anything else, such as the placeholder, leaves updates off.
pub fn release_key(updater: Option<&serde_json::Value>) -> Option<String> {
    let key = updater?.get("pubkey")?.as_str()?;
    let text = base64::engine::general_purpose::STANDARD.decode(key).ok()?;
    let text = String::from_utf8(text).ok()?;
    minisign_verify::PublicKey::decode(&text).ok()?;
    Some(key.to_string())
}

/// The installer a release's `latest.json` must point to: the tag's own file.
pub fn installer_url(releases: &str, version: &str) -> String {
    format!("{releases}v{version}/TavernLedger_{version}_x64-setup.exe")
}

/// Why a check or an install did not go through, for the window's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// GitHub could not be reached, or the download broke off.
    Network,
    /// No update information was found where the app looks.
    NotFound,
    /// The file is not signed by the project's key, or was signed for
    /// another version: refused.
    Signature,
    /// The update information is not what a release of this project writes.
    Invalid,
    /// The installer could not be started.
    Install,
}

/// Where updates stand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Phase {
    /// This build has no release key: no check is ever made.
    Off,
    /// No check yet in this run.
    Idle,
    Checking,
    Downloading {
        version: String,
    },
    /// Downloaded and verified; waits for the user.
    Ready {
        version: String,
    },
    /// The last check found nothing newer.
    Latest,
    Failed {
        reason: Reason,
    },
    Installing {
        version: String,
    },
}

/// What the window gets.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    /// The version running now.
    pub current: String,
    pub auto_check: bool,
    #[serde(flatten)]
    pub phase: Phase,
    /// When the last check ended (UTC seconds).
    pub checked_at: Option<i64>,
    /// The version the main window's notice offers, unless "Later" was pressed.
    pub notice: Option<String>,
    /// A setting that could not be read or saved.
    pub warning: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Saved {
    #[serde(default = "on")]
    auto_check: bool,
}

fn on() -> bool {
    true
}

impl Default for Saved {
    fn default() -> Self {
        Self { auto_check: true }
    }
}

/// A newer release, downloaded and verified, ready to install.
pub trait Downloaded: Send {
    fn version(&self) -> &str;
    /// Starts the installer; on Windows the app then quits.
    fn install(&self) -> Result<(), Reason>;
}

/// A newer release ready to install, nothing newer, or why not.
pub type Fetched = Result<Option<Box<dyn Downloaded>>, Reason>;

/// Where releases come from. `found` is told the newer version before its
/// download starts.
pub trait Source: Send + Sync {
    fn fetch(&self, found: &mut dyn FnMut(&str)) -> Fetched;
}

struct Inner {
    path: Option<PathBuf>,
    saved: Saved,
    phase: Phase,
    checked_at: Option<i64>,
    later: bool,
    warning: Option<String>,
    ready: Option<Box<dyn Downloaded>>,
}

pub struct Updates {
    current: String,
    inner: Mutex<Inner>,
    wake: Mutex<Option<Sender<()>>>,
}

impl Updates {
    /// `on` is whether this build has a release key.
    pub fn new(on: bool, current: &str) -> Self {
        Self {
            current: current.to_string(),
            inner: Mutex::new(Inner {
                path: None,
                saved: Saved::default(),
                phase: if on { Phase::Idle } else { Phase::Off },
                checked_at: None,
                later: false,
                warning: None,
                ready: None,
            }),
            wake: Mutex::new(None),
        }
    }

    pub fn load(&self, data_dir: &Path) {
        let path = data_dir.join(FILE);
        let mut inner = lock(&self.inner);
        inner.path = Some(path.clone());
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(saved) => inner.saved = saved,
                Err(_) => inner.warning = Some(READ_FAILED.into()),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(_) => inner.warning = Some(READ_FAILED.into()),
        }
    }

    pub fn view(&self) -> View {
        let inner = lock(&self.inner);
        let notice = match &inner.phase {
            Phase::Ready { version } if !inner.later => Some(version.clone()),
            _ => None,
        };
        View {
            current: self.current.clone(),
            auto_check: inner.saved.auto_check,
            phase: inner.phase.clone(),
            checked_at: inner.checked_at,
            notice,
            warning: inner.warning.clone(),
        }
    }

    fn is_on(&self) -> bool {
        lock(&self.inner).phase != Phase::Off
    }

    fn auto_check(&self) -> bool {
        lock(&self.inner).saved.auto_check
    }

    /// Saves the choice; it holds for this run even when saving fails.
    pub fn set_auto_check(&self, enabled: bool) -> Result<(), String> {
        let (path, text) = {
            let mut inner = lock(&self.inner);
            inner.saved.auto_check = enabled;
            inner.warning = None;
            (
                inner.path.clone(),
                serde_json::to_string_pretty(&inner.saved),
            )
        };
        let saved = match (path, text) {
            (Some(path), Ok(text)) => write_whole(&path, &text).is_ok(),
            _ => false,
        };
        if saved {
            return Ok(());
        }
        lock(&self.inner).warning = Some(SAVE_FAILED.into());
        Err(SAVE_FAILED.into())
    }

    /// Hides the main window's notice for this run.
    pub fn later(&self) {
        lock(&self.inner).later = true;
    }

    /// One check: nothing when the build has no key, or while a check runs
    /// or a downloaded version waits. `notify` is told every change.
    pub fn check(&self, source: &dyn Source, notify: &dyn Fn(View)) {
        {
            let mut inner = lock(&self.inner);
            match inner.phase {
                Phase::Off
                | Phase::Checking
                | Phase::Downloading { .. }
                | Phase::Ready { .. }
                | Phase::Installing { .. } => return,
                Phase::Idle | Phase::Latest | Phase::Failed { .. } => {}
            }
            inner.phase = Phase::Checking;
        }
        notify(self.view());
        let mut found = |version: &str| {
            lock(&self.inner).phase = Phase::Downloading {
                version: version.to_string(),
            };
            notify(self.view());
        };
        // A bug in the plugin or here must not leave "Checking…" for good
        // and stop the checking thread.
        let result = catch_unwind(AssertUnwindSafe(|| source.fetch(&mut found)))
            .unwrap_or(Err(Reason::Invalid));
        {
            let mut inner = lock(&self.inner);
            inner.checked_at = Some(now());
            match result {
                Ok(Some(downloaded)) => {
                    inner.phase = Phase::Ready {
                        version: downloaded.version().to_string(),
                    };
                    inner.later = false;
                    inner.ready = Some(downloaded);
                }
                Ok(None) => inner.phase = Phase::Latest,
                Err(reason) => inner.phase = Phase::Failed { reason },
            }
        }
        notify(self.view());
    }

    /// Installs the downloaded version when the user asks. Refused during a
    /// game: the installer closes the app, overlay and all. `game_on` is
    /// asked again right before the installer starts.
    pub fn install(&self, game_on: &dyn Fn() -> bool, notify: &dyn Fn(View)) -> Result<(), String> {
        const IN_GAME: &str = "Finish your game first: updating restarts the app.";
        if game_on() {
            return Err(IN_GAME.into());
        }
        let downloaded = {
            let mut inner = lock(&self.inner);
            let Phase::Ready { version } = inner.phase.clone() else {
                return Err("No update is ready to install.".into());
            };
            inner.phase = Phase::Installing { version };
            inner.ready.take()
        };
        notify(self.view());
        if game_on() {
            {
                let mut inner = lock(&self.inner);
                if let Phase::Installing { version } = inner.phase.clone() {
                    inner.phase = Phase::Ready { version };
                }
                inner.ready = downloaded;
            }
            notify(self.view());
            return Err(IN_GAME.into());
        }
        let result = downloaded.map_or(Err(Reason::Install), |d| d.install());
        // Only reached when the installer did not start: the app is still here
        // (on Windows a started installer ends this process).
        if let Err(reason) = result {
            lock(&self.inner).phase = Phase::Failed { reason };
            notify(self.view());
            return Err("The update could not be installed.".into());
        }
        Ok(())
    }

    /// Asks the checking thread for a check now.
    fn wake(&self) -> bool {
        lock(&self.wake)
            .as_ref()
            .is_some_and(|tx| tx.send(()).is_ok())
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Written whole to a side file first, so a crash never leaves half a file.
fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let side = path.with_extension("json.tmp");
    std::fs::write(&side, text)?;
    std::fs::rename(&side, path)
}

/// Starts the checking thread: a check a minute after start-up, then every
/// six hours while automatic checks are on, and whenever "Check now" asks.
/// Does nothing in a build without a release key.
pub fn start<R: Runtime>(app: &AppHandle<R>, source: Arc<dyn Source>) {
    let updates = app.state::<Arc<Updates>>().inner().clone();
    if !updates.is_on() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    *lock(&updates.wake) = Some(tx);
    let app = app.clone();
    thread::spawn(move || {
        let notify = |view: View| {
            let _ = app.emit_to(WINDOW, CHANGED, view);
        };
        let mut wait = FIRST_CHECK;
        loop {
            let asked = match rx.recv_timeout(wait) {
                Ok(()) => true,
                Err(RecvTimeoutError::Timeout) => false,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            // A check by hand also counts as the next automatic one.
            wait = EVERY;
            if asked || updates.auto_check() {
                updates.check(&*source, &notify);
            }
            // Clicks that came during that check asked for the one just made.
            while rx.try_recv().is_ok() {}
        }
    });
}

/// The project's GitHub releases, through the updater plugin.
pub struct GitHub<R: Runtime> {
    pub app: AppHandle<R>,
    /// [`RELEASES`]; a test points it at its own server.
    pub releases: String,
    /// Only for the tests' local server: skip the system proxy.
    pub no_proxy: bool,
}

struct Download {
    update: Update,
    bytes: Vec<u8>,
}

impl Downloaded for Download {
    fn version(&self) -> &str {
        &self.update.version
    }

    fn install(&self) -> Result<(), Reason> {
        self.update
            .install(&self.bytes)
            .map_err(|_| Reason::Install)
    }
}

impl<R: Runtime> Source for GitHub<R> {
    fn fetch(&self, found: &mut dyn FnMut(&str)) -> Fetched {
        tauri::async_runtime::block_on(async {
            let mut builder = self
                .app
                .updater_builder()
                .timeout(CHECK_TIMEOUT)
                .configure_client(|client| client.redirect(https_redirects_only()));
            if self.no_proxy {
                builder = builder.no_proxy();
            }
            let updater = builder.build().map_err(|e| reason(&e))?;
            let Some(mut update) = updater.check().await.map_err(|e| reason(&e))? else {
                return Ok(None);
            };
            if update.download_url.as_str() != installer_url(&self.releases, &update.version) {
                return Err(Reason::Invalid);
            }
            found(&update.version);
            update.timeout = Some(DOWNLOAD_TIMEOUT);
            // Verifies the signature and the version it was signed for.
            let bytes = update
                .download(|_, _| {}, || {})
                .await
                .map_err(|e| reason(&e))?;
            Ok(Some(
                Box::new(Download { update, bytes }) as Box<dyn Downloaded>
            ))
        })
    }
}

/// Redirects are followed only to `https` addresses, a few at most.
fn https_redirects_only() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.url().scheme() != "https" {
            attempt.error("an update redirect left https")
        } else if attempt.previous().len() >= MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else {
            attempt.follow()
        }
    })
}

/// The window's code for a failure; the plugin's own words go to the log.
fn reason(error: &PluginError) -> Reason {
    applog::warn("updates", &format!("update check failed: {error}"));
    match error {
        PluginError::Minisign(_)
        | PluginError::Base64(_)
        | PluginError::SignatureUtf8(_)
        | PluginError::SignedVersionMismatch { .. }
        | PluginError::MissingSignedVersion => Reason::Signature,
        // A latest.json that is not JSON is not a network problem.
        PluginError::Reqwest(e) if e.is_decode() => Reason::Invalid,
        PluginError::Reqwest(_) | PluginError::Network(_) | PluginError::Io(_) => Reason::Network,
        PluginError::ReleaseNotFound => Reason::NotFound,
        _ => Reason::Invalid,
    }
}

fn game_on(state: &AppState) -> bool {
    lock(&state.in_progress).is_some() || lock(&state.live).is_some()
}

fn notify_window<R: Runtime>(app: &AppHandle<R>) -> impl Fn(View) + '_ {
    move |view| {
        let _ = app.emit_to(WINDOW, CHANGED, view);
    }
}

#[tauri::command]
pub fn update_status(updates: State<'_, Arc<Updates>>) -> View {
    updates.view()
}

#[tauri::command]
pub fn update_set_auto(
    app: AppHandle,
    updates: State<'_, Arc<Updates>>,
    enabled: bool,
) -> Result<View, String> {
    let result = updates.set_auto_check(enabled);
    let view = updates.view();
    notify_window(&app)(view.clone());
    result.map(|()| view).logged("update_set_auto")
}

#[tauri::command]
pub fn update_check_now(updates: State<'_, Arc<Updates>>) -> Result<View, String> {
    let asked = if !updates.is_on() {
        Err("Updates are not set up in this build.".to_string())
    } else if !updates.wake() {
        Err("Checking for updates stopped. Restart the app.".to_string())
    } else {
        Ok(updates.view())
    };
    asked.logged("update_check_now")
}

#[tauri::command]
pub fn update_install(
    app: AppHandle,
    updates: State<'_, Arc<Updates>>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    updates
        .install(&|| game_on(&state), &notify_window(&app))
        .logged("update_install")
}

#[tauri::command]
pub fn update_later(app: AppHandle, updates: State<'_, Arc<Updates>>) -> View {
    updates.later();
    let view = updates.view();
    notify_window(&app)(view.clone());
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const CONF: &str = include_str!("../tauri.conf.json");

    fn updater_conf() -> serde_json::Value {
        let conf: serde_json::Value = serde_json::from_str(CONF).unwrap();
        conf["plugins"]["updater"].clone()
    }

    #[test]
    fn the_committed_build_has_no_release_key_so_updates_are_off() {
        let conf = updater_conf();
        assert!(conf["pubkey"].is_string(), "the key's place is there");
        assert_eq!(release_key(Some(&conf)), None);
        assert_eq!(release_key(None), None);
    }

    #[test]
    fn a_real_minisign_public_key_turns_updates_on() {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let text = pair.pk.to_box().unwrap().into_string();
        let key = base64::engine::general_purpose::STANDARD.encode(text);
        let conf = serde_json::json!({ "pubkey": key });
        assert_eq!(release_key(Some(&conf)), Some(key));
        // Not base64, base64 of something else, a bare minisign line: off.
        for bad in [
            "not a key",
            "aGVsbG8=",
            "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3",
        ] {
            assert_eq!(
                release_key(Some(&serde_json::json!({ "pubkey": bad }))),
                None,
                "{bad}"
            );
        }
    }

    #[test]
    fn the_endpoint_is_the_latest_github_release_over_https_and_nothing_is_relaxed() {
        let conf = updater_conf();
        assert_eq!(
            conf["endpoints"],
            serde_json::json!([
                "https://github.com/KBroneZ/tavern-ledger/releases/latest/download/latest.json"
            ])
        );
        assert_eq!(conf["requireSignedVersion"], true);
        for relaxed in [
            "dangerousInsecureTransportProtocol",
            "dangerousAcceptInvalidCerts",
            "dangerousAcceptInvalidHostnames",
            "allowDowngrades",
        ] {
            assert!(conf.get(relaxed).is_none(), "{relaxed}");
        }
        assert_eq!(conf["windows"]["installMode"], "passive");
        let full: serde_json::Value = serde_json::from_str(CONF).unwrap();
        assert_eq!(full["bundle"]["createUpdaterArtifacts"], true);
        assert_eq!(
            full["bundle"]["windows"]["nsis"]["installMode"], "currentUser",
            "per-user install, no admin rights to update"
        );
    }

    #[test]
    fn the_installer_address_is_the_tag_s_own_file() {
        assert_eq!(
            installer_url(RELEASES, "0.2.0"),
            "https://github.com/KBroneZ/tavern-ledger/releases/download/v0.2.0/TavernLedger_0.2.0_x64-setup.exe"
        );
    }

    struct Fake {
        calls: AtomicUsize,
        answer: fn() -> Fetched,
    }

    impl Source for Fake {
        fn fetch(&self, found: &mut dyn FnMut(&str)) -> Fetched {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let answer = (self.answer)();
            if let Ok(Some(d)) = &answer {
                found(d.version());
            }
            answer
        }
    }

    struct Ready(&'static str, Arc<AtomicUsize>);

    impl Downloaded for Ready {
        fn version(&self) -> &str {
            self.0
        }
        fn install(&self) -> Result<(), Reason> {
            self.1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn fake(answer: fn() -> Fetched) -> Fake {
        Fake {
            calls: AtomicUsize::new(0),
            answer,
        }
    }

    fn ready() -> Fetched {
        Ok(Some(Box::new(Ready(
            "0.2.0",
            Arc::new(AtomicUsize::new(0)),
        ))))
    }

    #[test]
    fn a_build_without_a_key_never_asks_the_source() {
        let updates = Updates::new(false, "0.1.0");
        let source = fake(ready);
        let seen = Mutex::new(Vec::new());
        updates.check(&source, &|v| lock(&seen).push(v));
        assert_eq!(source.calls.load(Ordering::SeqCst), 0, "no network");
        assert!(lock(&seen).is_empty());
        assert_eq!(updates.view().phase, Phase::Off);
        assert!(!updates.wake(), "no checking thread either");
    }

    #[test]
    fn a_newer_version_is_downloaded_then_offered_until_later_is_pressed() {
        let updates = Updates::new(true, "0.1.0");
        let source = fake(ready);
        let seen = Mutex::new(Vec::new());
        updates.check(&source, &|v| lock(&seen).push(v.phase));
        assert_eq!(
            *lock(&seen),
            [
                Phase::Checking,
                Phase::Downloading {
                    version: "0.2.0".into()
                },
                Phase::Ready {
                    version: "0.2.0".into()
                },
            ]
        );
        let view = updates.view();
        assert_eq!(view.notice.as_deref(), Some("0.2.0"));
        assert!(view.checked_at.is_some());
        // A version waiting is not downloaded again.
        updates.check(&source, &|_| {});
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
        updates.later();
        assert_eq!(updates.view().notice, None);
        assert!(matches!(updates.view().phase, Phase::Ready { .. }));
    }

    #[test]
    fn nothing_newer_and_a_failure_are_said_and_checked_again_later() {
        let updates = Updates::new(true, "0.1.0");
        updates.check(&fake(|| Ok(None)), &|_| {});
        assert_eq!(updates.view().phase, Phase::Latest);
        assert_eq!(updates.view().notice, None);
        updates.check(&fake(|| Err(Reason::Signature)), &|_| {});
        assert_eq!(
            updates.view().phase,
            Phase::Failed {
                reason: Reason::Signature
            }
        );
        let again = fake(ready);
        updates.check(&again, &|_| {});
        assert_eq!(again.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn install_is_refused_during_a_game_and_without_a_ready_version() {
        let updates = Updates::new(true, "0.1.0");
        assert!(
            updates.install(&|| false, &|_| {}).is_err(),
            "nothing ready"
        );
        let installs = Arc::new(AtomicUsize::new(0));
        let counter = installs.clone();
        let source = FakeReady(counter);
        updates.check(&source, &|_| {});
        let refused = updates.install(&|| true, &|_| {}).unwrap_err();
        assert!(refused.contains("Finish your game"), "{refused}");
        assert_eq!(installs.load(Ordering::SeqCst), 0);
        assert!(matches!(updates.view().phase, Phase::Ready { .. }));
        updates.install(&|| false, &|_| {}).unwrap();
        assert_eq!(installs.load(Ordering::SeqCst), 1);
        assert_eq!(
            updates.view().phase,
            Phase::Installing {
                version: "0.2.0".into()
            }
        );
    }

    struct Broken;

    impl Downloaded for Broken {
        fn version(&self) -> &str {
            "0.2.0"
        }
        fn install(&self) -> Result<(), Reason> {
            Err(Reason::Install)
        }
    }

    #[test]
    fn an_installer_that_does_not_start_is_said_and_a_game_starting_meanwhile_stops_it() {
        let updates = Updates::new(true, "0.1.0");
        updates.check(&fake(|| Ok(Some(Box::new(Broken)))), &|_| {});
        let error = updates.install(&|| false, &|_| {}).unwrap_err();
        assert_eq!(error, "The update could not be installed.");
        assert_eq!(
            updates.view().phase,
            Phase::Failed {
                reason: Reason::Install
            }
        );
        // A game that starts between the click and the installer: still ready.
        let installs = Arc::new(AtomicUsize::new(0));
        let updates = Updates::new(true, "0.1.0");
        updates.check(&FakeReady(installs.clone()), &|_| {});
        let asked = AtomicUsize::new(0);
        let game_on = || asked.fetch_add(1, Ordering::SeqCst) > 0;
        assert!(updates.install(&game_on, &|_| {}).is_err());
        assert_eq!(installs.load(Ordering::SeqCst), 0);
        assert_eq!(updates.view().notice.as_deref(), Some("0.2.0"));
        updates.install(&|| false, &|_| {}).unwrap();
        assert_eq!(installs.load(Ordering::SeqCst), 1, "the download was kept");
    }

    struct FakeReady(Arc<AtomicUsize>);

    impl Source for FakeReady {
        fn fetch(&self, _: &mut dyn FnMut(&str)) -> Fetched {
            Ok(Some(Box::new(Ready("0.2.0", self.0.clone()))))
        }
    }

    struct Panics;

    impl Source for Panics {
        fn fetch(&self, _: &mut dyn FnMut(&str)) -> Fetched {
            panic!("a bug in the source")
        }
    }

    #[test]
    fn a_check_that_panics_ends_as_a_failure_and_can_run_again() {
        let updates = Updates::new(true, "0.1.0");
        updates.check(&Panics, &|_| {});
        assert_eq!(
            updates.view().phase,
            Phase::Failed {
                reason: Reason::Invalid
            }
        );
        let again = fake(ready);
        updates.check(&again, &|_| {});
        assert_eq!(again.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn automatic_checks_are_on_by_default_and_the_choice_is_kept() {
        let dir = std::env::temp_dir().join(format!("tl-updates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let updates = Updates::new(true, "0.1.0");
        updates.load(&dir);
        assert!(updates.view().auto_check);
        updates.set_auto_check(false).unwrap();
        let again = Updates::new(true, "0.1.0");
        again.load(&dir);
        assert!(!again.view().auto_check);
        std::fs::write(dir.join(FILE), "{broken").unwrap();
        let broken = Updates::new(true, "0.1.0");
        broken.load(&dir);
        assert!(broken.view().auto_check);
        assert_eq!(broken.view().warning.as_deref(), Some(READ_FAILED));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_window_gets_the_phase_flat_with_its_words_as_codes() {
        let view = View {
            current: "0.1.0".into(),
            auto_check: true,
            phase: Phase::Failed {
                reason: Reason::NotFound,
            },
            checked_at: Some(5),
            notice: None,
            warning: None,
        };
        let json = serde_json::to_value(view).unwrap();
        assert_eq!(json["state"], "failed");
        assert_eq!(json["reason"], "not_found");
        assert_eq!(json["auto_check"], true);
    }
}
