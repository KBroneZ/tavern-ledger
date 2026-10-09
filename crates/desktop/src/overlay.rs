//! The overlay window (T-301, T-302): a second, transparent, always-on-top
//! window with no taskbar entry, shown only while a Battlegrounds game is on,
//! the user has it switched on and Hearthstone is the window in front. It
//! draws on top of the game and never touches it: no memory, no injection, no
//! clicks or keys sent to the game (D-004, D-006).
//!
//! The window covers the main screen and the panels are placed inside it.
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
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::CheckMenuItem;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, State, Wry,
};

use crate::foreground::{self, Foreground};
use crate::overlay_layout::{
    Layout, Panel, Rect, Settings, Theme, MAX_OPACITY, MIN_OPACITY, THEMES,
};
use crate::AppState;

pub const WINDOW: &str = "overlay";
/// The overlay page listens for this to redraw with new settings.
const SETTINGS_CHANGED: &str = "overlay-settings-changed";
/// How often the window is shown or hidden to follow the game and the
/// window in front.
const CHECK_EVERY: Duration = Duration::from_millis(500);
const PREFS_FILE: &str = "overlay.json";

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
    /// The overlay window's size in logical pixels (the main screen).
    screen: Mutex<(f64, f64)>,
    /// Where the window is now: x, y, width, height in physical pixels.
    placed: Mutex<Option<(i32, i32, u32, u32)>>,
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
    vars: BTreeMap<&'static str, String>,
    warning: Option<String>,
}

#[derive(Serialize)]
struct ThemeInfo {
    id: &'static str,
    label: &'static str,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
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

/// What decides whether the window shows. `front` is `None` when the system
/// cannot say what is in front: then it shows (the overlay only ever draws
/// while a game is on). While the layout is being edited it shows whatever
/// else is true, so there is something to arrange.
pub fn wants_visible(
    enabled: bool,
    game_on: bool,
    ignore_foreground: bool,
    front: Option<Foreground>,
    unlocked: bool,
) -> bool {
    unlocked
        || (enabled && game_on && (ignore_foreground || !matches!(front, Some(Foreground::Other))))
}

/// A game that starts while the layout is unlocked locks it again, so the
/// overlay never steals clicks mid-game unless the user unlocks it then.
pub fn should_relock(unlocked: bool, game_was_on: bool, game_is_on: bool) -> bool {
    unlocked && !game_was_on && game_is_on
}

/// Covers the main screen with the window, and makes every click go through
/// it to the window below. It never takes focus.
pub fn prepare(app: &AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window(WINDOW)
        .ok_or(tauri::Error::WindowNotFound)?;
    window.set_ignore_cursor_events(true)?;
    fit_window(app)?;
    Ok(())
}

/// Makes the window cover the main screen as it is now. Only touches the
/// window when the screen changed. True if it did.
fn fit_window(app: &AppHandle) -> tauri::Result<bool> {
    let window = app
        .get_webview_window(WINDOW)
        .ok_or(tauri::Error::WindowNotFound)?;
    let Some(monitor) = window.primary_monitor()? else {
        return Ok(false);
    };
    let (origin, size) = (monitor.position(), monitor.size());
    let placed = (origin.x, origin.y, size.width, size.height);
    let state = app.state::<Arc<AppState>>();
    if *lock(&state.overlay.placed) == Some(placed) {
        return Ok(false);
    }
    window.set_position(Position::Physical(PhysicalPosition::new(
        origin.x, origin.y,
    )))?;
    window.set_size(Size::Physical(PhysicalSize::new(size.width, size.height)))?;
    // Only now: a failed call is tried again on the next check.
    *lock(&state.overlay.placed) = Some(placed);
    let scale = monitor.scale_factor();
    *lock(&state.overlay.screen) = (
        f64::from(size.width) / scale,
        f64::from(size.height) / scale,
    );
    let _ = app.emit(SETTINGS_CHANGED, ());
    Ok(true)
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
    if let Some(item) = lock(&state.overlay.unlock_item).as_ref() {
        let _ = item.set_checked(state.overlay.unlocked());
    }
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
pub fn overlay_set_unlocked(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    unlocked: bool,
) -> View {
    set_unlocked(&app, &state, unlocked);
    state.overlay.view()
}

/// A show or hide that keeps failing is said once, not retried in silence.
const FAILURES_BEFORE_NOTICE: u32 = 5;

/// Shows or hides the window as the game, the setting and the window in
/// front change, until the app quits.
pub fn follow(app: AppHandle, state: Arc<AppState>) {
    thread::spawn(move || {
        let mut shown = false;
        let mut game_was_on = false;
        let mut failures: u32 = 0;
        loop {
            let game_on = lock(&state.live).is_some();
            if should_relock(state.overlay.unlocked(), game_was_on, game_on) {
                set_unlocked(&app, &state, false);
            }
            game_was_on = game_on;
            let _ = fit_window(&app);
            let want = wants_visible(
                state.overlay.enabled(),
                game_on,
                state.overlay.ignores_foreground(),
                foreground::current(),
                state.overlay.unlocked(),
            );
            if want != shown {
                let done = match app.get_webview_window(WINDOW) {
                    Some(window) if want => {
                        // On top again every time: another window may have taken the place.
                        window.show().and_then(|()| window.set_always_on_top(true))
                    }
                    Some(window) => window.hide(),
                    None => Err(tauri::Error::WindowNotFound),
                };
                if done.is_ok() {
                    shown = want;
                    failures = 0;
                } else {
                    failures += 1;
                    if failures == FAILURES_BEFORE_NOTICE {
                        let msg = "The overlay window could not be shown or hidden.";
                        crate::set_status(&app, &state, |s| s.notice = Some(msg.into()));
                    }
                }
            }
            thread::sleep(CHECK_EVERY);
        }
    });
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
                s.theme = Theme::Parchment;
                s.set_opacity(0.8);
                s.set_shown(Panel::Tribes, false);
            })
            .unwrap();
        let second = Overlay::default();
        second.load(&dir, false).unwrap();
        *second.screen.lock().unwrap() = (1920.0, 1080.0);
        let view = second.view();
        assert_eq!(view.theme, "parchment");
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

