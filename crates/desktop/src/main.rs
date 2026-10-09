//! Tavern Ledger desktop app: follows Power.log in the background and shows
//! the local game history. Only reads the game's log files (D-004).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, State, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tracker::discover::find_logs_dir;
use tracker::lock::{HistoryLock, LockError};
use tracker::stats::Stats;
use tracker::store::Store;
use tracker::Watcher;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const GAMES_CHANGED: &str = "games-changed";
const STATUS_CHANGED: &str = "status-changed";
/// Added by the Windows start-up entry: open in the tray, not on screen.
const MINIMIZED_ARG: &str = "--minimized";
const MENU_SHOW: &str = "show";
const MENU_AUTOSTART: &str = "autostart";
const MENU_QUIT: &str = "quit";

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
    /// A one-off message from the tray menu; the tracker never clears it.
    notice: Option<String>,
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

/// Totals, heroes and tribes for the window, from the same rows it lists:
/// the window renders them and computes nothing (T-102).
#[tauri::command]
fn game_stats(state: State<'_, Arc<AppState>>) -> Stats {
    stats_of(&state.games.lock().unwrap_or_else(|e| e.into_inner()))
}

fn stats_of(rows: &[GameRow]) -> Stats {
    tracker::stats::compute(rows.iter().map(|row| &row.report))
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

/// Another Tavern Ledger (or tavern-watch) is already following the log: show
/// the history without writing to it, and say why nothing new appears.
fn show_read_only(app: &AppHandle, state: &AppState, data_dir: &std::path::Path) {
    if let Ok(store) = Store::open(data_dir) {
        publish_games(app, state, &store);
    }
    let msg = "Tavern Ledger is already running (or tavern-watch is). This window only shows the history; use the other one to follow your games.";
    problem(app, state, msg.into());
}

/// Opens the history and follows the log until the app quits.
fn run_tracker(app: AppHandle, state: Arc<AppState>) {
    let Some(logs_dir) = find_logs_dir() else {
        let msg = "Hearthstone's Logs folder was not found. Is the game installed?";
        return problem(&app, &state, msg.into());
    };
    let data_dir = tracker::store::default_dir();
    // Held while this thread runs: only one process writes the history.
    let _lock = match HistoryLock::acquire(&data_dir) {
        Ok(lock) => lock,
        Err(LockError::Busy) => return show_read_only(&app, &state, &data_dir),
        Err(e) => return problem(&app, &state, format!("Cannot use the game history: {e}.")),
    };
    let store = match Store::open(&data_dir) {
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

fn starts_hidden(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|a| a == MINIMIZED_ARG)
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Turns the Windows start-up entry on or off; the menu shows the real state.
fn toggle_autostart(app: &AppHandle, item: &CheckMenuItem<Wry>) {
    let launcher = app.autolaunch();
    let result = match launcher.is_enabled() {
        Ok(true) => launcher.disable(),
        Ok(false) => launcher.enable(),
        Err(e) => Err(e),
    };
    let _ = item.set_checked(launcher.is_enabled().unwrap_or(false));
    let state = app.state::<Arc<AppState>>();
    let notice = result
        .is_err()
        .then(|| "Could not change the Windows start-up setting.".to_string());
    let failed = notice.is_some();
    set_status(app, &state, |s| s.notice = notice);
    if failed {
        // The window is usually hidden in the tray: show it so the message is seen.
        show_window(app);
    }
}

/// The start-up entry holds the exe path unquoted (auto-launch 0.5), so a
/// path with spaces could run the wrong program or nothing at all.
fn can_autostart(exe: &Path) -> bool {
    !exe.to_string_lossy().contains(char::is_whitespace)
}

/// Tray icon: the app keeps following the log while its window is closed.
fn setup_tray(app: &App) -> tauri::Result<()> {
    // Off by default: only the user turns it on, from this menu.
    let enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let available = std::env::current_exe().is_ok_and(|exe| can_autostart(&exe));
    let label = if available {
        "Start with Windows"
    } else {
        "Start with Windows (not available: the app's folder has spaces)"
    };
    let autostart =
        CheckMenuItem::with_id(app, MENU_AUTOSTART, label, available, enabled, None::<&str>)?;
    let show = MenuItem::with_id(app, MENU_SHOW, "Open Tavern Ledger", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &autostart, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Tavern Ledger: following your Battlegrounds games")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event: MenuEvent| match event.id().as_ref() {
            MENU_SHOW => show_window(app),
            MENU_AUTOSTART => toggle_autostart(app, &autostart),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn main() {
    let state = Arc::new(AppState::default());
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![list_games, game_stats, status])
        .on_window_event(|window, event| {
            // Closing the window hides it; "Quit" in the tray menu exits.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            setup_tray(app)?;
            // The window starts hidden (tauri.conf.json), so a start-up
            // launch never flashes it on screen.
            if !starts_hidden(std::env::args()) {
                show_window(app.handle());
            }
            let handle = app.handle().clone();
            thread::spawn(move || run_tracker(handle, state));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start Tavern Ledger");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn stats_are_computed_from_the_rows_the_window_lists() {
        let row = |game_type: &str, place: i64| GameRow {
            session: "Hearthstone_2026_10_09_00_00_00".into(),
            index: 1,
            report: serde_json::json!({
                "status": "ok", "game_type": game_type, "hero": "H", "final_place": place,
            }),
        };
        let stats = stats_of(&[row("GT_BATTLEGROUNDS", 3), row("GT_BATTLEGROUNDS_DUO", 1)]);
        let solo = &stats.modes[0];
        assert_eq!(
            (solo.mode.as_str(), solo.totals.games),
            ("GT_BATTLEGROUNDS", 1)
        );
        assert_eq!(solo.totals.average_place, Some(3.0));
        assert_eq!(stats.modes[1].totals.wins, Some(1));
    }

    #[test]
    fn starts_hidden_only_when_windows_starts_it() {
        assert!(starts_hidden(args(&["desktop.exe", MINIMIZED_ARG])));
        assert!(!starts_hidden(args(&["desktop.exe"])));
        assert!(!starts_hidden(args(&["desktop.exe", "--minimized-not"])));
    }

    #[test]
    fn autostart_needs_a_path_without_spaces() {
        // The start-up entry is written unquoted (auto-launch 0.5).
        assert!(can_autostart(Path::new(
            r"C:\Apps\TavernLedger\desktop.exe"
        )));
        assert!(!can_autostart(Path::new(
            r"C:\Program Files\Tavern Ledger\desktop.exe"
        )));
    }
}
