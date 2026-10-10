//! The overlay window (T-301, T-302): a second, transparent, always-on-top
//! window with no taskbar entry, shown only while a Battlegrounds game is on,
//! the user has it switched on and Hearthstone is the window in front. It
//! draws on top of the game and never touches it: no memory, no injection, no
//! clicks or keys sent to the game (D-004, D-006).
//!
//! The window covers the work area of the monitor that holds the game (the
//! main one when no game window is known, T-308) and the panels are placed
//! inside it.
//! Locked (the default, and always at start and when a game starts) every
//! click goes through it. Unlocked, the panels take the mouse so they can be
//! moved and resized; the window shows even with no game, with example data.
//!
//! Exclusive fullscreen hides any overlay; the game has to run windowed or
//! borderless.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use tauri::menu::CheckMenuItem;
use tauri::{AppHandle, Emitter, Manager, State, Wry};

use crate::hover::OverlayFrame;
use crate::overlay_layout::{
    Layout, Panel, Rect, Settings, Theme, MAX_OPACITY, MIN_OPACITY, THEMES,
};
use crate::screen_fit::Area;
use crate::AppState;

pub const WINDOW: &str = "overlay";
/// The overlay page listens for this to redraw with new settings.
pub(crate) const SETTINGS_CHANGED: &str = "overlay-settings-changed";
const PREFS_FILE: &str = "overlay.json";

/// x, y, width, height (physical pixels) and scale in thousandths.
pub(crate) type Placed = (i32, i32, u32, u32, u32);

/// Whether the user switched the overlay on, and how they arranged it, kept
/// in the data folder so it survives a restart. Off until the user turns it
/// on; locked (click-through) at every start.
#[derive(Default)]
pub struct Overlay {
    settings: Mutex<Settings>,
    /// `--overlay-dev`: on, whatever the window in front is, and the on/off
    /// choice never saved.
    dev: AtomicBool,
    /// Set when the window could not be made click-through and placed: it
    /// must then never be shown, whatever the setting says.
    broken: AtomicBool,
    /// Layout editing: the panels take the mouse. Never saved.
    unlocked: AtomicBool,
    prefs: Mutex<Option<PathBuf>>,
    /// The overlay window's size in logical pixels (a monitor's work area).
    screen: Mutex<(f64, f64)>,
    /// Where the window is now: x, y, width, height in physical pixels, and
    /// the monitor's scale in thousandths (a DPI change moves it too).
    placed: Mutex<Option<Placed>>,
    /// The window's physical origin, scale and logical size, once placed.
    frame: Mutex<Option<OverlayFrame>>,
    /// The game window's client area the last time it was seen (physical
    /// pixels), for the monitor choice and the leaderboard box.
    game: Mutex<Option<Area>>,
    enable_item: Mutex<Option<CheckMenuItem<Wry>>>,
    /// The last problem saving the settings, until a save works.
    warning: Mutex<Option<String>>,
    unlock_item: Mutex<Option<CheckMenuItem<Wry>>>,
    /// Held while a change is made and saved, so two saves cannot cross.
    saving: Mutex<()>,
}

/// What the overlay page needs to draw itself.
#[derive(Serialize)]
pub struct View {
    enabled: bool,
    unlocked: bool,
    theme: &'static str,
    themes: Vec<ThemeInfo>,
    opacity: f64,
    min_opacity: f64,
    max_opacity: f64,
    hidden: Vec<Panel>,
    layout: Layout,
    screen: (f64, f64),
    /// The leaderboard box in the window's own pixels, for adjusting it while
    /// unlocked; `None` until the game window has been seen on this monitor.
    leaderboard: Option<BoardView>,
    vars: BTreeMap<&'static str, String>,
    warning: Option<String>,
}

#[derive(Serialize)]
struct BoardView {
    area: Area,
    /// True when the user drew it; false for the measured default.
    custom: bool,
}

