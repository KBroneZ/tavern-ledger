//! Follows Hearthstone's `Power.log` and keeps a local history of
//! Battlegrounds games (T-101). Read-only towards the game: this crate only
//! reads the log files (D-004); the app's optional read of two values from the
//! game itself is in `game-memory` (D-063).

pub mod choices;
pub mod client_build;
pub mod damage;
pub mod dev_reconnect;
pub mod discover;
pub mod game_clock;
pub mod game_ratings;
pub mod game_tribes;
pub mod game_values;
pub mod hero_pick;
pub mod live;
pub mod live_heroes;
pub mod live_seats;
pub mod lobby_recap;
pub mod lobby_tribes;
pub mod lock;
pub mod odds;
pub mod odds_duos;
pub mod playback;
pub mod possible;
pub mod provenance;
pub mod quiet;
pub mod recap;
pub mod report_bundle;
pub mod scene;
pub mod setup;
pub mod shop;
pub mod stats;
pub mod store;
pub mod tail;
pub mod why;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

use bg_parser::lines::{for_each_chunk, Chunk};
use bg_parser::report::{GameReport, Status};
use bg_parser::LogReader;

use bg_parser::report::LobbySlot;
use client_build::ClientBuild;
use game_clock::{GameClock, LocalToUtc, Played};
use playback::Playback;
use scene::{Scene, SceneTracker};
use store::{GameKey, Store};
use tail::{Event, Follower};

/// A game written to the store during a poll.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub key: GameKey,
    pub report: GameReport,
}

/// The game in progress as the overlay shows it, as of some line (T-335).
#[derive(Default)]
struct LiveView {
    report: Option<GameReport>,
    lobby: Vec<LobbySlot>,
    next: Option<i64>,
    damage_cap: Option<i64>,
    /// The player's own board in a shop turn (T-605).
    own_board: Option<Vec<bg_parser::report::Minion>>,
    /// The player's game-long values in a shop turn (T-610).
    own_values: Option<bg_parser::report::CombatValues>,
    /// Duos: the player fights first next combat (T-608).
    fights_first: Option<bool>,
    /// The player's own game-long counters, in shop and combat turns (T-611).
    counters: Option<Vec<bg_parser::counters::Counter>>,
    /// The heroes on offer at hero select (T-365).
    hero_offer: Option<bg_parser::hero_select::HeroOffer>,
}

/// The session being read: its games and when each one was played.
struct SessionReader {
    name: String,
    log: LogReader,
    clock: GameClock,
    /// The live views, shown once the game has played their lines.
    playback: Playback<LiveView>,
}

impl SessionReader {
    fn new(name: &str) -> Self {
        SessionReader {
            name: name.to_string(),
            log: LogReader::default(),
            clock: GameClock::new(name),
            playback: Playback::default(),
        }
    }

    fn feed(&mut self, line: &str) {
        self.log.feed(line);
        self.clock.feed(line);
        self.playback.feed(line);
    }

    /// Keeps a view of the game in progress as the log has it now.
    fn record_live(&mut self) {
        let view = match self.log.in_progress_index() {
            Some(_) => LiveView {
                report: self.log.snapshot_current(),
                lobby: self.log.lobby_now(),
                next: self.log.next_opponent_now(),
                damage_cap: self.log.damage_cap_now(),
                own_board: self.log.own_board_now(),
                own_values: self.log.own_values_now(),
                fights_first: self.log.fights_first_now(),
                counters: self.log.counters_now(),
                hero_offer: self.log.hero_offer_now(),
            },
            None => LiveView::default(),
        };
        self.playback.record(view, Instant::now());
    }

    /// The view the game has shown on screen so far.
    fn live_view(&self) -> Option<&LiveView> {
        self.playback.view(Instant::now())
    }

    fn pending(&self, report: GameReport, to_utc: LocalToUtc) -> Pending {
        let played = self.clock.played(report.index, to_utc);
        (self.name.clone(), report, played)
    }

