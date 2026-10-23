//! One key and a tray item that hide and show everything the overlay draws
//! (T-614). The state lives in `Overlay` and is never saved: every start
//! begins shown. The page follows it (`ui/hide-all.js`).
//!
//! The key is a global shortcut, Ctrl+Shift+H (`RegisterHotKey` through
//! tauri-plugin-global-shortcut, registered here from Rust, with no JavaScript
//! access). It is taken only while Hearthstone is running and given back when
//! the game closes and on exit, so other programs keep it the rest of the
//! time. It reads no other key, sends nothing to the game and changes nothing
//! in it (D-004): while it is taken, that one combination goes to the app
//! instead of the program in front. If another program already holds it, the
//! app says so and the tray item still works.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Builder, Code, Modifiers, Shortcut, ShortcutState};

use crate::overlay::{self, SETTINGS_CHANGED};
use crate::{applog, AppState};

/// Said in the main window when the key cannot be taken; the tray item still works.
pub const KEY_TAKEN: &str = "Another program already uses Ctrl+Shift+H, so that key can't hide the overlay. Use \"Hide overlay\" in the tray menu instead.";
/// Said when the key could not be set up at all.
pub const KEY_FAILED: &str =
    "The key that hides the overlay could not be set up. Use \"Hide overlay\" in the tray menu instead.";

/// How long a "the game runs" answer is kept: listing every process each
/// half second would be wasteful, and a game starts slower than this.
const RECHECK_EVERY: Duration = Duration::from_secs(2);

/// Ctrl+Shift+H.
pub fn key() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyH)
}

/// Hiding while the layout is being edited locks it first: a hidden window
/// must never take clicks.
pub fn must_lock_first(hide: bool, unlocked: bool) -> bool {
    hide && unlocked
}

/// The key is taken while the game runs (or in a `--overlay-dev` run, which
/// tries the overlay over a stand-in window).
pub fn wants_key(game_running: bool, dev: bool) -> bool {
    game_running || dev
}

/// The key hides only what is on screen: pressed while the overlay draws
/// nothing (no game on, the game's menus), it changes nothing, so a game
/// never starts hidden because of a press nobody saw. Showing always works.
pub fn key_acts(hide: bool, window_shown: bool) -> bool {
    !hide || window_shown
}

/// Whether Hearthstone runs, asking the system at most every [`RECHECK_EVERY`].
#[derive(Default)]
pub struct GameCheck {
    last: Option<(Instant, bool)>,
}

impl GameCheck {
    pub fn running(&mut self, now: Instant, ask: impl FnOnce() -> bool) -> bool {
        match self.last {
            Some((at, running)) if now.duration_since(at) < RECHECK_EVERY => running,
            _ => {
                let running = ask();
                self.last = Some((now, running));
                running
            }
        }
    }
}

/// Adds the shortcut plugin; the key itself is taken by [`follow_game`].
/// A failure does not stop the app: the tray item works without the key.
pub fn start(app: &AppHandle, state: &AppState) {
    let plugin = Builder::new()
        .with_handler(|app, shortcut, event| {
            // Pressed only: the key's release is not a second toggle.
            if event.state == ShortcutState::Pressed && *shortcut == key() {
                pressed(app);
            }
        })
        .build();
    if let Err(e) = app.plugin(plugin) {
        applog::warn(
            "overlay",
            &format!("hide key: the shortcut plugin did not start: {e}"),
        );
        state.overlay.set_hide_key_taken(true);
        crate::set_status(app, state, |s| s.notice = Some(KEY_FAILED.into()));
    }
}

/// Takes the key while it is wanted and gives it back when not; called by the
/// overlay's follow loop. A key another program holds is tried again on the
/// next call, and said once until it works.
pub fn follow_game(app: &AppHandle, want: bool) {
    let handle = app.clone();
    // The key belongs to a window of the main thread: it is taken and given
    // back there.
    let _ = app.run_on_main_thread(move || {
        let state = handle.state::<Arc<AppState>>();
        let Some(shortcuts) =
            handle.try_state::<tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>>()
        else {
            return;
        };
        let held = shortcuts.is_registered(key());
        if want == held {
            return;
        }
        if !want {
            let _ = shortcuts.unregister(key());
            return;
        }
        let was_taken = state.overlay.hide_key_taken();
        match shortcuts.register(key()) {
            Ok(()) => {
                state.overlay.set_hide_key_taken(false);
                crate::set_status(&handle, &state, |s| {
                    if s.notice.as_deref() == Some(KEY_TAKEN) {
                        s.notice = None;
                    }
                });
            }
            Err(e) => {
                state.overlay.set_hide_key_taken(true);
                if !was_taken {
                    applog::warn(
                        "overlay",
                        &format!("hide key: Ctrl+Shift+H not registered: {e}"),
                    );
                    crate::set_status(&handle, &state, |s| s.notice = Some(KEY_TAKEN.into()));
                }
            }
        }
        if was_taken != state.overlay.hide_key_taken() {
            let _ = handle.emit(SETTINGS_CHANGED, ());
        }
    });
}

