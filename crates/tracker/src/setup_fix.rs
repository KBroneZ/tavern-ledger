//! The Fix button's file work (D-154): create or complete `log.config` and
//! `client.config` on the player's click, and nothing else.
//!
//! The only files this writes are the target, its backup (`<name>.bak`, then
//! `.bak2`, ...) and a temp file beside it, which is removed on every path.
//! The target's path is never given by a caller's text: [`log_config_target`]
//! and [`client_config_target`] derive the two paths from folders the app
//! already uses. A target that is a link (symbolic link, junction, any reparse
//! point, or a file with another hard link) is refused. Backup and temp are
//! made with "create new", which never follows or overwrites anything that is
//! already there. The target is replaced by a rename of the finished temp file
//! (no delete first), so a failure leaves the original as it was.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

use serde::Serialize;

use crate::setup::{merge_config, Wanted, CLIENT_CONFIG_FILE, LOG_CONFIG_FILE};

/// Where the game's installer-less `Hearthstone.exe` lives in an install folder.
const GAME_EXE: &str = "Hearthstone.exe";
/// `.bak`, `.bak2` ... `.bak99`: more than this and something is wrong.
const MAX_BACKUPS: u32 = 99;

/// Why a file was not fixed. The window has the words for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// A link or a file with another hard link.
    Link,
    /// A folder, or anything but a plain file, where the file should be.
    NotAFile,
    /// The text is not UTF-8 or ASCII (for example UTF-16).
    NotPlainText,
    /// The folder the file goes in is not there (and may not be made).
    FolderMissing,
    /// The install folder has no `Hearthstone.exe`.
    NoGameExe,
    /// The app does not know where the file goes.
    NoPath,
    AccessDenied,
    ReadOnly,
    PathTooLong,
    /// Another program holds the file open.
    Locked,
    /// The file changed between reading and writing it.
    Changed,
    DiskFull,
    Other,
}

impl Reason {
    /// What a failed file operation means for the player.
    pub fn of_io(error: &io::Error) -> Reason {
        // Windows' own codes first: ACCESS_DENIED, WRITE_PROTECT, SHARING and
        // LOCK_VIOLATION, FILENAME_EXCED_RANGE, DISK_FULL and HANDLE_DISK_FULL,
        // PATH_NOT_FOUND.
        match error.raw_os_error() {
            Some(5) => return Reason::AccessDenied,
            Some(19) => return Reason::ReadOnly,
            Some(32 | 33) => return Reason::Locked,
            Some(206) => return Reason::PathTooLong,
            Some(39 | 112) => return Reason::DiskFull,
            Some(3) => return Reason::FolderMissing,
            _ => {}
        }
        match error.kind() {
            io::ErrorKind::PermissionDenied => Reason::AccessDenied,
            io::ErrorKind::ReadOnlyFilesystem => Reason::ReadOnly,
            io::ErrorKind::StorageFull => Reason::DiskFull,
            io::ErrorKind::NotFound => Reason::FolderMissing,
            _ => Reason::Other,
        }
    }
}

/// What the click did to one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Written; `backup` when an older file was copied to `.bak` first.
    Fixed { created: bool, backup: bool },
    /// The file already has everything: nothing written, no backup.
    AlreadyOk,
    /// Not touched, for a reason that is not an error of the disk.
    Refused(Reason),
    /// Tried and could not; the original is as it was.
    Failed(Reason),
}

/// One file to fix.
#[derive(Clone, Copy, Debug)]
pub struct Target<'a> {
    pub path: &'a Path,
    pub wanted: &'a [Wanted],
    /// Make the folder when it is not there: only for the game's per-user
    /// settings folder, never for the install folder.
    pub create_parent: bool,
}

/// `<local app data>\Blizzard\Hearthstone\log.config`.
pub fn log_config_target(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("Blizzard")
        .join("Hearthstone")
        .join(LOG_CONFIG_FILE)
}

/// `client.config` beside `Hearthstone.exe`, if the folder really is the
/// game's.
pub fn client_config_target(install_dir: &Path) -> Result<PathBuf, Reason> {
    if install_dir.join(GAME_EXE).is_file() {
        Ok(install_dir.join(CLIENT_CONFIG_FILE))
    } else {
        Err(Reason::NoGameExe)
    }
}

/// Which extra file is being made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Backup,
    Temp,
}

/// The steps that touch the disk, so tests can make one fail.
pub(crate) trait Io {
    /// A new, empty file that must not exist yet.
    fn create_new(&self, role: Role, path: &Path) -> io::Result<fs::File>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// Called after the backup and temp file are ready, before the check that
    /// the target did not change and the replace.
    fn before_replace(&self, _target: &Path) {}
}

pub(crate) struct RealIo;

impl Io for RealIo {
    fn create_new(&self, _role: Role, path: &Path) -> io::Result<fs::File> {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }
}

pub fn fix(target: &Target<'_>) -> Outcome {
    fix_with(&RealIo, target)
}

pub(crate) fn fix_with(io: &dyn Io, target: &Target<'_>) -> Outcome {
    match run(io, target) {
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failed(Reason::of_io(&error)),
    }
}