    #[test]
    fn it_shows_only_while_on_in_a_game_with_the_game_in_front() {
        let game = Some(Foreground::Hearthstone);
        let other = Some(Foreground::Other);
        assert!(wants_visible(true, true, false, game, false));
        assert!(
            !wants_visible(false, true, false, game, false),
            "switched off"
        );
        assert!(
            !wants_visible(true, false, false, game, false),
            "no game on"
        );
        assert!(
            !wants_visible(true, true, false, other, false),
            "another window in front"
        );
    }

    #[test]
    fn when_the_window_in_front_cannot_be_read_it_still_shows() {
        assert!(wants_visible(true, true, false, None, false));
        assert!(!wants_visible(true, false, false, None, false));
    }

    #[test]
    fn the_dev_flag_ignores_the_window_in_front_but_not_the_game() {
        assert!(wants_visible(
            true,
            true,
            true,
            Some(Foreground::Other),
            false
        ));
        assert!(!wants_visible(
            true,
            false,
            true,
            Some(Foreground::Other),
            false
        ));
    }

    #[test]
    fn while_unlocked_it_shows_with_no_game_and_even_when_switched_off() {
        let other = Some(Foreground::Other);
        assert!(wants_visible(false, false, false, other, true));
        assert!(wants_visible(true, true, false, other, true));
    }

    #[test]
    fn a_game_starting_locks_an_unlocked_overlay_again() {
        assert!(should_relock(true, false, true));
        assert!(
            !should_relock(true, true, true),
            "a game already on: the user unlocked it on purpose"
        );
        assert!(!should_relock(true, false, false), "no game yet");
        assert!(!should_relock(false, false, true), "already locked");
        assert!(
            !should_relock(true, true, false),
            "a game ending does not lock"
        );
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
        assert_eq!(scripts.len(), 3);
        for script in scripts {
            assert!(ui.join(script).is_file(), "{script} is missing");
        }
    }

    #[test]
    fn the_layout_script_stays_in_its_own_scope() {
        // A top-level function there once replaced one of the same name in
        // overlay.js and the overlay never showed a game.
        let layout = include_str!("../ui/overlay-layout.js");
        assert!(layout.contains("(() => {") && layout.trim_end().ends_with("})();"));
    }
}
