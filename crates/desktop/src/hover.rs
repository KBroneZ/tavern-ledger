//! Hovering a hero on the game's own leaderboard (T-306, D-045): while a game
//! is on, the overlay is switched on and locked, and Hearthstone is in front,
//! the mouse position is checked about 20 times a second against where the
//! leaderboard is (from the game window's rectangle). When it rests on a
//! slot the overlay page is told which place it is, and shows that
//! opponent's card at the top centre of the game's picture, just below the
//! combat odds bar (T-309, D-049, T-341; centred again by the overlay's
//! default positions).
//! The overlay stays click-through:
//! the mouse is only watched, never taken. Nothing is read from the game.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::foreground;
use crate::leaderboard::{slot_area, slot_at};
use crate::screen_fit::{place_card, play_area, Area, CardPlace};
use crate::AppState;

/// The page listens for this: a `Hover`, or null when nothing is hovered.
pub const HOVER_CHANGED: &str = "overlay-hover";
/// About 20 checks a second while hovering can happen.
const WATCH_EVERY: Duration = Duration::from_millis(50);
/// Otherwise the thread only looks now and then whether it can start.
const IDLE_EVERY: Duration = Duration::from_millis(400);
/// The width the hover card asks for, as a share of the game picture's
/// height: the game lays its board out by the height, and seven minions in
/// one row there take about this much (T-333). The page sizes its text from
/// the width it gets (overlay-scale.js).
pub const CARD_SHARE: f64 = 0.85;

/// The card's width for a game picture of this height, in the overlay's own
/// pixels; `place_card` keeps it inside the game window and the screen.
pub fn card_width(game_height: f64) -> f64 {
    CARD_SHARE * game_height
}

