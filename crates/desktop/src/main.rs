//! Tavern Ledger desktop app: follows Power.log in the background and shows
//! the local game history. Only reads the game's log files (D-004).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracker::discover::find_logs_dir;
use tracker::store::Store;
use tracker::Watcher;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const GAMES_CHANGED: &str = "games-changed";
const STATUS_CHANGED: &str = "status-changed";

/// What the window shows above the list. Plain words, no log text.
#[derive(Clone, Debug, Default, Serialize)]
struct Status {
    logs_dir: Option<String>,
    history: Option<String>,
    session: Option<String>,
    /// Set when something is wrong; the app says so instead of showing nothing.
    problem: Option<String>,
    unreadable_lines: usize,
}

#[derive(Default)]
struct AppState {
    watcher: Mutex<Option<Watcher>>,
    status: Mutex<Status>,
}

#[derive(Serialize)]
struct GameRow {
    session: String,
    index: u64,
    report: serde_json::Value,
}

#[tauri::command]
fn list_games(state: State<'_, Arc<AppState>>) -> Vec<GameRow> {
    let watcher = state.watcher.lock().unwrap_or_else(|e| e.into_inner());
    let Some(watcher) = watcher.as_ref() else {
        return Vec::new();
    };
    watcher
        .store()
        .games()
        .map(|(key, report)| GameRow {
            session: key.session.clone(),
            index: key.index,
            report: report.clone(),
        })
        .collect()
}

#[tauri::command]
fn status(state: State<'_, Arc<AppState>>) -> Status {
    state
        .status
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn set_status(app: &AppHandle, state: &AppState, update: impl FnOnce(&mut Status)) {
    let snapshot = {
        let mut status = state.status.lock().unwrap_or_else(|e| e.into_inner());
        update(&mut status);
        status.clone()
    };
    let _ = app.emit(STATUS_CHANGED, snapshot);
}

/// Opens the history and follows the log until the app quits.
fn run_tracker(app: AppHandle, state: Arc<AppState>) {
    let Some(logs_dir) = find_logs_dir() else {
        set_status(&app, &state, |s| {
            s.problem =
                Some("Hearthstone's Logs folder was not found. Is the game installed?".into())
        });
        return;
    };
    let dir = tracker::store::default_dir();
    let store = match Store::open(&dir) {
        Ok(store) => store,
        Err(e) => {
            set_status(&app, &state, |s| {
                s.problem = Some(format!("Cannot open the game history ({:?}).", e.kind()))
            });
            return;
        }
    };
    set_status(&app, &state, |s| {
        s.logs_dir = Some(logs_dir.display().to_string());
        s.history = Some(store.path().display().to_string());
        s.unreadable_lines = store.unreadable_lines;
    });
    *state.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(Watcher::new(&logs_dir, store));
    let _ = app.emit(GAMES_CHANGED, ());

    loop {
        let (result, session) = {
            let mut guard = state.watcher.lock().unwrap_or_else(|e| e.into_inner());
            let watcher = guard.as_mut().expect("set above");
            (watcher.poll(), watcher.session().map(String::from))
        };
        match result {
            Ok(saved) => {
                set_status(&app, &state, |s| {
                    s.session = session;
                    s.problem = None;
                });
                if !saved.is_empty() {
                    let _ = app.emit(GAMES_CHANGED, ());
                }
            }
            Err(e) => set_status(&app, &state, |s| {
                s.problem = Some(format!(
                    "Could not read the log right now ({:?}); retrying.",
                    e.kind()
                ))
            }),
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn main() {
    let state = Arc::new(AppState::default());
    tauri::Builder::default()
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![list_games, status])
        .setup(move |app| {
            let handle = app.handle().clone();
            thread::spawn(move || run_tracker(handle, state));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start Tavern Ledger");
}