#[derive(Serialize)]
struct ThemeInfo {
    id: &'static str,
    label: &'static str,
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Overlay {
    /// Reads the saved settings. A missing file means the defaults; a file
    /// that cannot be read leaves the defaults (overlay off) and says so.
    pub fn load(&self, data_dir: &Path, dev: bool) -> Result<(), String> {
        self.dev.store(dev, Ordering::SeqCst);
        let path = data_dir.join(PREFS_FILE);
        *lock(&self.prefs) = Some(path.clone());
        match std::fs::read_to_string(&path) {
            Ok(text) => *lock(&self.settings) = Settings::parse(&text)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "The overlay settings could not be read ({:?}); the defaults are used.",
                    e.kind()
                ))
            }
        }
        Ok(())
    }

    pub fn enabled(&self) -> bool {
        (self.dev.load(Ordering::SeqCst) || lock(&self.settings).enabled)
            && !self.broken.load(Ordering::SeqCst)
    }

    /// Keeps the window hidden for the rest of this run.
    pub fn disable(&self) {
        self.broken.store(true, Ordering::SeqCst);
        self.unlocked.store(false, Ordering::SeqCst);
    }

    pub fn unlocked(&self) -> bool {
        self.unlocked.load(Ordering::SeqCst) && !self.broken.load(Ordering::SeqCst)
    }

    pub fn ignores_foreground(&self) -> bool {
        self.dev.load(Ordering::SeqCst)
    }

    /// Switches it on or off and saves the choice. The choice holds for this
    /// run even if saving fails; the error says it will not survive a restart.
    pub fn set_enabled(&self, on: bool) -> Result<(), String> {
        if self.ignores_foreground() {
            return Ok(());
        }
        self.change(|s, _| s.enabled = on)
    }

    /// Applies a change to the settings and saves them. The change holds for
    /// this run even if saving fails; the error says it will not survive a
    /// restart.
    pub fn change(&self, apply: impl FnOnce(&mut Settings, (f64, f64))) -> Result<(), String> {
        let _one_at_a_time = lock(&self.saving);
        let screen = *lock(&self.screen);
        let text = {
            let mut settings = lock(&self.settings);
            apply(&mut settings, screen);
            settings.to_text()
        };
        let result = self.save(text);
        *lock(&self.warning) = result.clone().err();
        result
    }

    fn save(&self, text: Result<String, serde_json::Error>) -> Result<(), String> {
        let failed = |kind: io::ErrorKind| {
            format!(
                "The overlay settings could not be saved ({kind:?}); they will be lost after a restart."
            )
        };
        let path = lock(&self.prefs)
            .clone()
            .ok_or("The overlay settings have no folder to be saved in; they will be lost after a restart.")?;
        let text = text.map_err(|_| failed(io::ErrorKind::InvalidData))?;
        write_whole(&path, &text).map_err(|e| failed(e.kind()))
    }

    /// The game window's client area the last time it was seen.
    pub fn game(&self) -> Option<Area> {
        *lock(&self.game)
    }

    /// True when the window was last placed exactly so.
    pub fn placed_as(&self, placed: Placed) -> bool {
        *lock(&self.placed) == Some(placed)
    }

    /// Records where the window now is.
    pub fn set_placed(&self, placed: Placed, frame: OverlayFrame) {
        *lock(&self.placed) = Some(placed);
        *lock(&self.screen) = frame.size;
        *lock(&self.frame) = Some(frame);
    }

    /// Remembers where the game window is (T-306).
    pub fn saw_game(&self, client: Area) {
        if client.usable() {
            *lock(&self.game) = Some(client);
        }
    }

    pub fn frame(&self) -> Option<OverlayFrame> {
        *lock(&self.frame)
    }

    /// The leaderboard box for this game window, relative to its client
    /// area, in physical pixels: the user's or the measured default.
    pub fn leaderboard_box(&self, client: Area) -> Area {
        lock(&self.settings).leaderboard_for((client.w, client.h)).0
    }

    fn board_view(&self, settings: &Settings) -> Option<BoardView> {
        let client = (*lock(&self.game))?;
        let frame = self.frame()?;
        let (area, custom) = settings.leaderboard_for((client.w, client.h));
        let on_screen = Area {
            x: client.x + area.x,
            y: client.y + area.y,
            ..area
        };
        Some(BoardView {
            area: on_screen.in_frame(frame.origin, frame.scale),
            custom,
        })
    }

    /// The box the user drew, given in the window's own pixels, kept for the
    /// game window's size. False when no game window is known.
    pub fn save_leaderboard(&self, drawn: Area) -> Result<bool, String> {
        let (Some(client), Some(frame)) = (*lock(&self.game), self.frame()) else {
            return Ok(false);
        };
        let physical = Area {
            x: drawn.x * frame.scale + frame.origin.0 - client.x,
            y: drawn.y * frame.scale + frame.origin.1 - client.y,
            w: drawn.w * frame.scale,
            h: drawn.h * frame.scale,
        };
        self.change(|s, _| s.set_leaderboard((client.w, client.h), physical))
            .map(|()| true)
    }

    pub fn reset_leaderboard(&self) -> Result<(), String> {
        let Some(client) = *lock(&self.game) else {
            return Ok(());
        };
        self.change(|s, _| s.reset_leaderboard((client.w, client.h)))
    }

    pub fn view(&self) -> View {
        let settings = lock(&self.settings).clone();
        let screen = *lock(&self.screen);
        View {
            enabled: self.enabled(),
            unlocked: self.unlocked(),
            theme: settings.theme.id(),
            themes: THEMES
                .iter()
                .map(|t| ThemeInfo {
                    id: t.id(),
                    label: t.label(),
                })
                .collect(),
            opacity: settings.opacity,
            min_opacity: MIN_OPACITY,
            max_opacity: MAX_OPACITY,
            hidden: settings.hidden.iter().copied().collect(),
            layout: settings.layout_for(screen),
            leaderboard: self.board_view(&settings),
            screen,
            vars: settings.theme.css_vars(settings.opacity),
            warning: lock(&self.warning).clone(),
        }
    }
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

