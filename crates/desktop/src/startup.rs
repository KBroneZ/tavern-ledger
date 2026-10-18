//! "Start with Windows": what the setting means and how each edition does it.
//!
//! The installer edition writes a `Run` entry through the autostart plugin
//! ([`RunKey`]). The Microsoft Store edition cannot: a package's registry
//! writes are virtualized, so Windows never sees them. It uses the package's
//! start-up task instead (`windows.startupTask` in the manifest, the WinRT
//! `StartupTask` API: `Task`, `store` feature only). The setting is off until
//! the player turns it on, in both.
//!
//! A start-up task can be switched off by the player in Task Manager or by a
//! policy, and the app cannot undo that ([`StartupState`]); the window says so
//! instead of pretending the switch worked. What [`apply`] decides is plain
//! logic over a [`Backend`], tested with a fake one.

use serde::Serialize;
use tauri::AppHandle;

use crate::edition;

/// The task's name in the manifest (`windows.startupTask`).
#[cfg_attr(not(feature = "store"), allow(dead_code))]
pub const TASK_ID: &str = "TavernLedgerStartup";

pub const FAILED: &str = "Could not change the Windows start-up setting.";
pub const BLOCKED_BY_USER: &str = "Windows has Tavern Ledger's start-up turned off. Turn it on in Task Manager > Startup apps; the app cannot do it for you.";
pub const BLOCKED_BY_POLICY: &str =
    "Your administrator or a policy decides whether Tavern Ledger starts with Windows.";

/// Where start with Windows stands, as Windows reports it. The five values are
/// the `StartupTaskState` ones; the installer edition only has `Off` and `On`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupState {
    Off,
    On,
    /// The player turned it off in Task Manager; only they can turn it on.
    OffByUser,
    /// An administrator or a policy turned it off.
    OffByPolicy,
    /// An administrator or a policy turned it on.
    OnByPolicy,
}

impl StartupState {
    /// From `StartupTaskState`'s value: 0 Disabled, 1 DisabledByUser,
    /// 2 Enabled, 3 DisabledByPolicy, 4 EnabledByPolicy. A newer Windows'
    /// value is not guessed.
    #[cfg_attr(not(feature = "store"), allow(dead_code))]
    pub fn from_task_state(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::Off),
            1 => Some(Self::OffByUser),
            2 => Some(Self::On),
            3 => Some(Self::OffByPolicy),
            4 => Some(Self::OnByPolicy),
            _ => None,
        }
    }

    pub fn enabled(self) -> bool {
        matches!(self, Self::On | Self::OnByPolicy)
    }

    /// Why the switch is stuck, when it is.
    pub fn blocked_message(self) -> Option<&'static str> {
        match self {
            Self::OffByUser => Some(BLOCKED_BY_USER),
            Self::OffByPolicy | Self::OnByPolicy => Some(BLOCKED_BY_POLICY),
            Self::Off | Self::On => None,
        }
    }
}

/// One way of starting with Windows. Each call is a single OS operation.
pub trait Backend {
    fn state(&self) -> Result<StartupState, String>;
    fn enable(&self) -> Result<(), String>;
    fn disable(&self) -> Result<(), String>;
}

/// Turns start with Windows on or off and returns the state Windows reports
/// afterwards. Never claims a change it did not see: a request that leaves
/// the state as it was is an error, and a switch the app cannot flip says why
/// without calling Windows.
pub fn apply(backend: &dyn Backend, on: bool) -> Result<StartupState, String> {
    let before = backend.state()?;
    if before.enabled() == on {
        return Ok(before);
    }
    if let Some(why) = before.blocked_message() {
        return Err(why.to_string());
    }
    if on {
        backend.enable()?;
    } else {
        backend.disable()?;
    }
    let after = backend.state()?;
    if after.enabled() == on {
        return Ok(after);
    }
    Err(after.blocked_message().unwrap_or(FAILED).to_string())
}

/// What the Settings page needs.
#[derive(Serialize)]
pub struct View {
    pub available: bool,
    pub enabled: bool,
    pub state: StartupState,
    /// Why the switch cannot be flipped, when the state says so.
    pub blocked: Option<&'static str>,
}

impl View {
    pub fn of(available: bool, state: StartupState) -> Self {
        Self {
            available,
            enabled: state.enabled(),
            state,
            blocked: state.blocked_message(),
        }
    }
}

/// Whether the app can offer the setting at all. The installer edition needs
/// its own path; the Store edition needs to run from the package (a Store build
/// started from a build folder has no start-up task).
pub fn available() -> bool {
    if edition::STORE {
        return store::has_package_identity();
    }
    std::env::current_exe().is_ok()
}

