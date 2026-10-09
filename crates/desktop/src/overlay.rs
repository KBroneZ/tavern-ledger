//! The overlay window (T-301): a second, transparent, always-on-top,
//! click-through window with no taskbar entry, shown only while a
//! Battlegrounds game is on, the user has it switched on and Hearthstone is
//! the window in front. It draws on top of the game and never touches it: no
//! memory, no injection, no clicks or keys sent to the game (D-004, D-006).
//!
//! Exclusive fullscreen hides any overlay; the game has to run windowed or
//! borderless.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, LogicalPosition, Manager, PhysicalPosition, Position};

use crate::foreground::{self, Foreground};
use crate::AppState;

pub const WINDOW: &str = "overlay";
/// How often the window is shown or hidden to follow the game and the
/// window in front.
const CHECK_EVERY: Duration = Duration::from_millis(500);
/// Gap between the screen's corner and the panels, in logical pixels. The
/// window starts on the left, below the top of the screen (T-302 will let
/// the user move it).
const MARGIN_X: f64 = 8.0;
const MARGIN_Y: f64 = 96.0;
const PREFS_FILE: &str = "overlay.json";

#[derive(Serialize, Deserialize)]
struct Prefs {
    enabled: bool,
}

/// Whether the user switched the overlay on, kept in the data folder so it
/// survives a restart. Off until the user turns it on.
#[derive(Default)]
pub struct Overlay {
    enabled: AtomicBool,
    /// `--overlay-dev`: on, whatever the window in front is, and never saved.
    dev: AtomicBool,
    /// Set when the window could not be made click-through and placed: it
    /// must then never be shown, whatever the setting says.
    broken: AtomicBool,
    prefs: Mutex<Option<PathBuf>>,
}

impl Overlay {
    /// Reads the saved choice. A missing file means off; a file that cannot be
    /// read leaves it off and says so.
    pub fn load(&self, data_dir: &Path, dev: bool) -> Result<(), String> {
        self.dev.store(dev, Ordering::SeqCst);
        let path = data_dir.join(PREFS_FILE);
        *self.prefs.lock().unwrap_or_else(|e| e.into_inner()) = Some(path.clone());
        if dev {
            self.enabled.store(true, Ordering::SeqCst);
            return Ok(());
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Prefs>(&text) {
                Ok(prefs) => self.enabled.store(prefs.enabled, Ordering::SeqCst),
                Err(_) => return Err("The overlay setting could not be read; it is off.".into()),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "The overlay setting could not be read ({:?}); it is off.",
                    e.kind()
                ))
            }
        }
        Ok(())
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst) && !self.broken.load(Ordering::SeqCst)
    }

    /// Keeps the window hidden for the rest of this run.
    pub fn disable(&self) {
        self.broken.store(true, Ordering::SeqCst);
    }

    pub fn ignores_foreground(&self) -> bool {
        self.dev.load(Ordering::SeqCst)
    }

    /// Switches it on or off and saves the choice. The choice holds for this
    /// run even if saving fails; the error says it will not survive a restart.
    pub fn set_enabled(&self, on: bool) -> Result<(), String> {
        self.enabled.store(on, Ordering::SeqCst);
        if self.ignores_foreground() {
            return Ok(());
        }
        let path = self
            .prefs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or("The overlay setting has no folder to be saved in.")?;
        save(&path, on).map_err(|e| {
            format!(
                "The overlay setting could not be saved ({:?}); it will be off after a restart.",
                e.kind()
            )
        })
    }
}

/// Written whole to a side file first, so a crash never leaves half a file.
fn save(path: &Path, enabled: bool) -> io::Result<()> {
    let text = serde_json::to_string(&Prefs { enabled }).map_err(io::Error::other)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let side = path.with_extension("json.tmp");
    std::fs::write(&side, text)?;
    std::fs::rename(&side, path)
}

/// What decides whether the window shows. `front` is `None` when the system
/// cannot say what is in front: then it shows (the overlay only ever draws
/// while a game is on).
pub fn wants_visible(
    enabled: bool,
    game_on: bool,
    ignore_foreground: bool,
    front: Option<Foreground>,
) -> bool {
    enabled && game_on && (ignore_foreground || !matches!(front, Some(Foreground::Other)))
}

/// Puts the window in its corner of the main screen. It never takes focus
/// and every click goes through it to the window below.
pub fn prepare(app: &AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window(WINDOW)
        .ok_or(tauri::Error::WindowNotFound)?;
    window.set_ignore_cursor_events(true)?;
    if let Some(monitor) = window.primary_monitor()? {
        let scale = monitor.scale_factor();
        let origin = monitor.position();
        let at = LogicalPosition::new(MARGIN_X, MARGIN_Y).to_physical::<i32>(scale);
        window.set_position(Position::Physical(PhysicalPosition::new(
            origin.x + at.x,
            origin.y + at.y,
        )))?;
    }
    Ok(())
}

/// A show or hide that keeps failing is said once, not retried in silence.
const FAILURES_BEFORE_NOTICE: u32 = 5;

/// Shows or hides the window as the game, the setting and the window in
/// front change, until the app quits.
pub fn follow(app: AppHandle, state: Arc<AppState>) {
    thread::spawn(move || {
        let mut shown = false;
        let mut failures: u32 = 0;
        loop {
            let want = wants_visible(
                state.overlay.enabled(),
                state
                    .live
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .is_some(),
                state.overlay.ignores_foreground(),
                foreground::current(),
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

    #[test]
    fn it_is_off_until_the_user_turns_it_on() {
        let overlay = Overlay::default();
        overlay.load(&temp_dir("default"), false).unwrap();
        assert!(!overlay.enabled());
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
    fn a_broken_setting_file_leaves_it_off_and_says_so() {
        let dir = temp_dir("broken");
        std::fs::write(dir.join(PREFS_FILE), "not json").unwrap();
        let overlay = Overlay::default();
        let err = overlay.load(&dir, false).unwrap_err();
        assert!(err.contains("could not be read"));
        assert!(!overlay.enabled());
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
    }

    #[test]
    fn the_dev_flag_turns_it_on_without_saving_anything() {
        let dir = temp_dir("dev");
        let overlay = Overlay::default();
        overlay.load(&dir, true).unwrap();
        assert!(overlay.enabled() && overlay.ignores_foreground());
        overlay.set_enabled(false).unwrap();
        assert!(!dir.join(PREFS_FILE).exists());
    }

    #[test]
    fn it_shows_only_while_on_in_a_game_with_the_game_in_front() {
        let game = Some(Foreground::Hearthstone);
        let other = Some(Foreground::Other);
        assert!(wants_visible(true, true, false, game));
        assert!(!wants_visible(false, true, false, game), "switched off");
        assert!(!wants_visible(true, false, false, game), "no game on");
        assert!(
            !wants_visible(true, true, false, other),
            "another window in front"
        );
    }

    #[test]
    fn when_the_window_in_front_cannot_be_read_it_still_shows() {
        assert!(wants_visible(true, true, false, None));
        assert!(!wants_visible(true, false, false, None));
    }

    #[test]
    fn the_dev_flag_ignores_the_window_in_front_but_not_the_game() {
        assert!(wants_visible(true, true, true, Some(Foreground::Other)));
        assert!(!wants_visible(true, false, true, Some(Foreground::Other)));
    }
}
