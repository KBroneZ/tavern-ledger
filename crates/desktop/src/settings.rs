//! The app's own settings (T-324): the language of the window, the overlay
//! and the tray menu (T-325, D-065), and how much the main window shows
//! (Simple or Advanced, T-344, D-072; the overlay has its own in
//! `overlay.json`). Kept in `settings.json` in the data
//! folder, next to `overlay.json` and the upload settings. A missing file
//! means the defaults (the system language); a broken one leaves them and
//! says so. The other options of the Settings page keep their own files
//! (overlay, upload) or Windows' own setting (start with Windows).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, MenuItem};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager, State, Wry};
use tauri_plugin_autostart::ManagerExt;

use crate::overlay::lock;
use crate::overlay_layout::Detail;
use crate::AppState;

pub const FILE: &str = "settings.json";
/// Sent to every window with the new [`View`]; a window whose language
/// changes draws itself again.
pub const CHANGED: &str = "app-settings-changed";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Es,
}

/// What is saved. `language: None` follows the system language; a missing
/// `window_detail` is Simple (D-067's default).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    #[serde(default)]
    pub language: Option<Language>,
    #[serde(default)]
    pub window_detail: Detail,
}

impl Saved {
    fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| {
            format!(
                "The app settings could not be read ({:?}); the defaults are used.",
                e.classify()
            )
        })
    }
}

/// What the windows get.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    language: Option<Language>,
    window_detail: Detail,
}

/// The tray menu's items, kept to change their words and the start-up tick.
/// Cloned out of the lock before use: their calls wait for the main thread.
#[derive(Clone)]
pub struct TrayItems {
    pub show: MenuItem<Wry>,
    pub overlay: CheckMenuItem<Wry>,
    pub unlock: CheckMenuItem<Wry>,
    pub reset_layout: MenuItem<Wry>,
    pub autostart: CheckMenuItem<Wry>,
    pub autostart_available: bool,
    pub quit: MenuItem<Wry>,
    pub icon: TrayIcon<Wry>,
}

#[derive(Default)]
pub struct Settings {
    path: Mutex<Option<PathBuf>>,
    saved: Mutex<Saved>,
    /// The system's language as the main window reads it (the web view
    /// follows Windows); English until it says.
    system: Mutex<Language>,
    tray: Mutex<Option<TrayItems>>,
}

impl Settings {
    pub fn load(&self, data_dir: &Path) -> Result<(), String> {
        let path = data_dir.join(FILE);
        *lock(&self.path) = Some(path.clone());
        match std::fs::read_to_string(&path) {
            Ok(text) => *lock(&self.saved) = Saved::parse(&text)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "The app settings could not be read ({:?}); the defaults are used.",
                    e.kind()
                ))
            }
        }
        Ok(())
    }

    pub fn view(&self) -> View {
        let saved = lock(&self.saved);
        View {
            language: saved.language,
            window_detail: saved.window_detail,
        }
    }

    /// The language the app is shown in: the saved choice, else the system's.
    pub fn language(&self) -> Language {
        lock(&self.saved).language.unwrap_or(*lock(&self.system))
    }

    /// Saves the choice. It holds for this run even if saving fails; the
    /// error says it will not survive a restart.
    fn set_language(&self, language: Option<Language>) -> Result<(), String> {
        self.save(|saved| saved.language = language, SAVE_FAILED)
    }

    fn set_window_detail(&self, detail: Detail) -> Result<(), String> {
        self.save(|saved| saved.window_detail = detail, DETAIL_SAVE_FAILED)
    }

    /// Changes the saved settings and writes them; the change holds for
    /// this run even when writing fails.
    fn save(&self, change: impl FnOnce(&mut Saved), failed: &str) -> Result<(), String> {
        let text = {
            let mut saved = lock(&self.saved);
            change(&mut saved);
            serde_json::to_string_pretty(&*saved)
        };
        let path = lock(&self.path).clone();
        let (Some(path), Ok(text)) = (path, text) else {
            return Err(failed.into());
        };
        write_whole(&path, &text).map_err(|_| failed.to_string())
    }

    pub fn keep_tray(&self, items: TrayItems) {
        *lock(&self.tray) = Some(items);
        self.relabel_tray();
    }

    /// Puts the tray menu's words in the app's language.
    fn relabel_tray(&self) {
        let words = tray_words(self.language());
        let items = lock(&self.tray).clone();
        if let Some(items) = items {
            let _ = items.show.set_text(words.show);
            let _ = items.overlay.set_text(words.overlay);
            let _ = items.unlock.set_text(words.unlock);
            let _ = items.reset_layout.set_text(words.reset_layout);
            let _ = items.autostart.set_text(if items.autostart_available {
                words.autostart
            } else {
                words.autostart_unavailable
            });
            let _ = items.quit.set_text(words.quit);
            let _ = items.icon.set_tooltip(Some(words.tooltip));
        }
    }

    pub fn autostart_available(&self) -> bool {
        lock(&self.tray)
            .as_ref()
            .is_some_and(|items| items.autostart_available)
    }

    fn tick_autostart(&self, on: bool) {
        let items = lock(&self.tray).clone();
        if let Some(items) = items {
            let _ = items.autostart.set_checked(on);
        }
    }
}