/// The state now; `Off` when Windows cannot say.
pub fn state(app: &AppHandle) -> StartupState {
    with_backend(app, |backend| backend.state()).unwrap_or(StartupState::Off)
}

/// [`apply`] on this edition's backend.
pub fn set(app: &AppHandle, on: bool) -> Result<StartupState, String> {
    with_backend(app, |backend| apply(backend, on))
}

#[cfg(not(feature = "store"))]
fn with_backend<T>(app: &AppHandle, work: impl FnOnce(&dyn Backend) -> T) -> T {
    work(&run_key::RunKey(app))
}

#[cfg(feature = "store")]
fn with_backend<T>(_app: &AppHandle, work: impl FnOnce(&dyn Backend) -> T) -> T {
    work(&store::Task)
}

/// Whether Windows started this process through the package's start-up task
/// (the window then stays in the tray). Always false in the installer edition,
/// which passes `--minimized` instead.
pub fn started_at_sign_in() -> bool {
    edition::STORE && store::started_by_startup_task()
}

#[cfg(not(feature = "store"))]
mod run_key {
    use std::io;
    use std::path::Path;

    use tauri::AppHandle;
    use tauri_plugin_autostart::ManagerExt;

    use super::{Backend, StartupState, FAILED};
    use crate::applog;

    /// Logs the plugin's or Windows' own words and gives the window the fixed
    /// sentence, which is translated and never carries a path.
    fn failed(what: &str, error: impl std::fmt::Display) -> String {
        applog::warn("startup", &format!("{what} failed: {error}"));
        FAILED.to_string()
    }

    /// Windows' list of programs to run at sign-in, where auto-launch writes
    /// the start-up entry.
    #[cfg(windows)]
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    /// The installer edition: a `Run` entry written by the autostart plugin.
    pub struct RunKey<'a>(pub &'a AppHandle);

    impl Backend for RunKey<'_> {
        fn state(&self) -> Result<StartupState, String> {
            // A start-up entry that cannot be read is no entry (as before).
            let on = self.0.autolaunch().is_enabled().unwrap_or_else(|e| {
                applog::warn(
                    "startup",
                    &format!("reading the start-up entry failed: {e}"),
                );
                false
            });
            Ok(if on {
                StartupState::On
            } else {
                StartupState::Off
            })
        }

        fn enable(&self) -> Result<(), String> {
            let launcher = self.0.autolaunch();
            launcher.enable().map_err(|e| failed("turning on", e))?;
            // An unquoted entry could start the wrong program: take it back.
            // The rewrite's error is the one reported.
            quote_startup_entry(self.0).map_err(|e| {
                let _ = launcher.disable();
                failed("quoting the entry", e)
            })
        }

        fn disable(&self) -> Result<(), String> {
            self.0
                .autolaunch()
                .disable()
                .map_err(|e| failed("turning off", e))
        }
    }

    /// The start-up entry's command line with the exe path in quotes.
    /// auto-launch 0.6 writes the path unquoted, and Windows cuts an unquoted
    /// path at its first space (the installer's folder `Tavern Ledger`, or a
    /// user name with a space), which runs the wrong program or none.
    pub(super) fn startup_command(exe: &Path, args: &[&str]) -> String {
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
}

#[cfg(not(feature = "store"))]
mod store {
    pub fn has_package_identity() -> bool {
        false
    }

    pub fn started_by_startup_task() -> bool {
        false
    }
}

/// The Store edition: the package's start-up task, through WinRT. Every call
/// runs on its own short-lived thread that joins the multithreaded apartment
/// first, so none depends on what the calling thread did with COM.
#[cfg(feature = "store")]
mod store {
    use std::thread;

    use windows::core::HSTRING;
    use windows::ApplicationModel::Activation::ActivationKind;
    use windows::ApplicationModel::{AppInstance, StartupTask};
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

    use super::{Backend, StartupState, FAILED, TASK_ID};
    use crate::applog;

    pub struct Task;

    /// Balances a successful `RoInitialize` when the worker thread is done,
    /// after every WinRT object it made has been dropped.
    struct Apartment;

