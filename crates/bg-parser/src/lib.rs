//! Rebuilds Battlegrounds games from a local `Power.log`.
//!
//! Port of the Python prototype `tools/parse_bg.py` (same report, same JSON),
//! without hslog. Feed lines as they arrive with [`LogReader`]; each
//! `CREATE_GAME` starts a new game, parsed on its own. Anything the log does
//! not provide is reported as not available, never guessed. If the log does
//! not match what this parser expects (e.g. after a game patch), the game is
//! "unsupported" instead of showing wrong data.

pub mod collector;
pub mod lines;
pub mod power;
pub mod report;
pub mod state;

use std::collections::{BTreeMap, HashMap};
use std::io::BufRead;

use collector::{hero_health, leaderboard_heroes, Collector};
use power::{card_names, classify, parse_power, Line, ParseError};
use report::{CombatEntry, GameReport, LobbyPlayer, Round, Status};
use state::Entity;

/// The parser's crate version, saved with every game (T-107).
pub const PARSER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Bump when a change makes the parser read the same log differently (new
/// fields, fixed bugs). Saved with every game so `tavern-watch --reparse`
/// and the server can tell which records an older parser wrote.
pub const PARSER_REVISION: u32 = 2;

/// Builds whose real logs this parser was checked against.
pub const TESTED_BUILDS: &[i64] = &[253216];
pub const TESTED_GAME_TYPES: &[&str] = &["GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO"];
const MAX_LOBBY: usize = 8;
const CREATE_GAME: &str = "GameState.DebugPrintPower() - CREATE_GAME";
/// Bounds on the card names kept per game, so a strange log cannot grow them.
const MAX_NAMES: usize = 1_000;
const MAX_NAME_LEN: usize = 100;

/// One game being read.
#[derive(Debug, Default)]
pub struct GameReader {
    collector: Collector,
    game_type: Option<String>,
    build: Option<i64>,
    error: Option<&'static str>,
    reported_complete: bool,
    /// Card id -> name as printed in the log; the report keeps heroes only.
    names: HashMap<String, String>,
}

impl GameReader {
    pub fn feed(&mut self, line: &str) {
        if self.error.is_none() && !has_timestamp(line) {
            // hslog rejects such lines; a broken game must not look fine.
            self.error = Some("RegexParsingError");
        }
        match classify(line) {
            Line::Game(data) => self.feed_game(data),
            Line::Power(data) if self.error.is_none() => {
                self.remember_names(data);
                if let Err(ParseError(name)) =
                    parse_power(data).and_then(|p| self.collector.feed(p))
                {
                    self.error = Some(name);
                }
            }
            _ => {}
        }
    }

    /// Keeps hero names only (their card ids hold "HERO"): the report shows
    /// nothing else, and minions cannot fill the bounded map.
    fn remember_names(&mut self, data: &str) {
        for (card, name) in card_names(data) {
            if !card.contains("HERO")
                || self.names.len() >= MAX_NAMES
                || name.chars().count() > MAX_NAME_LEN
            {
                continue;
            }
            self.names
                .entry(card.to_string())
                .or_insert_with(|| name.to_string());
        }
    }

    fn feed_game(&mut self, data: &str) {
        if let Some(rest) = data.strip_prefix("GameType=") {
            self.game_type = rest.split_whitespace().next().map(String::from);
        } else if let Some(rest) = data.strip_prefix("BuildNumber=") {
            self.build = rest.trim().parse().ok();
        } else if let Some(rest) = data.strip_prefix("PlayerID=") {
            if let Some((pid, name)) = rest.split_once(", PlayerName=") {
                if let Ok(pid) = pid.parse() {
                    self.collector.players.register_name(pid, name.trim());
                }
            }
        }
    }

    pub fn finish(mut self, index: usize) -> GameReport {
        if self.error.is_none() {
            if let Err(ParseError(name)) = self.collector.finish() {
                self.error = Some(name);
            }
        }
        build_report(index, &self)
    }

    /// True when the log says this is a Battlegrounds game (solo or Duos).
    pub fn is_battlegrounds(&self) -> bool {
        self.game_type
            .as_deref()
            .is_some_and(|t| TESTED_GAME_TYPES.contains(&t))
    }

    /// True once the log marks the game as over (STATE=COMPLETE).
    pub fn is_complete(&self) -> bool {
        self.collector
            .board
            .game()
            .is_some_and(|g| g.is("STATE", "COMPLETE"))
    }
}

/// "D 12:00:00.0000000 rest": level, time, then something.
fn has_timestamp(line: &str) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    let mut parts = line.splitn(3, ' ');
    let level = parts.next().unwrap_or("");
    let time = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    matches!(level, "D" | "W" | "E")
        && !time.is_empty()
        && time
            .chars()
            .all(|c| c.is_ascii_digit() || c == ':' || c == '.')
        && !rest.is_empty()
}