    /// Closes the session and queues its last games.
    fn finish(self, pending: &mut VecDeque<Pending>, to_utc: LocalToUtc) {
        let SessionReader {
            name, log, clock, ..
        } = self;
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
    /// The log moved since the live view was last kept (for the overlay).
    live_stale: bool,
    /// `LoadingScreen.log` of the same session (T-314): where the client is.
    scenes: Follower,
    scene: SceneTracker,
    /// Set after the scene file could not be read (the error kind): the
    /// scene is unknown until it reads again, never a stale one.
    scene_error: Option<io::ErrorKind>,
    /// A game the player left before its end (session, game index): it
    /// stays over even when the next game's loading screen comes.
    left: Option<(String, usize)>,
    logs_dir: PathBuf,
    /// The running client's build, also before its first game (T-337).
    build: ClientBuild,
}

impl Watcher {
    pub fn new(logs_dir: impl Into<PathBuf>, store: Store) -> Self {
        let logs_dir: PathBuf = logs_dir.into();
        Watcher {
            follower: Follower::new(logs_dir.clone()),
            reader: None,
            store,
            pending: VecDeque::new(),
            live_stale: true,
            to_utc: game_clock::system_local_to_utc,
            scenes: Follower::loading_screen(logs_dir.clone()),
            scene: SceneTracker::default(),
            scene_error: None,
            left: None,
            logs_dir,
            build: ClientBuild::default(),
        }
    }