    impl Apartment {
        fn enter() -> windows::core::Result<Self> {
            // SAFETY: a plain call on this thread with no pointers; S_FALSE
            // (already initialized) is a success that also needs balancing.
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }?;
            Ok(Self)
        }
    }

    impl Drop for Apartment {
        fn drop(&mut self) {
            // SAFETY: only built by `enter`, after a successful RoInitialize
            // on this thread.
            unsafe { RoUninitialize() };
        }
    }

    /// Runs `work` on a thread of its own, in the multithreaded apartment.
    fn in_apartment<T: Send + 'static>(
        work: impl FnOnce() -> windows::core::Result<T> + Send + 'static,
    ) -> Result<T, String> {
        thread::spawn(move || {
            // A thread that cannot join the apartment cannot call WinRT at all.
            let _apartment = Apartment::enter()?;
            work()
        })
        .join()
        .map_err(|_| FAILED.to_string())?
        .map_err(|e| {
            applog::warn("startup", &format!("start-up task call failed: {e}"));
            FAILED.to_string()
        })
    }

    fn task() -> windows::core::Result<StartupTask> {
        StartupTask::GetAsync(&HSTRING::from(TASK_ID))?.join()
    }

    impl Backend for Task {
        fn state(&self) -> Result<StartupState, String> {
            let code = in_apartment(|| Ok(task()?.State()?.0))?;
            StartupState::from_task_state(code).ok_or_else(|| {
                applog::warn("startup", &format!("unknown start-up task state {code}"));
                FAILED.to_string()
            })
        }

        fn enable(&self) -> Result<(), String> {
            // The state is read again by the caller: the answer here is not
            // trusted over what Windows reports next.
            in_apartment(|| task()?.RequestEnableAsync()?.join().map(|_| ()))
        }

        fn disable(&self) -> Result<(), String> {
            in_apartment(|| task()?.Disable())
        }
    }

    /// Whether this process runs from an installed package: the call only
    /// asks for the name's length, which a process without identity cannot get
    /// (`APPMODEL_ERROR_NO_PACKAGE`).
    pub fn has_package_identity() -> bool {
        let mut length = 0u32;
        // SAFETY: `length` is a valid, writable u32 and no buffer is passed.
        let code = unsafe { GetCurrentPackageFullName(&mut length, None) };
        code == ERROR_INSUFFICIENT_BUFFER
    }

    /// Whether Windows activated this process for the start-up task.
    pub fn started_by_startup_task() -> bool {
        if !has_package_identity() {
            return false;
        }
        in_apartment(|| {
            Ok(AppInstance::GetActivatedEventArgs()?.Kind()? == ActivationKind::StartupTask)
        })
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// A backend that records what it was asked and reports a scripted state.
    struct Fake {
        state: RefCell<StartupState>,
        /// What the state becomes after `enable` / `disable` (None: unchanged).
        after_enable: Option<StartupState>,
        after_disable: Option<StartupState>,
        calls: RefCell<Vec<&'static str>>,
        fail: Option<&'static str>,
    }

    impl Fake {
        fn new(state: StartupState) -> Self {
            Self {
                state: RefCell::new(state),
                after_enable: Some(StartupState::On),
                after_disable: Some(StartupState::Off),
                calls: RefCell::new(Vec::new()),
                fail: None,
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.borrow().clone()
        }
    }

    impl Backend for Fake {
        fn state(&self) -> Result<StartupState, String> {
            self.calls.borrow_mut().push("state");
            Ok(*self.state.borrow())
        }

        fn enable(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("enable");
            if let Some(error) = self.fail {
                return Err(error.into());
            }
            if let Some(next) = self.after_enable {
                *self.state.borrow_mut() = next;
            }
            Ok(())
        }

        fn disable(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("disable");
            if let Some(error) = self.fail {
                return Err(error.into());
            }
            if let Some(next) = self.after_disable {
                *self.state.borrow_mut() = next;
            }
            Ok(())
        }
    }

    #[test]
    fn the_five_windows_states_map_one_to_one_and_a_new_one_is_not_guessed() {
        use StartupState::*;
        let all = [Off, OffByUser, On, OffByPolicy, OnByPolicy];
        for (code, want) in all.into_iter().enumerate() {
            assert_eq!(StartupState::from_task_state(code as i32), Some(want));
        }
        assert_eq!(StartupState::from_task_state(5), None);
        assert_eq!(StartupState::from_task_state(-1), None);
    }

    #[test]
    fn only_off_and_on_can_be_changed_from_the_app() {
        use StartupState::*;
        for (state, enabled, can_change) in [
            (Off, false, true),
            (On, true, true),
            (OffByUser, false, false),
            (OffByPolicy, false, false),
            (OnByPolicy, true, false),
        ] {
            assert_eq!(state.enabled(), enabled, "{state:?}");
            assert_eq!(state.blocked_message().is_some(), !can_change, "{state:?}");
        }
    }

    #[test]
    fn turning_on_and_off_goes_through_windows_and_reports_what_it_says() {
        let fake = Fake::new(StartupState::Off);
        assert_eq!(apply(&fake, true), Ok(StartupState::On));
        assert_eq!(fake.calls(), ["state", "enable", "state"]);
        assert_eq!(apply(&fake, false), Ok(StartupState::Off));
        assert_eq!(&fake.calls()[3..], ["state", "disable", "state"]);
    }

    #[test]
    fn asking_for_the_state_it_already_has_calls_nothing() {
        let fake = Fake::new(StartupState::On);
        assert_eq!(apply(&fake, true), Ok(StartupState::On));
        assert_eq!(fake.calls(), ["state"]);
        let by_policy = Fake::new(StartupState::OnByPolicy);
        assert_eq!(apply(&by_policy, true), Ok(StartupState::OnByPolicy));
        assert_eq!(by_policy.calls(), ["state"]);
    }

    #[test]
    fn a_task_the_player_turned_off_stays_off_and_the_app_says_how_to_turn_it_on() {
        let fake = Fake::new(StartupState::OffByUser);
        assert_eq!(apply(&fake, true), Err(BLOCKED_BY_USER.to_string()));
        assert_eq!(
            fake.calls(),
            ["state"],
            "no request that Windows would ignore"
        );
        assert!(BLOCKED_BY_USER.contains("Task Manager"));
        // Asking to turn it off is already true.
        assert_eq!(apply(&fake, false), Ok(StartupState::OffByUser));
    }

    #[test]
    fn a_policy_decides_both_ways() {
        let off = Fake::new(StartupState::OffByPolicy);
        assert_eq!(apply(&off, true), Err(BLOCKED_BY_POLICY.to_string()));
        let on = Fake::new(StartupState::OnByPolicy);
        assert_eq!(apply(&on, false), Err(BLOCKED_BY_POLICY.to_string()));
        assert_eq!(on.calls(), ["state"]);
    }

    #[test]
    fn a_request_that_changes_nothing_is_an_error_not_a_success() {
        let mut fake = Fake::new(StartupState::Off);
        fake.after_enable = None;
        assert_eq!(apply(&fake, true), Err(FAILED.to_string()));
        // Windows says the player's choice won in the meantime.
        let mut raced = Fake::new(StartupState::Off);
        raced.after_enable = Some(StartupState::OffByUser);
        assert_eq!(apply(&raced, true), Err(BLOCKED_BY_USER.to_string()));
    }

    #[test]
    fn a_failing_call_is_passed_on() {
        let mut fake = Fake::new(StartupState::Off);
        fake.fail = Some("boom");
        assert_eq!(apply(&fake, true), Err("boom".to_string()));
    }

    #[test]
    fn a_failing_turn_off_is_passed_on_and_leaves_the_state_as_it_was() {
        let mut fake = Fake::new(StartupState::On);
        fake.fail = Some("boom");
        assert_eq!(apply(&fake, false), Err("boom".to_string()));
        assert_eq!(*fake.state.borrow(), StartupState::On);
    }

    #[test]
    fn every_sentence_the_window_may_show_has_its_spanish_words() {
        let words = include_str!("../ui/strings/common.js");
        for sentence in [FAILED, BLOCKED_BY_USER, BLOCKED_BY_POLICY] {
            assert!(
                words.contains(&format!("\"{sentence}\":")),
                "no translation for {sentence:?} in ui/strings/common.js"
            );
        }
    }

    #[test]
    fn the_view_tells_the_page_what_to_show() {
        let view = View::of(true, StartupState::OffByUser);
        assert!(view.available && !view.enabled);
        assert_eq!(view.blocked, Some(BLOCKED_BY_USER));
        let json = serde_json::to_value(View::of(true, StartupState::OnByPolicy)).unwrap();
        assert_eq!(json["state"], "on_by_policy");
        assert_eq!(json["enabled"], true);
        assert!(View::of(true, StartupState::On).blocked.is_none());
    }

    #[cfg(not(feature = "store"))]
    #[test]
    fn the_run_key_command_quotes_the_path() {
        let exe =
            std::path::Path::new(r"C:\Users\A B\AppData\Local\Tavern Ledger\TavernLedger.exe");
        assert_eq!(
            run_key::startup_command(exe, &["--minimized"]),
            r#""C:\Users\A B\AppData\Local\Tavern Ledger\TavernLedger.exe" --minimized"#
        );
    }

    #[cfg(not(feature = "store"))]
    #[test]
    fn the_run_key_command_quotes_a_path_without_spaces_too() {
        let exe = std::path::Path::new(r"C:\Apps\desktop.exe");
        assert_eq!(
            run_key::startup_command(exe, &[]),
            r#""C:\Apps\desktop.exe""#
        );
    }

    #[test]
    fn a_test_run_is_not_a_sign_in() {
        assert!(!started_at_sign_in(), "a test run is not a sign-in");
    }
}
