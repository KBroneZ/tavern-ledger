//! The setup notice (T-376, D-154): what the window says about `log.config`
//! and `client.config`, and the Fix button's work.
//!
//! The notice is worked out from one place only: the Hearthstone install
//! folder the app decided to use (automatic or chosen) and `%LOCALAPPDATA%`.
//! The tracker thread says when the decision changes and the check runs again
//! at once, so a folder choice, a fix by hand and a click on Fix all show in
//! the notice without waiting. The windows never send a path: the two files
//! are derived here, and nothing writes them but [`fix_files`] on a click.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tracker::setup::{
    check_install, FileCheck, SetupReport, Wanted, CLIENT_CONFIG_FILE, LOADING_SCREEN, LOG,
    LOG_CONFIG_FILE, POWER,
};
use tracker::setup_fix::{client_config_target, fix, log_config_target, Outcome, Reason, Target};

use crate::package;

/// `log.config` with only what the app needs.
const REQUIRED_LOG_CONFIG: [Wanted; 1] = [POWER];
/// `log.config` with the optional scene lines too.
const WITH_OPTIONAL_LOG_CONFIG: [Wanted; 2] = [POWER, LOADING_SCREEN];
const CLIENT_CONFIG: [Wanted; 1] = [LOG];

/// Where the check looks: the folder decision, as the tracker thread last made it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    /// The Hearthstone install in use; `None` while there is none.
    pub install_dir: Option<PathBuf>,
    /// What to say instead of the file checks while no folder can be used.
    pub folder_lines: Vec<String>,
}

#[derive(Default)]
pub struct SetupState {
    place: Mutex<Place>,
    /// Development builds only: another `%LOCALAPPDATA%`, so screenshots and
    /// tests never touch the player's real `log.config`.
    local_app_data: Mutex<Option<PathBuf>>,
    fixing: AtomicBool,
    /// This edition may not write the two files (the Store edition where the
    /// manifest's excluded folder cannot be relied on): the box lists the
    /// lines to write by hand and offers no Fix.
    manual_only: AtomicBool,
    /// The tracker stopped (or never started here): the folder decision is
    /// no longer followed, so nothing is offered to write.
    frozen: AtomicBool,
    /// One refresh at a time, so an older view never lands after a newer one.
    refresh: Mutex<()>,
    last_fix: Mutex<Option<FixView>>,
    fix_count: AtomicU64,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

impl SetupState {
    pub fn set_place(&self, place: Place) {
        let mut current = lock(&self.place);
        // What a click did in another folder says nothing about this one.
        if current.install_dir != place.install_dir {
            *lock(&self.last_fix) = None;
        }
        *current = place;
    }

    /// The folder decision is no longer followed (the tracker stopped): the
    /// notice still says what it knew, but offers nothing to write.
    pub fn freeze(&self) {
        self.frozen.store(true, Ordering::SeqCst);
    }

    pub fn set_manual_only(&self, manual: bool) {
        self.manual_only.store(manual, Ordering::SeqCst);
    }

    /// Whether the Fix button may write the two files at all.
    pub fn fix_allowed(&self) -> bool {
        !self.manual_only.load(Ordering::SeqCst)
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen.load(Ordering::SeqCst)
    }

    /// Held while a view is worked out and published.
    pub fn refreshing(&self) -> std::sync::MutexGuard<'_, ()> {
        lock(&self.refresh)
    }

    pub fn place(&self) -> Place {
        lock(&self.place).clone()
    }

    /// Development builds: use this folder in place of `%LOCALAPPDATA%`.
    pub fn use_local_app_data(&self, dir: Option<PathBuf>) {
        *lock(&self.local_app_data) = dir;
    }

    /// `%LOCALAPPDATA%`, or the development stand-in.
    pub fn local_app_data(&self) -> Option<PathBuf> {
        let injected = lock(&self.local_app_data).clone();
        injected.or_else(|| {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .filter(|dir| dir.is_absolute())
        })
    }