/// Splits a log at each CREATE_GAME and reads every game on its own.
#[derive(Debug, Default)]
pub struct LogReader {
    current: Option<GameReader>,
    done: Vec<GameReport>,
    count: usize,
}

impl LogReader {
    pub fn feed(&mut self, line: &str) {
        if line.contains(CREATE_GAME) {
            self.close_current();
            self.current = Some(GameReader::default());
        }
        if let Some(reader) = self.current.as_mut() {
            reader.feed(line);
        }
    }

    /// A line longer than [`lines::MAX_LINE`] was skipped: the game it was in
    /// cannot be trusted any more.
    pub fn feed_too_long(&mut self) {
        if let Some(reader) = self.current.as_mut() {
            reader.error.get_or_insert("LineTooLong");
        }
    }

    pub fn feed_chunk(&mut self, chunk: lines::Chunk<'_>) {
        match chunk {
            lines::Chunk::Line(line) => self.feed(&line),
            lines::Chunk::TooLong => self.feed_too_long(),
        }
    }

    fn close_current(&mut self) {
        if let Some(reader) = self.current.take() {
            self.count += 1;
            self.done.push(reader.finish(self.count));
        }
    }

    /// The game in progress, as soon as the log marks it complete, without
    /// waiting for the next CREATE_GAME. Given once per game; the report made
    /// when the game closes can add the few lines logged after that point, so
    /// keep the latest report for each game index.
    pub fn take_completed_current(&mut self) -> Option<GameReport> {
        let index = self.count + 1;
        let reader = self.current.as_mut()?;
        if reader.reported_complete || !reader.is_complete() {
            return None;
        }
        reader.reported_complete = true;
        Some(build_report(index, reader))
    }

    /// Number of the Battlegrounds game being played right now (the `index`
    /// its report will have), from hero select until the log marks it
    /// complete. `None` when no such game is on.
    pub fn in_progress_index(&self) -> Option<usize> {
        let reader = self.current.as_ref()?;
        (reader.is_battlegrounds() && !reader.is_complete()).then_some(self.count + 1)
    }

    /// Games finished so far (a new CREATE_GAME closes the previous one).
    pub fn take_finished(&mut self) -> Vec<GameReport> {
        std::mem::take(&mut self.done)
    }

    /// Closes the game in progress, if any, and returns every pending report.
    pub fn finish(mut self) -> Vec<GameReport> {
        self.close_current();
        self.done
    }
}

/// Reads a whole log. Invalid UTF-8 is replaced, as the Python prototype does.
pub fn parse_reader(input: impl BufRead) -> std::io::Result<Vec<GameReport>> {
    let mut reader = LogReader::default();
    lines::for_each_chunk(input, &mut |chunk| reader.feed_chunk(chunk))?;
    Ok(reader.finish())
}

fn build_report(index: usize, r: &GameReader) -> GameReport {
    let mut base = GameReport::new(index, Status::Unsupported, r.game_type.clone(), r.build);
    let game_type = match r.game_type.as_deref() {
        Some(t) if t.contains("BATTLEGROUNDS") => t,
        _ => {
            base.status = Status::NotBattlegrounds;
            return base;
        }
    };
    if !r.build.is_some_and(|b| TESTED_BUILDS.contains(&b)) {
        let build = r
            .build
            .map(|b| b.to_string())
            .unwrap_or_else(|| "None".into());
        base.warnings.push(format!("build {build} not tested"));
    }
    if !TESTED_GAME_TYPES.contains(&game_type) {
        base.warnings
            .push(format!("{game_type} not tested with real logs"));
    }
    let c = &r.collector;
    if r.error.is_some() || c.board.game().is_none() {
        let error = r.error.unwrap_or("no game data");
        base.problems.push(format!("parser error: {error}"));
        return base;
    }
    let mut report = summarize(base, c);
    report.card_names = hero_names(&report, &r.names);
    report
}

/// Names of the heroes the report mentions, when the log printed them.
fn hero_names(report: &GameReport, names: &HashMap<String, String>) -> BTreeMap<String, String> {
    let lobby = report.lobby.iter().map(|p| &p.hero);
    let combats = report
        .rounds
        .iter()
        .flat_map(|r| r.entries.iter().map(|e| &e.hero));
    [&report.hero, &report.teammate_hero]
        .into_iter()
        .chain(lobby)
        .chain(combats)
        .flatten()
        .filter_map(|card| Some((card.clone(), names.get(card)?.clone())))
        .collect()
}