const SAVE_FAILED: &str = "The language could not be saved; it will be lost after a restart.";
const DETAIL_SAVE_FAILED: &str =
    "The main window's detail could not be saved; it will be lost after a restart.";

/// Written whole to a side file first, so a crash never leaves half a file.
fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let side = path.with_extension("json.tmp");
    std::fs::write(&side, text)?;
    std::fs::rename(&side, path)
}

/// The tray menu in each language. The window's words are in ui/strings/.
#[derive(Debug, PartialEq)]
pub struct TrayWords {
    pub show: &'static str,
    pub overlay: &'static str,
    pub unlock: &'static str,
    pub reset_layout: &'static str,
    pub autostart: &'static str,
    pub autostart_unavailable: &'static str,
    pub quit: &'static str,
    pub tooltip: &'static str,
}

pub fn tray_words(language: Language) -> TrayWords {
    match language {
        Language::En => TrayWords {
            show: "Open Tavern Ledger",
            overlay: "Show overlay during a game",
            unlock: "Unlock overlay to move and resize panels",
            reset_layout: "Reset overlay layout",
            autostart: "Start with Windows",
            autostart_unavailable:
                "Start with Windows (not available: the app's folder has spaces)",
            quit: "Quit",
            tooltip: "Tavern Ledger: following your Battlegrounds games",
        },
        Language::Es => TrayWords {
            show: "Abrir Tavern Ledger",
            overlay: "Mostrar overlay durante la partida",
            unlock: "Desbloquear el overlay para mover y redimensionar paneles",
            reset_layout: "Restablecer la disposición del overlay",
            autostart: "Iniciar con Windows",
            autostart_unavailable:
                "Iniciar con Windows (no disponible: la carpeta de la app tiene espacios)",
            quit: "Salir",
            tooltip: "Tavern Ledger: sigue tus partidas de Campos de Batalla",
        },
    }
}

#[tauri::command]
pub fn app_settings(settings: State<'_, Arc<Settings>>) -> View {
    settings.view()
}

/// `language: None` goes back to the system language.
#[tauri::command]
pub fn app_set_language(
    app: AppHandle,
    settings: State<'_, Arc<Settings>>,
    language: Option<Language>,
) -> Result<View, String> {
    let result = settings.set_language(language);
    // The windows reload in the new language, so a failure also goes to the
    // status line, which survives the reload.
    if let Err(message) = &result {
        let state = app.state::<Arc<AppState>>();
        crate::set_status(&app, &state, |s| s.notice = Some(message.clone()));
    }
    settings.relabel_tray();
    let view = settings.view();
    let _ = app.emit(CHANGED, view.clone());
    result.map(|()| view)
}

