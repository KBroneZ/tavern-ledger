//! Is Hearthstone the window in front? Answered from the foreground window's
//! title and class only (D-032): no handle is opened on the game's process,
//! nothing is read from it and nothing is sent to it (D-004, D-006).

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
#[cfg(windows)]
pub fn current() -> Option<Foreground> {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetWindowTextW(window: *mut c_void, text: *mut u16, max_count: i32) -> i32;
        fn GetClassNameW(window: *mut c_void, text: *mut u16, max_count: i32) -> i32;
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

    // SAFETY: no arguments; returns a window handle or null.
    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    Some(classify(
        &read(window, GetWindowTextW),
        &read(window, GetClassNameW),
    ))
}

#[cfg(not(windows))]
pub fn current() -> Option<Foreground> {
    None
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
