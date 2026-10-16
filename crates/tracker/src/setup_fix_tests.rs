//! Tests of the Fix button's file work (D-154), in temp folders only: never
//! the player's own `log.config` or `client.config`.

use super::*;
use crate::setup::{check_setup, FileCheck, LOADING_SCREEN, LOG, POWER};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU32, Ordering};

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tavern-ledger-fix-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn target<'a>(path: &'a Path, wanted: &'a [Wanted]) -> Target<'a> {
    Target {
        path,
        wanted,
        create_parent: false,
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

#[test]
fn a_missing_file_is_created_with_exactly_the_required_content() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    let outcome = fix(&target(&path, &[LOG]));
    assert_eq!(
        outcome,
        Outcome::Fixed {
            created: true,
            backup: false
        }
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        b"[Log]\r\nFileSizeLimit.Int=-1\r\n"
    );
    assert_eq!(names(&dir), ["client.config"], "no backup, no temp file");
}

#[test]
fn the_logs_folder_is_created_only_when_asked() {
    let dir = temp_dir();
    let path = dir.join("Blizzard").join("Hearthstone").join("log.config");
    assert_eq!(
        fix(&target(&path, &[POWER])),
        Outcome::Refused(Reason::FolderMissing)
    );
    assert!(!dir.join("Blizzard").exists());
    let outcome = fix(&Target {
        create_parent: true,
        ..target(&path, &[POWER])
    });
    assert!(matches!(outcome, Outcome::Fixed { created: true, .. }));
    assert!(path.is_file());
}

#[test]
fn an_existing_file_is_copied_to_bak_first_and_only_missing_lines_are_added() {
    let dir = temp_dir();
    let path = dir.join("log.config");
    let original = "; mine\n[Decks]\nLogLevel=1\n";
    fs::write(&path, original).unwrap();
    let outcome = fix(&target(&path, &[POWER]));
    assert_eq!(
        outcome,
        Outcome::Fixed {
            created: false,
            backup: true
        }
    );
    assert_eq!(
        fs::read_to_string(dir.join("log.config.bak")).unwrap(),
        original
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        format!("{original}\n[Power]\nLogLevel=1\nFilePrinting=true\nVerbose=true\n")
    );
    assert_eq!(names(&dir), ["log.config", "log.config.bak"]);
}

#[test]
fn an_older_backup_is_never_overwritten() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(dir.join("client.config.bak"), "older").unwrap();
    fs::write(dir.join("client.config.bak2"), "older still").unwrap();
    fs::write(&path, "[Log]\nFileSizeLimit.Int=10000\n").unwrap();
    assert!(matches!(
        fix(&target(&path, &[LOG])),
        Outcome::Fixed { backup: true, .. }
    ));
    assert_eq!(
        fs::read_to_string(dir.join("client.config.bak")).unwrap(),
        "older"
    );
    assert_eq!(
        fs::read_to_string(dir.join("client.config.bak2")).unwrap(),
        "older still"
    );
    assert_eq!(
        fs::read_to_string(dir.join("client.config.bak3")).unwrap(),
        "[Log]\nFileSizeLimit.Int=10000\n"
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "[Log]\nFileSizeLimit.Int=-1\n"
    );
}

#[test]
fn the_second_run_changes_nothing_and_makes_no_new_backup() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(&path, "[Graphics]\nWidth=1\n").unwrap();
    assert!(matches!(fix(&target(&path, &[LOG])), Outcome::Fixed { .. }));
    let after_first = fs::read(&path).unwrap();
    let files = names(&dir);
    assert_eq!(fix(&target(&path, &[LOG])), Outcome::AlreadyOk);
    assert_eq!(fs::read(&path).unwrap(), after_first);
    assert_eq!(names(&dir), files);
}

#[test]
fn what_the_fix_writes_passes_the_setup_check() {
    let dir = temp_dir();
    let log = dir.join("log.config");
    let client = dir.join("client.config");
    fs::write(&client, "[Log]\nFileSizeLimit.Int=10000\n").unwrap();
    fix(&target(&log, &[POWER, LOADING_SCREEN]));
    fix(&target(&client, &[LOG]));
    let report = check_setup(Some(&log), Some(&client));
    assert!(report.is_ok());
    assert_eq!(report.loading_screen, FileCheck::Ok);
}

