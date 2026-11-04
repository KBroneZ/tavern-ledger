//! The in-app preview (T-614): a tray item shows the overlay with its example
//! data over the desktop, click-through, so a player can see what it looks
//! like without a game. The same item closes it, and it closes by itself when
//! Hearthstone runs, a game starts or the game's window comes to the front: it
//! never sits over the game. An overlay that is off for this run (it could not
//! be made click-through) never shows it. Never saved: every start begins
//! without it.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::foreground::{self, Foreground};
use crate::overlay::{self, lock, SETTINGS_CHANGED};
use crate::AppState;

/// The preview goes once Hearthstone runs, a game is on or the game's window
/// is in front.
pub fn should_close(
    preview: bool,
    game_running: bool,
    game_on: bool,
    front: Option<Foreground>,
) -> bool {
    preview && (game_running || game_on || front == Some(Foreground::Hearthstone))
}

/// The tray item.
pub fn toggle(app: &AppHandle) {
    let state = app.state::<Arc<AppState>>();
    set(app, &state, !state.overlay.preview());
}

/// Opens or closes the preview, and tells the page and the tray. It does not
/// open over a game: the item then stays unticked.
pub fn set(app: &AppHandle, state: &AppState, on: bool) {
    let open = on
        && state.overlay.preview_allowed()
        && !should_close(
            true,
            game_memory::process::is_running(),
            lock(&state.live).is_some(),
            foreground::current(),
        );
    if open {
        // A preview nobody can see helps no one: whatever was hidden shows.
        state.overlay.hide_all(false);
        overlay::refresh_hide_item(state);
    }
    state.overlay.set_preview(open);
    overlay::refresh_preview_item(state);
    let _ = app.emit(SETTINGS_CHANGED, ());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_closes_when_the_game_runs_a_game_starts_or_the_game_comes_to_the_front() {
        let other = Some(Foreground::Other);
        let game = Some(Foreground::Hearthstone);
        // preview, Hearthstone running, game on, window in front
        assert!(
            !should_close(true, false, false, other),
            "over the desktop: it stays"
        );
        assert!(
            !should_close(true, false, false, None),
            "front unknown, no game: it stays"
        );
        assert!(
            should_close(true, true, false, other),
            "Hearthstone runs, in its menus"
        );
        assert!(should_close(true, false, true, other), "a game started");
        assert!(
            should_close(true, false, false, game),
            "Hearthstone in front"
        );
        assert!(!should_close(false, true, true, game), "nothing to close");
    }
}
