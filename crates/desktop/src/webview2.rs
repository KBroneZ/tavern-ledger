//! The WebView2 runtime check, before any window (T-379, T-381).
//!
//! A window needs the Microsoft Edge WebView2 runtime. The installer edition
//! carries Microsoft's bootstrapper and installs it when the registry has no
//! runtime; a Store package cannot install it (the manifest's
//! `ExternalDependency` only works through App Installer, and a line in the
//! listing installs nothing). Either way the app looks for a usable runtime
//! before it makes a window and, when there is none, says so in plain words
//! with Microsoft's download link instead of the window library's raw
//! "Could not find the WebView2 Runtime" dialog and two panic lines in the
//! app log.
//!
//! A runtime is **usable** when the loader will find it, the way it does: the
//! `pv` version under the WebView2 client's key (per machine in both registry
//! views, and per user) is above 0.0.0.0 *and* the folder the registry points
//! at holds `msedgewebview2.exe`. That folder is `EBWebView` under the
//! matching `ClientState` key, else `location` and the version under the
//! `Clients` key. A registration whose folder is not there (session 198's
//! Windows Sandbox: registered 155.0.4283.45, files 155.0.4283.62) is
//! **stale**: the installer skips its bootstrapper because the key exists, and
//! the window library fails. There is no minimum version: the loader's own
//! rule is "any installed runtime"; the app asks for no newer one.

/// Microsoft's page for the Evergreen runtime.
pub const DOWNLOAD_URL: &str = "https://developer.microsoft.com/microsoft-edge/webview2/";

/// Where Windows keeps the runtime's version: `(hive, key)`, each holding `pv`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hive {
    LocalMachine,
    CurrentUser,
}

const CLIENT: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/// The three places Microsoft lists: per-machine in the 32-bit view
/// (`WOW6432Node`), per-machine in the native view, and per-user.
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

/// The `ClientState` key beside a `Clients` key: it holds `EBWebView`, the
/// folder of the installed version.
pub fn state_key(client: &str) -> String {
    client.replacen(r"\Clients\", r"\ClientState\", 1)
}

/// What the machine has, as far as the window library is concerned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Runtime {
    /// A registered runtime whose files are there.
    Usable,
    /// No runtime is registered.
    Missing,
    /// A runtime is registered at this version, but its folder is not there
    /// (or the registry names no folder, or one the app will not look in).
    Stale { version: String },
}

/// The file that tells a runtime folder is a runtime.
const RUNTIME_EXE: &str = "msedgewebview2.exe";

/// A folder on a local drive (`C:\...`). The registry value is only ever used
/// to look for one file, but a network path would make the start wait on a
/// server, so it is not looked in.
fn is_local_folder(folder: &str) -> bool {
    let bytes = folder.as_bytes();
    bytes.len() > 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":\\"
}

