//! Follows Hearthstone's `Power.log` and keeps a local history of
//! Battlegrounds games (T-101). Read-only towards the game: it only reads the
//! log files (D-004).

pub mod dev_reconnect;
pub mod discover;
pub mod game_clock;
pub mod hero_pick;
pub mod live;
pub mod lobby_tribes;
pub mod lock;
pub mod possible;
pub mod provenance;
pub mod recap;
pub mod report_bundle;
pub mod setup;
pub mod shop;
pub mod stats;
pub mod store;
pub mod tail;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use bg_parser::lines::{for_each_chunk, Chunk};
use bg_parser::report::{GameReport, Status};
use bg_parser::LogReader;

use game_clock::{GameClock, LocalToUtc, Played};
use store::{GameKey, Store};
use tail::{Event, Follower};

/// A game written to the store during a poll.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub key: GameKey,
    pub report: GameReport,
}

/// The session being read: its games and when each one was played.
struct SessionReader {
    name: String,
    log: LogReader,
    clock: GameClock,
}

impl SessionReader {
    fn new(name: &str) -> Self {
        SessionReader {
            name: name.to_string(),
            log: LogReader::default(),
            clock: GameClock::new(name),
        }
    }

    fn feed(&mut self, line: &str) {
        self.log.feed(line);
        self.clock.feed(line);
    }

    fn pending(&self, report: GameReport, to_utc: LocalToUtc) -> Pending {
        let played = self.clock.played(report.index, to_utc);
        (self.name.clone(), report, played)
    }

    /// Closes the session and queues its last games.
    fn finish(self, pending: &mut VecDeque<Pending>, to_utc: LocalToUtc) {
        let SessionReader { name, log, clock } = self;
        pending.extend(log.finish().into_iter().map(|g| {
            let played = clock.played(g.index, to_utc);
            (name.clone(), g, played)
        }));
    }
}

/// A finished game waiting to be saved: session, report, when it was played.
type Pending = (String, GameReport, Option<Played>);

pub struct Watcher {
    follower: Follower,
    reader: Option<SessionReader>,
    store: Store,
    /// Finished games not saved yet. They survive a failed poll or a failed
    /// write and are saved on the next poll.
    pending: VecDeque<Pending>,
    to_utc: LocalToUtc,
    /// The game in progress as of the last poll that read lines (for the
    /// overlay). Rebuilt only when the log moved.
    live: Option<GameReport>,
    live_stale: bool,
}

impl Watcher {
    pub fn new(logs_dir: impl Into<PathBuf>, store: Store) -> Self {
        Watcher {
            follower: Follower::new(logs_dir),
            reader: None,
            store,
            pending: VecDeque::new(),
            live: None,
            live_stale: true,
            to_utc: game_clock::system_local_to_utc,
        }
    }

    /// Uses `to_utc` instead of the system's time zone (for tests).
    pub fn with_local_time(mut self, to_utc: LocalToUtc) -> Self {
        self.to_utc = to_utc;
        self
    }

    /// Reads what the game wrote since the last poll and saves every game
    /// that finished. On a read error the games found before it are still
    /// saved; the error is returned afterwards.
    pub fn poll(&mut self) -> io::Result<Vec<Saved>> {
        let reader = &mut self.reader;
        let pending = &mut self.pending;
        let live_stale = &mut self.live_stale;
        let to_utc = self.to_utc;
        let read = self.follower.poll(&mut |event| match event {
            Event::Session(name) => {
                *live_stale = true;
                if let Some(old) = reader.take() {
                    old.finish(pending, to_utc);
                }
                *reader = Some(SessionReader::new(name));
            }
            Event::Line(line) => {
                *live_stale = true;
                if let Some(r) = reader.as_mut() {
                    r.feed(line);
                    let done: Vec<GameReport> = r
                        .log
                        .take_finished()
                        .into_iter()
                        .chain(r.log.take_completed_current())
                        .collect();
                    pending.extend(done.into_iter().map(|g| r.pending(g, to_utc)));
                }
            }
            Event::TooLong => {
                *live_stale = true;
                if let Some(r) = reader.as_mut() {
                    r.log.feed_too_long();
                }
            }
        });
        let saved = self.save_pending()?;
        read.map(|()| saved)
    }