/// Simple or Advanced for the main window (T-344). Every window hears it
/// (`CHANGED`); the main window redraws without reloading.
#[tauri::command]
pub fn app_set_window_detail(
    app: AppHandle,
    settings: State<'_, Arc<Settings>>,
    detail: Detail,
) -> Result<View, String> {
    let result = settings.set_window_detail(detail);
    let view = settings.view();
    let _ = app.emit(CHANGED, view.clone());
    result.map(|()| view)
}

/// The main window says which language the system has; the tray follows it
/// while no language is chosen.
#[tauri::command]
pub fn app_set_system_language(settings: State<'_, Arc<Settings>>, language: Language) {
    *lock(&settings.system) = language;
    settings.relabel_tray();
}

#[derive(Serialize)]
pub struct Autostart {
    available: bool,
    enabled: bool,
}

#[tauri::command]
pub fn app_autostart(app: AppHandle, settings: State<'_, Arc<Settings>>) -> Autostart {
    Autostart {
        available: settings.autostart_available(),
        enabled: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

/// Turns the Windows start-up entry on or off, as the tray item does; both
/// show the real state afterwards.
#[tauri::command]
pub fn app_set_autostart(
    app: AppHandle,
    settings: State<'_, Arc<Settings>>,
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<Autostart, String> {
    if !settings.autostart_available() {
        return Err(AUTOSTART_FAILED.into());
    }
    let result = set_autostart(&app, &settings, &state, enabled);
    let now = app_autostart(app, settings);
    result.map(|()| now)
}

pub const AUTOSTART_FAILED: &str = "Could not change the Windows start-up setting.";
/// Sent when the start-up entry changed from the tray, so the Settings page follows.
pub const AUTOSTART_CHANGED: &str = "autostart-changed";

pub fn set_autostart(
    app: &AppHandle,
    settings: &Settings,
    state: &AppState,
    on: bool,
) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if on {
        launcher.enable().map_err(|e| e.to_string()).and_then(|()| {
            // An unquoted entry could start the wrong program: take it back.
            // The rewrite's error is the one reported.
            quote_startup_entry(app).map_err(|e| {
                let _ = launcher.disable();
                e.to_string()
            })
        })
    } else {
        launcher.disable().map_err(|e| e.to_string())
    };
    settings.tick_autostart(launcher.is_enabled().unwrap_or(false));
    let notice = result.is_err().then(|| AUTOSTART_FAILED.to_string());
    let failed = notice.is_some();
    crate::set_status(app, state, |s| s.notice = notice);
    let _ = app.emit(AUTOSTART_CHANGED, ());
    if failed {
        Err(AUTOSTART_FAILED.into())
    } else {
        Ok(())
    }
}

/// Windows' list of programs to run at sign-in, where auto-launch writes the
/// start-up entry.
#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// The start-up entry's command line with the exe path in quotes. auto-launch
/// 0.5 writes the path unquoted, and Windows cuts an unquoted path at its
/// first space (the installer's folder `Tavern Ledger`, or a user name with a
/// space), which runs the wrong program or none.
fn startup_command(exe: &Path, args: &[&str]) -> String {
    let mut command = format!("\"{}\"", exe.display());
    for arg in args {
        command.push(' ');
        command.push_str(arg);
    }
    command
}

/// Writes the entry auto-launch just made again, under the same name, with
/// the path quoted.
#[cfg(windows)]
fn quote_startup_entry(app: &AppHandle) -> io::Result<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    let exe = std::env::current_exe()?;
    winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?
        .set_value(
            &app.package_info().name,
            &startup_command(&exe, &[crate::MINIMIZED_ARG]),
        )
}

#[cfg(not(windows))]
fn quote_startup_entry(_app: &AppHandle) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_command_quotes_the_path() {
        let exe = Path::new(r"C:\Users\A B\AppData\Local\Tavern Ledger\TavernLedger.exe");
        assert_eq!(
            startup_command(exe, &["--minimized"]),
            r#""C:\Users\A B\AppData\Local\Tavern Ledger\TavernLedger.exe" --minimized"#
        );
    }

    #[test]
    fn startup_command_quotes_a_path_without_spaces_too() {
        let exe = Path::new(r"C:\Apps\desktop.exe");
        assert_eq!(startup_command(exe, &[]), r#""C:\Apps\desktop.exe""#);
    }

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tl-settings-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn no_file_means_the_system_language() {
        let settings = Settings::default();
        settings.load(&dir("none")).unwrap();
        assert_eq!(
            settings.view(),
            View {
                language: None,
                window_detail: Detail::Simple
            }
        );
        assert_eq!(settings.language(), Language::En);
        *lock(&settings.system) = Language::Es;
        assert_eq!(settings.language(), Language::Es);
    }

    #[test]
    fn the_chosen_language_is_saved_and_read_back_and_wins_over_the_system() {
        let dir = dir("saved");
        let settings = Settings::default();
        settings.load(&dir).unwrap();
        settings.set_language(Some(Language::Es)).unwrap();
        let again = Settings::default();
        again.load(&dir).unwrap();
        assert_eq!(
            again.view(),
            View {
                language: Some(Language::Es),
                window_detail: Detail::Simple,
            }
        );
        assert_eq!(again.language(), Language::Es);
        again.set_language(None).unwrap();
        let back = Settings::default();
        back.load(&dir).unwrap();
        assert_eq!(
            back.view(),
            View {
                language: None,
                window_detail: Detail::Simple
            }
        );
    }

    #[test]
    fn a_broken_file_leaves_the_defaults_and_says_so() {
        let dir = dir("broken");
        std::fs::write(dir.join(FILE), "not json").unwrap();
        let settings = Settings::default();
        let err = settings.load(&dir).unwrap_err();
        assert!(err.contains("defaults are used"), "{err}");
        assert_eq!(
            settings.view(),
            View {
                language: None,
                window_detail: Detail::Simple
            }
        );
        // An unknown language is not guessed either.
        std::fs::write(dir.join(FILE), r#"{"language":"fr"}"#).unwrap();
        assert!(Settings::default().load(&dir).is_err());
    }

    #[test]
    fn the_language_is_written_as_its_short_code() {
        let text = serde_json::to_string(&Saved {
            language: Some(Language::Es),
            window_detail: Detail::Simple,
        })
        .unwrap();
        assert_eq!(text, r#"{"language":"es","window_detail":"simple"}"#);
    }

    #[test]
    fn the_main_windows_detail_is_simple_until_chosen_and_kept() {
        let dir = dir("detail");
        // A file from before T-344 has no detail: Simple, the language kept.
        std::fs::write(dir.join(FILE), r#"{"language":"es"}"#).unwrap();
        let settings = Settings::default();
        settings.load(&dir).unwrap();
        assert_eq!(settings.view().window_detail, Detail::Simple);
        settings.set_window_detail(Detail::Advanced).unwrap();
        let again = Settings::default();
        again.load(&dir).unwrap();
        assert_eq!(
            again.view(),
            View {
                language: Some(Language::Es),
                window_detail: Detail::Advanced,
            }
        );
        // An unknown detail is not guessed.
        std::fs::write(dir.join(FILE), r#"{"window_detail":"pro"}"#).unwrap();
        assert!(Settings::default().load(&dir).is_err());
    }

    #[test]
    fn the_tray_menu_has_every_word_in_every_language() {
        for language in [Language::En, Language::Es] {
            let w = tray_words(language);
            for word in [
                w.show,
                w.overlay,
                w.unlock,
                w.reset_layout,
                w.autostart,
                w.autostart_unavailable,
                w.quit,
                w.tooltip,
            ] {
                assert!(!word.trim().is_empty(), "{language:?}");
            }
        }
        assert_ne!(tray_words(Language::En), tray_words(Language::Es));
    }
}