/// Whether the runtime is usable, given a way to read a registry value
/// (`hive`, `key`, value name) and to tell a folder holds the runtime.
pub fn check_with(
    read: impl Fn(Hive, &str, &str) -> Option<String>,
    holds_runtime: impl Fn(&str) -> bool,
) -> Runtime {
    let mut stale = None;
    for (hive, client) in locations() {
        let Some(pv) = read(hive, &client, "pv").filter(|pv| is_version(pv)) else {
            continue;
        };
        let pv = pv.trim().to_string();
        let non_blank = |value: Option<String>| {
            value
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let folder = non_blank(read(hive, &state_key(&client), "EBWebView")).or_else(|| {
            non_blank(read(hive, &client, "location"))
                .map(|location| format!(r"{}\{pv}", location.trim_end_matches('\\')))
        });
        if folder
            .as_deref()
            .is_some_and(|folder| is_local_folder(folder) && holds_runtime(folder))
        {
            return Runtime::Usable;
        }
        stale.get_or_insert(pv);
    }
    stale.map_or(Runtime::Missing, |version| Runtime::Stale { version })
}

/// Whether a fixed-version runtime was named through the environment
/// (`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`, which the window library honours):
/// it has no `pv` value, and it must not be taken for a missing runtime.
pub fn is_fixed_runtime(folder: Option<&str>) -> bool {
    folder.is_some_and(|folder| !folder.trim().is_empty())
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

/// What the player is told for a runtime that is registered but not there:
/// the version, why the window cannot open, and the installer that puts the
/// runtime back (the bootstrapper only updates a runtime it thinks is current).
fn stale_message(version: &str) -> String {
    format!(
        "Tavern Ledger needs the Microsoft Edge WebView2 Runtime to show its window. This PC lists version {version}, but its files are missing or do not match, so the window cannot open.\n\n\
Installing Microsoft's \"Evergreen Standalone Installer\" (on the same page) puts the runtime back. Then start Tavern Ledger again.\n\n\
Open the download page now?\n\n\
Tavern Ledger necesita el runtime Microsoft Edge WebView2 para mostrar su ventana. Este PC indica la versión {version}, pero faltan sus archivos o no coinciden, así que la ventana no se puede abrir.\n\n\
Instalar el \"Evergreen Standalone Installer\" de Microsoft (en la misma página) devuelve el runtime. Después vuelve a abrir Tavern Ledger.\n\n\
¿Abrir ahora la página de descarga?\n\n\
{DOWNLOAD_URL}"
    )
}

/// The message for a state; empty when there is nothing to say.
pub fn message_for(runtime: &Runtime) -> String {
    match runtime {
        Runtime::Usable => String::new(),
        Runtime::Missing => message(),
        Runtime::Stale { version } => stale_message(version),
    }
}

#[cfg(windows)]
mod win {
    use windows::core::{w, HSTRING};
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_ICONERROR, MB_SETFOREGROUND, MB_YESNO,
    };
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    use super::{
        check_with, is_fixed_runtime, message_for, Hive, Runtime, DOWNLOAD_URL, RUNTIME_EXE,
    };
    use crate::applog;

    fn read_value(hive: Hive, key: &str, name: &str) -> Option<String> {
        let root = match hive {
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
            Hive::CurrentUser => HKEY_CURRENT_USER,
        };
        winreg::RegKey::predef(root)
            .open_subkey_with_flags(key, KEY_READ)
            .ok()?
            .get_value::<String, _>(name)
            .ok()
    }

    fn holds_runtime(folder: &str) -> bool {
        std::path::Path::new(folder).join(RUNTIME_EXE).is_file()
    }

    /// Stops the app with a plain message when the runtime is missing or stale.
    pub fn require_runtime() {
        let fixed = std::env::var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER").ok();
        if is_fixed_runtime(fixed.as_deref()) {
            return;
        }
        let runtime = check_with(read_value, holds_runtime);
        if runtime == Runtime::Usable {
            return;
        }
        match &runtime {
            Runtime::Stale { version } => applog::error(
                "app",
                &format!(
                    "The WebView2 runtime is registered as {version} but its files are not there; the app cannot open a window."
                ),
            ),
            _ => applog::error(
                "app",
                "The WebView2 runtime is not installed; the app cannot open a window.",
            ),
        }
        let text = HSTRING::from(message_for(&runtime));
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
            if let Err(error) = tracker::quiet::command(rundll)
                .args(["url.dll,FileProtocolHandler", DOWNLOAD_URL])
                .spawn()
            {
                applog::warn(
                    "app",
                    &format!("could not open the WebView2 download page: {error}"),
                );
            }
        }
        std::process::exit(1);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_key_that_is_not_there_reads_as_no_value() {
            assert_eq!(
                read_value(Hive::CurrentUser, r"Software\TavernLedgerNoSuchKey\x", "pv"),
                None
            );
        }
    }
}

