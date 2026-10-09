//! Follows Hearthstone's `Power.log` and keeps a local history of
//! Battlegrounds games (T-101). Read-only towards the game: it only reads the
//! log files (D-004).

pub mod discover;
pub mod lock;
pub mod provenance;
pub mod recap;
pub mod report_bundle;
pub mod setup;
pub mod stats;
pub mod store;
pub mod tail;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
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

/// Reads one session folder (Power_old.log, then Power.log) into reports.
fn read_session(session_dir: &Path) -> io::Result<Vec<GameReport>> {
    let mut reader = LogReader::default();
    for file in [tail::POWER_OLD_LOG, tail::POWER_LOG] {
        let path = session_dir.join(file);
        if path.is_file() {
            let input = BufReader::new(File::open(&path)?);
            for_each_chunk(input, &mut |chunk| reader.feed_chunk(chunk))?;
        }
    }
    Ok(reader.finish())
}

fn session_name(session_dir: &Path) -> String {
    session_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Reads one finished session folder (Power_old.log, then Power.log) and
/// saves its games. For games played before the tracker existed.
pub fn import_session(session_dir: &Path, store: &mut Store) -> io::Result<Vec<Saved>> {
    let name = session_name(session_dir);
    let mut saved = Vec::new();
    for report in read_session(session_dir)? {
        saved.extend(save_report(store, name.clone(), &report)?);
    }
    Ok(saved)
}

/// Why a stored game could not be re-read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnavailableReason {
    /// The session folder or its Power logs are gone.
    LogsGone,
    /// The logs on disk hold fewer games than the history has for the
    /// session (the game rotated part of them away), so game numbers could
    /// have shifted. Nothing of the session is overwritten.
    LogsIncomplete,
    /// A log could not be read (the error kind).
    Unreadable(String),
    /// The stored game is complete (`ok`) but the logs on disk only give a
    /// worse reading (cut short, unsupported). The good record is kept.
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unavailable {
    pub key: GameKey,
    pub reason: UnavailableReason,
}

#[derive(Debug, Default)]
pub struct ReparseOutcome {
    /// Sessions whose logs were on disk and complete enough to re-read.
    pub sessions_read: usize,
    /// Games whose report changed and were saved (the last record wins, D-015).
    pub changed: Vec<Saved>,
    pub unavailable: Vec<Unavailable>,
}

/// Re-reads every session in the history whose logs are still in
/// `logs_dir` and saves the games whose report differs from the stored one.
/// Games whose logs are gone or only give a worse reading are listed, never
/// changed or deleted; games not in the history are not added (T-107). A
/// game whose report is unchanged keeps its old parser stamp.
pub fn reparse(logs_dir: &Path, store: &mut Store) -> io::Result<ReparseOutcome> {
    let mut stored: BTreeMap<String, Vec<GameKey>> = BTreeMap::new();
    for (key, _) in store.games() {
        stored
            .entry(key.session.clone())
            .or_default()
            .push(key.clone());
    }
    let mut outcome = ReparseOutcome::default();
    for (session, keys) in stored {
        let dir = logs_dir.join(&session);
        let has_logs = [tail::POWER_OLD_LOG, tail::POWER_LOG]
            .iter()
            .any(|f| dir.join(f).is_file());
        let mut skip = |reason: UnavailableReason, keys: Vec<GameKey>| {
            let missing = keys.into_iter().map(|key| Unavailable {
                key,
                reason: reason.clone(),
            });
            outcome.unavailable.extend(missing);
        };
        if !has_logs {
            skip(UnavailableReason::LogsGone, keys);
            continue;
        }
        let reports = match read_session(&dir) {
            Ok(reports) => reports,
            Err(e) => {
                skip(
                    UnavailableReason::Unreadable(format!("{:?}", e.kind())),
                    keys,
                );
                continue;
            }
        };
        let found: BTreeSet<u64> = reports.iter().map(|r| r.index as u64).collect();
        let missing: Vec<GameKey> = keys
            .into_iter()
            .filter(|k| !found.contains(&k.index))
            .collect();
        if !missing.is_empty() {
            skip(UnavailableReason::LogsIncomplete, missing);
            continue;
        }
        outcome.sessions_read += 1;
        for report in &reports {
            let key = GameKey {
                session: session.clone(),
                index: report.index as u64,
            };
            // Only games already in the history are updated.
            let Some(stored) = store.report(&key) else {
                continue;
            };
            let was_ok = stored.get("status").and_then(|s| s.as_str()) == Some("ok");
            if was_ok && report.status != Status::Ok {
                outcome.unavailable.push(Unavailable {
                    key,
                    reason: UnavailableReason::Degraded,
                });
                continue;
            }
            outcome
                .changed
                .extend(save_report(store, session.clone(), report)?);
        }
    }
    Ok(outcome)
}
