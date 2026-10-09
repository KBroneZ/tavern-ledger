//! Follows `Power.log` in the newest session folder as the game writes it.
//!
//! Only complete lines are passed on: a line still being written is read
//! again on the next poll. The game may rotate the file (`Power.log` becomes
//! `Power_old.log` and a new `Power.log` starts) or start a new session
//! folder; in both cases the rest of the old file is read first.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::discover::latest_session;

pub const POWER_LOG: &str = "Power.log";
pub const POWER_OLD_LOG: &str = "Power_old.log";
const READ_BUFFER: usize = 1 << 20;

/// What a poll found, in log order.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<'a> {
    /// A session folder starts being followed (its name, e.g. Hearthstone_2026_...).
    Session(&'a str),
    Line(&'a str),
}

#[derive(Debug)]
struct Position {
    dir: PathBuf,
    name: String,
    offset: u64,
    created: Option<SystemTime>,
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
                if let Some(old) = self.current.as_mut() {
                    read_new(old, on_event)?; // the rest of the previous session
                }
                let name = latest.file_name().map(|n| n.to_string_lossy().into_owned());
                let name = name.unwrap_or_default();
                on_event(Event::Session(&name));
                self.current = Some(Position {
                    dir: latest,
                    name,
                    offset: 0,
                    created: None,
                });
            }
        }
        match self.current.as_mut() {
            Some(pos) => read_new(pos, on_event),
            None => Ok(()),
        }
    }

    /// The session folder being followed, if any.
    pub fn session(&self) -> Option<&str> {
        self.current.as_ref().map(|p| p.name.as_str())
    }
}

fn read_new(pos: &mut Position, on_event: &mut dyn FnMut(Event<'_>)) -> io::Result<()> {
    let path = pos.dir.join(POWER_LOG);
    let meta = match fs::metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return drain_rotated(pos, on_event);
        }
        Err(e) => return Err(e),
    };
    let created = meta.created().ok();
    let rotated = meta.len() < pos.offset || (pos.created.is_some() && created != pos.created);
    if rotated {
        drain_rotated(pos, on_event)?;
    }
    pos.created = created;
    pos.offset = read_lines(&path, pos.offset, false, on_event)?;
    Ok(())
}

/// After a rotation the lines not read yet are at the end of Power_old.log.
fn drain_rotated(pos: &mut Position, on_event: &mut dyn FnMut(Event<'_>)) -> io::Result<()> {
    if pos.offset == 0 {
        return Ok(());
    }
    let old = pos.dir.join(POWER_OLD_LOG);
    if fs::metadata(&old).is_ok_and(|m| m.len() >= pos.offset) {
        read_lines(&old, pos.offset, true, on_event)?;
    }
    pos.offset = 0;
    pos.created = None;
    Ok(())
}

/// Calls `on_event` for each complete line from `offset`; returns the offset
/// after the last complete line. With `include_partial`, a final line without
/// a newline is passed on too (the file will not grow any more).
fn read_lines(
    path: &Path,
    offset: u64,
    include_partial: bool,
    on_event: &mut dyn FnMut(Event<'_>),
) -> io::Result<u64> {
    let mut file = BufReader::with_capacity(READ_BUFFER, File::open(path)?);
    file.seek(SeekFrom::Start(offset))?;
    let mut position = offset;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = file.read_until(b'\n', &mut buf)?;
        if n == 0 {
            return Ok(position);
        }
        let complete = buf.ends_with(b"\n");
        if !complete && !include_partial {
            return Ok(position);
        }
        let line = String::from_utf8_lossy(&buf);
        on_event(Event::Line(line.trim_end_matches(['\r', '\n'])));
        position += n as u64;
    }
}
