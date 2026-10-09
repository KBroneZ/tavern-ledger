//! Tavern Ledger desktop app: follows Power.log in the background and shows
//! the local game history. Only reads the game's log files (D-004).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod upload;

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager, State, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tracker::discover::find_logs_dir;
use tracker::lobby_tribes::{self, Action, Entries, GameStarts, LobbyView};
use tracker::lock::{HistoryLock, LockError};
use tracker::provenance::{row_sources, RowSources};
use tracker::recap::{recap, Recap};
use tracker::report_bundle::{self, BundleInput};
use tracker::setup::{check_setup, client_config_path, default_log_config_path};
use tracker::stats::Stats;
use tracker::store::{GameKey, ParserStamp, Store};
use tracker::Watcher;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// The setup is checked again every this many polls (the user may fix it
/// while the app runs).
const SETUP_CHECK_EVERY: u32 = 30;
const GAMES_CHANGED: &str = "games-changed";
const STATUS_CHANGED: &str = "status-changed";
/// Lobby tribes entered by hand changed, or the game in progress did (T-303).
const LOBBY_TRIBES_CHANGED: &str = "lobby-tribes-changed";
/// A game the history did not have before was saved: the window shows its recap.
const GAME_FINISHED: &str = "game-finished";
/// Added by the Windows start-up entry: open in the tray, not on screen.
const MINIMIZED_ARG: &str = "--minimized";
/// `--data-dir <folder>`: keep the history (and the upload settings) there
/// instead of %APPDATA%\TavernLedger, e.g. to try a build without touching
/// the real history.
const DATA_DIR_ARG: &str = "--data-dir";
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
    /// What to change in `log.config` / `client.config` (T-106); empty when
    /// the game is set up to write a complete Power.log.
    setup: Vec<String>,
}

#[derive(Clone, Serialize)]
struct GameRow {
    session: String,
    index: u64,
    report: serde_json::Value,
    /// The parser that wrote the record; null is "unknown version".
    parser: Option<ParserStamp>,
    /// Where each value of the row comes from (T-109).
    sources: RowSources,
}

/// Shared with the window. The watcher itself lives only in the tracker
/// thread, so reading a big log never blocks the window.
#[derive(Default)]
struct AppState {
    games: Mutex<Vec<GameRow>>,
    status: Mutex<Status>,
    /// Lobby tribes entered by hand (T-303). `None` when this window does not
    /// own the history (read-only) or the file could not be opened.
    entries: Mutex<Option<Entries>>,
    entries_problem: Mutex<Option<String>>,
    /// The Battlegrounds game being played now, not saved yet.
    in_progress: Mutex<Option<GameKey>>,
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
    let games = state.games.lock().unwrap_or_else(|e| e.into_inner());
    let entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    stats_of(&games, entries.as_ref())
}

fn stats_of(rows: &[GameRow], entries: Option<&Entries>) -> Stats {
    tracker::stats::compute_with_entered(rows.iter().map(|row| {
        let key = GameKey {
            session: row.session.clone(),
            index: row.index,
        };
        (&row.report, entries.and_then(|e| e.entry(&key)))
    }))
}

/// The recap of one game in the history: results, health, tribes and
/// warnings, each with its source (T-202, T-203). The window renders it.
#[tauri::command]
fn game_recap(
    state: State<'_, Arc<AppState>>,
    session: String,
    index: u64,
) -> Result<Recap, String> {
    recap_of(&state, &session, index)
}

fn recap_of(state: &AppState, session: &str, index: u64) -> Result<Recap, String> {
    let games = state.games.lock().unwrap_or_else(|e| e.into_inner());
    let entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    let key = GameKey {
        session: session.to_string(),
        index,
    };
    games
        .iter()
        .find(|g| g.session == session && g.index == index)
        .map(|row| {
            recap(&row.report).with_entered_tribes(entries.as_ref().and_then(|e| e.entry(&key)))
        })
        .ok_or_else(|| "That game is not in the history.".to_string())
}