    /// Closes the game in progress (e.g. when the app quits) and saves it.
    pub fn finish(mut self) -> io::Result<(Vec<Saved>, Store)> {
        if let Some(r) = self.reader.take() {
            r.finish(&mut self.pending, self.to_utc);
        }
        let saved = self.save_pending()?;
        Ok((saved, self.store))
    }

    fn save_pending(&mut self) -> io::Result<Vec<Saved>> {
        let mut saved = Vec::new();
        while let Some((session, report, played)) = self.pending.pop_front() {
            match save_report(&mut self.store, session.clone(), &report, played) {
                Ok(Some(s)) => saved.push(s),
                Ok(None) => {}
                Err(e) => {
                    self.pending.push_front((session, report, played));
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

    /// The Battlegrounds game being played now (not saved yet), from hero
    /// select until the log marks it complete (T-303).
    pub fn in_progress(&self) -> Option<GameKey> {
        let reader = self.reader.as_ref()?;
        Some(GameKey {
            session: reader.name.clone(),
            index: reader.log.in_progress_index()? as u64,
        })
    }

    /// What the Battlegrounds game being played now looks like so far, for
    /// the overlay; `None` when no game is on. Built by the same parser as the
    /// saved report and kept until the log moves.
    pub fn live_report(&mut self) -> Option<&GameReport> {
        let snapshot = match self.reader.as_ref() {
            Some(reader) if reader.log.in_progress_index().is_some() => {
                if !self.live_stale && self.live.is_some() {
                    return self.live.as_ref();
                }
                reader.log.snapshot_current()
            }
            _ => None,
        };
        self.live_stale = false;
        self.live = snapshot;
        self.live.as_ref()
    }

    /// Every lobby hero's leaderboard place and tavern tier now, for the
    /// overlay (T-306); empty when no Battlegrounds game is on.
    pub fn live_lobby(&self) -> Vec<bg_parser::report::LobbySlot> {
        self.reader
            .as_ref()
            .map(|r| r.log.lobby_now())
            .unwrap_or_default()
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
    played: Option<Played>,
) -> io::Result<Option<Saved>> {
    if report.status == Status::NotBattlegrounds {
        return Ok(None);
    }
    let key = GameKey {
        session,
        index: report.index as u64,
    };
    let value = serde_json::to_value(report).map_err(io::Error::other)?;
    Ok(store
        .save_played(key.clone(), value, played)?
        .then(|| Saved {
            key,
            report: report.clone(),
        }))
}

/// Reads one session folder (Power_old.log, then Power.log) into reports,
/// each with when it was played.
fn read_session(session_dir: &Path) -> io::Result<Vec<(GameReport, Option<Played>)>> {
    let mut reader = SessionReader::new(&session_name(session_dir));
    for file in [tail::POWER_OLD_LOG, tail::POWER_LOG] {
        let path = session_dir.join(file);
        if path.is_file() {
            let input = BufReader::new(File::open(&path)?);
            for_each_chunk(input, &mut |chunk| match chunk {
                Chunk::Line(line) => reader.feed(&line),
                Chunk::TooLong => reader.log.feed_too_long(),
            })?;
        }
    }
    let mut pending = VecDeque::new();
    reader.finish(&mut pending, game_clock::system_local_to_utc);
    Ok(pending.into_iter().map(|(_, g, p)| (g, p)).collect())
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
    for (report, played) in read_session(session_dir)? {
        saved.extend(save_report(store, name.clone(), &report, played)?);
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
        let found: BTreeSet<u64> = reports.iter().map(|(r, _)| r.index as u64).collect();
        let missing: Vec<GameKey> = keys
            .into_iter()
            .filter(|k| !found.contains(&k.index))
            .collect();
        if !missing.is_empty() {
            skip(UnavailableReason::LogsIncomplete, missing);
            continue;
        }
        outcome.sessions_read += 1;
        for (report, played) in &reports {
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
                .extend(save_report(store, session.clone(), report, *played)?);
        }
    }
    Ok(outcome)
}
