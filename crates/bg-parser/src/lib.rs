//! Rebuilds Battlegrounds games from a local `Power.log`.
//!
//! Port of the Python prototype `tools/parse_bg.py` (same report, same JSON),
//! without hslog. Feed lines as they arrive with [`LogReader`]; each
//! `CREATE_GAME` starts a new game, parsed on its own. Anything the log does
//! not provide is reported as not available, never guessed. If the log does
//! not match what this parser expects (e.g. after a game patch), the game is
//! "unsupported" instead of showing wrong data.

pub mod collector;
pub mod power;
pub mod report;
pub mod state;

use std::collections::BTreeMap;
use std::io::BufRead;

use collector::{hero_health, leaderboard_heroes, Collector};
use power::{classify, parse_power, Line, ParseError};
use report::{CombatEntry, GameReport, LobbyPlayer, Round, Status};
use state::Entity;

/// Builds whose real logs this parser was checked against.
pub const TESTED_BUILDS: &[i64] = &[253216];
pub const TESTED_GAME_TYPES: &[&str] = &["GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO"];
const MAX_LOBBY: usize = 8;
const CREATE_GAME: &str = "GameState.DebugPrintPower() - CREATE_GAME";

/// One game being read.
#[derive(Debug, Default)]
pub struct GameReader {
    collector: Collector,
    game_type: Option<String>,
    build: Option<i64>,
    error: Option<&'static str>,
    reported_complete: bool,
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
                if let Err(ParseError(name)) =
                    parse_power(data).and_then(|p| self.collector.feed(p))
                {
                    self.error = Some(name);
                }
            }
            _ => {}
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
    for line in input.split(b'\n') {
        reader.feed(&String::from_utf8_lossy(&line?));
    }
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
    summarize(base, c)
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

fn rounds_of(c: &Collector, heroes: &[(i64, &Entity)], last_health: Option<i64>) -> Vec<Round> {
    let mut health: BTreeMap<i64, Option<i64>> = c.health.clone();
    if let (Some(h), Some(&last)) = (last_health, c.entries.keys().next_back()) {
        // The game ends right after the last combat: no shop turn records it.
        health.entry(last).or_insert(Some(h));
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
        })
        .collect()
}
