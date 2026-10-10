//! Hovering a hero on the game's own leaderboard (T-306, D-045): while a game
//! is on, the overlay is switched on and locked, and Hearthstone is in front,
//! the mouse position is checked about 20 times a second against where the
//! leaderboard is (from the game window's rectangle). When it rests on a
//! slot the overlay page is told which place it is, and shows that
//! opponent's card at the top of the game window, centred (T-309, D-049).
//! The overlay stays click-through:
//! the mouse is only watched, never taken. Nothing is read from the game.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::foreground;
use crate::leaderboard::{slot_area, slot_at};
use crate::screen_fit::{place_card, Area, CardPlace};
use crate::AppState;

/// The page listens for this: a `Hover`, or null when nothing is hovered.
pub const HOVER_CHANGED: &str = "overlay-hover";
/// About 20 checks a second while hovering can happen.
const WATCH_EVERY: Duration = Duration::from_millis(50);
/// Otherwise the thread only looks now and then whether it can start.
const IDLE_EVERY: Duration = Duration::from_millis(400);
/// The width the hover card asks for, in the overlay's own pixels: room for
/// a full board of seven minions in one row.
pub const CARD_WIDTH: f64 = 640.0;

/// What the page needs to show the card: the place hovered (1 is the top
/// slot), the slot's rectangle and where the card goes (the top of the game
/// window), both in the overlay window's own pixels.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Hover {
    pub place: i64,
    pub slot: Area,
    pub card: CardPlace,
}

/// Where the overlay window is: its physical origin, its scale and its size
/// in its own (logical) pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayFrame {
    pub origin: (f64, f64),
    pub scale: f64,
    pub size: (f64, f64),
}

/// The hover for this cursor, or `None`. Pure, so it is tested: `client`
/// is the game's client area and `board` the leaderboard box inside it
/// (relative to it), both in physical pixels.
pub fn hover_at(
    client: Area,
    board: Area,
    slots: i64,
    cursor: (f64, f64),
    frame: OverlayFrame,
) -> Option<Hover> {
    let board = Area {
        x: client.x + board.x,
        y: client.y + board.y,
        ..board
    };
    let place = slot_at(board, slots, cursor)?;
    let slot = slot_area(board, slots, place).in_frame(frame.origin, frame.scale);
    let game = client.in_frame(frame.origin, frame.scale);
    let bounds = Area {
        x: 0.0,
        y: 0.0,
        w: frame.size.0,
        h: frame.size.1,
    };
    Some(Hover {
        place,
        slot,
        card: place_card(game, bounds, CARD_WIDTH),
    })
}

/// The game's client area now: the window in front if it is the game, or
/// with `--overlay-dev` a window titled "Hearthstone" (a stand-in).
pub fn game_client(dev: bool) -> Option<Area> {
    foreground::game_client().or_else(|| {
        if dev {
            foreground::named_game_client()
        } else {
            None
        }
    })
}

fn current(state: &AppState) -> Option<Hover> {
    let overlay = &state.overlay;
    if !overlay.enabled() || overlay.unlocked() {
        return None;
    }
    let slots = {
        let live = state.live.lock().unwrap_or_else(|e| e.into_inner());
        live.as_ref()?.leaderboard_slots?
    };
    let client = game_client(overlay.ignores_foreground())?;
    overlay.saw_game(client);
    let board = overlay.leaderboard_box(client);
    hover_at(
        client,
        board,
        slots,
        foreground::cursor()?,
        overlay.frame()?,
    )
}

/// Watches the mouse until the app quits; tells the page only on a change.
pub fn watch(app: AppHandle, state: Arc<AppState>) {
    thread::spawn(move || {
        let mut shown: Option<Hover> = None;
        loop {
            let can_hover = state.overlay.enabled()
                && !state.overlay.unlocked()
                && state
                    .live
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .is_some();
            let now = if can_hover { current(&state) } else { None };
            if now != shown {
                let _ = app.emit(HOVER_CHANGED, &now);
                shown = now;
            }
            thread::sleep(if can_hover { WATCH_EVERY } else { IDLE_EVERY });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leaderboard::default_area;

    fn frame(origin: (f64, f64), scale: f64, size: (f64, f64)) -> OverlayFrame {
        OverlayFrame {
            origin,
            scale,
            size,
        }
    }

    fn client(x: f64, y: f64, w: f64, h: f64) -> Area {
        Area { x, y, w, h }
    }

    fn middle_of(client: Area, board: Area, slots: i64, place: i64) -> (f64, f64) {
        let s = slot_area(board, slots, place);
        (client.x + s.x + s.w / 2.0, client.y + s.y + s.h / 2.0)
    }

    #[test]
    fn hovering_each_slot_gives_its_place_at_two_window_sizes() {
        for (w, h) in [(1920.0, 1080.0), (1280.0, 720.0)] {
            let game = client(0.0, 0.0, w, h);
            let board = default_area((w, h));
            let f = frame((0.0, 0.0), 1.0, (w, h - 40.0));
            for place in 1..=8 {
                let hover = hover_at(game, board, 8, middle_of(game, board, 8, place), f)
                    .expect("on the leaderboard");
                assert_eq!(hover.place, place);
                assert_eq!(hover.card.x + hover.card.width / 2.0, w / 2.0, "centred");
                assert_eq!(hover.card.y, crate::screen_fit::GAP, "at the top");
                assert!(hover.card.x + hover.card.width <= w);
            }
        }
    }

    #[test]
    fn off_the_leaderboard_nothing_is_hovered() {
        let game = client(0.0, 0.0, 1920.0, 1080.0);
        let board = default_area((1920.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1920.0, 1040.0));
        assert_eq!(hover_at(game, board, 8, (960.0, 540.0), f), None);
        assert_eq!(hover_at(game, board, 8, (5.0, 5.0), f), None);
        assert_eq!(
            hover_at(game, board, 0, middle_of(game, board, 8, 3), f),
            None,
            "no slot count yet"
        );
    }

    #[test]
    fn a_windowed_game_on_a_scaled_second_monitor_lines_up_with_the_overlay() {
        // Game window at (2100, 150) on a 150% monitor whose work area starts
        // at x 1920: the slot is given in the overlay's own pixels.
        let game = client(2100.0, 150.0, 1280.0, 720.0);
        let board = default_area((1280.0, 720.0));
        let f = frame((1920.0, 0.0), 1.5, (2560.0, 1408.0));
        let point = middle_of(game, board, 8, 4);
        let hover = hover_at(game, board, 8, point, f).unwrap();
        assert_eq!(hover.place, 4);
        let back_x = hover.slot.x * 1.5 + 1920.0;
        assert!((back_x - (2100.0 + board.x)).abs() < 0.01);
        // The card is centred on the game window, at its top, in the
        // overlay's pixels.
        let centre = (hover.card.x + hover.card.width / 2.0) * 1.5 + 1920.0;
        assert!((centre - (2100.0 + 640.0)).abs() < 0.01);
        assert!((hover.card.y * 1.5 - (150.0 + 8.0 * 1.5)).abs() < 0.01);
    }

    #[test]
    fn duos_maps_two_portraits_to_one_team_slot() {
        let game = client(0.0, 0.0, 1920.0, 1080.0);
        let board = default_area((1920.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1920.0, 1040.0));
        let places: Vec<i64> = (1..=8)
            .map(|portrait| {
                hover_at(game, board, 4, middle_of(game, board, 8, portrait), f)
                    .unwrap()
                    .place
            })
            .collect();
        assert_eq!(places, [1, 1, 2, 2, 3, 3, 4, 4]);
    }
}