    /// Finished games that could not be saved yet; a watcher in this state
    /// must not be let go.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// The history this watcher saves into, to follow another Logs folder
    /// with the same history.
    pub fn into_store(self) -> Store {
        self.store
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
        let build = &mut self.build;
        let to_utc = self.to_utc;
        let read = self.follower.poll(&mut |event| match event {
            Event::Session(name) => {
                *live_stale = true;
                build.start(name);
                if let Some(old) = reader.take() {
                    old.finish(pending, to_utc);
                }
                *reader = Some(SessionReader::new(name));
            }
            Event::Line(line) => {
                *live_stale = true;
                build.feed_power(line);
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
        if std::mem::take(&mut self.live_stale) {
            if let Some(r) = self.reader.as_mut() {
                r.record_live();
            }
        }
        self.build.look_in_net_log(&self.logs_dir);
        self.poll_scenes();
        match self.scene() {
            Some(scene) if scene.left_game() => {
                if let Some(r) = self.reader.as_ref() {
                    if let Some(index) = r.log.in_progress_index() {
                        self.left = Some((r.name.clone(), index));
                    }
                }
            }
            // The game screen loaded: it comes after the new game's
            // CREATE_GAME, so whatever game is open now is that one. Also
            // undoes a mark made when CREATE_GAME was read before the scene
            // change (the two files are written apart).
            Some(Scene::Game) => self.left = None,
            _ => {}
        }
        let saved = self.save_pending()?;
        read.map(|()| saved)
    }

    /// The last poll stopped before the end of what the game wrote (a long
    /// session read from its start): poll again without waiting.
    pub fn catching_up(&self) -> bool {
        self.follower.behind()
    }

    /// Reads the new scene changes. A read error leaves the scene unknown
    /// (the app then works from Power.log alone) and never stops a game
    /// from being saved.
    fn poll_scenes(&mut self) {
        let scene = &mut self.scene;
        let read = self.scenes.poll(&mut |event| match event {
            Event::Session(_) => *scene = SceneTracker::default(),
            Event::Line(line) => scene.feed(line),
            Event::TooLong => {}
        });
        let error = read.err().map(|e| e.kind());
        if error.is_some() != self.scene_error.is_some() {
            self.live_stale = true;
        }
        self.scene_error = error;
    }

    /// The game build of the running client: from the session's games, else
    /// from the start of its `Net.log`; None when unknown (T-337).
    pub fn client_build(&self) -> Option<i64> {
        self.build.now()
    }

    /// Why `LoadingScreen.log` could not be read in the last poll; None when
    /// it was read or is not there (off in log.config).
    pub fn scene_error(&self) -> Option<io::ErrorKind> {
        self.scene_error
    }

    /// Where the client is, from `LoadingScreen.log` (T-314); None when the
    /// file is off in log.config, says nothing yet or cannot be read.
    pub fn scene(&self) -> Option<Scene> {
        self.scene_error
            .is_none()
            .then(|| self.scene.now())
            .flatten()
    }

    /// The player left the game screen: a game the log still has open was
    /// cut off, so it is not "on" any more for the overlay and the window,
    /// also once the next game starts loading.
    fn game_left(&self) -> bool {
        if self.scene().is_some_and(Scene::left_game) {
            return true;
        }
        let Some(r) = self.reader.as_ref() else {
            return false;
        };
        match (&self.left, r.log.in_progress_index()) {
            (Some((session, index)), Some(now)) => *session == r.name && *index == now,
            _ => false,
        }
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
        if self.game_left() {
            return None;
        }
        let reader = self.reader.as_ref()?;
        Some(GameKey {
            session: reader.name.clone(),
            index: reader.log.in_progress_index()? as u64,
        })
    }

    /// What the Battlegrounds game being played now looks like so far, for
    /// the overlay; `None` when no game is on. Built by the same parser as the
    /// saved report, as of the last line the game has played on screen
    /// (T-335).
    pub fn live_report(&self) -> Option<&GameReport> {
        self.shown_live()?.report.as_ref()
    }

    /// Every lobby hero's leaderboard place and tavern tier as the game shows
    /// them, for the overlay (T-306); empty when no Battlegrounds game is on.
    pub fn live_lobby(&self) -> Vec<LobbySlot> {
        self.shown_live()
            .map(|v| v.lobby.clone())
            .unwrap_or_default()
    }

    /// The seat the game marks as the next opponent (T-313); None when no
    /// Battlegrounds game is on or the log has no mark yet.
    pub fn live_next_opponent(&self) -> Option<i64> {
        self.shown_live()?.next
    }

    /// The combat damage cap the game shows now (T-315); None when no
    /// Battlegrounds game is on or no cap applies.
    pub fn live_damage_cap(&self) -> Option<i64> {
        self.shown_live()?.damage_cap
    }

    /// The player's own board in a shop turn, as the game shows it (T-605);
    /// None in a combat turn or when no Battlegrounds game is on.
    pub fn live_own_board(&self) -> Option<&[bg_parser::report::Minion]> {
        self.shown_live()?.own_board.as_deref()
    }

    /// The player's game-long values in a shop turn, as the log has them
    /// (T-610); None in a combat turn or when no Battlegrounds game is on.
    pub fn live_own_values(&self) -> Option<bg_parser::report::CombatValues> {
        self.shown_live()?.own_values
    }

    /// Duos: whether the player's board fights first next combat, as the
    /// game has set it (T-608); None when not set or no game is on.
    pub fn live_fights_first(&self) -> Option<bool> {
        self.shown_live()?.fights_first
    }

    /// The player's own counters as the game shows them now (T-611); `None`
    /// when no Battlegrounds game is on or the reader has an error.
    pub fn live_counters(&self) -> Option<Vec<bg_parser::counters::Counter>> {
        self.shown_live()?.counters.clone()
    }

    /// The heroes on offer at hero select as the game shows them (T-365);
    /// None when no Battlegrounds game is on, before the offer and after the
    /// pick.
    pub fn live_hero_offer(&self) -> Option<bg_parser::hero_select::HeroOffer> {
        self.shown_live()?.hero_offer.clone()
    }

    fn shown_live(&self) -> Option<&LiveView> {
        if self.game_left() {
            return None;
        }
        self.reader.as_ref()?.live_view()
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