#[test]
fn text_that_is_not_plain_is_refused_and_nothing_is_written() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    let utf16: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("[Log]\n".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    fs::write(&path, &utf16).unwrap();
    assert_eq!(
        fix(&target(&path, &[LOG])),
        Outcome::Refused(Reason::NotPlainText)
    );
    assert_eq!(fs::read(&path).unwrap(), utf16);
    assert_eq!(names(&dir), ["client.config"]);
}

#[test]
fn a_folder_where_the_file_should_be_is_refused() {
    let dir = temp_dir();
    let path = dir.join("log.config");
    fs::create_dir(&path).unwrap();
    assert_eq!(
        fix(&target(&path, &[POWER])),
        Outcome::Refused(Reason::NotAFile)
    );
    assert_eq!(names(&dir), ["log.config"]);
}

#[test]
fn a_hard_linked_file_is_refused_and_both_names_stay_as_they_were() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    let other = dir.join("elsewhere.txt");
    fs::write(&other, "[Graphics]\n").unwrap();
    if fs::hard_link(&other, &path).is_err() {
        return; // This file system has no hard links.
    }
    assert_eq!(fix(&target(&path, &[LOG])), Outcome::Refused(Reason::Link));
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Graphics]\n");
    assert_eq!(fs::read_to_string(&other).unwrap(), "[Graphics]\n");
    assert_eq!(names(&dir), ["client.config", "elsewhere.txt"]);
}

#[test]
fn a_symbolic_link_is_refused_and_what_it_points_to_is_not_touched() {
    let dir = temp_dir();
    let real = dir.join("real.config");
    let path = dir.join("client.config");
    fs::write(&real, "[Graphics]\n").unwrap();
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(&real, &path);
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&real, &path);
    if made.is_err() {
        return; // Windows without the right to make links.
    }
    assert_eq!(fix(&target(&path, &[LOG])), Outcome::Refused(Reason::Link));
    assert_eq!(fs::read_to_string(&real).unwrap(), "[Graphics]\n");
    assert_eq!(names(&dir), ["client.config", "real.config"]);
}

#[test]
fn a_dangling_symbolic_link_is_refused_too() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(dir.join("gone"), &path);
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(dir.join("gone"), &path);
    if made.is_err() {
        return;
    }
    assert_eq!(fix(&target(&path, &[LOG])), Outcome::Refused(Reason::Link));
    assert!(!dir.join("gone").exists());
}

#[test]
fn a_read_only_file_fails_plainly_before_any_backup_is_made() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(&path, "[Graphics]\n").unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    assert_eq!(
        fix(&target(&path, &[LOG])),
        Outcome::Failed(Reason::ReadOnly)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Graphics]\n");
    assert_eq!(names(&dir), ["client.config"]);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions).unwrap();
}

/// Real file work with a few steps that can be made to fail or to act.
#[derive(Default)]
struct Spy {
    fail_temp: bool,
    fail_backup: bool,
    fail_rename: bool,
    /// Runs between the read and the replace, like another program writing.
    meanwhile: RefCell<Option<Box<dyn Fn()>>>,
    renames: Cell<u32>,
}

impl Io for Spy {
    fn create_new(&self, role: Role, path: &Path) -> io::Result<fs::File> {
        let denied = || Err(io::Error::from(io::ErrorKind::PermissionDenied));
        match role {
            Role::Temp if self.fail_temp => denied(),
            Role::Backup if self.fail_backup => denied(),
            _ => RealIo.create_new(role, path),
        }
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.renames.set(self.renames.get() + 1);
        if self.fail_rename {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        RealIo.rename(from, to)
    }
    fn before_replace(&self, _target: &Path) {
        if let Some(act) = self.meanwhile.borrow().as_ref() {
            act();
        }
    }
}

#[test]
fn a_backup_that_cannot_be_made_stops_everything_and_leaves_no_temp_file() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(&path, "[Graphics]\n").unwrap();
    let spy = Spy {
        fail_backup: true,
        ..Spy::default()
    };
    assert_eq!(
        fix_with(&spy, &target(&path, &[LOG])),
        Outcome::Failed(Reason::AccessDenied)
    );
    assert_eq!(spy.renames.get(), 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Graphics]\n");
    assert_eq!(names(&dir), ["client.config"]);
}

#[test]
fn a_temp_file_that_cannot_be_made_stops_before_any_backup() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(&path, "[Graphics]\n").unwrap();
    let spy = Spy {
        fail_temp: true,
        ..Spy::default()
    };
    assert_eq!(
        fix_with(&spy, &target(&path, &[LOG])),
        Outcome::Failed(Reason::AccessDenied)
    );
    assert_eq!(names(&dir), ["client.config"]);
}

