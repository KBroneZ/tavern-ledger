//! Tavern Ledger desktop app: follows Power.log in the background and shows
//! the local game history. Only reads the game's log files (D-004).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::panic::{catch_unwind, AssertUnwindSafe};
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
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
struct Status {
    logs_dir: Option<String>,
    history: Option<String>,
    session: Option<String>,
    /// False while the followed session has no Power.log.
    power_log: bool,
    /// Set when something is wrong; the app says so instead of showing nothing.
    problem: Option<String>,
    unreadable_lines: usize,
}

#[derive(Clone, Serialize)]
struct GameRow {
    session: String,
    index: u64,
    report: serde_json::Value,
}

/// Shared with the window. The watcher itself lives only in the tracker
/// thread, so reading a big log never blocks the window.
#[derive(Default)]
struct AppState {
    games: Mutex<Vec<GameRow>>,
    status: Mutex<Status>,
}

#[tauri::command]
fn list_games(state: State<'_, Arc<AppState>>) -> Vec<GameRow> {
    state
        .games
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

#[tauri::command]
fn status(state: State<'_, Arc<AppState>>) -> Status {
    state
        .status
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Updates the status and tells the window, only if something changed.
fn set_status(app: &AppHandle, state: &AppState, update: impl FnOnce(&mut Status)) {
    let changed = {
        let mut status = state.status.lock().unwrap_or_else(|e| e.into_inner());
        let before = status.clone();
        update(&mut status);
        (*status != before).then(|| status.clone())
    };
    if let Some(snapshot) = changed {
        let _ = app.emit(STATUS_CHANGED, snapshot);
    }
}

fn publish_games(app: &AppHandle, state: &AppState, store: &Store) {
    let rows = store
        .games()
        .map(|(key, report)| GameRow {
            session: key.session.clone(),
            index: key.index,
            report: report.clone(),
        })
        .collect();
    *state.games.lock().unwrap_or_else(|e| e.into_inner()) = rows;
    let _ = app.emit(GAMES_CHANGED, ());
}

fn problem(app: &AppHandle, state: &AppState, message: String) {
    set_status(app, state, |s| s.problem = Some(message));
}

/// Opens the history and follows the log until the app quits.
fn run_tracker(app: AppHandle, state: Arc<AppState>) {
    let Some(logs_dir) = find_logs_dir() else {
        let msg = "Hearthstone's Logs folder was not found. Is the game installed?";
        return problem(&app, &state, msg.into());
    };
    let store = match Store::open(&tracker::store::default_dir()) {
        Ok(store) => store,
        Err(e) => {
            return problem(
                &app,
                &state,
                format!("Cannot open the game history ({:?}).", e.kind()),
            )
        }
    };
    set_status(&app, &state, |s| {
        s.logs_dir = Some(logs_dir.display().to_string());
        s.history = Some(store.path().display().to_string());
        s.unreadable_lines = store.unreadable_lines;
    });
    publish_games(&app, &state, &store);
    let mut watcher = Watcher::new(&logs_dir, store);

    loop {
        // A bug must not stop the tracker silently while the window looks fine.
        let Ok(result) = catch_unwind(AssertUnwindSafe(|| watcher.poll())) else {
            let msg = "The tracker stopped after an internal error. Restart the app; your history is safe.";
            return problem(&app, &state, msg.into());
        };
        let session = watcher.session().map(String::from);
        let power_log = watcher.has_power_log();
        match result {
            Ok(saved) => {
                set_status(&app, &state, |s| {
                    s.session = session;
                    s.power_log = power_log;
                    s.problem = None;
                });
                if !saved.is_empty() {
                    publish_games(&app, &state, watcher.store());
                }
            }
            Err(e) => {
                // Games read before the error are already saved.
                publish_games(&app, &state, watcher.store());
                problem(
                    &app,
                    &state,
                    format!(
                        "Could not read the log right now ({:?}); retrying.",
                        e.kind()
                    ),
                );
            }
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