fn validate(c: &Collector, heroes: &[(i64, &Entity)]) -> Vec<String> {
    let mut problems = Vec::new();
    match (c.local_pid, c.dummy_pid) {
        (Some(local), Some(_)) => {
            if !heroes.iter().any(|(pid, _)| *pid == local) {
                problems.push("no leaderboard hero for the local player".into());
            }
        }
        _ => problems.push("could not identify the local player and the shop player".into()),
    }
    if heroes.is_empty() || heroes.len() > MAX_LOBBY {
        problems.push(format!("unexpected lobby size: {}", heroes.len()));
    }
    problems
}

fn summarize(mut base: GameReport, c: &Collector) -> GameReport {
    let heroes = leaderboard_heroes(&c.board);
    let problems = validate(c, &heroes);
    if !problems.is_empty() {
        base.problems = problems;
        return base;
    }
    let game = c.board.game().expect("checked in build_report");
    let complete = game.is("STATE", "COMPLETE");
    let local_pid = c.local_pid.expect("checked in validate");
    let local = c
        .local
        .and_then(|id| c.board.get(id))
        .expect("local player entity exists");
    let hero_of = |pid: i64| heroes.iter().find(|(p, _)| *p == pid).map(|(_, h)| *h);
    let own = hero_of(local_pid).expect("checked in validate");
    let mate = local.opt_int("BACON_DUO_TEAMMATE_PLAYER_ID");
    let place = |h: &Entity| {
        if complete {
            Some(h.int("PLAYER_LEADERBOARD_PLACE"))
        } else {
            None
        }
    };

    base.status = if complete {
        Status::Ok
    } else {
        Status::Incomplete
    };
    base.local_player_id = Some(local_pid);
    base.hero = own.card_id.clone();
    base.teammate_player_id = mate;
    base.teammate_hero = mate.and_then(hero_of).and_then(|h| h.card_id.clone());
    base.final_place = place(own);
    base.final_health = Some(hero_health(own));
    let mut sorted = heroes.clone();
    sorted.sort_by_key(|(pid, _)| *pid);
    base.lobby = sorted
        .iter()
        .map(|(pid, h)| LobbyPlayer {
            player_id: *pid,
            hero: h.card_id.clone(),
            // The local hero has no team tag; the local player entity has it.
            duo_team: h.opt_int("BACON_DUO_TEAM_ID").or_else(|| {
                if *pid == local_pid {
                    local.opt_int("BACON_DUO_TEAM_ID")
                } else {
                    None
                }
            }),
            final_place: place(h),
            final_health: hero_health(h),
        })
        .collect();
    base.shop_tribes = count_tribes(c);
    base.rounds = rounds_of(c, &heroes, complete.then(|| hero_health(own)));
    base.start_health = c.lobby_health.get(&0).cloned().unwrap_or_default();
    base
}

/// Most common first; ties keep first-seen order.
fn count_tribes(c: &Collector) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for (_, race) in &c.shop_races {
        match counts.iter_mut().find(|(r, _)| r == race) {
            Some(slot) => slot.1 += 1,
            None => counts.push((race.clone(), 1)),
        }
    }
    counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n)); // stable
    counts
}

fn lobby_health_from(heroes: &[(i64, &Entity)]) -> BTreeMap<i64, i64> {
    heroes
        .iter()
        .map(|(pid, hero)| (*pid, hero_health(hero)))
        .collect()
}

fn rounds_of(c: &Collector, heroes: &[(i64, &Entity)], last_health: Option<i64>) -> Vec<Round> {
    let mut health: BTreeMap<i64, Option<i64>> = c.health.clone();
    if let (Some(h), Some(&last)) = (last_health, c.entries.keys().next_back()) {
        // The game ends right after the last combat: no shop turn records it.
        health.entry(last).or_insert(Some(h));
    }
    let mut lobby_health = c.lobby_health.clone();
    if let (true, Some(&last)) = (last_health.is_some(), c.entries.keys().next_back()) {
        // Same for everyone: the final heroes are the health after the last combat.
        lobby_health
            .entry(last)
            .or_insert_with(|| lobby_health_from(heroes));
    }
    // Combat hero copies share the card id of the lobby hero. Ghosts of
    // eliminated players do not, so their id comes from the combat tag.
    let by_card = |card: &Option<String>| -> Option<i64> {
        let card = card.as_ref()?;
        heroes
            .iter()
            .rev()
            .find(|(_, h)| h.card_id.as_ref() == Some(card))
            .map(|(pid, _)| *pid)
    };
    c.entries
        .iter()
        .map(|(&n, entries)| Round {
            number: n,
            entries: entries
                .iter()
                .map(|(side, card, hint, board)| CombatEntry {
                    side: *side,
                    player_id: by_card(card).or(*hint),
                    hero: card.clone(),
                    board: board.clone(),
                })
                .collect(),
            own_health_after: health.get(&n).copied().flatten(),
            health_after: lobby_health.get(&n).cloned().unwrap_or_default(),
        })
        .collect()
}