    /// The check, now, for the folders in use. Reads the two files and
    /// nothing else; never writes.
    pub fn report(&self) -> SetupReport {
        let place = self.place();
        check_install(
            self.local_app_data().as_deref(),
            place.install_dir.as_deref(),
        )
    }

    /// What the notice shows now: its lines and the button's state.
    pub fn view(&self) -> (Vec<String>, SetupView) {
        let place = self.place();
        let report = place.folder_lines.is_empty().then(|| self.report());
        let last = lock(&self.last_fix).clone();
        let fixing = self.fixing.load(Ordering::SeqCst);
        let (lines, mut view) =
            compose(report.as_ref(), &place.folder_lines, fixing, last.as_ref());
        if self.is_frozen() && !fixing {
            // Said as lines only: no sentence about fixing, no buttons.
            view.state = "unknown";
            view.can_fix = false;
            view.can_fix_optional = false;
        } else if !self.fix_allowed() {
            // The lines say what to write; there is nothing to click.
            if matches!(view.state, "needs_fix" | "error") {
                view.state = "manual";
            }
            view.can_fix = false;
            view.can_fix_optional = false;
        }
        (lines, view)
    }

    /// Claims the right to fix; `None` while another click is still working.
    /// The claim is given back when the ticket is dropped, whatever happens.
    pub fn begin_fix(&self) -> Option<FixTicket<'_>> {
        self.fixing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| FixTicket(self))
    }

    pub fn next_fix_id(&self) -> u64 {
        self.fix_count.fetch_add(1, Ordering::SeqCst) + 1
    }
}

/// The right to run a fix; only one exists at a time.
pub struct FixTicket<'a>(&'a SetupState);

impl FixTicket<'_> {
    /// Keeps what the click did, for the notice, and gives the claim back.
    pub fn finish(self, result: FixView) {
        *lock(&self.0.last_fix) = Some(result);
    }
}

impl Drop for FixTicket<'_> {
    fn drop(&mut self) {
        self.0.fixing.store(false, Ordering::SeqCst);
    }
}

/// The button's state, as the window needs it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SetupView {
    /// `unknown` (not checked yet), `ok`, `needs_fix`, `cannot_check`,
    /// `fixing` or `error` (the last Fix did not work and a problem remains).
    pub state: &'static str,
    /// A needed line or file is missing and the app can write it.
    pub can_fix: bool,
    /// Only the optional scene lines are missing and the app can write them.
    pub can_fix_optional: bool,
    /// The last click's result, shown until the player closes it.
    pub fix: Option<FixView>,
}

impl Default for SetupView {
    fn default() -> Self {
        Self {
            state: "unknown",
            can_fix: false,
            can_fix_optional: false,
            fix: None,
        }
    }
}

/// What the last click did, file by file.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FixView {
    /// Counts the clicks; a window that closed one result does not close the next.
    pub id: u64,
    pub files: Vec<FileFix>,
    /// Hearthstone was seen running: logging only changes after a restart.
    pub game_running: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FileFix {
    pub file: &'static str,
    /// `fixed`, `created`, `already_ok`, `refused` or `failed`.
    pub result: &'static str,
    pub reason: Option<Reason>,
    /// An older file was copied to `.bak` (or `.bak2`...) first.
    pub backup: bool,
}

impl FileFix {
    fn of(file: &'static str, outcome: Outcome) -> Self {
        let (result, reason, backup) = match outcome {
            Outcome::Fixed { created, backup } => {
                (if created { "created" } else { "fixed" }, None, backup)
            }
            Outcome::AlreadyOk => ("already_ok", None, false),
            Outcome::Refused(reason) => ("refused", Some(reason), false),
            Outcome::Failed(reason) => ("failed", Some(reason), false),
        };
        Self {
            file,
            result,
            reason,
            backup,
        }
    }

    fn worked(&self) -> bool {
        matches!(self.result, "fixed" | "created")
    }

    fn went_wrong(&self) -> bool {
        matches!(self.result, "refused" | "failed")
    }
}

fn needs_writing(check: &FileCheck) -> bool {
    matches!(
        check,
        FileCheck::FileMissing | FileCheck::SettingsMissing(_)
    )
}