/// Both editions check for a usable runtime before they make a window.
#[cfg(windows)]
pub use win::require_runtime;

#[cfg(not(windows))]
pub fn require_runtime() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
    fn the_three_places_microsoft_lists_are_all_looked_in() {
        let places = locations();
        assert_eq!(places.len(), 3);
        for (hive, client) in &places {
            let folder = format!(r"{APP}\155.0.4283.60");
            let machine = Machine::new(&[&folder])
                .with(*hive, client.clone(), "pv", "155.0.4283.60")
                .with(*hive, state_key(client), "EBWebView", &folder);
            assert_eq!(machine.check(), Runtime::Usable, "{hive:?} {client}");
        }
        assert!(places
            .iter()
            .all(|(_, key)| key.contains("{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}")));
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
    fn a_fixed_runtime_named_in_the_environment_is_not_a_missing_one() {
        assert!(is_fixed_runtime(Some(r"C:\WebView2\Fixed")));
        assert!(!is_fixed_runtime(None));
        assert!(!is_fixed_runtime(Some("  ")));
        assert!(!is_fixed_runtime(Some("")));
    }

    // ---- T-381: a registered runtime whose files are not there ----

    /// A machine where the runtime is registered by `values` and the folders
    /// in `present` hold a runtime (`msedgewebview2.exe`).
    struct Machine {
        values: HashMap<(Hive, String, String), String>,
        present: Vec<String>,
    }

    impl Machine {
        fn new(present: &[&str]) -> Self {
            Machine {
                values: HashMap::new(),
                present: present.iter().map(|p| p.to_string()).collect(),
            }
        }

        fn with(mut self, hive: Hive, key: String, name: &str, value: &str) -> Self {
            self.values
                .insert((hive, key, name.to_string()), value.to_string());
            self
        }

        fn check(&self) -> Runtime {
            check_with(
                |hive, key, name| {
                    self.values
                        .get(&(hive, key.to_string(), name.to_string()))
                        .cloned()
                },
                |folder| self.present.iter().any(|p| p == folder),
            )
        }
    }

    const APP: &str = r"C:\Program Files (x86)\Microsoft\EdgeWebView\Application";

    /// What the Windows Sandbox image of session 198 has: `Clients` says .45,
    /// `ClientState\EBWebView` points at the .45 folder, and only .62 is on disk.
    fn stale_sandbox() -> Machine {
        let (hive, client) = locations()[0].clone();
        Machine::new(&[&format!(r"{APP}\155.0.4283.62")])
            .with(hive, client.clone(), "pv", "155.0.4283.45")
            .with(hive, client.clone(), "location", APP)
            .with(
                hive,
                state_key(&client),
                "EBWebView",
                &format!(r"{APP}\155.0.4283.45"),
            )
    }

    #[test]
    fn a_runtime_registered_where_its_files_are_is_usable() {
        let (hive, client) = locations()[0].clone();
        let folder = format!(r"{APP}\155.0.4283.62");
        let machine = Machine::new(&[&folder])
            .with(hive, client.clone(), "pv", "155.0.4283.62")
            .with(hive, state_key(&client), "EBWebView", &folder);
        assert_eq!(machine.check(), Runtime::Usable);
    }

    #[test]
    fn the_folder_comes_from_location_and_pv_when_client_state_does_not_say() {
        let (hive, client) = locations()[0].clone();
        let machine = Machine::new(&[&format!(r"{APP}\155.0.4283.62")])
            .with(hive, client.clone(), "pv", "155.0.4283.62")
            .with(hive, client, "location", APP);
        assert_eq!(machine.check(), Runtime::Usable);
    }

    #[test]
    fn a_registration_that_differs_from_the_files_is_stale_not_missing() {
        // The failing machine of session 198: registered 155.0.4283.45, files 155.0.4283.62.
        assert_eq!(
            stale_sandbox().check(),
            Runtime::Stale {
                version: "155.0.4283.45".to_string()
            }
        );
    }

    #[test]
    fn a_registration_with_no_folder_to_look_in_is_stale() {
        let (hive, client) = locations()[2].clone();
        let machine = Machine::new(&[]).with(hive, client, "pv", "120.0.1.2");
        assert_eq!(
            machine.check(),
            Runtime::Stale {
                version: "120.0.1.2".to_string()
            }
        );
    }

    #[test]
    fn no_registration_at_all_is_missing_and_a_zero_one_is_too() {
        assert_eq!(Machine::new(&[]).check(), Runtime::Missing);
        let (hive, client) = locations()[0].clone();
        assert_eq!(
            Machine::new(&[])
                .with(hive, client, "pv", "0.0.0.0")
                .check(),
            Runtime::Missing
        );
    }

    #[test]
    fn one_good_place_is_enough_when_another_is_stale() {
        // A per-machine runtime that is stale and a per-user one that works.
        let (user_hive, user_client) = locations()[2].clone();
        let user_folder = r"C:\Users\x\AppData\Local\Microsoft\EdgeWebView\Application\130.0.1.2";
        let mut machine = stale_sandbox()
            .with(user_hive, user_client.clone(), "pv", "130.0.1.2")
            .with(user_hive, state_key(&user_client), "EBWebView", user_folder);
        machine.present.push(user_folder.to_string());
        assert_eq!(machine.check(), Runtime::Usable);
    }

    #[test]
    fn a_blank_client_state_value_falls_back_to_location_and_pv() {
        // `EBWebView` is only used to look for a file, never run or opened.
        let (hive, client) = locations()[0].clone();
        let machine = Machine::new(&[&format!(r"{APP}\155.0.4283.62")])
            .with(hive, client.clone(), "pv", "155.0.4283.62")
            .with(hive, client.clone(), "location", APP)
            .with(hive, state_key(&client), "EBWebView", "   ");
        assert_eq!(machine.check(), Runtime::Usable);
    }

    #[test]
    fn the_client_state_key_sits_beside_the_clients_key() {
        let (_, client) = locations()[0].clone();
        assert!(client.contains(r"EdgeUpdate\Clients\{F3017226"));
        assert!(state_key(&client).contains(r"EdgeUpdate\ClientState\{F3017226"));
    }

    #[test]
    fn the_stale_message_names_the_version_the_fix_and_both_languages() {
        let text = message_for(&Runtime::Stale {
            version: "155.0.4283.45".to_string(),
        });
        assert!(text.contains("155.0.4283.45"));
        assert!(text.contains("Evergreen Standalone Installer"));
        assert!(text.contains("necesita"), "Spanish too");
        assert!(text.contains(DOWNLOAD_URL));
        // The missing one is the message the Store edition has always shown.
        assert_eq!(message_for(&Runtime::Missing), message());
        assert!(message_for(&Runtime::Usable).is_empty());
    }

    #[test]
    fn only_a_folder_on_a_local_drive_is_looked_in() {
        for good in [r"C:\Program Files (x86)\x", r"d:\EdgeWebView\1.2.3.4"] {
            assert!(is_local_folder(good), "{good}");
        }
        for bad in [
            r"\\server\share\x",
            "C:",
            r"C:\",
            "relative\\path",
            "",
            "/usr/x",
        ] {
            assert!(!is_local_folder(bad), "{bad:?}");
        }
        // A registry value naming a network folder is stale, whatever answers there.
        let (hive, client) = locations()[0].clone();
        let machine = Machine::new(&[r"\\server\share\runtime"])
            .with(hive, client.clone(), "pv", "155.0.4283.62")
            .with(
                hive,
                state_key(&client),
                "EBWebView",
                r"\\server\share\runtime",
            );
        assert_eq!(
            machine.check(),
            Runtime::Stale {
                version: "155.0.4283.62".to_string()
            }
        );
    }
}
