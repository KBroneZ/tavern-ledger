//! Follows `Power.log` (or another of the game's logs, such as
//! `LoadingScreen.log`) in the newest session folder as the game writes it.
//!
//! Bytes are read once: a line still being written stays in the splitter
//! until its newline arrives. The game may rotate the file (`Power.log`
//! becomes `Power_old.log` and a new `Power.log` starts) or start a new
//! session folder; in both cases the rest of the old file is read first.
//! Rotation is detected by size and by the file's first bytes (on NTFS a
//! file recreated under the same name can keep the old creation time).

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use bg_parser::lines::{Chunk, LineSplitter};

use crate::discover::latest_session;

pub const POWER_LOG: &str = "Power.log";
pub const POWER_OLD_LOG: &str = "Power_old.log";
pub const LOADING_SCREEN_LOG: &str = "LoadingScreen.log";
pub const LOADING_SCREEN_OLD_LOG: &str = "LoadingScreen_old.log";
const READ_CHUNK: usize = 1 << 20;
const HEAD_LEN: usize = 256;
/// Bytes of the followed file read in one poll. A long real session takes a
/// few polls to catch up; a file that grows faster than any game writes
/// cannot keep one poll busy for long.
const POLL_BUDGET: u64 = 64 << 20;

/// What a poll found, in log order.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<'a> {
    /// A session folder starts being followed (its name, e.g. Hearthstone_2026_...).
    Session(&'a str),
    Line(&'a str),
    /// A line longer than bg_parser::lines::MAX_LINE was skipped.
    TooLong,
}

/// The file followed and the name the game gives it once rotated.
#[derive(Clone, Copy, Debug)]
struct Files {
    current: &'static str,
    old: &'static str,
}

#[derive(Debug)]
struct Position {
    dir: PathBuf,
    name: String,
    files: Files,
    offset: u64,
    /// First bytes of the file being read, to notice a rotation.
    head: Vec<u8>,
    splitter: LineSplitter,
}

#[derive(Debug)]
pub struct Follower {
    logs_dir: PathBuf,
    current: Option<Position>,
    files: Files,
    budget: u64,
}

impl Follower {
    /// Follows the newest session from its first line, so games played
    /// before the app started are not lost.
    pub fn new(logs_dir: impl Into<PathBuf>) -> Self {
        Self::of(logs_dir, POWER_LOG, POWER_OLD_LOG)
    }

    /// Follows `LoadingScreen.log` instead (T-314), the same way.
    pub fn loading_screen(logs_dir: impl Into<PathBuf>) -> Self {
        Self::of(logs_dir, LOADING_SCREEN_LOG, LOADING_SCREEN_OLD_LOG)
    }

    fn of(logs_dir: impl Into<PathBuf>, current: &'static str, old: &'static str) -> Self {
        Follower {
            logs_dir: logs_dir.into(),
            current: None,
            files: Files { current, old },
            budget: POLL_BUDGET,
        }
    }

    #[cfg(test)]
    fn with_budget(mut self, budget: u64) -> Self {
        self.budget = budget;
        self
    }

    pub fn poll(&mut self, on_event: &mut dyn FnMut(Event<'_>)) -> io::Result<()> {
        if let Some(latest) = latest_session(&self.logs_dir) {
            if self.current.as_ref().map(|p| &p.dir) != Some(&latest) {
                if let Some(mut old) = self.current.take() {
                    // The rest of the previous session; if it cannot be read
                    // now, the new session still starts.
                    let result = read_new(&mut old, u64::MAX, on_event);
                    old.splitter.finish(&mut forward(on_event));
                    self.start(latest, on_event);
                    result?;
                } else {
                    self.start(latest, on_event);
                }
            }
        }
        match self.current.as_mut() {
            Some(pos) => read_new(pos, self.budget, on_event),
            None => Ok(()),
        }
    }

    fn start(&mut self, dir: PathBuf, on_event: &mut dyn FnMut(Event<'_>)) {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        on_event(Event::Session(&name));
        self.current = Some(Position {
            dir,
            name,
            files: self.files,
            offset: 0,
            head: Vec::new(),
            splitter: LineSplitter::default(),
        });
    }

    /// The session folder being followed, if any.
    pub fn session(&self) -> Option<&str> {
        self.current.as_ref().map(|p| p.name.as_str())
    }

    /// True once the followed session has a Power.log (false means the game
    /// has not played yet, or logging is off in log.config).
    pub fn has_power_log(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|p| p.offset > 0 || p.dir.join(p.files.current).is_file())
    }
}

fn forward<'f>(on_event: &'f mut dyn FnMut(Event<'_>)) -> impl FnMut(Chunk<'_>) + 'f {
    move |chunk| match chunk {
        Chunk::Line(line) => on_event(Event::Line(&line)),
        Chunk::TooLong => on_event(Event::TooLong),
    }
}

/// Reads at most `budget` bytes of the current file; the rest waits for the
/// next poll.
fn read_new(
    pos: &mut Position,
    budget: u64,
    on_event: &mut dyn FnMut(Event<'_>),
) -> io::Result<()> {
    let path = pos.dir.join(pos.files.current);
    let len = match fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return drain_rotated(pos, on_event),
        Err(e) => return Err(e),
    };
    let head = read_head(&path)?;
    if pos.offset > 0 && (len < pos.offset || !same_start(&head, &pos.head)) {
        drain_rotated(pos, on_event)?;
    }
    pos.head = head;
    read_from(&path, pos, budget, on_event)
}

/// The file may have been shorter than HEAD_LEN when its head was stored.
fn same_start(a: &[u8], b: &[u8]) -> bool {
    let n = a.len().min(b.len());
    a[..n] == b[..n]
}

/// After a rotation the bytes not read yet are at the end of Power_old.log
/// (only if that file is the one we were reading: same first bytes).
fn drain_rotated(pos: &mut Position, on_event: &mut dyn FnMut(Event<'_>)) -> io::Result<()> {
    if pos.offset == 0 {
        return Ok(());
    }
    let old = pos.dir.join(pos.files.old);
    let is_ours = fs::metadata(&old).is_ok_and(|m| m.len() >= pos.offset)
        && read_head(&old).is_ok_and(|head| same_start(&head, &pos.head));
    if is_ours {
        read_from(&old, pos, u64::MAX, on_event)?;
    }
    pos.splitter.finish(&mut forward(on_event));
    pos.offset = 0;
    pos.head.clear();
    Ok(())
}

fn read_head(path: &Path) -> io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(HEAD_LEN);
    File::open(path)?
        .take(HEAD_LEN as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

/// Reads from `pos.offset` to the end or for `budget` bytes, advancing the
/// offset chunk by chunk, so an error halfway never makes the same bytes be
/// read twice.
fn read_from(
    path: &Path,
    pos: &mut Position,
    budget: u64,
    on_event: &mut dyn FnMut(Event<'_>),
) -> io::Result<()> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(pos.offset))?;
    let mut buf = vec![0u8; READ_CHUNK];
    let mut left = budget;
    loop {
        let want = buf.len().min(usize::try_from(left).unwrap_or(usize::MAX));
        let n = file.read(&mut buf[..want])?;
        if n == 0 {
            return Ok(());
        }
        left -= n as u64;
        pos.splitter.push(&buf[..n], &mut forward(on_event));
        pos.offset += n as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poll_reads_at_most_its_budget_and_the_next_polls_go_on() {
        let dir = std::env::temp_dir().join(format!("tl-tail-budget-{}", std::process::id()));
        let session = dir.join("Hearthstone_2026_01_01_00_00_00");
        fs::create_dir_all(&session).unwrap();
        let line = "D 12:00:00.0000000 GameState.DebugPrintPower() - x\n";
        fs::write(session.join(POWER_LOG), line.repeat(1000)).unwrap();
        let mut follower = Follower::new(&dir).with_budget(10 * line.len() as u64);
        let mut poll = || {
            let mut lines = 0;
            follower
                .poll(&mut |event| lines += usize::from(matches!(event, Event::Line(_))))
                .unwrap();
            lines
        };
        assert_eq!(poll(), 10);
        let rest: usize = (0..200).map(|_| poll()).sum();
        assert_eq!(rest, 990, "every line once, none twice");
        fs::remove_dir_all(&dir).unwrap();
    }
}
