//! Is Hearthstone the window in front? Answered from the foreground window's
//! title and class only (D-032): no handle is opened on the game's process,
//! nothing is read from it and nothing is sent to it (D-004, D-006). For the
//! leaderboard hover (T-306, D-045) the game window's client rectangle and the
//! mouse position are read too: where things are on screen, never what the
//! game draws.

use crate::screen_fit::Area;

/// What the window in front is, as far as its title and class say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foreground {
    Hearthstone,
    Other,
}

/// The game's window title.
const TITLE: &str = "Hearthstone";
/// The game is a Unity game; its window class does not change with the
/// language of the client, its title might. Another Unity game in front
/// would also match, which only means the overlay shows over it while a
/// Hearthstone game is on.
const CLASS: &str = "UnityWndClass";

pub fn classify(title: &str, class: &str) -> Foreground {
    if title.trim().eq_ignore_ascii_case(TITLE) || class == CLASS {
        Foreground::Hearthstone
    } else {
        Foreground::Other
    }
}

/// The window in front now. `None` when there is none (a moment while
/// switching windows) or when this system cannot say.
pub fn current() -> Option<Foreground> {
    win::foreground().map(|(_, kind)| kind)
}

/// The client area of the game's window when it is the window in front, in
/// physical screen pixels (T-306): its rectangle only, nothing else of it.
/// Stricter than [`classify`]: the title must be the game's, so another
/// Unity game in front never moves the overlay or its leaderboard.
pub fn game_client() -> Option<Area> {
    match win::foreground()? {
        (window, Foreground::Hearthstone) if is_game_title(&win::title(window)) => {
            win::client_area(window)
        }
        _ => None,
    }
}

fn is_game_title(title: &str) -> bool {
    title.trim().eq_ignore_ascii_case(TITLE)
}

/// `--overlay-dev` only: a window titled "Hearthstone", in front or not, so
/// a stand-in window can be used with a replayed log.
pub fn named_game_client() -> Option<Area> {
    win::client_area(win::find_by_title(TITLE)?)
}

/// Where the mouse pointer is, in physical screen pixels.
pub fn cursor() -> Option<(f64, f64)> {
    win::cursor()
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    use super::{classify, Foreground};
    use crate::screen_fit::Area;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetWindowTextW(window: *mut c_void, text: *mut u16, max_count: i32) -> i32;
        fn GetClassNameW(window: *mut c_void, text: *mut u16, max_count: i32) -> i32;
        fn GetClientRect(window: *mut c_void, rect: *mut Rect) -> i32;
        fn ClientToScreen(window: *mut c_void, point: *mut Point) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
    }

    /// Longer titles are cut: only the start is compared.
    const MAX: usize = 256;

    fn read(
        window: *mut c_void,
        get: unsafe extern "system" fn(*mut c_void, *mut u16, i32) -> i32,
    ) -> String {
        let mut buffer = [0u16; MAX];
        // SAFETY: `window` came from GetForegroundWindow and is only used for
        // this read; `buffer` holds MAX UTF-16 units and the call is told so.
        let len = unsafe { get(window, buffer.as_mut_ptr(), MAX as i32) };
        let len = usize::try_from(len).unwrap_or(0).min(MAX);
        String::from_utf16_lossy(&buffer[..len])
    }

    pub fn title(window: *mut c_void) -> String {
        read(window, GetWindowTextW)
    }

    pub fn foreground() -> Option<(*mut c_void, Foreground)> {
        // SAFETY: no arguments; returns a window handle or null.
        let window = unsafe { GetForegroundWindow() };
        if window.is_null() {
            return None;
        }
        let kind = classify(&read(window, GetWindowTextW), &read(window, GetClassNameW));
        Some((window, kind))
    }

    /// The window's client area on screen; `None` when the calls fail or it
    /// has no size (minimised).
    pub fn client_area(window: *mut c_void) -> Option<Area> {
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut origin = Point { x: 0, y: 0 };
        // SAFETY: `window` is a window handle the system just gave us; both
        // calls only write the structs passed, which live for the call. A
        // handle that closed meanwhile makes them fail, which is handled.
        let ok = unsafe {
            GetClientRect(window, &mut rect) != 0 && ClientToScreen(window, &mut origin) != 0
        };
        let area = Area {
            x: f64::from(origin.x),
            y: f64::from(origin.y),
            w: f64::from(rect.right - rect.left),
            h: f64::from(rect.bottom - rect.top),
        };
        (ok && area.usable()).then_some(area)
    }

    pub fn cursor() -> Option<(f64, f64)> {
        let mut point = Point { x: 0, y: 0 };
        // SAFETY: the call only writes the struct passed.
        let ok = unsafe { GetCursorPos(&mut point) } != 0;
        ok.then(|| (f64::from(point.x), f64::from(point.y)))
    }

    pub fn find_by_title(title: &str) -> Option<*mut c_void> {
        let wide: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the
        // call; a null class means any class.
        let window = unsafe { FindWindowW(std::ptr::null(), wide.as_ptr()) };
        (!window.is_null()).then_some(window)
    }
}

#[cfg(not(windows))]
mod win {
    use super::Foreground;
    use crate::screen_fit::Area;

    pub fn foreground() -> Option<((), Foreground)> {
        None
    }

    pub fn title(_window: ()) -> String {
        String::new()
    }

    pub fn client_area(_window: ()) -> Option<Area> {
        None
    }

    pub fn cursor() -> Option<(f64, f64)> {
        None
    }

    pub fn find_by_title(_title: &str) -> Option<()> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_game_is_known_by_its_title() {
        assert_eq!(classify("Hearthstone", "Anything"), Foreground::Hearthstone);
        assert_eq!(classify("hearthstone", ""), Foreground::Hearthstone);
        assert_eq!(classify(" Hearthstone ", ""), Foreground::Hearthstone);
    }

    #[test]
    fn the_game_is_known_by_its_window_class_in_any_language() {
        assert_eq!(
            classify("炉石传说", "UnityWndClass"),
            Foreground::Hearthstone
        );
    }

    #[test]
    fn only_the_game_title_gives_the_leaderboard_window() {
        assert!(is_game_title("Hearthstone"));
        assert!(is_game_title(" hearthstone "));
        assert!(!is_game_title("Some other Unity game"));
        assert!(!is_game_title(""));
    }

    #[test]
    fn other_windows_are_other() {
        assert_eq!(
            classify("Tavern Ledger", "Chrome_WidgetWin_1"),
            Foreground::Other
        );
        assert_eq!(classify("", ""), Foreground::Other);
        // A title that merely contains the word is not the game.
        assert_eq!(
            classify("Hearthstone guide - Browser", "Chrome_WidgetWin_1"),
            Foreground::Other
        );
    }
}
