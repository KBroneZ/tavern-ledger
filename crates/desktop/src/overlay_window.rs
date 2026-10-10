//! Where the overlay window is and when it shows (T-301, T-308): it covers
//! the work area of the monitor that holds the game (the main one when no
//! game window is known), follows the game and the window in front, and
//! locks itself again when a game starts. Split from `overlay.rs`.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow,
};

use crate::foreground::{self, Foreground};
use crate::hover::{self, OverlayFrame};
use crate::overlay::{lock, set_unlocked, SETTINGS_CHANGED, WINDOW};
use crate::screen_fit::{overlay_area, pick_screen, Area, Screen};
use crate::AppState;

/// How often the window is shown or hidden to follow the game and the
/// window in front.
const CHECK_EVERY: Duration = Duration::from_millis(500);

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

/// Covers the game's monitor with the window, and makes every click go
/// through it to the window below. It never takes focus.
pub fn prepare(app: &AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window(WINDOW)
        .ok_or(tauri::Error::WindowNotFound)?;
    window.set_ignore_cursor_events(true)?;
    fit_window(app)?;
    Ok(())
}

fn physical(position: &PhysicalPosition<i32>, size: &PhysicalSize<u32>) -> Area {
    Area {
        x: f64::from(position.x),
        y: f64::from(position.y),
        w: f64::from(size.width),
        h: f64::from(size.height),
    }
}

/// Every monitor as the system reports it now.
fn screens(window: &WebviewWindow) -> tauri::Result<Vec<Screen>> {
    let primary = window.primary_monitor()?;
    Ok(window
        .available_monitors()?
        .iter()
        .map(|m| Screen {
            whole: physical(m.position(), m.size()),
            work: physical(&m.work_area().position, &m.work_area().size),
            scale: m.scale_factor(),
            primary: primary
                .as_ref()
                .is_some_and(|p| p.position() == m.position() && p.size() == m.size()),
        })
        .collect())
}

/// Makes the window cover the work area of the monitor that holds the game
/// (T-308), as it is now: a resolution, DPI or monitor change moves it too.
/// Only touches the window when that changed. True if it did.
fn fit_window(app: &AppHandle) -> tauri::Result<bool> {
    let window = app
        .get_webview_window(WINDOW)
        .ok_or(tauri::Error::WindowNotFound)?;
    let screens = screens(&window)?;
    let state = app.state::<Arc<AppState>>();
    let Some(index) = pick_screen(&screens, state.overlay.game()) else {
        return Ok(false);
    };
    let (area, scale) = overlay_area(&screens[index]);
    let (x, y, w, h) = (area.x as i32, area.y as i32, area.w as u32, area.h as u32);
    let placed = (x, y, w, h, (scale * 1000.0).round() as u32);
    let (position, size) = (PhysicalPosition::new(x, y), PhysicalSize::new(w, h));
    // The window itself is checked too: moving it to a monitor with another
    // DPI makes Windows resize it after our call, so it is set again then.
    let in_place =
        window.outer_position().ok() == Some(position) && window.outer_size().ok() == Some(size);
    let same = state.overlay.placed_as(placed);
    if same && in_place {
        return Ok(false);
    }
    window.set_position(Position::Physical(position))?;
    window.set_size(Size::Physical(size))?;
    if !same {
        // Only now: a failed call is tried again on the next check.
        state.overlay.set_placed(
            placed,
            OverlayFrame {
                origin: (f64::from(x), f64::from(y)),
                scale,
                size: (f64::from(w) / scale, f64::from(h) / scale),
            },
        );
        let _ = app.emit(SETTINGS_CHANGED, ());
    }
    Ok(true)
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
            let seen = hover::game_client(state.overlay.ignores_foreground());
            if let Some(client) = seen {
                let moved = state.overlay.game() != Some(client);
                state.overlay.saw_game(client);
                if moved && state.overlay.unlocked() {
                    // The box drawn over the game follows the game window.
                    let _ = app.emit(SETTINGS_CHANGED, ());
                }
            }
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
}
