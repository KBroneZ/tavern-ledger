//! Where the game's leaderboard is on screen (T-306): the column of hero
//! portraits at the left of the game. Worked out from the game window's
//! client rectangle only; nothing is read from the game (D-004, D-006).
//!
//! The game scales its layout with the window's height and centres it, so
//! the leaderboard is a fixed share of the height, left of the centre. The
//! numbers below were measured on the user's own screenshots at 3840x2160
//! (D-045); a patch can move the leaderboard, so the user can drag a box over
//! it instead (saved per window size in `overlay.json`).
//!
//! All rectangles here are relative to the client area's top-left corner,
//! in physical pixels.

use crate::screen_fit::Area;

/// Left edge of the portraits, as a share of the height left of the centre.
pub const LEFT_FROM_CENTER: f64 = 0.665;
/// Width of the portrait column, as a share of the height.
pub const WIDTH: f64 = 0.085;
/// Top of the first slot, as a share of the height.
pub const TOP: f64 = 0.16;
/// Height of all slots together, as a share of the height.
pub const HEIGHT: f64 = 0.68;

/// A box the user draws is at least this big (physical pixels).
pub const MIN_W: f64 = 16.0;
pub const MIN_H: f64 = 48.0;

/// The leaderboard of a client area of this size, by the measured shares.
/// Kept inside the client area (a window narrower than 4:3 cuts it).
pub fn default_area(client: (f64, f64)) -> Area {
    let (w, h) = client;
    let left = w / 2.0 - LEFT_FROM_CENTER * h;
    let area = Area {
        x: left,
        y: TOP * h,
        w: WIDTH * h,
        h: HEIGHT * h,
    };
    fit_area(area, client).unwrap_or_default()
}

/// A box made to fit the client area, at least [`MIN_W`] by [`MIN_H`]
/// (less only when the window itself is smaller). `None` when its numbers
/// are not numbers or the client area has no size.
pub fn fit_area(area: Area, client: (f64, f64)) -> Option<Area> {
    let (cw, ch) = client;
    if !(area.finite() && cw.is_finite() && ch.is_finite() && cw >= 1.0 && ch >= 1.0) {
        return None;
    }
    let w = area.w.clamp(MIN_W.min(cw), cw);
    let h = area.h.clamp(MIN_H.min(ch), ch);
    Some(Area {
        x: area.x.clamp(0.0, cw - w),
        y: area.y.clamp(0.0, ch - h),
        w,
        h,
    })
}

/// The place (1 is the top slot) under `point`, with the leaderboard split
/// into `slots` equal slots. `None` outside it or with no slots.
pub fn slot_at(area: Area, slots: i64, point: (f64, f64)) -> Option<i64> {
    if slots < 1 || !area.usable() || !area.contains(point) {
        return None;
    }
    let share = (point.1 - area.y) / area.h;
    let place = (share * slots as f64).floor() as i64 + 1;
    Some(place.clamp(1, slots))
}

/// The rectangle of one slot (place 1 is the top one).
pub fn slot_area(area: Area, slots: i64, place: i64) -> Area {
    let slots = slots.max(1);
    let each = area.h / slots as f64;
    Area {
        x: area.x,
        y: area.y + each * (place.clamp(1, slots) - 1) as f64,
        w: area.w,
        h: each,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UHD: (f64, f64) = (3840.0, 2160.0);
    const FHD: (f64, f64) = (1920.0, 1080.0);
    const HD: (f64, f64) = (1280.0, 720.0);

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.5
    }

    /// Measured on the user's 3840x2160 screenshots (D-045): the portraits'
    /// left edge near x 505, the first slot near y 350, slots about 185 px
    /// apart, the eighth ending near y 1800.
    #[test]
    fn the_default_matches_the_measured_leaderboard_at_4k() {
        let a = default_area(UHD);
        assert!(close(a.x, 3840.0 / 2.0 - 0.665 * 2160.0), "{a:?}");
        assert!((480.0..=530.0).contains(&a.x), "{a:?}");
        assert!((330.0..=370.0).contains(&a.y), "{a:?}");
        assert!((1780.0..=1830.0).contains(&a.bottom()), "{a:?}");
        let pitch = slot_area(a, 8, 2).y - slot_area(a, 8, 1).y;
        assert!((175.0..=195.0).contains(&pitch), "{pitch}");
    }

    #[test]
    fn the_default_scales_with_the_window_height() {
        let uhd = default_area(UHD);
        for size in [FHD, HD] {
            let k = size.1 / UHD.1;
            let a = default_area(size);
            assert!(close(a.y, uhd.y * k) && close(a.w, uhd.w * k) && close(a.h, uhd.h * k));
            assert!(
                close(a.x, uhd.x * k),
                "16:9 keeps the same share of the width"
            );
        }
    }

    #[test]
    fn a_wide_window_keeps_the_leaderboard_by_the_centre() {
        let a = default_area((2560.0, 1080.0));
        let b = default_area(FHD);
        assert!(close(a.x - b.x, (2560.0 - 1920.0) / 2.0));
        assert!(close(a.y, b.y) && close(a.h, b.h));
    }

    #[test]
    fn a_narrow_window_cuts_the_leaderboard_at_its_edge() {
        let a = default_area((800.0, 1000.0));
        assert!(a.x >= 0.0 && a.right() <= 800.0, "{a:?}");
    }

    #[test]
    fn each_place_has_its_slot_from_the_top() {
        let a = default_area(FHD);
        let mid = |place| {
            let s = slot_area(a, 8, place);
            (s.x + s.w / 2.0, s.y + s.h / 2.0)
        };
        for place in 1..=8 {
            assert_eq!(slot_at(a, 8, mid(place)), Some(place));
        }
        assert_eq!(
            slot_at(a, 4, mid(1)),
            Some(1),
            "Duos: a team holds two portraits"
        );
        assert_eq!(slot_at(a, 4, mid(2)), Some(1));
        assert_eq!(slot_at(a, 4, mid(8)), Some(4));
    }

    #[test]
    fn outside_the_leaderboard_there_is_no_slot() {
        let a = default_area(FHD);
        assert_eq!(slot_at(a, 8, (a.x - 1.0, a.y + 10.0)), None);
        assert_eq!(slot_at(a, 8, (a.right() + 1.0, a.y + 10.0)), None);
        assert_eq!(slot_at(a, 8, (a.x + 5.0, a.y - 1.0)), None);
        assert_eq!(slot_at(a, 8, (a.x + 5.0, a.bottom())), None);
        assert_eq!(
            slot_at(a, 0, (a.x + 5.0, a.y + 5.0)),
            None,
            "no slots known"
        );
    }

    #[test]
    fn a_box_the_user_draws_is_made_to_fit_the_window() {
        let a = fit_area(
            Area {
                x: -50.0,
                y: 5000.0,
                w: 2.0,
                h: 9999.0,
            },
            FHD,
        )
        .unwrap();
        assert!(a.x >= 0.0 && a.y >= 0.0 && a.right() <= 1920.0 && a.bottom() <= 1080.0);
        assert_eq!((a.w, a.h), (MIN_W, 1080.0));
        let bad = Area {
            x: f64::NAN,
            ..Area::default()
        };
        assert_eq!(fit_area(bad, FHD), None);
        assert_eq!(fit_area(Area::default(), (0.0, 0.0)), None);
    }
}