/// The lobby tribes entered by hand, the fixed list to pick from and the game
/// in progress (T-303). The window renders it and decides nothing.
#[tauri::command]
fn lobby_tribes_view(state: State<'_, Arc<AppState>>) -> LobbyView {
    lobby_view_of(&state)
}

/// Enters, clears or moves lobby tribes. Wrong input is refused with words,
/// never saved; returns the new view.
#[tauri::command]
fn edit_lobby_tribes(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    action: Action,
) -> Result<LobbyView, String> {
    edit_lobby_tribes_of(&state, action)?;
    let _ = app.emit(LOBBY_TRIBES_CHANGED, ());
    Ok(lobby_view_of(&state))
}

fn lobby_view_of(state: &AppState) -> LobbyView {
    let in_progress = state
        .in_progress
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let problem = state
        .entries_problem
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    lobby_tribes::view(entries.as_ref(), problem, in_progress.as_ref())
}

fn edit_lobby_tribes_of(state: &AppState, action: Action) -> Result<(), String> {
    let mut known: BTreeSet<GameKey> = state
        .games
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(|g| GameKey {
            session: g.session.clone(),
            index: g.index,
        })
        .collect();
    known.extend(
        state
            .in_progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
    );
    let mut entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    let Some(entries) = entries.as_mut() else {
        let why = state
            .entries_problem
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        return Err(why.unwrap_or_else(|| {
            "Entering tribes is not available: this window does not own the history.".into()
        }));
    };
    lobby_tribes::apply(entries, action, |key| known.contains(key)).map_err(|e| e.to_string())
}

/// The "report a problem" bundle of one game, as the text of the file. The
/// window shows it before anything is saved; nothing is sent (T-110).
#[tauri::command]
fn preview_problem_report(
    state: State<'_, Arc<AppState>>,
    session: String,
    index: u64,
) -> Result<String, String> {
    problem_report_text(&state, &session, index)
}

/// Builds the bundle again (never trusts text from the window) and saves it
/// in the Downloads folder. Returns where it went.
#[tauri::command]
fn save_problem_report(
    state: State<'_, Arc<AppState>>,
    session: String,
    index: u64,
) -> Result<String, String> {
    let text = problem_report_text(&state, &session, index)?;
    let dir = report_bundle::downloads_dir()
        .ok_or("The Downloads folder was not found, so nothing was saved.")?;
    let name = report_bundle::file_name(&session, index);
    report_bundle::save_in(&dir, &name, &text)
        .map(|path| path.display().to_string())
        .map_err(|e| format!("Could not save the report ({:?}).", e.kind()))
}

fn problem_report_text(state: &AppState, session: &str, index: u64) -> Result<String, String> {
    let games = state.games.lock().unwrap_or_else(|e| e.into_inner());
    let row = games
        .iter()
        .find(|g| g.session == session && g.index == index)
        .ok_or("That game is not in the history.")?;
    let logs_dir = state
        .status
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .logs_dir
        .clone();
    let client_config = logs_dir
        .as_deref()
        .and_then(|dir| client_config_path(Path::new(dir)));
    let setup = check_setup(
        default_log_config_path().as_deref(),
        client_config.as_deref(),
    );
    let input = BundleInput {
        session: &row.session,
        index: row.index,
        report: &row.report,
        parser: row.parser.as_ref(),
        app_version: env!("CARGO_PKG_VERSION"),
        setup: &setup,
        created_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    };
    report_bundle::build(&input)
        .map(|bundle| report_bundle::to_text(&bundle))
        .map_err(|refusal| {
            format!("No report was made: the game holds something that must not leave the app ({refusal}).")
        })
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
            parser: store.parser(key).cloned(),
            sources: row_sources(&key.session, report),
        })
        .collect();
    *state.games.lock().unwrap_or_else(|e| e.into_inner()) = rows;
    let _ = app.emit(GAMES_CHANGED, ());
}

