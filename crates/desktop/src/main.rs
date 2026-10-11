//! Tavern Ledger desktop app: follows Power.log in the background and shows
//! the local game history. Reads the game's log files (D-004) and, only if
//! the user turns it on, two values from the game itself, read-only: the
//! lobby's tribes and the player's own rating (D-063, `game_memory`).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod card_data;
mod foreground;
mod game_memory;
mod hover;
mod leaderboard;
mod over_game_guard;
mod overlay;
mod overlay_layout;
mod overlay_window;
mod screen_fit;
mod settings;
#[cfg(test)]
mod ui_checks;
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
use tauri::{App, AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tracker::dev_reconnect::Reconnects;
use tracker::discover::find_logs_dir;
use tracker::live::{live_game, LiveGame};
use tracker::lobby_tribes::{self, Action, Entries, GameStarts, LobbyView};
use tracker::lock::{HistoryLock, LockError};
use tracker::provenance::{row_sources, RowSources};
use tracker::recap::{recap, Recap};
use tracker::report_bundle::{self, BundleInput};
use tracker::scene::Scene;
use tracker::setup::{check_setup, client_config_path, default_log_config_path};
use tracker::stats::curious::{CardNames, Curious};
use tracker::stats::{HeroLinks, Stats};
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
/// The game in progress, as the overlay shows it, changed (T-301).
const LIVE_CHANGED: &str = "live-changed";
/// Added by the Windows start-up entry: open in the tray, not on screen.
const MINIMIZED_ARG: &str = "--minimized";
/// `--data-dir <folder>`: keep the history (and the upload settings) there
/// instead of %APPDATA%\TavernLedger, e.g. to try a build without touching
/// the real history.
const DATA_DIR_ARG: &str = "--data-dir";
/// `--logs-dir <folder>`: follow the logs there instead of the game's own
/// Logs folder, e.g. a replayed log (development only, T-D02).
const LOGS_DIR_ARG: &str = "--logs-dir";
/// Development only: the overlay is on, shows whatever window is in front and
/// does not save the on/off choice, so a replayed game can be checked without playing.
const OVERLAY_DEV_ARG: &str = "--overlay-dev";
/// Development only: starts with the layout unlocked, so a test can drive the
/// mouse without opening the tray menu. A normal start is always locked.
const OVERLAY_UNLOCK_ARG: &str = "--overlay-unlock";
const MENU_SHOW: &str = "show";
const MENU_OVERLAY: &str = "overlay";
const MENU_UNLOCK: &str = "overlay-unlock";
const MENU_RESET_LAYOUT: &str = "overlay-reset-layout";
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
    /// Set when the reconnect dev tool's file exists but could not be read
    /// whole (D-043); never set for a user who did not run the tool.
    dev_reconnects: Option<String>,
    /// Where the game client is, from LoadingScreen.log (T-314), in words;
    /// None when that log is off or says nothing yet.
    scene: Option<String>,
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
    /// Played with the reconnect dev tool (D-043): marked in the history and
    /// left out of the stats and the upload.
    dev_reconnect: bool,
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
    /// The game in progress as the overlay shows it, without the tribes
    /// entered by hand (those are added when it is read).
    live: Mutex<Option<LiveGame>>,
    /// Where the game client is (T-314), for the overlay's start.
    scene: Mutex<Option<Scene>>,
    overlay: overlay::Overlay,
    /// The switch and the values read from the game (D-063); off by default.
    game_memory: game_memory::GameMemory,
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
fn game_stats(state: State<'_, Arc<AppState>>, cards: State<'_, card_data::Cards>) -> Stats {
    let links = card_data::hero_links(&cards);
    let games = state.games.lock().unwrap_or_else(|e| e.into_inner());
    let entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    stats_of(&games, entries.as_ref(), &links)
}

/// Games played with the reconnect dev tool are not counted (D-043). Skins
/// are grouped by the game's own link looked up in `links` (T-214); the
/// D-019 name rule only where that link is missing, and labelled so.
fn stats_of(rows: &[GameRow], entries: Option<&Entries>, links: &HeroLinks) -> Stats {
    let counted = rows.iter().filter(|row| !row.dev_reconnect);
    tracker::stats::compute_with_links(
        counted.map(|row| {
            let key = GameKey {
                session: row.session.clone(),
                index: row.index,
            };
            (&row.report, entries.and_then(|e| e.entry(&key)))
        }),
        links,
    )
}

/// Fun totals over every counted game (T-216), each with its source and
/// the games it covers. Card names (for golden Brann and Titus) come from
/// the card data; without it that stat says so.
#[tauri::command]
fn curious_stats(state: State<'_, Arc<AppState>>, cards: State<'_, card_data::Cards>) -> Curious {
    let store = card_data::store_with_data(&cards);
    let games = state.games.lock().unwrap_or_else(|e| e.into_inner());
    let name = |id: &str| store.and_then(|s| s.name(id));
    let names: Option<CardNames<'_>> = store.map(|_| &name as _);
    curious_of(&games, names)
}

/// Games played with the reconnect dev tool are not counted here either.
fn curious_of(rows: &[GameRow], names: Option<CardNames<'_>>) -> Curious {
    let counted = rows.iter().filter(|row| !row.dev_reconnect);
    tracker::stats::curious::compute(counted.map(|row| &row.report), names)
}

/// The recap of one game in the history: results, health, tribes and
/// warnings, each with its source (T-202, T-203). The window renders it.
/// Trinkets and the Old God are named by the card data when it is there.
#[tauri::command]
fn game_recap(
    state: State<'_, Arc<AppState>>,
    cards: State<'_, card_data::Cards>,
    session: String,
    index: u64,
) -> Result<Recap, String> {
    let store = card_data::store_with_data(&cards);
    let card_of = |dbf: i64| store.and_then(|s| s.card_of_dbf(dbf));
    recap_of(&state, &session, index).map(|r| r.with_dbf_cards(&card_of))
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

/// What the overlay shows: the game in progress, or nothing when no
/// Battlegrounds game is on (T-301). The window renders it and decides nothing.
#[tauri::command]
fn overlay_state(
    state: State<'_, Arc<AppState>>,
    cards: State<'_, card_data::Cards>,
) -> Option<LiveGame> {
    let pool = card_data::pool_cards(&cards);
    let store = card_data::store_with_data(&cards);
    let card_of = |dbf: i64| store.and_then(|s| s.card_of_dbf(dbf));
    let shown = overlay_state_of(&state)?.with_possible(&pool);
    // Tribes from the game that the tavern contradicted stay dropped for the
    // rest of the game (D-063).
    if shown.game_tribes.note.is_some() {
        if let Some(key) = state
            .in_progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            state.game_memory.refuse(key);
        }
    }
    Some(shown.with_trinket_cards(card_of))
}

/// Where the game client is (T-314): the overlay says "game loading" while
/// a Battlegrounds game loads, before the log has the game.
#[tauri::command]
fn overlay_scene(state: State<'_, Arc<AppState>>) -> Option<Scene> {
    *state.scene.lock().unwrap_or_else(|e| e.into_inner())
}

fn overlay_state_of(state: &AppState) -> Option<LiveGame> {
    let live = state
        .live
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()?;
    let key = state
        .in_progress
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let entries = state.entries.lock().unwrap_or_else(|e| e.into_inner());
    let entered = key.as_ref().and_then(|key| entries.as_ref()?.entry(key));
    let (game_tribes, reader) = state.game_memory.tribes_for(key.as_ref());
    Some(
        live.with_entered_tribes(entered)
            .with_game_tribes(game_tribes.as_deref(), reader),
    )
}

/// The switch for reading from the game (D-063), its status and the own
/// rating read, for the settings page.
#[tauri::command]
fn game_memory_status(state: State<'_, Arc<AppState>>) -> game_memory::View {
    state.game_memory.view()
}

#[tauri::command]
fn game_memory_set_enabled(state: State<'_, Arc<AppState>>, enabled: bool) -> game_memory::View {
    // A failed save is in the view's warning; the choice holds for this run.
    let _ = state.game_memory.set_enabled(enabled);
    state.game_memory.view()
}

/// Asks the game for its values when the switch is on (D-063). A bug in the
/// reader turns it off for this run instead of stopping the tracker.
fn read_from_game(app: &AppHandle, state: &AppState) {
    let game = state
        .in_progress
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let scene = *state.scene.lock().unwrap_or_else(|e| e.into_inner());
    let build = state
        .live
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|live| live.build);
    let tick = || {
        state
            .game_memory
            .tick(&game_memory::GameReader, game.as_ref(), build, scene)
    };
    if catch_unwind(AssertUnwindSafe(tick)).is_err() {
        state.game_memory.stop_for_run();
        let msg = "Reading from the game stopped after an internal error; it is off until the app restarts.";
        set_status(app, state, |s| s.notice = Some(msg.into()));
    }
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

/// The game in progress as the overlay shows it.
fn live_now(watcher: &mut Watcher) -> Option<LiveGame> {
    let lobby = watcher.live_lobby();
    let next = watcher.live_next_opponent();
    let report = serde_json::to_value(watcher.live_report()?).ok()?;
    Some(
        live_game(&report)
            .with_leaderboard(&lobby)
            .with_next_opponent(next),
    )
}

/// Keeps the scene up to date for the overlay and tells it when it changed;
/// a scene file that cannot be read is said in the status, never silent.
fn publish_scene(
    app: &AppHandle,
    state: &AppState,
    now: Option<Scene>,
    error: Option<std::io::ErrorKind>,
) {
    let changed = {
        let mut current = state.scene.lock().unwrap_or_else(|e| e.into_inner());
        let changed = *current != now;
        *current = now;
        changed
    };
    if changed {
        let _ = app.emit(LIVE_CHANGED, ());
    }
    let words = match error {
        Some(kind) => Some(format!(
            "game screen unknown (LoadingScreen.log could not be read: {kind:?})"
        )),
        None => now.map(|n| n.describe().to_string()),
    };
    set_status(app, state, |s| s.scene = words);
}

/// Keeps the overlay's game up to date and tells it, only if it changed.
fn publish_live(app: &AppHandle, state: &AppState, now: Option<LiveGame>) {
    let changed = {
        let mut current = state.live.lock().unwrap_or_else(|e| e.into_inner());
        let changed = *current != now;
        *current = now;
        changed
    };
    if changed {
        let _ = app.emit(LIVE_CHANGED, ());
    }
}

fn publish_games(app: &AppHandle, state: &AppState, store: &Store) {
    let reconnects = store
        .path()
        .parent()
        .map(Reconnects::load)
        .unwrap_or_default();
    let rows = rows_of(store, &reconnects);
    *state.games.lock().unwrap_or_else(|e| e.into_inner()) = rows;
    set_status(app, state, |s| s.dev_reconnects = reconnects.warning);
    let _ = app.emit(GAMES_CHANGED, ());
}

fn rows_of(store: &Store, reconnects: &Reconnects) -> Vec<GameRow> {
    let marked = reconnects.marked(store);
    store
        .games()
        .map(|(key, report)| GameRow {
            session: key.session.clone(),
            index: key.index,
            report: report.clone(),
            parser: store.parser(key).cloned(),
            sources: row_sources(&key.session, report),
            dev_reconnect: marked.contains(key),
        })
        .collect()
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
    let report = check_setup(log_config.as_deref(), client_config.as_deref());
    // What to fix first, then what is optional (T-314).
    let messages: Vec<String> = report
        .messages()
        .into_iter()
        .chain(report.notes())
        .collect();
    set_status(app, state, |s| s.setup = messages);
}

/// Opens the history and follows the log until the app quits.
fn run_tracker(app: AppHandle, state: Arc<AppState>) {
    let Some(logs_dir) = path_from(std::env::args(), LOGS_DIR_ARG).or_else(find_logs_dir) else {
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
            // The overlay must not keep showing a game nobody follows any more.
            publish_live(&app, &state, None);
            let msg = "The tracker stopped after an internal error. Restart the app; your history is safe.";
            return problem(&app, &state, msg.into());
        };
        // The overlay is on its own guard: a bug in it must not cost a saved game.
        match catch_unwind(AssertUnwindSafe(|| live_now(&mut watcher))) {
            Ok(live) => publish_live(&app, &state, live),
            Err(_) => {
                publish_live(&app, &state, None);
                let msg = "The overlay stopped after an internal error. Restart the app; your history is safe.";
                set_status(&app, &state, |s| s.notice = Some(msg.into()));
            }
        }
        publish_scene(&app, &state, watcher.scene(), watcher.scene_error());
        track_game_in_progress(&app, &state, &mut starts, watcher.in_progress());
        read_from_game(&app, &state);
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
    path_from(args, DATA_DIR_ARG)
}

/// The folder that follows `flag` on the command line, if any.
fn path_from(args: impl IntoIterator<Item = String>, flag: &str) -> Option<PathBuf> {
    let mut args = args.into_iter();
    args.by_ref().find(|a| a == flag)?;
    args.next().filter(|d| !d.is_empty()).map(PathBuf::from)
}

fn overlay_dev(args: impl IntoIterator<Item = String>) -> bool {
    args.into_iter().any(|a| a == OVERLAY_DEV_ARG)
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

/// Turns the Windows start-up entry on or off; the menu and the Settings
/// page show the real state.
fn toggle_autostart(app: &AppHandle) {
    let settings = app.state::<Arc<settings::Settings>>();
    let state = app.state::<Arc<AppState>>();
    let on = !app.autolaunch().is_enabled().unwrap_or(false);
    if settings::set_autostart(app, &settings, &state, on).is_err() {
        // The window is usually hidden in the tray: show it so the message is seen.
        show_window(app);
    }
}

/// Turns the overlay on or off; the menu and the window show the real state.
fn toggle_overlay(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>();
    let result = overlay::set_enabled(app, &state, !state.overlay.enabled());
    let failed = result.is_err();
    set_status(app, &state, |s| s.notice = result.err());
    if failed {
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
    // Built with the English words; `keep_tray` puts them in the app's language.
    let words = settings::tray_words(settings::Language::En);
    let label = if available {
        words.autostart
    } else {
        words.autostart_unavailable
    };
    let autostart =
        CheckMenuItem::with_id(app, MENU_AUTOSTART, label, available, enabled, None::<&str>)?;
    let state = app.state::<Arc<AppState>>();
    let overlay = CheckMenuItem::with_id(
        app,
        MENU_OVERLAY,
        words.overlay,
        true,
        state.overlay.enabled(),
        None::<&str>,
    )?;
    let unlock = CheckMenuItem::with_id(app, MENU_UNLOCK, words.unlock, true, false, None::<&str>)?;
    overlay::keep_enable_item(&state, overlay.clone());
    overlay::keep_unlock_item(&state, unlock.clone());
    let reset_layout = MenuItem::with_id(
        app,
        MENU_RESET_LAYOUT,
        words.reset_layout,
        true,
        None::<&str>,
    )?;
    let show = MenuItem::with_id(app, MENU_SHOW, words.show, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, words.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show, &overlay, &unlock, &reset_layout, &autostart, &quit],
    )?;
    let items = (
        show.clone(),
        overlay.clone(),
        unlock.clone(),
        reset_layout.clone(),
    );
    let kept_autostart = autostart.clone();
    let quit_item = quit.clone();
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip(words.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event: MenuEvent| match event.id().as_ref() {
            MENU_SHOW => show_window(app),
            MENU_AUTOSTART => toggle_autostart(app),
            MENU_OVERLAY => toggle_overlay(app),
            MENU_UNLOCK => {
                let state = app.state::<Arc<AppState>>();
                overlay::set_unlocked(app, &state, !state.overlay.unlocked());
            }
            MENU_RESET_LAYOUT => {
                let state = app.state::<Arc<AppState>>();
                overlay::change(app, &state, |s, _| s.reset_layout());
            }
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
    let icon = tray.build(app)?;
    let (show, overlay, unlock, reset_layout) = items;
    app.state::<Arc<settings::Settings>>()
        .keep_tray(settings::TrayItems {
            show,
            overlay,
            unlock,
            reset_layout,
            autostart: kept_autostart,
            autostart_available: available,
            quit: quit_item,
            icon,
        });
    Ok(())
}

fn main() {
    let state = Arc::new(AppState::default());
    let app_settings = Arc::new(settings::Settings::default());
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .manage(state.clone())
        .manage(app_settings.clone())
        .manage(upload::Upload::default())
        .manage(card_data::Cards::default())
        .register_asynchronous_uri_scheme_protocol(card_data::SCHEME, card_data::protocol)
        .invoke_handler(tauri::generate_handler![
            list_games,
            game_stats,
            curious_stats,
            status,
            game_recap,
            preview_problem_report,
            save_problem_report,
            lobby_tribes_view,
            edit_lobby_tribes,
            overlay_state,
            overlay_scene,
            overlay::overlay_settings,
            overlay::overlay_save_layout,
            overlay::overlay_set_theme,
            overlay::overlay_set_detail,
            overlay::overlay_set_opacity,
            overlay::overlay_set_shown,
            overlay::overlay_reset_layout,
            overlay::overlay_set_unlocked,
            overlay::overlay_set_enabled,
            overlay::overlay_save_leaderboard,
            overlay::overlay_reset_leaderboard,
            upload::upload_status,
            upload::upload_set_enabled,
            upload::upload_sign_in,
            upload::upload_cancel_sign_in,
            upload::upload_sign_out,
            card_data::card_names,
            card_data::card_data_status,
            game_memory_status,
            game_memory_set_enabled,
            settings::app_settings,
            settings::app_set_language,
            settings::app_set_system_language,
            settings::app_autostart,
            settings::app_set_autostart,
        ])
        .on_window_event(|window, event| {
            // Closing the window hides it; "Quit" in the tray menu exits.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let data_dir =
                data_dir_from(std::env::args()).unwrap_or_else(tracker::store::default_dir);
            let loaded = state.overlay.load(&data_dir, overlay_dev(std::env::args()));
            let game_memory_loaded = state.game_memory.load(&data_dir);
            let settings_loaded = app_settings.load(&data_dir);
            card_data::start(app.handle(), &data_dir);
            setup_tray(app)?;
            // A test build never shows its overlay over the user's game (T-336).
            let guarded = over_game_guard::guarded(cfg!(debug_assertions), std::env::args());
            overlay_window::follow(app.handle().clone(), state.clone(), guarded);
            hover::watch(app.handle().clone(), state.clone());
            let prepared = overlay_window::prepare(app.handle()).map_err(|e| {
                state.overlay.disable();
                format!("The overlay could not be set up ({e}); it is off.")
            });
            let unlock_asked = std::env::args().any(|a| a == OVERLAY_UNLOCK_ARG);
            if overlay_dev(std::env::args()) && unlock_asked {
                overlay::set_unlocked(app.handle(), &state, true);
            }
            if let Some(message) = loaded
                .err()
                .or(prepared.err())
                .or(settings_loaded.err())
                .or(game_memory_loaded.err())
            {
                set_status(app.handle(), &state, |s| s.notice = Some(message));
            }
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
            dev_reconnect: false,
            report: serde_json::json!({
                "status": "ok", "game_type": game_type, "hero": "H", "final_place": place,
            }),
        };
        let stats = stats_of(
            &[row("GT_BATTLEGROUNDS", 3), row("GT_BATTLEGROUNDS_DUO", 1)],
            None,
            &HeroLinks::new(),
        );
        let solo = &stats.modes[0];
        assert_eq!(
            (solo.mode.as_str(), solo.totals.games),
            ("GT_BATTLEGROUNDS", 1)
        );
        assert_eq!(solo.totals.average_place, Some(3.0));
        assert_eq!(stats.modes[1].totals.wins, Some(1));
    }

    fn plain_row(report: serde_json::Value, dev_reconnect: bool) -> GameRow {
        GameRow {
            session: "Hearthstone_2026_10_09_00_00_00".into(),
            index: 1,
            parser: None,
            sources: row_sources("", &serde_json::Value::Null),
            dev_reconnect,
            report,
        }
    }

    #[test]
    fn the_stats_group_skins_by_the_games_link_and_fall_back_to_the_name_rule() {
        let base = serde_json::json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "BG20_HERO_102", "final_place": 1,
        });
        // The D-019 rule cannot group this skin (another prefix); the game's link can.
        let skin = serde_json::json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "TB_BaconShop_HERO_102_SKIN_G",
            "final_place": 3, "skin_parents": {"TB_BaconShop_HERO_102_SKIN_G": 59_999},
        });
        // An older record without the link: the name rule, labelled.
        let old = serde_json::json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "BG20_HERO_102_SKIN_C", "final_place": 2,
        });
        let rows = [
            plain_row(base, false),
            plain_row(skin, false),
            plain_row(old, false),
        ];
        let links = HeroLinks::from([(59_999, "BG20_HERO_102".to_string())]);
        let stats = stats_of(&rows, None, &links);
        let heroes = &stats.modes[0].heroes;
        assert_eq!(heroes.len(), 1, "all three under one hero");
        assert_eq!(heroes[0].tally.games, 3);
        assert_eq!(heroes[0].variants_by_rule, ["BG20_HERO_102_SKIN_C"]);
        assert_eq!(heroes[0].grouping, tracker::provenance::Source::Inferred);
        // Without the card data the linked skin stays apart: nothing is guessed.
        let plain = stats_of(&rows, None, &HeroLinks::new());
        assert_eq!(plain.modes[0].heroes.len(), 2);
    }

    #[test]
    fn curious_stats_skip_reconnect_games_and_use_the_card_names() {
        let sold = |gold: i64| {
            serde_json::json!({
                "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "H", "final_place": 1,
                "shop": {"turns": [{
                    "turn": 1, "start_ms": 0, "end_ms": 1000, "tier": 1, "gold": 3,
                    "gold_spent": gold, "rolls": 0, "free_rolls": 0, "buys": [], "spell_buys": 0,
                    "sells": ["BG_GOLDEN_G"], "freezes": 0, "offers": [], "actions": 0,
                    "extra_gold": 0, "sell_gold": 1, "buy_gold": 0, "spell_gold": 0,
                    "free_refreshes": 0, "passes": null,
                }], "tier_ups": [], "actions": [], "start_ms": 0, "end_ms": 1000, "ended": true},
            })
        };
        let rows = [plain_row(sold(3), false), plain_row(sold(100), true)];
        let name = |id: &str| (id == "BG_GOLDEN_G").then(|| "Brann Bronzebeard".to_string());
        let curious = curious_of(&rows, Some(&name as CardNames<'_>));
        let stat = |key: &str| curious.stats.iter().find(|s| s.key == key).unwrap().clone();
        assert_eq!(curious.games, 1, "the reconnect game is not counted");
        assert_eq!(stat("gold_spent").value, Some(3));
        assert_eq!(stat("golden_brann_titus_sold").value, Some(1));
        let blind = curious_of(&rows, None);
        let stat = blind
            .stats
            .iter()
            .find(|s| s.key == "golden_brann_titus_sold")
            .unwrap();
        assert_eq!(stat.value, None, "no card data: unknown, not 0");
    }

    #[test]
    fn games_played_with_the_reconnect_dev_tool_are_listed_marked_and_not_counted() {
        use tracker::game_clock::Played;
        let dir = std::env::temp_dir().join(format!("tl-desktop-devrc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = Store::open(&dir).unwrap();
        // Game 1 at 2026-10-09 08:00-08:20 UTC, game 2 an hour later.
        for (index, from, place) in [(1, 1_791_532_800, 1), (2, 1_791_536_400, 7)] {
            let key = GameKey {
                session: "Hearthstone_2026_10_09_07_00_00".into(),
                index,
            };
            let report = serde_json::json!({
                "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "H", "final_place": place,
            });
            let played = Some(Played {
                from,
                to: from + 1200,
            });
            store.save_played(key, report, played).unwrap();
        }
        let reconnects = Reconnects::parse("{\"utc\": \"2026-10-09T08:05:00Z\"}\n");
        let rows = rows_of(&store, &reconnects);
        assert_eq!(
            rows.iter().map(|r| r.dev_reconnect).collect::<Vec<_>>(),
            [true, false]
        );
        let stats = stats_of(&rows, None, &HeroLinks::new());
        assert_eq!(stats.modes[0].totals.games, 1);
        assert_eq!(stats.modes[0].totals.average_place, Some(7.0));
        assert_eq!(stats.modes[0].heroes[0].tally.games, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn state_with(report: serde_json::Value) -> AppState {
        let state = AppState::default();
        state.games.lock().unwrap().push(GameRow {
            session: "Hearthstone_2026_10_09_00_00_00".into(),
            index: 1,
            sources: row_sources("Hearthstone_2026_10_09_00_00_00", &report),
            parser: None,
            dev_reconnect: false,
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
        let stats = stats_of(&games, entries.as_ref(), &HeroLinks::new());
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
    fn the_logs_folder_can_be_chosen_on_the_command_line_for_a_replay() {
        assert_eq!(
            path_from(
                args(&["desktop.exe", LOGS_DIR_ARG, r"C:\Temp\x\Logs"]),
                LOGS_DIR_ARG
            ),
            Some(PathBuf::from(r"C:\Temp\x\Logs"))
        );
        // One flag never reads the other's value.
        let both = args(&["d.exe", DATA_DIR_ARG, "data", LOGS_DIR_ARG, "logs"]);
        assert_eq!(
            path_from(both.clone(), LOGS_DIR_ARG),
            Some(PathBuf::from("logs"))
        );
        assert_eq!(data_dir_from(both), Some(PathBuf::from("data")));
        assert_eq!(path_from(args(&["d.exe"]), LOGS_DIR_ARG), None);
    }

    #[test]
    fn the_overlay_dev_flag_is_off_unless_given() {
        assert!(overlay_dev(args(&["desktop.exe", OVERLAY_DEV_ARG])));
        assert!(!overlay_dev(args(&["desktop.exe"])));
    }

    fn live_report(tribes: serde_json::Value) -> LiveGame {
        live_game(&serde_json::json!({
            "status": "incomplete", "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1,
            "hero": "H", "lobby": [{"player_id": 1, "hero": "H"}, {"player_id": 2, "hero": "O"}],
            "start_health": {"1": 40, "2": 40}, "shop_tribes": tribes, "rounds": [],
        }))
    }

    #[test]
    fn the_overlay_has_nothing_to_show_when_no_game_is_on() {
        assert!(overlay_state_of(&AppState::default()).is_none());
    }

    #[test]
    fn the_overlay_shows_the_tribes_entered_for_the_game_in_progress_apart_from_the_tavern_ones() {
        let (state, _dir) = state_with_entries(serde_json::json!({"status": "ok"}));
        let now = GameKey {
            session: "Hearthstone_2026_10_09_00_00_00".into(),
            index: 2,
        };
        *state.in_progress.lock().unwrap() = Some(now.clone());
        *state.live.lock().unwrap() = Some(live_report(serde_json::json!({"MURLOC": 3})));
        assert!(overlay_state_of(&state).unwrap().entered_tribes.is_empty());

        let tribes = five(["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]);
        let set = Action::Set {
            session: now.session,
            index: 2,
            tribes: tribes.clone(),
        };
        edit_lobby_tribes_of(&state, set).unwrap();
        let shown = overlay_state_of(&state).unwrap();
        assert_eq!(shown.entered_tribes, tribes);
        assert_eq!(
            shown.tribes[0].tribe, "MURLOC",
            "the tavern list is untouched"
        );
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