/// What the page needs to show the card: the place hovered (1 is the top
/// slot), the slot's rectangle and where the card goes (the top of the game's
/// picture), both in the overlay window's own pixels.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Hover {
    pub place: i64,
    /// In Duos, which of the team's two portraits is under the mouse: 0 the
    /// top one, 1 the bottom one (T-340); None in Solo.
    pub member: Option<i64>,
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
/// (relative to it), both in physical pixels. `per_slot` is how many
/// portraits a slot holds: 2 for a Duos team, else 1.
pub fn hover_at(
    client: Area,
    board: Area,
    slots: i64,
    per_slot: i64,
    cursor: (f64, f64),
    frame: OverlayFrame,
) -> Option<Hover> {
    let board = Area {
        x: client.x + board.x,
        y: client.y + board.y,
        ..board
    };
    let place = slot_at(board, slots, cursor)?;
    // The portraits split the leaderboard evenly, so the portrait under the
    // mouse is the slot's top or bottom half.
    let member = (per_slot > 1)
        .then(|| slot_at(board, slots * per_slot, cursor))
        .flatten()
        .map(|portrait| (portrait - 1 - (place - 1) * per_slot).clamp(0, per_slot - 1));
    let slot = slot_area(board, slots, place).in_frame(frame.origin, frame.scale);
    let game = client.in_frame(frame.origin, frame.scale);
    // The game lays its board out inside its 16:9 picture: the card is sized
    // to that and centred on it, whatever shape the window is.
    let play = play_area(game);
    let bounds = Area {
        x: 0.0,
        y: 0.0,
        w: frame.size.0,
        h: frame.size.1,
    };
    Some(Hover {
        place,
        member,
        slot,
        card: place_card(play, bounds, card_width(play.h)),
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
    let (slots, per_slot) = {
        let live = state.live.lock().unwrap_or_else(|e| e.into_inner());
        let live = live.as_ref()?;
        (live.leaderboard_slots?, if live.is_duos { 2 } else { 1 })
    };
    let client = game_client(overlay.ignores_foreground())?;
    overlay.saw_game(client);
    let board = overlay.leaderboard_box(client);
    hover_at(
        client,
        board,
        slots,
        per_slot,
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
    use crate::screen_fit::GAP;

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
                let hover = hover_at(game, board, 8, 1, middle_of(game, board, 8, place), f)
                    .expect("on the leaderboard");
                assert_eq!(hover.place, place);
                assert!(
                    (hover.card.x + hover.card.width / 2.0 - w / 2.0).abs() < 0.01,
                    "centred"
                );
                assert_eq!(hover.card.y, GAP, "at the top");
                assert!(hover.card.x + hover.card.width <= w - GAP);
            }
        }
    }

    #[test]
    fn off_the_leaderboard_nothing_is_hovered() {
        let game = client(0.0, 0.0, 1920.0, 1080.0);
        let board = default_area((1920.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1920.0, 1040.0));
        assert_eq!(hover_at(game, board, 8, 1, (960.0, 540.0), f), None);
        assert_eq!(hover_at(game, board, 8, 1, (5.0, 5.0), f), None);
        assert_eq!(
            hover_at(game, board, 0, 1, middle_of(game, board, 8, 3), f),
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
        let hover = hover_at(game, board, 8, 1, point, f).unwrap();
        assert_eq!(hover.place, 4);
        let back_x = hover.slot.x * 1.5 + 1920.0;
        assert!((back_x - (2100.0 + board.x)).abs() < 0.01);
        // The card is centred on the game window, at its top, in the
        // overlay's pixels, and stays inside the game window.
        let middle = (hover.card.x + hover.card.width / 2.0) * 1.5 + 1920.0;
        assert!((middle - (2100.0 + 640.0)).abs() < 0.01);
        let right = (hover.card.x + hover.card.width) * 1.5 + 1920.0;
        assert!(right <= 2100.0 + 1280.0 - 8.0 * 1.5 + 0.01);
        assert!((hover.card.y * 1.5 - (150.0 + 8.0 * 1.5)).abs() < 0.01);
    }

    #[test]
    fn duos_maps_two_portraits_to_one_team_slot() {
        let game = client(0.0, 0.0, 1920.0, 1080.0);
        let board = default_area((1920.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1920.0, 1040.0));
        let places: Vec<i64> = (1..=8)
            .map(|portrait| {
                hover_at(game, board, 4, 2, middle_of(game, board, 8, portrait), f)
                    .unwrap()
                    .place
            })
            .collect();
        assert_eq!(places, [1, 1, 2, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn duos_says_which_portrait_of_the_team_is_hovered() {
        let game = client(0.0, 0.0, 1920.0, 1080.0);
        let board = default_area((1920.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1920.0, 1040.0));
        let members: Vec<Option<i64>> = (1..=8)
            .map(|portrait| {
                hover_at(game, board, 4, 2, middle_of(game, board, 8, portrait), f)
                    .unwrap()
                    .member
            })
            .collect();
        let want: Vec<Option<i64>> = [0, 1, 0, 1, 0, 1, 0, 1].into_iter().map(Some).collect();
        assert_eq!(members, want);
        // Solo has no team: no member.
        let solo = hover_at(game, board, 8, 1, middle_of(game, board, 8, 3), f).unwrap();
        assert_eq!(solo.member, None);
    }

    #[test]
    fn the_board_is_centred_on_the_game_at_any_scale_in_solo_and_duos() {
        // 3840x2160 at 150% and 1920x1080 at 100%, Solo and Duos, every slot.
        for (w, h, scale, work_h) in [(3840.0, 2160.0, 1.5, 1408.0), (1920.0, 1080.0, 1.0, 1040.0)]
        {
            let game = client(0.0, 0.0, w, h);
            let board = default_area((w, h));
            let f = frame((0.0, 0.0), scale, (w / scale, work_h));
            for (slots, per_slot) in [(8, 1), (4, 2)] {
                for portrait in 1..=8 {
                    let point = middle_of(game, board, 8, portrait);
                    let card = hover_at(game, board, slots, per_slot, point, f)
                        .unwrap()
                        .card;
                    let middle = card.x + card.width / 2.0;
                    assert!((middle - w / scale / 2.0).abs() < 0.01, "{card:?}");
                    assert!(card.x >= 0.0 && card.x + card.width <= w / scale - GAP + 0.01);
                    // Still the size the game's board asks for (D-067).
                    assert!((card.width - 0.85 * h / scale).abs() < 0.01, "{card:?}");
                }
            }
        }
    }

    #[test]
    fn on_an_ultrawide_window_the_board_is_sized_to_and_centred_on_the_16_by_9_picture() {
        let game = client(0.0, 0.0, 3440.0, 1440.0);
        let board = default_area((3440.0, 1440.0));
        let f = frame((0.0, 0.0), 1.0, (3440.0, 1400.0));
        let card = hover_at(game, board, 8, 1, middle_of(game, board, 8, 2), f)
            .unwrap()
            .card;
        assert!((card.width - 0.85 * 1440.0).abs() < 0.01, "{card:?}");
        assert!(
            (card.x + card.width / 2.0 - 1720.0).abs() < 0.01,
            "{card:?}"
        );
        assert_eq!(card.y, GAP);
    }

    #[test]
    fn a_taller_game_window_on_a_scaled_second_monitor_gets_its_card_on_the_picture() {
        // A 1440x1080 (4:3) game at (2100, 150) on a 150% monitor whose work
        // area starts at x 1920: the picture is 1440x810 with 135 px bars.
        let game = client(2100.0, 150.0, 1440.0, 1080.0);
        let board = default_area((1440.0, 1080.0));
        let f = frame((1920.0, 0.0), 1.5, (2560.0, 1408.0));
        let card = hover_at(game, board, 8, 1, middle_of(game, board, 8, 2), f)
            .unwrap()
            .card;
        assert!((card.width * 1.5 - 0.85 * 810.0).abs() < 0.01, "{card:?}");
        let middle = (card.x + card.width / 2.0) * 1.5 + 1920.0;
        assert!((middle - (2100.0 + 720.0)).abs() < 0.01);
        let top = card.y * 1.5;
        assert!((top - (150.0 + 135.0 + 8.0 * 1.5)).abs() < 0.01, "{card:?}");
    }

    #[test]
    fn a_game_window_smaller_than_the_card_keeps_a_card_inside_it() {
        let game = client(0.0, 0.0, 300.0, 200.0);
        let board = default_area((300.0, 200.0));
        let f = frame((0.0, 0.0), 1.0, (300.0, 200.0));
        let card = hover_at(game, board, 8, 1, middle_of(game, board, 8, 2), f)
            .unwrap()
            .card;
        assert!(card.x >= 0.0 && card.x + card.width <= 300.0, "{card:?}");
        assert!(card.width > 0.0);
    }

    #[test]
    fn in_a_taller_window_the_board_follows_the_picture_not_the_window() {
        // 4:3: the picture is 1440x810, centred with 135 px bars above and below.
        let game = client(0.0, 0.0, 1440.0, 1080.0);
        let board = default_area((1440.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1440.0, 1040.0));
        let card = hover_at(game, board, 8, 1, middle_of(game, board, 8, 1), f)
            .unwrap()
            .card;
        assert!((card.width - 0.85 * 810.0).abs() < 0.01, "{card:?}");
        assert!((card.x + card.width / 2.0 - 720.0).abs() < 0.01);
        assert_eq!(card.y, 135.0 + GAP, "at the top of the picture");
    }

    #[test]
    fn the_board_is_sized_to_the_game_at_4k_150_and_at_1080p() {
        // 3840x2160 at 150%: the overlay's pixels are 2/3 of the screen's.
        let game = client(0.0, 0.0, 3840.0, 2160.0);
        let board = default_area((3840.0, 2160.0));
        let f = frame((0.0, 0.0), 1.5, (2560.0, 1408.0));
        let hover = hover_at(game, board, 8, 1, middle_of(game, board, 8, 2), f).unwrap();
        assert!(
            (hover.card.width - 0.85 * 1440.0).abs() < 0.01,
            "{:?}",
            hover.card
        );
        assert!((hover.card.x + hover.card.width / 2.0 - 1280.0).abs() < 0.01);
        // 1920x1080 at 100%: the same share of the screen.
        let game = client(0.0, 0.0, 1920.0, 1080.0);
        let board = default_area((1920.0, 1080.0));
        let f = frame((0.0, 0.0), 1.0, (1920.0, 1040.0));
        let hover = hover_at(game, board, 8, 1, middle_of(game, board, 8, 2), f).unwrap();
        assert!((hover.card.width - 0.85 * 1080.0).abs() < 0.01);
        // A narrow window keeps it inside the game.
        let game = client(0.0, 0.0, 1000.0, 1080.0);
        let board = default_area((1000.0, 1080.0));
        let hover = hover_at(game, board, 8, 1, middle_of(game, board, 8, 2), f).unwrap();
        assert!(hover.card.x >= 0.0 && hover.card.x + hover.card.width <= 1000.0);
    }
}