/// Which of the games just saved are new to the history. A game read again
/// (a newer parser adds fields) is saved but is not a game that just ended.
fn new_games<'a>(
    known: &mut BTreeSet<GameKey>,
    saved: impl IntoIterator<Item = &'a GameKey>,
) -> Vec<GameKey> {
    saved
        .into_iter()
        .filter(|key| known.insert((*key).clone()))
        .cloned()
        .collect()
}

#[derive(Clone, Serialize)]
struct GameFinished {
    session: String,
    index: u64,
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

/// Reads both config files (read-only) and puts what to fix in the status.
fn refresh_setup(app: &AppHandle, state: &AppState, logs_dir: &Path) {
    let log_config = default_log_config_path();
    let client_config = client_config_path(logs_dir);
    let messages = check_setup(log_config.as_deref(), client_config.as_deref()).messages();
    set_status(app, state, |s| s.setup = messages);
}

/// Opens the history and follows the log until the app quits.
fn run_tracker(app: AppHandle, state: Arc<AppState>) {
    let Some(logs_dir) = find_logs_dir() else {
        let msg = "Hearthstone's Logs folder was not found. Is the game installed?";
        return problem(&app, &state, msg.into());
    };
    let data_dir = data_dir_from(std::env::args()).unwrap_or_else(tracker::store::default_dir);
    // Held while this thread runs: only one process writes the history.
    let _lock = match HistoryLock::acquire(&data_dir) {
        Ok(lock) => lock,
        Err(LockError::Busy) => return show_read_only(&app, &state, &data_dir),
        Err(e) => return problem(&app, &state, format!("Cannot use the game history: {e}.")),
    };
    match Entries::open(&data_dir) {
        Ok(entries) => *state.entries.lock().unwrap_or_else(|e| e.into_inner()) = Some(entries),
        Err(e) => {
            let msg = format!(
                "Cannot open the entered tribes ({:?}); they are off.",
                e.kind()
            );
            *state
                .entries_problem
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(msg);
        }
    }
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
    upload::start(&app, data_dir);
    let mut known: BTreeSet<GameKey> = store.games().map(|(key, _)| key.clone()).collect();
    let mut watcher = Watcher::new(&logs_dir, store);
    let mut starts = GameStarts::default();

    let mut polls_until_setup_check: u32 = 0;
    loop {
        if polls_until_setup_check == 0 {
            refresh_setup(&app, &state, &logs_dir);
            polls_until_setup_check = SETUP_CHECK_EVERY;
        }
        polls_until_setup_check -= 1;
        // A bug must not stop the tracker silently while the window looks fine.
        let Ok(result) = catch_unwind(AssertUnwindSafe(|| watcher.poll())) else {
            let msg = "The tracker stopped after an internal error. Restart the app; your history is safe.";
            return problem(&app, &state, msg.into());
        };
        track_game_in_progress(&app, &state, &mut starts, watcher.in_progress());
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
                    upload::kick(&app);
                    // Only the newest: games found at start-up (played while the
                    // app was closed) must not each pop up a recap.
                    if let Some(key) = new_games(&mut known, saved.iter().map(|s| &s.key)).pop() {
                        let finished = GameFinished {
                            session: key.session,
                            index: key.index,
                        };
                        let _ = app.emit(GAME_FINISHED, finished);
                    }
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

fn data_dir_from(args: impl IntoIterator<Item = String>) -> Option<PathBuf> {
    let mut args = args.into_iter();
    args.by_ref().find(|a| a == DATA_DIR_ARG)?;
    args.next().filter(|d| !d.is_empty()).map(PathBuf::from)
}

/// Keeps the game in progress up to date for the window and gives a waiting
/// lobby-tribes entry to a game that just started (T-303).
fn track_game_in_progress(
    app: &AppHandle,
    state: &AppState,
    starts: &mut GameStarts,
    now: Option<GameKey>,
) {
    let mut changed = {
        let mut current = state.in_progress.lock().unwrap_or_else(|e| e.into_inner());
        let changed = *current != now;
        *current = now.clone();
        changed
    };
    let mut entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(entries) = entries.as_mut() {
        match starts.observe(entries, now) {
            Ok(attached) => changed |= attached.is_some(),
            Err(e) => {
                *state
                    .entries_problem
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(e.to_string());
                changed = true;
            }
        }
    }
    drop(entries);
    if changed {
        let _ = app.emit(LOBBY_TRIBES_CHANGED, ());
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
        .manage(upload::Upload::default())
        .invoke_handler(tauri::generate_handler![
            list_games,
            game_stats,
            status,
            game_recap,
            preview_problem_report,
            save_problem_report,
            lobby_tribes_view,
            edit_lobby_tribes,
            upload::upload_status,
            upload::upload_set_enabled,
            upload::upload_sign_in,
            upload::upload_cancel_sign_in,
            upload::upload_sign_out,
        ])
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
            parser: None,
            sources: row_sources("", &serde_json::Value::Null),
            report: serde_json::json!({
                "status": "ok", "game_type": game_type, "hero": "H", "final_place": place,
            }),
        };
        let stats = stats_of(
            &[row("GT_BATTLEGROUNDS", 3), row("GT_BATTLEGROUNDS_DUO", 1)],
            None,
        );
        let solo = &stats.modes[0];
        assert_eq!(
            (solo.mode.as_str(), solo.totals.games),
            ("GT_BATTLEGROUNDS", 1)
        );
        assert_eq!(solo.totals.average_place, Some(3.0));
        assert_eq!(stats.modes[1].totals.wins, Some(1));
    }

    fn state_with(report: serde_json::Value) -> AppState {
        let state = AppState::default();
        state.games.lock().unwrap().push(GameRow {
            session: "Hearthstone_2026_10_09_00_00_00".into(),
            index: 1,
            sources: row_sources("Hearthstone_2026_10_09_00_00_00", &report),
            parser: None,
            report,
        });
        state
    }

    fn state_with_entries(report: serde_json::Value) -> (AppState, std::path::PathBuf) {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("tavern-ledger-desktop-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let state = state_with(report);
        *state.entries.lock().unwrap() = Some(Entries::open(&dir).unwrap());
        (state, dir)
    }

    fn five(list: [&str; 5]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tribes_entered_for_a_listed_game_show_in_its_recap_and_the_stats() {
        let session = "Hearthstone_2026_10_09_00_00_00";
        let (state, _dir) = state_with_entries(serde_json::json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "H", "final_place": 2,
            "shop_tribes": {"MURLOC": 3},
        }));
        let tribes = five(["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]);
        edit_lobby_tribes_of(
            &state,
            Action::Set {
                session: session.into(),
                index: 1,
                tribes: tribes.clone(),
            },
        )
        .unwrap();
        let r = recap_of(&state, session, 1).unwrap();
        assert_eq!(r.entered_tribes, tribes);
        assert_eq!(r.tribes[0].tribe, "MURLOC", "the tavern list is untouched");
        let games = state.games.lock().unwrap();
        let entries = state.entries.lock().unwrap();
        let stats = stats_of(&games, entries.as_ref());
        assert_eq!(stats.modes[0].games_with_entered_tribes, 1);
        assert_eq!(stats.modes[0].games_with_tribes, 1);
    }

    #[test]
    fn tribes_cannot_be_entered_for_a_game_that_does_not_exist() {
        let (state, _dir) = state_with_entries(serde_json::json!({"status": "ok"}));
        let err = edit_lobby_tribes_of(
            &state,
            Action::Set {
                session: "other".into(),
                index: 1,
                tribes: five(["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]),
            },
        )
        .unwrap_err();
        assert_eq!(err, "That game is not in the history.");
    }

    #[test]
    fn the_game_in_progress_takes_entered_tribes_before_it_is_saved() {
        let (state, _dir) = state_with_entries(serde_json::json!({"status": "ok"}));
        let now = GameKey {
            session: "Hearthstone_2026_10_09_00_00_00".into(),
            index: 2,
        };
        *state.in_progress.lock().unwrap() = Some(now.clone());
        let tribes = five(["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]);
        let set = Action::Set {
            session: now.session.clone(),
            index: 2,
            tribes: tribes.clone(),
        };
        edit_lobby_tribes_of(&state, set).unwrap();
        assert_eq!(lobby_view_of(&state).in_progress_tribes, Some(tribes));
    }

    #[test]
    fn without_the_history_lock_tribes_cannot_be_entered_and_the_view_says_so() {
        let state = state_with(serde_json::json!({"status": "ok"}));
        let err = edit_lobby_tribes_of(&state, Action::ClearPending).unwrap_err();
        assert!(err.contains("not available"));
        assert!(!lobby_view_of(&state).available);
    }

    #[test]
    fn the_problem_report_of_a_listed_game_is_json_text() {
        let state = state_with(serde_json::json!({"status": "ok", "build": 1}));
        let text = problem_report_text(&state, "Hearthstone_2026_10_09_00_00_00", 1).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["game"]["build"], 1);
        assert_eq!(parsed["app_version"], env!("CARGO_PKG_VERSION"));
    }

    fn key(session: &str, index: u64) -> GameKey {
        GameKey {
            session: session.into(),
            index,
        }
    }

    #[test]
    fn only_games_new_to_the_history_count_as_just_finished() {
        let mut known = BTreeSet::new();
        known.insert(key("S1", 1));
        let batch = [key("S1", 1), key("S1", 2), key("S2", 1)];
        let fresh = new_games(&mut known, &batch);
        let names: Vec<_> = fresh
            .iter()
            .map(|k| (k.session.as_str(), k.index))
            .collect();
        assert_eq!(names, [("S1", 2), ("S2", 1)]);
        // Saved again later (read again): no longer new.
        assert!(new_games(&mut known, &batch).is_empty());
    }

    #[test]
    fn the_recap_of_a_listed_game_comes_from_its_report() {
        let state = state_with(serde_json::json!({"status": "ok", "hero": "H"}));
        let r = recap_of(&state, "Hearthstone_2026_10_09_00_00_00", 1).unwrap();
        assert_eq!(r.hero.value.map(|h| h.id), Some("H".to_string()));
    }

    #[test]
    fn a_game_that_is_not_listed_has_no_recap() {
        let state = state_with(serde_json::json!({}));
        assert!(recap_of(&state, "Hearthstone_2026_10_09_00_00_00", 2).is_err());
        assert!(recap_of(&state, "other", 1).is_err());
    }

    #[test]
    fn a_game_that_is_not_listed_has_no_report() {
        let state = state_with(serde_json::json!({}));
        assert!(problem_report_text(&state, "Hearthstone_2026_10_09_00_00_00", 2).is_err());
    }

    #[test]
    fn a_game_with_a_name_in_it_gives_no_report_and_the_message_hides_it() {
        let state = state_with(serde_json::json!({"warnings": ["Someone#4321 left"]}));
        let err = problem_report_text(&state, "Hearthstone_2026_10_09_00_00_00", 1).unwrap_err();
        assert!(err.starts_with("No report was made"));
        assert!(!err.contains("Someone"));
    }

    #[test]
    fn starts_hidden_only_when_windows_starts_it() {
        assert!(starts_hidden(args(&["desktop.exe", MINIMIZED_ARG])));
        assert!(!starts_hidden(args(&["desktop.exe"])));
        assert!(!starts_hidden(args(&["desktop.exe", "--minimized-not"])));
    }

    #[test]
    fn the_data_folder_can_be_chosen_on_the_command_line() {
        assert_eq!(
            data_dir_from(args(&["desktop.exe", DATA_DIR_ARG, r"C:\Temp\tl"])),
            Some(PathBuf::from(r"C:\Temp\tl"))
        );
        assert_eq!(data_dir_from(args(&["desktop.exe"])), None);
        assert_eq!(data_dir_from(args(&["desktop.exe", DATA_DIR_ARG])), None);
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
