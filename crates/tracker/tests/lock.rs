//! Only one process may write the game history at a time (the app or
//! tavern-watch): the second one is told so instead of writing too.

use std::fs;

use tracker::lock::{HistoryLock, LockError};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tavern-lock-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn second_holder_is_refused_while_the_first_holds_it() {
    let dir = temp_dir("busy");
    let first = HistoryLock::acquire(&dir).expect("first lock");
    assert!(matches!(HistoryLock::acquire(&dir), Err(LockError::Busy)));
    drop(first);
    let _again = HistoryLock::acquire(&dir).expect("free after the first is dropped");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn locking_creates_the_folder_and_leaves_the_history_alone() {
    let dir = temp_dir("fresh").join("nested");
    let _lock = HistoryLock::acquire(&dir).expect("lock in a new folder");
    assert!(dir.is_dir());
    assert!(!dir.join(tracker::store::FILE_NAME).exists());
    let _ = fs::remove_dir_all(dir.parent().unwrap());
}

#[test]
fn a_leftover_lock_file_does_not_block() {
    // A crash leaves the file behind; the OS lock is gone with the process.
    let dir = temp_dir("stale");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(tracker::lock::LOCK_FILE), b"").unwrap();
    let _lock = HistoryLock::acquire(&dir).expect("a stale file is not a lock");
    let _ = fs::remove_dir_all(&dir);
}
