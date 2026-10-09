//! Follows `Power.log` in the newest session folder as the game writes it.
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
const READ_CHUNK: usize = 1 << 20;
const HEAD_LEN: usize = 256;

/// What a poll found, in log order.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<'a> {
    /// A session folder starts being followed (its name, e.g. Hearthstone_2026_...).
    Session(&'a str),
    Line(&'a str),
    /// A line longer than bg_parser::lines::MAX_LINE was skipped.
    TooLong,
}

#[derive(Debug)]
struct Position {
    dir: PathBuf,
    name: String,
    offset: u64,
    /// First bytes of the file being read, to notice a rotation.
    head: Vec<u8>,
    splitter: LineSplitter,
}

#[derive(Debug)]
pub struct Follower {
    logs_dir: PathBuf,
    current: Option<Position>,
}

impl Follower {
    /// Follows the newest session from its first line, so games played
    /// before the app started are not lost.
    pub fn new(logs_dir: impl Into<PathBuf>) -> Self {
        Follower {
            logs_dir: logs_dir.into(),
            current: None,
        }
    }

    pub fn poll(&mut self, on_event: &mut dyn FnMut(Event<'_>)) -> io::Result<()> {
        if let Some(latest) = latest_session(&self.logs_dir) {
            if self.current.as_ref().map(|p| &p.dir) != Some(&latest) {
                if let Some(mut old) = self.current.take() {
                    // The rest of the previous session; if it cannot be read
                    // now, the new session still starts.
                    let result = read_new(&mut old, on_event);
                    old.splitter.finish(&mut forward(on_event));
                    self.start(latest, on_event);
                    result?;
                } else {
                    self.start(latest, on_event);
                }
            }
        }
        match self.current.as_mut() {
            Some(pos) => read_new(pos, on_event),
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
            .is_some_and(|p| p.offset > 0 || p.dir.join(POWER_LOG).is_file())
    }
}

fn forward<'f>(on_event: &'f mut dyn FnMut(Event<'_>)) -> impl FnMut(Chunk<'_>) + 'f {
    move |chunk| match chunk {
        Chunk::Line(line) => on_event(Event::Line(&line)),
        Chunk::TooLong => on_event(Event::TooLong),
    }
}

fn read_new(pos: &mut Position, on_event: &mut dyn FnMut(Event<'_>)) -> io::Result<()> {
    let path = pos.dir.join(POWER_LOG);
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
    read_from(&path, pos, on_event)
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
    let old = pos.dir.join(POWER_OLD_LOG);
    let is_ours = fs::metadata(&old).is_ok_and(|m| m.len() >= pos.offset)
        && read_head(&old).is_ok_and(|head| same_start(&head, &pos.head));
    if is_ours {
        read_from(&old, pos, on_event)?;
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

/// Reads from `pos.offset` to the end, advancing the offset chunk by chunk,
/// so an error halfway never makes the same bytes be read twice.
fn read_from(
    path: &Path,
    pos: &mut Position,
    on_event: &mut dyn FnMut(Event<'_>),
) -> io::Result<()> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(pos.offset))?;
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        pos.splitter.push(&buf[..n], &mut forward(on_event));
        pos.offset += n as u64;
    }
}