/// The notice's lines and button state from a check (or, while no folder can
/// be used, from the lines saying so). Pure.
pub fn compose(
    report: Option<&SetupReport>,
    folder_lines: &[String],
    fixing: bool,
    last: Option<&FixView>,
) -> (Vec<String>, SetupView) {
    let Some(report) = report else {
        // No folder to check: the lines say why, and nothing can be fixed.
        let state = if fixing { "fixing" } else { "cannot_check" };
        let view = SetupView {
            state,
            fix: last.cloned(),
            ..SetupView::default()
        };
        return (folder_lines.to_vec(), view);
    };
    let lines: Vec<String> = report
        .messages()
        .into_iter()
        .chain(report.notes())
        .collect();
    let can_fix = needs_writing(&report.log_config) || needs_writing(&report.client_config);
    let can_fix_optional = !report.notes().is_empty();
    let problem = !report.is_ok();
    let failed = last.is_some_and(|fix| fix.files.iter().any(FileFix::went_wrong));
    let state = if fixing {
        "fixing"
    } else if can_fix && failed {
        "error"
    } else if can_fix {
        "needs_fix"
    } else if problem {
        "cannot_check"
    } else {
        "ok"
    };
    // A failure that no longer matters (the player fixed the file by hand) is
    // not shown again; what worked still is, for the restart note. While the
    // optional lines are still missing, a failed click stays: it may have been
    // the one that asked for them.
    let fix = last.cloned().and_then(|mut fix| {
        if !problem && !can_fix_optional {
            fix.files.retain(FileFix::worked);
        }
        (!fix.files.is_empty()).then_some(fix)
    });
    let view = SetupView {
        state,
        can_fix,
        can_fix_optional,
        fix,
    };
    (lines, view)
}

/// Fixes the two files, each on its own: one can work while the other does
/// not. The paths come from the folders the app uses, never from the window.
/// For an app that is not in a package (the tests).
#[cfg(test)]
pub fn fix_files(
    local_app_data: Option<&Path>,
    install_dir: Option<&Path>,
    optional: bool,
) -> Vec<FileFix> {
    fix_files_in(local_app_data, install_dir, optional, None)
}

/// [`fix_files`] for an app that may run from a package. `private_root` is the
/// package's private `LocalCache\Local` folder: a package's new files under
/// `%LOCALAPPDATA%` can end up there instead of where the game reads them,
/// and the app cannot tell from inside. A file written by this fix that has a
/// copy there is reported as failed, never as created or fixed.
pub fn fix_files_in(
    local_app_data: Option<&Path>,
    install_dir: Option<&Path>,
    optional: bool,
    private_root: Option<&Path>,
) -> Vec<FileFix> {
    let log_config = match local_app_data {
        None => Outcome::Refused(Reason::NoPath),
        Some(local) => {
            let wanted: &[Wanted] = if optional {
                &WITH_OPTIONAL_LOG_CONFIG
            } else {
                &REQUIRED_LOG_CONFIG
            };
            let path = log_config_target(local);
            let outcome = fix(&Target {
                path: &path,
                wanted,
                create_parent: true,
            });
            let redirected = private_root
                .and_then(|private| package::private_copy(private, local, &path))
                .is_some_and(|copy| copy.exists());
            match outcome {
                Outcome::Fixed { .. } if redirected => Outcome::Failed(Reason::Virtualized),
                other => other,
            }
        }
    };
    let client_config = match install_dir.map(client_config_target) {
        None => Outcome::Refused(Reason::NoPath),
        Some(Err(reason)) => Outcome::Refused(reason),
        Some(Ok(path)) => fix(&Target {
            path: &path,
            wanted: &CLIENT_CONFIG,
            create_parent: false,
        }),
    };
    vec![
        FileFix::of(LOG_CONFIG_FILE, log_config),
        FileFix::of(CLIENT_CONFIG_FILE, client_config),
    ]
}

#[cfg(test)]
#[path = "setup_notice_tests.rs"]
mod tests;
