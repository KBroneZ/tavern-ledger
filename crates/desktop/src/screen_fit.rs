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

/// Space kept between the leaderboard and the card, and from screen edges.
pub const GAP: f64 = 8.0;
/// The card is never narrower than this, even on a tiny screen.
const MIN_CARD_WIDTH: f64 = 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Right,
    Left,
}

/// Where the hover card goes, in the overlay window's own pixels. The page
/// measures the card's height and puts it at `center_y`, moved to stay
/// between `top` and `bottom`; taller than that, it is cut to that height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct CardPlace {
    pub x: f64,
    pub width: f64,
    pub side: Side,
    pub top: f64,
    pub bottom: f64,
    pub center_y: f64,
}

/// Puts a card of `want_width` next to `anchor` (the hovered slot) inside
/// `bounds` (the overlay window): to its right when it fits, else to its
/// left, else on the side with more room, narrower.
pub fn place_card(anchor: Area, bounds: Area, want_width: f64) -> CardPlace {
    let inner_left = bounds.x + GAP;
    let inner_right = bounds.right() - GAP;
    let room_right = inner_right - (anchor.right() + GAP);
    let room_left = (anchor.x - GAP) - inner_left;
    let want = want_width.max(MIN_CARD_WIDTH);
    let (side, room) = if room_right >= want {
        (Side::Right, room_right)
    } else if room_left >= want {
        (Side::Left, room_left)
    } else if room_right >= room_left {
        (Side::Right, room_right)
    } else {
        (Side::Left, room_left)
    };
    let max_width = (inner_right - inner_left).max(MIN_CARD_WIDTH.min(bounds.w));
    let width = want.min(room.max(MIN_CARD_WIDTH)).min(max_width);
    let x = match side {
        Side::Right => anchor.right() + GAP,
        Side::Left => anchor.x - GAP - width,
    };
    let x = x.clamp(bounds.x, (bounds.right() - width).max(bounds.x));
    let top = bounds.y + GAP;
    let bottom = (bounds.bottom() - GAP).max(top);
    CardPlace {
        x,
        width,
        side,
        top,
        bottom,
        center_y: (anchor.y + anchor.h / 2.0).clamp(top, bottom),
    }
}

#[cfg(test)]
/// The top of a card of `height` centred on `place.center_y`, moved to stay
/// inside `top..bottom` (the page does the same with the measured height).
pub fn card_top(place: &CardPlace, height: f64) -> f64 {
    let height = height.min(place.bottom - place.top);
    (place.center_y - height / 2.0).clamp(place.top, place.bottom - height)
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

    #[test]
    fn the_card_goes_right_of_the_leaderboard_when_it_fits() {
        let slot = area(200.0, 400.0, 80.0, 90.0);
        let p = place_card(slot, SCREEN, 360.0);
        assert_eq!(p.side, Side::Right);
        assert_eq!(p.x, 280.0 + GAP);
        assert_eq!(p.width, 360.0);
        assert_eq!(p.center_y, 445.0);
    }

    #[test]
    fn the_card_flips_left_when_the_right_has_no_room() {
        let slot = area(1700.0, 400.0, 80.0, 90.0);
        let p = place_card(slot, SCREEN, 360.0);
        assert_eq!(p.side, Side::Left);
        assert_eq!(p.x, 1700.0 - GAP - 360.0);
        assert!(p.x >= 0.0);
    }

    #[test]
    fn with_room_on_neither_side_the_card_shrinks_but_stays_on_screen() {
        let small = area(0.0, 0.0, 500.0, 400.0);
        let slot = area(150.0, 100.0, 80.0, 60.0);
        let p = place_card(slot, small, 360.0);
        assert!(p.width < 360.0);
        assert!(p.x >= small.x && p.x + p.width <= small.right(), "{p:?}");
    }

    #[test]
    fn a_card_wider_than_the_screen_is_cut_to_the_screen() {
        let tiny = area(0.0, 0.0, 300.0, 200.0);
        let p = place_card(area(10.0, 10.0, 20.0, 20.0), tiny, 5000.0);
        assert!(p.x >= 0.0 && p.x + p.width <= 300.0, "{p:?}");
    }

    #[test]
    fn the_card_never_goes_above_or_below_the_screen() {
        let p = place_card(area(200.0, 1050.0, 80.0, 90.0), SCREEN, 360.0);
        let top = card_top(&p, 500.0);
        assert!(top >= GAP && top + 500.0 <= 1080.0 - GAP, "{top}");
        let p = place_card(area(200.0, -40.0, 80.0, 60.0), SCREEN, 360.0);
        assert_eq!(card_top(&p, 300.0), GAP);
        // Taller than the screen: cut to it, starting at the top.
        let p = place_card(area(200.0, 500.0, 80.0, 60.0), SCREEN, 360.0);
        assert_eq!(card_top(&p, 5000.0), GAP);
    }

    #[test]
    fn a_monitor_left_of_the_primary_works_in_its_own_pixels() {
        let bounds = area(0.0, 0.0, 1280.0, 1032.0);
        let game_slot = area(-1700.0, 500.0, 80.0, 90.0).in_frame((-1920.0, 0.0), 1.5);
        assert!((game_slot.x - 146.666).abs() < 0.01);
        let p = place_card(game_slot, bounds, 360.0);
        assert!(p.x + p.width <= bounds.right());
    }

    #[test]
    fn a_bad_scale_is_taken_as_one() {
        let a = area(10.0, 10.0, 10.0, 10.0);
        assert_eq!(a.in_frame((0.0, 0.0), 0.0), a);
        assert_eq!(a.in_frame((0.0, 0.0), f64::NAN), a);
    }
}
