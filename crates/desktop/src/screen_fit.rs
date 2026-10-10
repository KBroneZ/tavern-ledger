//! Keeping the overlay on screen (T-308): which monitor the overlay covers,
//! and where the hover card goes so it never leaves that monitor's work
//! area. Pure rectangles, no window code, so every rule here has a test.

use serde::{Deserialize, Serialize};

/// A rectangle: top-left corner, width and height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Area {
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    pub fn finite(&self) -> bool {
        [self.x, self.y, self.w, self.h]
            .iter()
            .all(|n| n.is_finite())
    }

    /// A rectangle with a real size and finite numbers.
    pub fn usable(&self) -> bool {
        self.finite() && self.w >= 1.0 && self.h >= 1.0
    }

    pub fn contains(&self, (px, py): (f64, f64)) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }

    fn overlap(&self, other: &Area) -> f64 {
        let w = self.right().min(other.right()) - self.x.max(other.x);
        let h = self.bottom().min(other.bottom()) - self.y.max(other.y);
        if w > 0.0 && h > 0.0 {
            w * h
        } else {
            0.0
        }
    }

    /// The same rectangle in another unit: `(self - origin) / scale`.
    pub fn in_frame(self, origin: (f64, f64), scale: f64) -> Area {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        Area {
            x: (self.x - origin.0) / scale,
            y: (self.y - origin.1) / scale,
            w: self.w / scale,
            h: self.h / scale,
        }
    }
}

/// A monitor as the system reports it, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub whole: Area,
    /// Without the taskbar and docked bars.
    pub work: Area,
    pub scale: f64,
    pub primary: bool,
}

/// The monitor the overlay covers: the one holding most of the game's
/// window; with no game window known (or one off every screen), the primary
/// one, else the first. `None` only when there is no monitor at all.
pub fn pick_screen(screens: &[Screen], game: Option<Area>) -> Option<usize> {
    let by_game = game.filter(Area::usable).and_then(|g| {
        screens
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.whole.overlap(&g)))
            .filter(|(_, overlap)| *overlap > 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    });
    by_game
        .or_else(|| screens.iter().position(|s| s.primary))
        .or_else(|| (!screens.is_empty()).then_some(0))
}

/// The rectangle the overlay window covers on this monitor, in physical
/// pixels, and the monitor's scale: the work area, or the whole monitor when
/// the work area has no size; a scale that is not a number is taken as 1.
pub fn overlay_area(screen: &Screen) -> (Area, f64) {
    let area = if screen.work.usable() {
        screen.work
    } else {
        screen.whole
    };
    let scale = if screen.scale.is_finite() && screen.scale > 0.0 {
        screen.scale
    } else {
        1.0
    };
    (area, scale)
}

/// Space kept between the card and the game window's or the screen's edges.
pub const GAP: f64 = 8.0;
/// The card is never narrower than this, even on a tiny screen.
const MIN_CARD_WIDTH: f64 = 120.0;

/// Where the hover card goes, in the overlay window's own pixels: its
/// top-left corner, its width and the most height it may take before it is
/// cut (the page measures the rest).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct CardPlace {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub max_height: f64,
}

