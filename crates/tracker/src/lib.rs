//! Follows Hearthstone's `Power.log` and keeps a local history of
//! Battlegrounds games (T-101). Read-only towards the game: it only reads the
//! log files (D-004).

pub mod discover;
pub mod store;
pub mod tail;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

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
}

impl Watcher {
    pub fn new(logs_dir: impl Into<PathBuf>, store: Store) -> Self {
        Watcher {
            follower: Follower::new(logs_dir),
            reader: None,
            store,
        }
    }

    /// Reads what the game wrote since the last poll and saves every game
    /// that finished.
    pub fn poll(&mut self) -> io::Result<Vec<Saved>> {
        let mut finished: Vec<(String, GameReport)> = Vec::new();
        let reader = &mut self.reader;
        self.follower.poll(&mut |event| match event {
            Event::Session(name) => {
                if let Some((old, r)) = reader.take() {
                    finished.extend(r.finish().into_iter().map(|g| (old.clone(), g)));
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
                    finished.extend(done.map(|g| (session.clone(), g)));
                }
            }
        })?;
        self.save_all(finished)
    }

    /// Closes the game in progress (e.g. when the app quits) and saves it.
    pub fn finish(mut self) -> io::Result<(Vec<Saved>, Store)> {
        let finished = match self.reader.take() {
            Some((session, r)) => r
                .finish()
                .into_iter()
                .map(|g| (session.clone(), g))
                .collect(),
            None => Vec::new(),
        };
        let saved = self.save_all(finished)?;
        Ok((saved, self.store))
    }

    fn save_all(&mut self, finished: Vec<(String, GameReport)>) -> io::Result<Vec<Saved>> {
        save_reports(&mut self.store, finished)
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn session(&self) -> Option<&str> {
        self.follower.session()
    }
}

/// Saves Battlegrounds reports. Other game modes are not tracked;
/// unsupported games are kept so the history shows them instead of silently
/// missing them.
fn save_reports(store: &mut Store, reports: Vec<(String, GameReport)>) -> io::Result<Vec<Saved>> {
    let mut saved = Vec::new();
    for (session, report) in reports {
        if report.status == Status::NotBattlegrounds {
            continue;
        }
        let key = GameKey {
            session,
            index: report.index as u64,
        };
        let value = serde_json::to_value(&report).map_err(io::Error::other)?;
        if store.save(key.clone(), value)? {
            saved.push(Saved { key, report });
        }
    }
    Ok(saved)
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
            for line in input.split(b'\n') {
                reader.feed(String::from_utf8_lossy(&line?).trim_end_matches('\r'));
            }
        }
    }
    let reports = reader
        .finish()
        .into_iter()
        .map(|r| (name.clone(), r))
        .collect();
    save_reports(store, reports)
}