#[test]
fn a_replace_that_fails_leaves_the_original_and_removes_temp_and_backup() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(&path, "[Graphics]\n").unwrap();
    let spy = Spy {
        fail_rename: true,
        ..Spy::default()
    };
    assert_eq!(
        fix_with(&spy, &target(&path, &[LOG])),
        Outcome::Failed(Reason::AccessDenied)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Graphics]\n");
    assert_eq!(names(&dir), ["client.config"]);
}

#[test]
fn a_file_changed_by_someone_else_in_between_is_not_replaced() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    fs::write(&path, "[Graphics]\n").unwrap();
    let spy = Spy::default();
    let elsewhere = path.clone();
    *spy.meanwhile.borrow_mut() = Some(Box::new(move || {
        fs::write(&elsewhere, "[Graphics]\nWidth=2\n").unwrap();
    }));
    assert_eq!(
        fix_with(&spy, &target(&path, &[LOG])),
        Outcome::Failed(Reason::Changed)
    );
    assert_eq!(spy.renames.get(), 0);
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Graphics]\nWidth=2\n");
    assert_eq!(names(&dir), ["client.config"]);
}

#[test]
fn a_file_that_appeared_in_between_is_not_replaced() {
    let dir = temp_dir();
    let path = dir.join("client.config");
    let spy = Spy::default();
    let elsewhere = path.clone();
    *spy.meanwhile.borrow_mut() = Some(Box::new(move || {
        fs::write(&elsewhere, "[Graphics]\n").unwrap();
    }));
    assert_eq!(
        fix_with(&spy, &target(&path, &[LOG])),
        Outcome::Failed(Reason::Changed)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "[Graphics]\n");
    assert_eq!(names(&dir), ["client.config"]);
}

#[test]
fn errors_are_told_apart_by_what_the_player_can_do() {
    let os = io::Error::from_raw_os_error;
    #[cfg(windows)]
    {
        assert_eq!(Reason::of_io(&os(5)), Reason::AccessDenied);
        assert_eq!(Reason::of_io(&os(19)), Reason::ReadOnly);
        assert_eq!(Reason::of_io(&os(32)), Reason::Locked);
        assert_eq!(Reason::of_io(&os(33)), Reason::Locked);
        assert_eq!(Reason::of_io(&os(206)), Reason::PathTooLong);
        assert_eq!(Reason::of_io(&os(112)), Reason::DiskFull);
        assert_eq!(Reason::of_io(&os(3)), Reason::FolderMissing);
    }
    assert_eq!(
        Reason::of_io(&io::Error::from(io::ErrorKind::PermissionDenied)),
        Reason::AccessDenied
    );
    assert_eq!(
        Reason::of_io(&io::Error::from(io::ErrorKind::ReadOnlyFilesystem)),
        Reason::ReadOnly
    );
    assert_eq!(
        Reason::of_io(&io::Error::from(io::ErrorKind::StorageFull)),
        Reason::DiskFull
    );
    assert_eq!(
        Reason::of_io(&io::Error::from(io::ErrorKind::Interrupted)),
        Reason::Other
    );
}

#[test]
fn the_two_targets_are_the_only_paths_the_app_derives() {
    let local = Path::new("local");
    assert_eq!(
        log_config_target(local),
        local
            .join("Blizzard")
            .join("Hearthstone")
            .join("log.config")
    );
    let dir = temp_dir();
    assert_eq!(
        client_config_target(&dir),
        Err(Reason::NoGameExe),
        "an install folder without Hearthstone.exe is refused"
    );
    fs::write(dir.join("Hearthstone.exe"), b"").unwrap();
    assert_eq!(client_config_target(&dir), Ok(dir.join("client.config")));
    let missing = dir.join("nope");
    assert_eq!(client_config_target(&missing), Err(Reason::NoGameExe));
}

#[cfg(windows)]
#[test]
fn a_junction_where_the_file_should_be_is_refused_and_nothing_is_written_through_it() {
    let dir = temp_dir();
    let real = dir.join("real");
    fs::create_dir(&real).unwrap();
    let path = dir.join("client.config");
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&path)
        .arg(&real)
        .output();
    if !made.is_ok_and(|out| out.status.success()) {
        return;
    }
    assert_eq!(fix(&target(&path, &[LOG])), Outcome::Refused(Reason::Link));
    assert!(fs::read_dir(&real).unwrap().next().is_none());
    fs::remove_dir(&path).unwrap();
}