/// Puts a card of `want_width` at the top of `game` (the game window's
/// client area), centred on it, like the board strip the game itself shows
/// (T-309, D-049). It stays inside `bounds` (the overlay window, which covers
/// the monitor holding the game): narrower than the game window or the
/// screen when it must be, moved in when the game window reaches past the
/// screen's edge.
pub fn place_card(game: Area, bounds: Area, want_width: f64) -> CardPlace {
    let inner_left = bounds.x + GAP;
    let inner_right = bounds.right() - GAP;
    let screen_room = (inner_right - inner_left).max(MIN_CARD_WIDTH.min(bounds.w));
    let game_room = if game.usable() {
        game.w - 2.0 * GAP
    } else {
        screen_room
    };
    let width = want_width
        .max(MIN_CARD_WIDTH)
        .min(game_room.max(MIN_CARD_WIDTH))
        .min(screen_room);
    let center = if game.usable() {
        game.x + game.w / 2.0
    } else {
        bounds.x + bounds.w / 2.0
    };
    let x = (center - width / 2.0).clamp(bounds.x, (bounds.right() - width).max(bounds.x));
    let top = if game.usable() { game.y } else { bounds.y };
    // `clamp` needs its lowest bound at most its highest, even on a window
    // shorter than two gaps.
    let highest = bounds.y + GAP.min(bounds.h);
    let lowest = (bounds.bottom() - GAP).max(highest);
    let y = (top + GAP).clamp(highest, lowest);
    CardPlace {
        x,
        y,
        width,
        max_height: (bounds.bottom() - GAP - y).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: f64, y: f64, w: f64, h: f64) -> Area {
        Area { x, y, w, h }
    }

    fn screen(x: f64, w: f64, h: f64, primary: bool) -> Screen {
        Screen {
            whole: area(x, 0.0, w, h),
            work: area(x, 0.0, w, h - 48.0),
            scale: 1.0,
            primary,
        }
    }

    const SCREEN: Area = Area {
        x: 0.0,
        y: 0.0,
        w: 1920.0,
        h: 1080.0,
    };

    #[test]
    fn the_overlay_goes_on_the_monitor_that_holds_most_of_the_game() {
        let screens = [
            screen(0.0, 1920.0, 1080.0, true),
            screen(1920.0, 3840.0, 2160.0, false),
        ];
        assert_eq!(
            pick_screen(&screens, Some(area(2000.0, 0.0, 3840.0, 2160.0))),
            Some(1)
        );
        // Straddling: the bigger share wins.
        assert_eq!(
            pick_screen(&screens, Some(area(1500.0, 0.0, 1000.0, 600.0))),
            Some(1)
        );
        assert_eq!(
            pick_screen(&screens, Some(area(100.0, 0.0, 1280.0, 720.0))),
            Some(0)
        );
    }

    #[test]
    fn with_no_game_window_or_one_off_screen_it_is_the_primary_monitor() {
        let screens = [
            screen(-1920.0, 1920.0, 1080.0, false),
            screen(0.0, 2560.0, 1440.0, true),
        ];
        assert_eq!(pick_screen(&screens, None), Some(1));
        assert_eq!(
            pick_screen(&screens, Some(area(90000.0, 0.0, 800.0, 600.0))),
            Some(1)
        );
        assert_eq!(
            pick_screen(&screens, Some(area(f64::NAN, 0.0, 800.0, 600.0))),
            Some(1)
        );
        let no_primary = [screen(0.0, 800.0, 600.0, false)];
        assert_eq!(pick_screen(&no_primary, None), Some(0));
        assert_eq!(pick_screen(&[], None), None);
    }

    #[test]
    fn the_overlay_covers_the_work_area_not_the_taskbar() {
        let s = screen(0.0, 3840.0, 2160.0, true);
        assert_eq!(overlay_area(&s), (s.work, 1.0));
        let broken = Screen {
            work: Area::default(),
            scale: f64::NAN,
            ..s
        };
        assert_eq!(overlay_area(&broken), (s.whole, 1.0));
    }

    const UHD: Area = Area {
        x: 0.0,
        y: 0.0,
        w: 3840.0,
        h: 2112.0,
    };

    fn inside(p: &CardPlace, bounds: Area) -> bool {
        p.x >= bounds.x
            && p.x + p.width <= bounds.right()
            && p.y >= bounds.y
            && p.y + p.max_height <= bounds.bottom()
    }

    #[test]
    fn the_card_is_centred_at_the_top_of_a_full_screen_game() {
        for (bounds, game) in [
            (SCREEN, area(0.0, 0.0, 1920.0, 1080.0)),
            (UHD, area(0.0, 0.0, 3840.0, 2160.0)),
        ] {
            let p = place_card(game, bounds, 640.0);
            assert_eq!(p.width, 640.0);
            assert_eq!(p.x + p.width / 2.0, game.w / 2.0, "centred");
            assert_eq!(p.y, GAP);
            assert!(inside(&p, bounds), "{p:?}");
        }
    }

    #[test]
    fn a_window_away_from_the_origin_gets_the_card_at_its_own_top() {
        let game = area(500.0, 200.0, 1280.0, 720.0);
        let p = place_card(game, SCREEN, 640.0);
        assert_eq!(p.x, 500.0 + 640.0 - 320.0);
        assert_eq!(p.y, 200.0 + GAP);
        assert_eq!(p.max_height, 1080.0 - GAP - p.y);
    }

    #[test]
    fn a_small_window_shrinks_the_card_to_it() {
        let game = area(100.0, 100.0, 400.0, 300.0);
        let p = place_card(game, SCREEN, 640.0);
        assert_eq!(p.width, 400.0 - 2.0 * GAP);
        assert_eq!(p.x, 100.0 + GAP);
        assert!(inside(&p, SCREEN));
    }

    #[test]
    fn a_card_wider_than_the_screen_is_cut_to_the_screen() {
        let tiny = area(0.0, 0.0, 300.0, 200.0);
        let p = place_card(area(0.0, 0.0, 300.0, 200.0), tiny, 5000.0);
        assert!(inside(&p, tiny), "{p:?}");
        let p = place_card(
            area(0.0, 0.0, 90.0, 60.0),
            area(0.0, 0.0, 90.0, 60.0),
            640.0,
        );
        assert!(p.x >= 0.0 && p.x + p.width <= 90.0, "{p:?}");
    }

    #[test]
    fn a_window_past_the_screen_edge_keeps_the_card_on_screen() {
        // Mostly on this screen, but reaching past its left and top edges.
        let game = area(-300.0, -100.0, 1280.0, 720.0);
        let p = place_card(game, SCREEN, 640.0);
        assert!(inside(&p, SCREEN), "{p:?}");
        assert_eq!(p.y, GAP);
        // Past the right edge.
        let p = place_card(area(1500.0, 0.0, 1280.0, 720.0), SCREEN, 640.0);
        assert_eq!(p.x + p.width, 1920.0);
    }

    #[test]
    fn on_a_second_scaled_monitor_the_card_is_in_that_overlay_s_pixels() {
        // Game full screen on a 4K monitor at 150% right of a 1080p primary:
        // the overlay covers that monitor; the game is given in its pixels.
        let bounds = area(0.0, 0.0, 2560.0, 1408.0);
        let game = area(1920.0, 0.0, 3840.0, 2160.0).in_frame((1920.0, 0.0), 1.5);
        let p = place_card(game, bounds, 640.0);
        assert_eq!(p.x, 1280.0 - 320.0);
        assert_eq!(p.y, GAP);
        assert!(inside(&p, bounds));
    }

    #[test]
    fn a_window_shorter_than_two_gaps_does_not_break_the_placement() {
        let flat = area(0.0, 0.0, 400.0, 10.0);
        let p = place_card(area(0.0, 0.0, 400.0, 10.0), flat, 640.0);
        assert_eq!(p.y, GAP);
        assert_eq!(p.max_height, 0.0);
    }

    #[test]
    fn with_no_usable_game_window_the_card_is_centred_on_the_screen() {
        let p = place_card(area(f64::NAN, 0.0, 0.0, 0.0), SCREEN, 640.0);
        assert_eq!(p.x, 640.0);
        assert_eq!(p.y, GAP);
    }

    #[test]
    fn a_bad_scale_is_taken_as_one() {
        let a = area(10.0, 10.0, 10.0, 10.0);
        assert_eq!(a.in_frame((0.0, 0.0), 0.0), a);
        assert_eq!(a.in_frame((0.0, 0.0), f64::NAN), a);
    }
}
