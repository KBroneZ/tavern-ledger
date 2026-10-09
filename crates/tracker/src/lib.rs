//! Follows Hearthstone's `Power.log` and keeps a local history of
//! Battlegrounds games (T-101). Read-only towards the game: it only reads the
//! log files (D-004).

pub mod discover;
pub mod lock;
pub mod store;
pub mod tail;

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use bg_parser::lines::for_each_chunk;
use bg_parser::report::{GameReport, Status};
use bg_parser::LogReader;

use store::{GameKey, Store};
use tail::{Event, Follower};

/// A game written to the store during a poll.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub key: GameKey,
    pub report: GameReport,
}

pub struct Watcher {
    follower: Follower,
    reader: Option<(String, LogReader)>,
    store: Store,
    /// Finished games not saved yet. They survive a failed poll or a failed
    /// write and are saved on the next poll.
    pending: VecDeque<(String, GameReport)>,
}

impl Watcher {
    pub fn new(logs_dir: impl Into<PathBuf>, store: Store) -> Self {
        Watcher {
            follower: Follower::new(logs_dir),
            reader: None,
            store,
            pending: VecDeque::new(),
        }
    }

    /// Reads what the game wrote since the last poll and saves every game
    /// that finished. On a read error the games found before it are still
    /// saved; the error is returned afterwards.
    pub fn poll(&mut self) -> io::Result<Vec<Saved>> {
        let reader = &mut self.reader;
        let pending = &mut self.pending;
        let read = self.follower.poll(&mut |event| match event {
            Event::Session(name) => {
                if let Some((old, r)) = reader.take() {
                    pending.extend(r.finish().into_iter().map(|g| (old.clone(), g)));
                }
                *reader = Some((name.to_string(), LogReader::default()));
            }
            Event::Line(line) => {
                if let Some((session, r)) = reader.as_mut() {
                    r.feed(line);
                    let done = r
                        .take_finished()
                        .into_iter()
                        .chain(r.take_completed_current());
                    pending.extend(done.map(|g| (session.clone(), g)));
                }
            }
            Event::TooLong => {
                if let Some((_, r)) = reader.as_mut() {
                    r.feed_too_long();
                }
            }
        });
        let saved = self.save_pending()?;
        read.map(|()| saved)
    }

    /// Closes the game in progress (e.g. when the app quits) and saves it.
    pub fn finish(mut self) -> io::Result<(Vec<Saved>, Store)> {
        if let Some((session, r)) = self.reader.take() {
            self.pending
                .extend(r.finish().into_iter().map(|g| (session.clone(), g)));
        }
        let saved = self.save_pending()?;
        Ok((saved, self.store))
    }

    fn save_pending(&mut self) -> io::Result<Vec<Saved>> {
        let mut saved = Vec::new();
        while let Some((session, report)) = self.pending.pop_front() {
            match save_report(&mut self.store, session.clone(), &report) {
                Ok(Some(s)) => saved.push(s),
                Ok(None) => {}
                Err(e) => {
                    self.pending.push_front((session, report));
                    return Err(e);
                }
            }
        }
        Ok(saved)
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn session(&self) -> Option<&str> {
        self.follower.session()
    }

    /// See [`Follower::has_power_log`].
    pub fn has_power_log(&self) -> bool {
        self.follower.has_power_log()
    }
}

/// Saves one Battlegrounds report. Other game modes are not tracked;
/// unsupported games are kept so the history shows them instead of silently
/// missing them.
fn save_report(
    store: &mut Store,
    session: String,
    report: &GameReport,
) -> io::Result<Option<Saved>> {
    if report.status == Status::NotBattlegrounds {
        return Ok(None);
    }
    let key = GameKey {
        session,
        index: report.index as u64,
    };
    let value = serde_json::to_value(report).map_err(io::Error::other)?;
    Ok(store.save(key.clone(), value)?.then(|| Saved {
        key,
        report: report.clone(),
    }))
}

/// Reads one finished session folder (Power_old.log, then Power.log) and
/// saves its games. For games played before the tracker existed.
pub fn import_session(session_dir: &Path, store: &mut Store) -> io::Result<Vec<Saved>> {
    let name = session_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut reader = LogReader::default();
    for file in [tail::POWER_OLD_LOG, tail::POWER_LOG] {
        let path = session_dir.join(file);
        if path.is_file() {
            let input = BufReader::new(File::open(&path)?);
            for_each_chunk(input, &mut |chunk| reader.feed_chunk(chunk))?;
        }
    }
    let mut saved = Vec::new();
    for report in reader.finish() {
        saved.extend(save_report(store, name.clone(), &report)?);
    }
    Ok(saved)
}
