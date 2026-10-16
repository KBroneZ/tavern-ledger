//! Keeping the overlay on screen (T-308, T-349): which monitor the overlay
//! covers, which part of it (the game's picture, not its title bar), and
//! where the hover card goes so it never leaves that monitor's work area. Pure rectangles, no window code, so every rule here has a test.

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

    /// The part both rectangles share, `None` when they only touch or miss.
    pub fn intersection(&self, other: &Area) -> Option<Area> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let shared = Area {
            x,
            y,
            w: self.right().min(other.right()) - x,
            h: self.bottom().min(other.bottom()) - y,
        };
        shared.usable().then_some(shared)
    }

    /// The size in a window's own (logical) pixels at this scale; a scale
    /// that is not a number is taken as 1.
    pub fn logical_size(&self, scale: f64) -> (f64, f64) {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        (self.w / scale, self.h / scale)
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

/// The least of the game's picture (physical pixels) worth covering: a
/// window dragged mostly off the monitor leaves the whole work area instead,
/// so the layout is never saved for, or scaled from, a sliver.
pub const MIN_COVER: (f64, f64) = (320.0, 180.0);

/// The rectangle the overlay window covers on this monitor, in physical
/// pixels, and the monitor's scale. It is the game's client area (its picture,
/// without title bar or borders: `GetClientRect` + `ClientToScreen`) cut to
/// the monitor's work area, so every overlay piece is placed inside the game
/// in windowed, borderless and full-screen modes alike (T-349). With no
/// usable game window, or one that misses this monitor, it is the work area,
/// or the whole monitor when the work area has no size. A scale that is not a
/// number is taken as 1.
pub fn overlay_area(screen: &Screen, game: Option<Area>) -> (Area, f64) {
    let monitor = if screen.work.usable() {
        screen.work
    } else {
        screen.whole
    };
    let area = game
        .filter(Area::usable)
        .and_then(|client| monitor.intersection(&client))
        .filter(|shown| shown.w >= MIN_COVER.0 && shown.h >= MIN_COVER.1)
        .unwrap_or(monitor);
    let scale = if screen.scale.is_finite() && screen.scale > 0.0 {
        screen.scale
    } else {
        1.0
    };
    (area, scale)
}

/// The game's picture is 16:9. In a window of another shape it is centred with
/// bars, and every default place of the overlay is a share of this area.
pub const PLAY_SHAPE: (f64, f64) = (16.0, 9.0);

/// The part of a window the game's 16:9 picture takes: the whole window when
/// it is 16:9, centred with bars on a wider (ultrawide) or taller one. A
/// window with no usable size is returned as it is.
pub fn play_area(window: Area) -> Area {
    if !window.usable() {
        return window;
    }
    // Products, not a ratio, so a window that is exactly 16:9 stays exact.
    let w = if window.w * PLAY_SHAPE.1 <= window.h * PLAY_SHAPE.0 {
        window.w
    } else {
        window.h * PLAY_SHAPE.0 / PLAY_SHAPE.1
    };
    let h = w * PLAY_SHAPE.1 / PLAY_SHAPE.0;
    Area {
        x: window.x + (window.w - w) / 2.0,
        y: window.y + (window.h - h) / 2.0,
        w,
        h,
    }
}

/// Space kept between the card and the game window's or the screen's edges.
pub const GAP: f64 = 8.0;
/// The card is never narrower than this, even on a tiny screen.
pub const MIN_CARD_WIDTH: f64 = 120.0;

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

/// Puts a card of `want_width` at the top of `game` (the game's 16:9 picture,
/// `play_area` of its client area), centred on it, like the board strip the game itself shows
/// (T-309, D-049). It stays inside `bounds` (the overlay window, which covers
/// the game's picture on the monitor holding it): narrower than the game window or the
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
    fn the_play_area_is_the_whole_window_when_it_is_16_by_9() {
        for (w, h) in [(1920.0, 1080.0), (1280.0, 720.0), (2560.0, 1440.0)] {
            assert_eq!(play_area(area(0.0, 0.0, w, h)), area(0.0, 0.0, w, h));
        }
    }

    #[test]
    fn on_an_ultrawide_window_the_play_area_is_centred_with_bars_at_the_sides() {
        let p = play_area(area(0.0, 0.0, 3440.0, 1440.0));
        assert_eq!(p, area(440.0, 0.0, 2560.0, 1440.0));
    }

    #[test]
    fn on_a_taller_window_the_play_area_is_centred_with_bars_above_and_below() {
        let p = play_area(area(0.0, 0.0, 1024.0, 768.0));
        assert_eq!(p, area(0.0, 96.0, 1024.0, 576.0));
    }

    #[test]
    fn the_play_area_follows_a_window_away_from_the_origin() {
        let p = play_area(area(1920.0, 100.0, 3440.0, 1440.0));
        assert_eq!(p, area(2360.0, 100.0, 2560.0, 1440.0));
    }

    #[test]
    fn a_window_with_no_size_has_no_play_area_of_its_own() {
        let none = area(0.0, 0.0, 0.0, 0.0);
        assert_eq!(play_area(none), none);
    }

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
        assert_eq!(overlay_area(&s, None), (s.work, 1.0));
        let broken = Screen {
            work: Area::default(),
            scale: f64::NAN,
            ..s
        };
        assert_eq!(overlay_area(&broken, None), (s.whole, 1.0));
    }

    /// What `GetClientRect` + `ClientToScreen` give for a framed window: its
    /// rectangle less the title bar (31 px at 100 %) and the borders (8 px).
    fn framed_client(window: Area, scale: f64) -> Area {
        let (title, border) = (31.0 * scale, 8.0 * scale);
        area(
            window.x + border,
            window.y + title,
            window.w - 2.0 * border,
            window.h - title - border,
        )
    }

    #[test]
    fn a_windowed_game_gets_the_overlay_on_its_picture_not_its_title_bar() {
        let s = screen(0.0, 1920.0, 1080.0, true);
        let window = area(100.0, 80.0, 1296.0, 759.0);
        let client = framed_client(window, 1.0);
        let (covered, scale) = overlay_area(&s, Some(client));
        assert_eq!((covered, scale), (client, 1.0));
        assert!(covered.y >= window.y + 31.0, "below the title bar");
        assert!(covered.x > window.x && covered.right() < window.right());
    }

    #[test]
    fn a_borderless_game_is_covered_whole_and_a_full_screen_one_as_before() {
        let s = screen(0.0, 1920.0, 1080.0, true);
        // Borderless on the monitor: the client is the monitor, the taskbar
        // is still left free.
        assert_eq!(overlay_area(&s, Some(s.whole)).0, s.work);
        // Full screen with the taskbar off the screen's edge: unchanged.
        assert_eq!(overlay_area(&s, Some(s.whole)), overlay_area(&s, None));
        // A bigger client than the monitor (game 4K on a 1080p screen) is cut.
        assert_eq!(
            overlay_area(&s, Some(area(0.0, 0.0, 3840.0, 2160.0))).0,
            s.work
        );
    }

    #[test]
    fn a_game_window_reaching_off_the_monitor_is_cut_to_what_is_on_it() {
        let s = screen(0.0, 1920.0, 1080.0, true);
        let (covered, _) = overlay_area(&s, Some(area(-300.0, -100.0, 1280.0, 720.0)));
        assert_eq!(covered, area(0.0, 0.0, 980.0, 620.0));
        let (covered, _) = overlay_area(&s, Some(area(1500.0, 800.0, 1280.0, 720.0)));
        assert_eq!(covered, area(1500.0, 800.0, 420.0, 232.0));
    }

    #[test]
    fn a_game_window_that_cannot_be_used_leaves_the_whole_work_area() {
        let s = screen(0.0, 1920.0, 1080.0, true);
        for game in [
            area(f64::NAN, 0.0, 800.0, 600.0),
            area(10.0, 10.0, 0.0, 0.0),
            // Entirely on another monitor.
            area(5000.0, 0.0, 800.0, 600.0),
        ] {
            assert_eq!(overlay_area(&s, Some(game)).0, s.work, "{game:?}");
        }
    }

    #[test]
    fn at_125_and_150_percent_the_overlay_keeps_the_client_in_physical_pixels() {
        for scale in [1.25, 1.5] {
            let s = Screen {
                scale,
                ..screen(0.0, 2560.0, 1440.0, true)
            };
            let window = area(160.0, 120.0, 1600.0, 900.0);
            let client = framed_client(window, scale);
            let (covered, got) = overlay_area(&s, Some(client));
            assert_eq!((covered, got), (client, scale));
            // The page works in logical pixels: the same rectangle divided.
            let (lw, lh) = covered.logical_size(got);
            assert_eq!((lw, lh), (client.w / scale, client.h / scale));
            // A hover inside the client lands inside the overlay's page.
            let inside = (client.x + client.w / 2.0, client.y + 10.0);
            let page = covered.in_frame((covered.x, covered.y), got);
            assert!(page.contains(((inside.0 - covered.x) / got, (inside.1 - covered.y) / got)));
        }
    }

    #[test]
    fn a_game_on_a_second_monitor_at_negative_coordinates_is_covered_there() {
        let left = Screen {
            scale: 1.5,
            ..screen(-2560.0, 2560.0, 1440.0, false)
        };
        let main = screen(0.0, 1920.0, 1080.0, true);
        let screens = [main, left];
        let client = framed_client(area(-2200.0, 100.0, 1600.0, 900.0), 1.5);
        let index = pick_screen(&screens, Some(client)).unwrap();
        assert_eq!(index, 1);
        let (covered, scale) = overlay_area(&screens[index], Some(client));
        assert_eq!((covered, scale), (client, 1.5));
        assert!(covered.x < 0.0);
        // Straddling the two monitors: the overlay stays on the one with most
        // of the game, and goes no further than its edge.
        let straddle = area(-600.0, 100.0, 1280.0, 720.0);
        let index = pick_screen(&screens, Some(straddle)).unwrap();
        assert_eq!(index, 0);
        let (covered, _) = overlay_area(&screens[index], Some(straddle));
        assert_eq!(covered.x, 0.0);
        assert_eq!(covered.w, 680.0);
    }

    #[test]
    fn a_sliver_of_the_game_left_on_the_monitor_leaves_the_whole_work_area() {
        let s = screen(0.0, 1920.0, 1080.0, true);
        // Dragged almost off the left edge: 60 px of the game still show.
        let sliver = area(-1220.0, 100.0, 1280.0, 720.0);
        assert_eq!(overlay_area(&s, Some(sliver)).0, s.work);
        // Just big enough to keep.
        let enough = area(0.0, 0.0, MIN_COVER.0, MIN_COVER.1);
        assert_eq!(overlay_area(&s, Some(enough)).0, enough);
        let short = area(0.0, 0.0, MIN_COVER.0, MIN_COVER.1 - 1.0);
        assert_eq!(overlay_area(&s, Some(short)).0, s.work);
    }

    #[test]
    fn a_game_only_over_the_taskbar_strip_leaves_the_whole_work_area() {
        let s = screen(0.0, 1920.0, 1080.0, true);
        let on_taskbar = area(100.0, 1040.0, 1280.0, 40.0);
        assert_eq!(pick_screen(&[s], Some(on_taskbar)), Some(0));
        assert_eq!(overlay_area(&s, Some(on_taskbar)).0, s.work);
    }

    #[test]
    fn rectangles_that_only_touch_or_miss_share_nothing() {
        let a = area(0.0, 0.0, 100.0, 100.0);
        assert_eq!(a.intersection(&area(100.0, 0.0, 50.0, 50.0)), None);
        assert_eq!(a.intersection(&area(50.0, 100.0, 50.0, 50.0)), None);
        assert_eq!(a.intersection(&area(0.5, 0.5, 0.2, 0.2)), None);
        assert_eq!(
            a.intersection(&area(99.0, 99.0, 50.0, 50.0)),
            Some(area(99.0, 99.0, 1.0, 1.0))
        );
    }

    #[test]
    fn the_overlay_window_size_is_the_covered_area_in_logical_pixels() {
        let a = area(10.0, 20.0, 1500.0, 900.0);
        assert_eq!(a.logical_size(1.5), (1000.0, 600.0));
        assert_eq!(a.logical_size(0.0), (1500.0, 900.0));
        assert_eq!(a.logical_size(f64::NAN), (1500.0, 900.0));
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
