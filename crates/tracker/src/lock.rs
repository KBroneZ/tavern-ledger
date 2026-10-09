//! One writer for the game history at a time: the desktop app and
//! tavern-watch both take this lock before following the log (D-016).
//!
//! It is an OS file lock on `games.lock` next to the history, so the system
//! drops it when the process ends, crash included: a leftover file never
//! blocks the next start.

use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

pub const LOCK_FILE: &str = "games.lock";

#[derive(Debug)]
pub enum LockError {
    /// Another Tavern Ledger process holds the history.
    Busy,
    Io(io::Error),
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockError::Busy => write!(f, "another Tavern Ledger process is using the history"),
            LockError::Io(e) => write!(f, "cannot lock the history ({:?})", e.kind()),
        }
    }
}

impl std::error::Error for LockError {}

/// Held for as long as the process writes the history; dropping it unlocks.
#[derive(Debug)]
pub struct HistoryLock {
    _file: File,
}

impl HistoryLock {
    pub fn acquire(dir: &Path) -> Result<HistoryLock, LockError> {
        fs::create_dir_all(dir).map_err(LockError::Io)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(LOCK_FILE))
            .map_err(LockError::Io)?;
        match file.try_lock() {
            Ok(()) => Ok(HistoryLock { _file: file }),
            Err(TryLockError::WouldBlock) => Err(LockError::Busy),
            Err(TryLockError::Error(e)) => Err(LockError::Io(e)),
        }
    }
}
