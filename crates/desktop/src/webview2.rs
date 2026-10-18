//! The WebView2 runtime check of the Microsoft Store edition (T-379).
//!
//! The installer edition carries Microsoft's bootstrapper and installs the
//! runtime when it is missing. A Store package cannot: the manifest's
//! `ExternalDependency` only works through App Installer, and a line in the
//! listing installs nothing. So the Store edition looks for the runtime before
//! it makes any window and, when it is missing, says so in plain words with
//! Microsoft's download link instead of failing with an error from the window
//! library.
//!
//! The runtime is found the way Microsoft documents: the `pv` value under the
//! WebView2 client's key, in the per-machine and per-user places, which must
//! hold a version above 0.0.0.0.

// The installer edition never looks (it installs the runtime itself); the
// checks stay compiled for the tests that run on both editions.
#![cfg_attr(not(feature = "store"), allow(dead_code))]

/// Microsoft's page for the Evergreen runtime.
pub const DOWNLOAD_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

/// Where Windows keeps the runtime's version: `(hive, key)`, each holding `pv`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hive {
    LocalMachine,
    CurrentUser,
}

const CLIENT: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// The three places Microsoft lists (per-machine 64-bit view, per-machine
/// 32-bit view, per-user).
pub fn locations() -> [(Hive, String); 3] {
    [
        (
            Hive::LocalMachine,
            format!(r"SOFTWARE\WOW6432Node\{CLIENT}"),
        ),
        (Hive::LocalMachine, format!(r"SOFTWARE\{CLIENT}")),
        (Hive::CurrentUser, format!(r"Software\{CLIENT}")),
    ]
}

/// Whether a `pv` value is a real version: dotted numbers with at least one
/// above zero. Empty, `0.0.0.0` and anything that is not numbers is "not
/// installed".
pub fn is_version(pv: &str) -> bool {
    let parts: Vec<Option<u32>> = pv.trim().split('.').map(|p| p.parse().ok()).collect();
    !pv.trim().is_empty()
        && parts.iter().all(Option::is_some)
        && parts.iter().flatten().any(|&n| n > 0)
}

/// Whether the runtime is installed, given a way to read `pv` from a place.
pub fn installed_with(read: impl Fn(Hive, &str) -> Option<String>) -> bool {
    locations()
        .iter()
        .any(|(hive, key)| read(*hive, key).is_some_and(|pv| is_version(&pv)))
}

/// What the player is told when the runtime is missing, in both languages
/// (nothing is loaded yet that could tell which one they use).
pub fn message() -> String {
    format!(
        "Tavern Ledger needs the Microsoft Edge WebView2 Runtime to show its window, and it is not installed on this PC. \
It is free from Microsoft, and already part of Windows 11 and most Windows 10 PCs.\n\n\
Open the download page now? Install the \"Evergreen Bootstrapper\", then start Tavern Ledger again.\n\n\
Tavern Ledger necesita el runtime Microsoft Edge WebView2 para mostrar su ventana y no está instalado en este PC. \
Es gratuito, de Microsoft, y ya viene con Windows 11 y con la mayoría de los PC con Windows 10.\n\n\
¿Abrir ahora la página de descarga? Instala el \"Evergreen Bootstrapper\" y vuelve a abrir Tavern Ledger.\n\n\
{DOWNLOAD_URL}"
    )
}

#[cfg(all(windows, feature = "store"))]
mod win {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use windows::core::{w, HSTRING};
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONERROR, MB_SETFOREGROUND, MB_YESNO,
    };
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    use super::{installed_with, message, Hive, DOWNLOAD_URL};
    use crate::applog;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn read_pv(hive: Hive, key: &str) -> Option<String> {
        let root = match hive {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        };
        winreg::RegKey::predef(root)
            .open_subkey_with_flags(key, KEY_READ)
            .ok()?
            .get_value::<String, _>("pv")
            .ok()
    }

    /// Stops the app with a plain message when the runtime is missing.
    pub fn require_runtime() {
        if installed_with(read_pv) {
            return;
        }
        applog::error(
            "app",
            "The WebView2 runtime is not installed; the app cannot open a window.",
        );
        let text = HSTRING::from(message());
        // SAFETY: both strings outlive the call; no owner window.
        let answer = unsafe {
            MessageBoxW(
                None,
                &text,
                w!("Tavern Ledger"),
                MB_YESNO | MB_ICONERROR | MB_SETFOREGROUND,
            )
        };
        if answer == IDYES {
            // Microsoft's page, opened by the system's URL handler; the
            // address is a constant, nothing from the player is passed.
            let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
            let rundll = std::path::Path::new(&system)
                .join("System32")
                .join("rundll32.exe");
            let _ = Command::new(rundll)
                .args(["url.dll,FileProtocolHandler", DOWNLOAD_URL])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
        std::process::exit(1);
    }
}

/// The Store edition checks for the runtime before it makes a window; the
/// installer edition installs it itself.
#[cfg(all(windows, feature = "store"))]
pub use win::require_runtime;

#[cfg(not(all(windows, feature = "store")))]
pub fn require_runtime() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn reader(found: &[(Hive, String, &str)]) -> impl Fn(Hive, &str) -> Option<String> {
        let map: HashMap<(Hive, String), String> = found
            .iter()
            .map(|(hive, key, pv)| ((*hive, key.clone()), pv.to_string()))
            .collect();
        move |hive, key| map.get(&(hive, key.to_string())).cloned()
    }

    #[test]
    fn a_real_version_counts_and_the_empty_and_zero_ones_do_not() {
        for good in ["155.0.4283.60", "1.0.0.0", " 120.0.1 ", "0.0.0.1"] {
            assert!(is_version(good), "{good:?}");
        }
        for bad in [
            "",
            "   ",
            "0.0.0.0",
            "0",
            "0.0",
            "abc",
            "1.x.3",
            "1..2",
            "-1.0",
            "1.0.0.0\u{0}",
        ] {
            assert!(!is_version(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_runtime_is_found_in_any_of_the_three_places_microsoft_lists() {
        let places = locations();
        assert_eq!(places.len(), 3);
        for (hive, key) in &places {
            let found = [(*hive, key.clone(), "155.0.4283.60")];
            assert!(installed_with(reader(&found)), "{hive:?} {key}");
        }
        assert!(places
            .iter()
            .all(|(_, key)| key.contains("{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}")));
    }

    #[test]
    fn no_value_or_a_zero_one_in_every_place_means_not_installed() {
        assert!(!installed_with(reader(&[])));
        let places = locations();
        let zeros: Vec<_> = places
            .iter()
            .map(|(hive, key)| (*hive, key.clone(), "0.0.0.0"))
            .collect();
        assert!(!installed_with(reader(&zeros)));
        // One good place is enough even when another says zero.
        let mixed = [
            (places[0].0, places[0].1.clone(), "0.0.0.0"),
            (places[2].0, places[2].1.clone(), "130.0.1.2"),
        ];
        assert!(installed_with(reader(&mixed)));
    }

    #[test]
    fn the_message_says_what_is_missing_where_to_get_it_and_comes_in_both_languages() {
        let text = message();
        assert!(text.contains("WebView2"));
        assert!(text.contains("Evergreen Bootstrapper"));
        assert!(text.contains("necesita"), "Spanish too");
        assert!(text.contains(DOWNLOAD_URL));
        assert!(DOWNLOAD_URL.starts_with("https://developer.microsoft.com/"));
    }

    #[test]
    fn a_test_run_never_stops_the_app() {
        // The installer edition (and any non-Windows build) has nothing to check.
        #[cfg(not(feature = "store"))]
        require_runtime();
    }
}