/// Unlocks or locks the layout editing. Locking is the safe direction: if the
/// window cannot be made click-through again it is hidden for the rest of the
/// run, never left over the game taking clicks.
pub fn set_unlocked(app: &AppHandle, state: &AppState, on: bool) {
    if on && state.overlay.broken.load(Ordering::SeqCst) {
        refresh_unlock_item(state);
        return;
    }
    // The flag is set first and the window is made to match it, then the flag
    // is read again: if another caller changed it meanwhile, the window is
    // set once more, so the two always end up the same. No lock is held over
    // the window calls (they wait for the main thread).
    state.overlay.unlocked.store(on, Ordering::SeqCst);
    let done = loop {
        let want = state.overlay.unlocked.load(Ordering::SeqCst);
        let result = app
            .get_webview_window(WINDOW)
            .ok_or(tauri::Error::WindowNotFound)
            .and_then(|w| w.set_ignore_cursor_events(!want));
        if result.is_err() || state.overlay.unlocked.load(Ordering::SeqCst) == want {
            break result;
        }
    };
    if done.is_err() {
        state.overlay.disable();
        if let Some(window) = app.get_webview_window(WINDOW) {
            let _ = window.hide();
        }
        let msg = "The overlay could not change between locked and unlocked; it is off until you restart the app.";
        crate::set_status(app, state, |s| s.notice = Some(msg.into()));
    }
    refresh_unlock_item(state);
    let _ = app.emit(SETTINGS_CHANGED, ());
}

/// The tray entry shows the real state, not what the click toggled it to.
fn refresh_unlock_item(state: &AppState) {
    let item = lock(&state.overlay.unlock_item).clone();
    if let Some(item) = item {
        let _ = item.set_checked(state.overlay.unlocked());
    }
}

/// The tray's "show overlay" entry, so it follows the switch in the window.
pub fn keep_enable_item(state: &AppState, item: CheckMenuItem<Wry>) {
    *lock(&state.overlay.enable_item) = Some(item);
}

/// Switches the overlay on or off from the tray or the main window (T-308);
/// both show the real state afterwards, and a failed save is said.
pub fn set_enabled(app: &AppHandle, state: &AppState, on: bool) -> Result<(), String> {
    let result = state.overlay.set_enabled(on);
    // Cloned out first: no lock is held over a call to the menu.
    let item = lock(&state.overlay.enable_item).clone();
    if let Some(item) = item {
        let _ = item.set_checked(state.overlay.enabled());
    }
    let _ = app.emit(SETTINGS_CHANGED, ());
    result
}

/// The tray's "unlock" entry, so the menu can show the real state.
pub fn keep_unlock_item(state: &AppState, item: CheckMenuItem<Wry>) {
    *lock(&state.overlay.unlock_item) = Some(item);
}

/// Applies a change from the overlay page or the tray, tells the page, and
/// says in the app if the change could not be saved.
pub fn change(
    app: &AppHandle,
    state: &AppState,
    apply: impl FnOnce(&mut Settings, (f64, f64)),
) -> View {
    if let Err(message) = state.overlay.change(apply) {
        crate::set_status(app, state, |s| s.notice = Some(message));
    }
    let _ = app.emit(SETTINGS_CHANGED, ());
    state.overlay.view()
}