/// Gives the key back to the system. Called on the way out.
pub fn stop(app: &AppHandle) {
    if let Some(shortcuts) =
        app.try_state::<tauri_plugin_global_shortcut::GlobalShortcut<tauri::Wry>>()
    {
        let _ = shortcuts.unregister_all();
    }
}

/// The key: hides only what is on screen, shows always.
fn pressed(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>();
    let hide = !state.overlay.all_hidden();
    if key_acts(hide, state.overlay.window_shown()) {
        set(app, &state, hide);
    }
}

/// The tray item.
pub fn toggle(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>();
    set(app, &state, !state.overlay.all_hidden());
}

/// Hides or shows everything the overlay draws, and tells the page and the tray.
pub fn set(app: &AppHandle, state: &AppState, hide: bool) {
    if must_lock_first(hide, state.overlay.unlocked()) {
        overlay::set_unlocked(app, state, false);
    }
    state.overlay.hide_all(hide);
    overlay::refresh_hide_item(state);
    let _ = app.emit(SETTINGS_CHANGED, ());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_ctrl_shift_h_and_nothing_else() {
        let k = key();
        assert!(k.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyH));
        assert!(!k.matches(Modifiers::CONTROL, Code::KeyH));
        assert!(!k.matches(
            Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
            Code::KeyH
        ));
        assert!(!k.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyG));
    }

    #[test]
    fn hiding_while_the_layout_is_unlocked_locks_it_first() {
        assert!(must_lock_first(true, true));
        assert!(!must_lock_first(true, false), "already locked");
        assert!(!must_lock_first(false, true), "showing changes nothing");
        assert!(!must_lock_first(false, false));
    }

    #[test]
    fn the_key_is_taken_only_while_the_game_runs() {
        assert!(wants_key(true, false));
        assert!(!wants_key(false, false), "other programs keep it");
        assert!(
            wants_key(false, true),
            "a --overlay-dev run over a stand-in window"
        );
    }

    #[test]
    fn the_key_hides_only_what_is_on_screen_and_always_shows() {
        assert!(key_acts(true, true));
        assert!(
            !key_acts(true, false),
            "nothing drawn: a press changes nothing"
        );
        assert!(key_acts(false, false), "showing again always works");
        assert!(key_acts(false, true));
    }

    #[test]
    fn the_game_is_looked_for_at_most_every_two_seconds() {
        let mut check = GameCheck::default();
        let start = Instant::now();
        let mut asked = 0;
        let mut ask = |answer: bool| {
            asked += 1;
            answer
        };
        assert!(check.running(start, || ask(true)));
        assert!(check.running(start + Duration::from_millis(1500), || ask(false)));
        assert!(!check.running(start + RECHECK_EVERY, || ask(false)));
        assert_eq!(asked, 2);
    }

    #[test]
    fn the_messages_name_the_key_and_the_tray_item() {
        assert!(KEY_TAKEN.contains("Ctrl+Shift+H") && KEY_TAKEN.contains("tray"));
        assert!(KEY_FAILED.contains("tray"));
    }

    #[test]
    fn the_tray_item_names_the_same_key_as_the_page_in_both_languages() {
        use crate::settings::{tray_words, Language};
        let page = include_str!("../ui/strings/overlay.js");
        for (language, name) in [
            (Language::En, "Ctrl+Shift+H"),
            (Language::Es, "Ctrl+Mayús+H"),
        ] {
            assert!(tray_words(language).hide_all.contains(name), "{language:?}");
            assert!(
                page.contains(&format!("\"ov.hidden.key\": \"{name}\"")),
                "{language:?}"
            );
        }
    }

    const MAIN: &str = include_str!("main.rs");
    const FOLLOW: &str = include_str!("overlay_window.rs");

    #[test]
    fn the_plugin_starts_in_setup_the_game_takes_the_key_and_exit_gives_it_back() {
        assert!(MAIN.contains("hide_all::start(app.handle(), &state)"));
        assert!(FOLLOW.contains("hide_all::follow_game("));
        assert!(FOLLOW.contains("hide_all::wants_key(running, state.overlay.ignores_foreground())"));
        assert!(FOLLOW.contains("state.overlay.set_window_shown(shown);"));
        let exit = MAIN.find("if let tauri::RunEvent::Exit = event").unwrap();
        assert!(MAIN[exit..].contains("hide_all::stop(app)"));
        // Added in setup, where a failure is handled, never on the builder.
        assert!(!MAIN.contains("tauri_plugin_global_shortcut"));
    }

    #[test]
    fn no_page_can_reach_the_shortcut_plugin() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for entry in std::fs::read_dir(root.join("capabilities")).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(!text.contains("global-shortcut"), "{path:?}");
        }
        // No capability is declared anywhere else either.
        let conf = std::fs::read_to_string(root.join("tauri.conf.json")).unwrap();
        assert!(!conf.contains("global-shortcut") && !conf.contains("\"capabilities\""));
    }
}