/// What a path holds right now.
#[derive(PartialEq, Eq)]
enum Snapshot {
    Absent,
    File {
        bytes: Vec<u8>,
        modified: Option<SystemTime>,
    },
}

/// Files made along the way; removed when this drops unless kept.
struct Made(Vec<PathBuf>);

impl Made {
    fn keep(mut self) {
        self.0.clear();
    }
}

impl Drop for Made {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn run(io: &dyn Io, target: &Target<'_>) -> io::Result<Outcome> {
    let path = target.path;
    let before = match snapshot(path)? {
        Err(reason) => return Ok(Outcome::Refused(reason)),
        Ok(snapshot) => snapshot,
    };
    let original: &[u8] = match &before {
        Snapshot::File { bytes, .. } => bytes,
        Snapshot::Absent => b"",
    };
    let merged = match merge_config(original, target.wanted) {
        Err(_) => return Ok(Outcome::Refused(Reason::NotPlainText)),
        Ok(None) => return Ok(Outcome::AlreadyOk),
        Ok(Some(merged)) => merged,
    };
    let existing = matches!(before, Snapshot::File { .. });
    if existing && fs::metadata(path)?.permissions().readonly() {
        return Ok(Outcome::Failed(Reason::ReadOnly));
    }
    let dir = path.parent().unwrap_or(Path::new(""));
    if !existing && !dir.is_dir() {
        if !target.create_parent {
            return Ok(Outcome::Refused(Reason::FolderMissing));
        }
        fs::create_dir_all(dir)?;
    }
    let mut made = Made(Vec::new());
    let temp = temp_path(path);
    write_new(io, Role::Temp, &temp, &merged, &mut made)?;
    if existing {
        make_backup(io, path, original, &mut made)?;
    }
    io.before_replace(path);
    if snapshot(path)? != Ok(before) {
        return Ok(Outcome::Failed(Reason::Changed));
    }
    io.rename(&temp, path)?;
    // The temp file is the target now, and the backup stays.
    made.keep();
    Ok(Outcome::Fixed {
        created: !existing,
        backup: existing,
    })
}

/// Writes a new file with these bytes and puts it to disk.
fn write_new(
    io: &dyn Io,
    role: Role,
    path: &Path,
    bytes: &[u8],
    made: &mut Made,
) -> io::Result<()> {
    let mut file = io.create_new(role, path)?;
    made.0.push(path.to_path_buf());
    file.write_all(bytes)?;
    file.sync_all()
}

/// `<name>.bak`, or the first of `.bak2`, `.bak3`... that is free.
fn make_backup(io: &dyn Io, path: &Path, bytes: &[u8], made: &mut Made) -> io::Result<()> {
    for number in 1..=MAX_BACKUPS {
        let suffix = if number == 1 {
            ".bak".to_string()
        } else {
            format!(".bak{number}")
        };
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        let backup = PathBuf::from(name);
        match write_new(io, Role::Backup, &backup, bytes, made) {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            other => return other,
        }
    }
    Err(io::Error::from(io::ErrorKind::AlreadyExists))
}

fn temp_path(path: &Path) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(
        ".tl-{}-{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    PathBuf::from(name)
}

/// What is at `path`, or why the app will not touch it. A link is looked at,
/// never followed.
fn snapshot(path: &Path) -> io::Result<Result<Snapshot, Reason>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Ok(Snapshot::Absent));
        }
        Err(error) => return Err(error),
    };
    if is_link(&meta) {
        return Ok(Err(Reason::Link));
    }
    if !meta.is_file() {
        return Ok(Err(Reason::NotAFile));
    }
    if link_count(path)? > 1 {
        return Ok(Err(Reason::Link));
    }
    Ok(Ok(Snapshot::File {
        bytes: fs::read(path)?,
        modified: meta.modified().ok(),
    }))
}

#[cfg(windows)]
fn is_link(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    /// `FILE_ATTRIBUTE_REPARSE_POINT`: symbolic links, junctions, mount points.
    const REPARSE_POINT: u32 = 0x400;
    meta.file_type().is_symlink() || meta.file_attributes() & REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link(meta: &fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

#[cfg(windows)]
fn link_count(path: &Path) -> io::Result<u32> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
    };
    let file = fs::File::open(path)?;
    // SAFETY: an all-zero value is a valid, empty BY_HANDLE_FILE_INFORMATION
    // (plain integers only).
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle belongs to `file`, which is open for the whole call,
    // and `info` is a valid place for the answer.
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info.nNumberOfLinks)
}

#[cfg(unix)]
fn link_count(path: &Path) -> io::Result<u32> {
    use std::os::unix::fs::MetadataExt;
    Ok(fs::symlink_metadata(path)?
        .nlink()
        .try_into()
        .unwrap_or(u32::MAX))
}

#[cfg(not(any(windows, unix)))]
fn link_count(_path: &Path) -> io::Result<u32> {
    Ok(1)
}

#[cfg(test)]
#[path = "setup_fix_tests.rs"]
mod tests;