#[tauri::command]
pub fn overlay_settings(state: State<'_, Arc<AppState>>) -> View {
    state.overlay.view()
}

#[tauri::command]
pub fn overlay_save_layout(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    panels: BTreeMap<Panel, Rect>,
) -> View {
    change(&app, &state, |s, screen| s.set_layout(screen, &panels))
}

#[tauri::command]
pub fn overlay_set_theme(app: AppHandle, state: State<'_, Arc<AppState>>, theme: Theme) -> View {
    change(&app, &state, |s, _| s.theme = theme)
}

#[tauri::command]
pub fn overlay_set_opacity(app: AppHandle, state: State<'_, Arc<AppState>>, value: f64) -> View {
    change(&app, &state, |s, _| s.set_opacity(value))
}

#[tauri::command]
pub fn overlay_set_shown(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    panel: Panel,
    shown: bool,
) -> View {
    change(&app, &state, |s, _| s.set_shown(panel, shown))
}

#[tauri::command]
pub fn overlay_reset_layout(app: AppHandle, state: State<'_, Arc<AppState>>) -> View {
    change(&app, &state, |s, _| s.reset_layout())
}

#[tauri::command]
pub fn overlay_set_enabled(app: AppHandle, state: State<'_, Arc<AppState>>, enabled: bool) -> View {
    if let Err(message) = set_enabled(&app, &state, enabled) {
        crate::set_status(&app, &state, |s| s.notice = Some(message));
    }
    state.overlay.view()
}

/// Keeps the leaderboard box the user drew over the game (T-306).
#[tauri::command]
pub fn overlay_save_leaderboard(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    area: Area,
) -> View {
    match state.overlay.save_leaderboard(area) {
        Ok(true) => {}
        Ok(false) => {
            let msg = "The leaderboard box was not kept: bring Hearthstone to the front once, then try again.";
            crate::set_status(&app, &state, |s| s.notice = Some(msg.into()));
        }
        Err(message) => crate::set_status(&app, &state, |s| s.notice = Some(message)),
    }
    let _ = app.emit(SETTINGS_CHANGED, ());
    state.overlay.view()
}

#[tauri::command]
pub fn overlay_reset_leaderboard(app: AppHandle, state: State<'_, Arc<AppState>>) -> View {
    if let Err(message) = state.overlay.reset_leaderboard() {
        crate::set_status(&app, &state, |s| s.notice = Some(message));
    }
    let _ = app.emit(SETTINGS_CHANGED, ());
    state.overlay.view()
}

#[tauri::command]
pub fn overlay_set_unlocked(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    unlocked: bool,
) -> View {
    set_unlocked(&app, &state, unlocked);
    state.overlay.view()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tavern-ledger-overlay-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn rect(x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            w: 300.0,
            h: None,
        }
    }

    #[test]
    fn it_is_off_until_the_user_turns_it_on() {
        let overlay = Overlay::default();
        overlay.load(&temp_dir("default"), false).unwrap();
        assert!(!overlay.enabled());
        assert!(!overlay.unlocked());
    }

    #[test]
    fn the_choice_is_kept_for_the_next_start() {
        let dir = temp_dir("kept");
        let first = Overlay::default();
        first.load(&dir, false).unwrap();
        first.set_enabled(true).unwrap();
        let second = Overlay::default();
        second.load(&dir, false).unwrap();
        assert!(second.enabled());
        second.set_enabled(false).unwrap();
        let third = Overlay::default();
        third.load(&dir, false).unwrap();
        assert!(!third.enabled());
    }

    #[test]
    fn a_broken_setting_file_leaves_the_defaults_and_says_so() {
        let dir = temp_dir("broken");
        std::fs::write(dir.join(PREFS_FILE), "not json").unwrap();
        let overlay = Overlay::default();
        let err = overlay.load(&dir, false).unwrap_err();
        assert!(err.contains("could not be read"));
        assert!(!overlay.enabled());
        assert!(overlay.view().layout.panels.is_empty());
    }

    #[test]
    fn a_choice_that_cannot_be_saved_still_holds_and_the_error_says_so() {
        let dir = temp_dir("unsavable");
        let overlay = Overlay::default();
        overlay.load(&dir, false).unwrap();
        // A file where the folder should be: the side file cannot be made.
        let blocked = dir.join("blocked");
        std::fs::write(&blocked, "x").unwrap();
        *overlay.prefs.lock().unwrap() = Some(blocked.join(PREFS_FILE));
        let err = overlay.set_enabled(true).unwrap_err();
        assert!(err.contains("after a restart"));
        assert!(overlay.enabled());
        assert!(overlay.view().warning.is_some(), "the page can say it too");
    }

    #[test]
    fn a_later_good_save_clears_the_warning() {
        let dir = temp_dir("warning");
        let overlay = Overlay::default();
        overlay.load(&dir, false).unwrap();
        *overlay.prefs.lock().unwrap() = None;
        assert!(overlay.change(|s, _| s.theme = Theme::Contrast).is_err());
        *overlay.prefs.lock().unwrap() = Some(dir.join(PREFS_FILE));
        overlay.change(|s, _| s.theme = Theme::Contrast).unwrap();
        assert!(overlay.view().warning.is_none());
    }

    #[test]
    fn the_dev_flag_turns_it_on_without_saving_the_choice() {
        let dir = temp_dir("dev");
        let overlay = Overlay::default();
        overlay.load(&dir, true).unwrap();
        assert!(overlay.enabled() && overlay.ignores_foreground());
        overlay.set_enabled(false).unwrap();
        assert!(!dir.join(PREFS_FILE).exists());
        assert!(overlay.enabled());
    }

    #[test]
    fn layout_theme_and_opacity_are_kept_for_the_next_start() {
        let dir = temp_dir("arranged");
        let first = Overlay::default();
        first.load(&dir, false).unwrap();
        *first.screen.lock().unwrap() = (1920.0, 1080.0);
        first
            .change(|s, screen| {
                s.set_layout(
                    screen,
                    &BTreeMap::from([(Panel::Opponents, rect(500.0, 120.0))]),
                );
                s.theme = Theme::Contrast;
                s.set_opacity(0.8);
                s.set_shown(Panel::Tribes, false);
            })
            .unwrap();
        let second = Overlay::default();
        second.load(&dir, false).unwrap();
        *second.screen.lock().unwrap() = (1920.0, 1080.0);
        let view = second.view();
        assert_eq!(view.theme, "contrast");
        assert_eq!(view.opacity, 0.8);
        assert_eq!(view.hidden, vec![Panel::Tribes]);
        assert_eq!(view.layout.panels[&Panel::Opponents], rect(500.0, 120.0));
        assert!(!view.unlocked, "unlocking is never saved");
    }

    #[test]
    fn the_file_a_t_301_app_wrote_still_loads() {
        let dir = temp_dir("old");
        std::fs::write(dir.join(PREFS_FILE), r#"{"enabled":true}"#).unwrap();
        let overlay = Overlay::default();
        overlay.load(&dir, false).unwrap();
        assert!(overlay.enabled());
    }

    #[test]
    fn a_broken_overlay_cannot_be_unlocked() {
        let overlay = Overlay::default();
        overlay.unlocked.store(true, Ordering::SeqCst);
        overlay.disable();
        assert!(!overlay.unlocked());
    }

    const OVERLAY_HTML: &str = include_str!("../ui/overlay.html");

    #[test]
    fn every_panel_has_a_slot_in_the_page() {
        for panel in [
            Panel::Status,
            Panel::Tribes,
            Panel::Opponents,
            Panel::Legend,
        ] {
            let id = serde_json::to_string(&panel).unwrap();
            let slot = format!("data-panel={id}");
            assert!(OVERLAY_HTML.contains(&slot), "no slot for {id}");
        }
    }

    #[test]
    fn every_script_the_page_loads_is_there() {
        let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
        let scripts: Vec<_> = OVERLAY_HTML
            .split("<script src=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .collect();
        assert_eq!(scripts.len(), 5);
        for script in scripts {
            assert!(ui.join(script).is_file(), "{script} is missing");
        }
    }

    #[test]
    fn the_layout_script_stays_in_its_own_scope() {
        // A top-level function there once replaced one of the same name in
        // overlay.js and the overlay never showed a game.
        for script in [
            include_str!("../ui/overlay-layout.js"),
            include_str!("../ui/overlay-hover.js"),
        ] {
            assert!(script.contains("(() => {") && script.trim_end().ends_with("})();"));
        }
    }
}
