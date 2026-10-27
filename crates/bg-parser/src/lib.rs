//! Rebuilds Battlegrounds games from a local `Power.log`.
//!
//! Port of the Python prototype `tools/parse_bg.py` (same report, same JSON),
//! without hslog. Feed lines as they arrive with [`LogReader`]; each
//! `CREATE_GAME` starts a new game, parsed on its own. Anything the log does
//! not provide is reported as not available, never guessed. If the log does
//! not match what this parser expects (e.g. after a game patch), the game is
//! "unsupported" instead of showing wrong data.

pub mod buffs;
pub mod clock;
pub mod collector;
pub mod counters;
pub mod discover;
pub mod gifts;
pub mod hero_select;
pub mod kit;
pub mod leaderboard;
pub mod lines;
pub mod power;
pub mod replay;
pub mod report;
pub mod shop;
pub mod sim_marks;
pub mod state;

use std::collections::{BTreeMap, HashMap};
use std::io::BufRead;

use clock::Clock;
use collector::{hero_health, leaderboard_heroes, Collector};
use hero_select::HeroOffer;
use power::{card_names, classify, parse_power, Line, ParseError, REGEX_PARSING_ERROR};
use report::{CombatEntry, GameReport, LobbyPlayer, LobbySlot, OpeningLeg, Round, Status};
use state::Entity;

/// The parser's crate version, saved with every game (T-107).
pub const PARSER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Bump when a change makes the parser read the same log differently (new
/// fields, fixed bugs). Saved with every game so `tavern-watch --reparse`
/// and the server can tell which records an older parser wrote.
pub const PARSER_REVISION: u32 = 17;

/// Builds whose real logs this parser was checked against.
pub const TESTED_BUILDS: &[i64] = &[253216, 254477];
/// Client builds (`.product.db`, HearthstoneJSON) whose games still write an
/// older, tested `BuildNumber`: `(client build, build its games write)`. Only
/// added after real games on that client were read clean. `tools/game_build_watch.py`
/// reads this list.
pub const CLIENT_BUILDS: &[(i64, i64)] = &[(255130, 254477)];
pub const TESTED_GAME_TYPES: &[&str] = &["GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO"];
const MAX_LOBBY: usize = 8;
/// The highest tavern tier; a value past it is not a tier the game has.
const MAX_TECH_LEVEL: i64 = 7;
/// The line that starts every game; [`LogReader`] splits the log at it.
pub const CREATE_GAME: &str = "GameState.DebugPrintPower() - CREATE_GAME";
/// Bounds on the card names kept per game, so a strange log cannot grow them.
const MAX_NAMES: usize = 1_000;
const MAX_NAME_LEN: usize = 100;
/// Lines a game may lose (a stray line, or the last line cut where the game
/// stopped writing the file) and still be read; one more and the log is not
/// trustworthy. A real game has tens of thousands of lines and a cut costs
/// one or two.
pub const MAX_SKIPPED_LINES: usize = 10;
/// Reasons a game cannot be read because of its lines; shown to the player.
const NO_TIMES: &str = "NoTimes";
const TOO_MANY_BAD_LINES: &str = "TooManyBadLines";
/// Hero names and skin links per report: far more than a lobby and a hero
/// pick show, and what upload-game accepts.
const MAX_HERO_MAPS: usize = 64;
/// The game's Old God, set once on the game entity at the start.
const OLD_GOD: &str = "BACON_GLOBAL_OLD_GOD_DBID";
/// The next opponent's seat, on the local leaderboard hero (T-313).
const NEXT_OPPONENT: &str = "NEXT_OPPONENT_PLAYER_ID";
const FIGHTS_FIRST: &str = "BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT";
/// The combat damage cap on the game entity, and whether it applies (T-315).
const DAMAGE_CAP: &str = "BACON_COMBAT_DAMAGE_CAP";
const DAMAGE_CAP_ON: &str = "BACON_COMBAT_DAMAGE_CAP_ENABLED";
/// Far above any cap the game has used (15 on the user's games).
const MAX_DAMAGE_CAP: i64 = 99;

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
    clock: Clock,
    /// Lines with a time, and lines skipped as unreadable (see
    /// [`MAX_SKIPPED_LINES`]).
    timed_lines: usize,
    skipped_lines: usize,
    /// A malformed packet was skipped and nothing valid has followed yet.
    /// That is a cut last line; a valid packet after it means a packet was
    /// lost in the middle (its tags would land on the wrong entity).
    malformed_last: bool,
    /// The last line the reader met was skipped: the log may end in a cut.
    last_line_skipped: bool,
}

impl GameReader {
    pub fn feed(&mut self, line: &str) {
        if self.error.is_none() {
            if line.trim().is_empty() {
                return;
            }
            if has_timestamp(line) {
                self.timed_lines += 1;
                self.last_line_skipped = false;
            } else {
                // A stray or cut line: skip it, but a game line still says
                // the game's type and build.
                if let Line::Game(data) = classify(line) {
                    self.feed_game(data);
                }
                self.skip_line();
                return;
            }
        }
        if let Some(ms) = self.clock.at(line) {
            self.collector.now_ms = ms;
            self.collector.shop.saw_line(ms);
        }
        match classify(line) {
            Line::Game(data) => self.feed_game(data),
            Line::Power(data, indent) if self.error.is_none() => {
                self.remember_names(data);
                self.collector.indent = indent;
                match parse_power(data) {
                    Err(ParseError(REGEX_PARSING_ERROR)) => {
                        self.malformed_last = true;
                        self.skip_line();
                    }
                    Err(ParseError(name)) => self.error = Some(name),
                    Ok(_) if self.malformed_last => self.error = Some(REGEX_PARSING_ERROR),
                    Ok(packet) => {
                        if let Err(ParseError(name)) = self.collector.feed(packet) {
                            self.error = Some(name);
                        }
                    }
                }
            }
            Line::SendOption if self.error.is_none() => {
                self.collector.shop.on_option(self.collector.now_ms);
            }
            Line::SendChoice(kind) if self.error.is_none() => {
                self.collector.shop.on_choice(self.collector.now_ms, kind);
            }
            Line::Choices(body) if self.error.is_none() => {
                // A packet still taking tag lines is complete by now (a reroll's
                // CHANGE_ENTITY can come right before the pick).
                if let Err(ParseError(name)) = self.collector.finish() {
                    self.error = Some(name);
                    return;
                }
                let c = &mut self.collector;
                c.hero_select.on_choices(body, &c.board, c.local_pid);
                let turn = leaderboard::shop_turn(c.turn());
                c.discovers.on_choices(body, &c.board, c.local_pid, turn);
                return;
            }
            Line::Chosen(body) if self.error.is_none() => {
                if let Err(ParseError(name)) = self.collector.finish() {
                    self.error = Some(name);
                    return;
                }
                let c = &mut self.collector;
                c.hero_select.on_chosen(body, &c.board);
                c.discovers.on_chosen(body);
                return;
            }
            _ => {}
        }
        self.collector.hero_select.on_other_line();
        self.collector.discovers.on_other_line();
    }

    /// One more line the reader cannot use; past [`MAX_SKIPPED_LINES`] the
    /// game is unreadable, and says why.
    fn skip_line(&mut self) {
        self.skipped_lines += 1;
        self.last_line_skipped = true;
        // With no timed line yet the reason is still open ([`Self::line_error`]
        // says "no times" at the end, unless times come).
        if self.timed_lines > 0 && self.skipped_lines > MAX_SKIPPED_LINES {
            self.error = Some(TOO_MANY_BAD_LINES);
        }
    }

    /// Why the lines alone make the game unreadable: no line has a time, or
    /// too many were skipped. `None` while the lines are good enough.
    fn line_error(&self) -> Option<&'static str> {
        if self.skipped_lines == 0 {
            None
        } else if self.timed_lines == 0 {
            Some(NO_TIMES)
        } else if self.skipped_lines > MAX_SKIPPED_LINES {
            Some(TOO_MANY_BAD_LINES)
        } else {
            None
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
        self.error = self.error.or_else(|| self.line_error());
        if self.error.is_none() {
            if let Err(ParseError(name)) = self.collector.finish() {
                self.error = Some(name);
            }
        }
        build_report(index, &self)
    }

    /// Every lobby hero's leaderboard place and tavern tier as the log has
    /// them now, by player id (T-306). Empty before the lobby is known or
    /// once the game could not be read.
    pub fn lobby_now(&self) -> Vec<LobbySlot> {
        if self.error.is_some() {
            return Vec::new();
        }
        let mut slots: Vec<LobbySlot> = leaderboard_heroes(&self.collector.board)
            .into_iter()
            .take(MAX_LOBBY)
            .map(|(pid, hero)| LobbySlot {
                player_id: pid,
                place: hero
                    .opt_int("PLAYER_LEADERBOARD_PLACE")
                    .filter(|p| (1..=MAX_LOBBY as i64).contains(p)),
                tech_level: hero
                    .opt_int("PLAYER_TECH_LEVEL")
                    .filter(|t| (1..=MAX_TECH_LEVEL).contains(t)),
                health: collector::hero_shown_health(hero),
                armor: hero
                    .opt_int("ARMOR")
                    .filter(|a| (0..=collector::MAX_HERO_POINTS).contains(a)),
            })
            .collect();
        slots.sort_by_key(|s| s.player_id);
        slots
    }

    /// The heroes on offer at hero select as the log has them now (T-365).
    /// None before the offer, after the pick, or once the game could not be
    /// read.
    pub fn hero_offer_now(&self) -> Option<HeroOffer> {
        if self.error.is_some() {
            return None;
        }
        self.collector.hero_select.offer_now(
            &self.collector.board,
            &self.names,
            self.collector.pending_change(),
        )
    }

    /// The seat the game marks as the next opponent on its leaderboard
    /// (T-313): the local leaderboard hero's `NEXT_OPPONENT_PLAYER_ID`, set
    /// once each shop turn (123 of 123 shop turns on the user's 11 games, and
    /// the hero met in the next combat or, in Duos, their teammate, every
    /// time). Live only; `None` when it is not set, not another lobby seat,
    /// or the game could not be read.
    pub fn next_opponent_now(&self) -> Option<i64> {
        if self.error.is_some() {
            return None;
        }
        let local = self.collector.local_pid?;
        let heroes = leaderboard_heroes(&self.collector.board);
        let (_, own) = heroes.iter().find(|(pid, _)| *pid == local)?;
        let next = own.opt_int(NEXT_OPPONENT)?;
        heroes
            .iter()
            .any(|(pid, _)| *pid == next && *pid != local)
            .then_some(next)
    }

    /// Duos: whether the player's own board fights before their teammate's
    /// in the next combat (T-608): the local leaderboard hero's
    /// `BACON_DUO_PLAYER_FIGHTS_FIRST_NEXT_COMBAT`, set each shop turn from
    /// the second one. On the user's 6 Duos sessions of 2026-10-11 it named
    /// the team's first board in 74 of 77 combats; the other 3 had an empty
    /// first board. Live only; `None` when the log has not set it or the
    /// game could not be read.
    pub fn fights_first_now(&self) -> Option<bool> {
        if self.error.is_some() {
            return None;
        }
        let hero = self.collector.own_leaderboard_hero()?;
        if !hero.has(FIGHTS_FIRST) {
            return None;
        }
        match hero.int(FIGHTS_FIRST) {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    /// The most damage a lost combat can deal now, which the game shows the
    /// player ("Max damage: 15", T-315): the game entity's
    /// `BACON_COMBAT_DAMAGE_CAP` while `BACON_COMBAT_DAMAGE_CAP_ENABLED` is 1.
    /// On the user's 7 Duos games of 2026-10-11 it is 15 from the first shop
    /// turn (the game holds another value before it) until the game turns it
    /// off late in the game (game turns 18 to 28), or to the end. Live only;
    /// `None` before the first shop turn, when off or not set, or when the
    /// game could not be read.
    pub fn damage_cap_now(&self) -> Option<i64> {
        if self.error.is_some() {
            return None;
        }
        let game = self.collector.board.game()?;
        if game.opt_int("TURN")? < 1 || game.opt_int(DAMAGE_CAP_ON)? != 1 {
            return None;
        }
        game.opt_int(DAMAGE_CAP)
            .filter(|cap| (1..=MAX_DAMAGE_CAP).contains(cap))
    }

    /// The number of the combat the game is playing now (D-152): the combat
    /// turn's half, the same number the report gives that combat's round.
    /// `None` in a shop turn, before the first combat, or when the game could
    /// not be read.
    pub fn combat_round_now(&self) -> Option<i64> {
        if self.error.is_some() {
            return None;
        }
        let turn = self.collector.turn();
        (turn >= 2 && turn % 2 == 0).then_some(turn / 2)
    }

    /// How many boards have entered play so far in the combat the game is
    /// playing now (a board enters play when it first attacks); `None` in a
    /// shop turn or when the game could not be read. It only grows within a
    /// combat (both sides' boards in Solo enter together, a Duos team's
    /// second board later), so a reader can tell the moment the odds' input
    /// changes without building a report.
    pub fn combat_boards_in_now(&self) -> Option<usize> {
        let round = self.combat_round_now()?;
        let seen = self.collector.entries.get(&round)?;
        Some(seen.iter().filter(|e| e.3.is_some()).count())
    }

    /// How many legs of the combat the game is playing now have begun their
    /// start of combat and not reached their first attack (the report's
    /// `opening`, T-628, D-165); `None` in a shop turn or when the game could
    /// not be read. Cheap, like [`GameReader::combat_boards_in_now`].
    pub fn combat_legs_opening_now(&self) -> Option<usize> {
        self.combat_round_now()?;
        Some(self.collector.opening_count())
    }

    /// The player's own board as it is now in a shop turn (T-605): what the
    /// player sees on their side of the tavern, with the lasting keywords the
    /// simulator reads. `None` in a combat turn, before the first shop turn,
    /// or when the game could not be read.
    pub fn own_board_now(&self) -> Option<Vec<report::Minion>> {
        if self.error.is_some() {
            return None;
        }
        let turn = self.collector.turn();
        if turn < 1 || turn % 2 == 0 {
            return None;
        }
        let pid = self.collector.local_pid?;
        Some(collector::board_of(&self.collector.board, pid))
    }

    /// The player's game-long values as they are now in a shop turn (T-610):
    /// what the overlay's odds play the player's board with. `None` when
    /// [`Self::own_board_now`] is.
    pub fn own_values_now(&self) -> Option<report::CombatValues> {
        self.own_board_now()?;
        Some(self.collector.combat_values(report::Side::Own))
    }

    /// The local player's own game-long counters as they are now, in a shop
    /// or a combat turn (T-611, D-139): the same values the simulator reads
    /// ([`Self::own_values_now`]) but with no turn gate, only the kinds in
    /// [`counters::ENABLED_KINDS`]. `None` before the player is known or when
    /// the game could not be read; `Some(vec![])` when nothing applies. Live
    /// only, never saved.
    pub fn counters_now(&self) -> Option<Vec<counters::Counter>> {
        if self.error.is_some() {
            return None;
        }
        let local = self.collector.local?;
        self.collector.board.get(local)?;
        let inputs = self.collector.counters.inputs();
        Some(counters::counters_from(inputs, counters::ENABLED_KINDS))
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

    /// What the game in progress looks like so far, built by the same code
    /// as the saved report (so live and saved agree). It never changes what
    /// the game's final report will say. `None` before any game starts.
    pub fn snapshot_current(&self) -> Option<GameReport> {
        let reader = self.current.as_ref()?;
        Some(build_report(self.count + 1, reader))
    }

    /// The leaderboard of the Battlegrounds game in progress, as the log has
    /// it now (see [`GameReader::lobby_now`]); empty when no such game is on.
    pub fn lobby_now(&self) -> Vec<LobbySlot> {
        match self.current.as_ref() {
            Some(reader) if self.in_progress_index().is_some() => reader.lobby_now(),
            _ => Vec::new(),
        }
    }

    /// The heroes on offer in the Battlegrounds game in progress (see
    /// [`GameReader::hero_offer_now`]); `None` when no such game is on.
    pub fn hero_offer_now(&self) -> Option<HeroOffer> {
        self.in_progress_index()?;
        self.current.as_ref()?.hero_offer_now()
    }

    /// The next opponent of the Battlegrounds game in progress (see
    /// [`GameReader::next_opponent_now`]); `None` when no such game is on.
    pub fn next_opponent_now(&self) -> Option<i64> {
        self.in_progress_index()?;
        self.current.as_ref()?.next_opponent_now()
    }

    /// Whether the player fights first in the next Duos combat (see
    /// [`GameReader::fights_first_now`]); `None` when no such game is on.
    pub fn fights_first_now(&self) -> Option<bool> {
        self.in_progress_index()?;
        self.current.as_ref()?.fights_first_now()
    }

    /// The damage cap of the Battlegrounds game in progress (see
    /// [`GameReader::damage_cap_now`]); `None` when no such game is on.
    pub fn damage_cap_now(&self) -> Option<i64> {
        self.in_progress_index()?;
        self.current.as_ref()?.damage_cap_now()
    }

    /// The combat the Battlegrounds game in progress is playing (see
    /// [`GameReader::combat_round_now`]); `None` when no such game is on.
    pub fn combat_round_now(&self) -> Option<i64> {
        self.in_progress_index()?;
        self.current.as_ref()?.combat_round_now()
    }

    /// How many boards have entered play in the combat of the Battlegrounds
    /// game in progress (see [`GameReader::combat_boards_in_now`]); `None`
    /// when no such game is on or it is not a combat turn.
    pub fn combat_boards_in_now(&self) -> Option<usize> {
        self.in_progress_index()?;
        self.current.as_ref()?.combat_boards_in_now()
    }

    /// How many legs of the combat of the Battlegrounds game in progress are
    /// between their start of combat and their first attack (see
    /// [`GameReader::combat_legs_opening_now`]); `None` when no such game is
    /// on or it is not a combat turn.
    pub fn combat_legs_opening_now(&self) -> Option<usize> {
        self.in_progress_index()?;
        self.current.as_ref()?.combat_legs_opening_now()
    }

    /// The own board in a shop turn of the Battlegrounds game in progress
    /// (see [`GameReader::own_board_now`]); `None` when no such game is on.
    pub fn own_board_now(&self) -> Option<Vec<report::Minion>> {
        self.in_progress_index()?;
        self.current.as_ref()?.own_board_now()
    }

    /// The player's game-long values in a shop turn of the Battlegrounds game
    /// in progress (see [`GameReader::own_values_now`]).
    pub fn own_values_now(&self) -> Option<report::CombatValues> {
        self.in_progress_index()?;
        self.current.as_ref()?.own_values_now()
    }

    /// The player's counters in the Battlegrounds game in progress (see
    /// [`GameReader::counters_now`]); `None` when no such game is on.
    pub fn counters_now(&self) -> Option<Vec<counters::Counter>> {
        self.in_progress_index()?;
        self.current.as_ref()?.counters_now()
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
    if r.skipped_lines > 0 {
        let s = if r.skipped_lines == 1 { "" } else { "s" };
        base.warnings.push(format!(
            "skipped {} unreadable log line{s}",
            r.skipped_lines
        ));
    }
    if r.last_line_skipped && !r.is_complete() {
        base.warnings.push(WARNING_LOG_CUT.into());
    }
    let c = &r.collector;
    let error = r.error.or_else(|| r.line_error());
    if error.is_some() || c.board.game().is_none() {
        base.problems
            .push(problem_text(error.unwrap_or("no game data")));
        return base;
    }
    let mut report = summarize(base, c);
    report.card_names = hero_names(&report, &r.names);
    report.skin_parents = skin_parents(&report, c);
    report
}

/// What the player reads for a game whose lines make it unreadable: they say
/// what to do. Only letters, digits and `: . ( ) -` (the server checks them).
pub const PROBLEM_NO_TIMES: &str =
    "the game log has no times (restart Hearthstone and play a new game)";
pub const PROBLEM_TOO_MANY_BAD_LINES: &str =
    "the game log has too many unreadable lines (restart Hearthstone and play a new game)";

/// Said when a game that did not finish ends in a line cut in the middle:
/// the game stops writing `Power.log` at about 10 MB unless `client.config`
/// lifts the limit. Only letters, digits and `: . ( ) -` (the server checks).
pub const WARNING_LOG_CUT: &str =
    "the log was cut in the middle of a line (without client.config the game stops writing Power.log at 10 MB)";

/// Whether a report's problem says the game could not be read (not just that
/// the lobby is not there yet).
pub fn is_read_failure(problem: &str) -> bool {
    problem.contains("parser error")
        || problem == PROBLEM_NO_TIMES
        || problem == PROBLEM_TOO_MANY_BAD_LINES
}

/// The problem a game's error becomes; any cause other than the two found in
/// the lines is the parser's own name.
fn problem_text(error: &str) -> String {
    match error {
        NO_TIMES => PROBLEM_NO_TIMES.into(),
        TOO_MANY_BAD_LINES => PROBLEM_TOO_MANY_BAD_LINES.into(),
        other => format!("parser error: {other}"),
    }
}

/// Every hero card id the report mentions.
fn heroes_in(report: &GameReport) -> impl Iterator<Item = &String> {
    let lobby = report.lobby.iter().filter_map(|p| p.hero.as_ref());
    let combats = report
        .rounds
        .iter()
        .flat_map(|r| r.entries.iter().filter_map(|e| e.hero.as_ref()));
    let offered = report.hero_select.iter().flat_map(|h| h.offered.iter());
    [&report.hero, &report.teammate_hero]
        .into_iter()
        .flatten()
        .chain(lobby)
        .chain(combats)
        .chain(offered)
}

/// Names of the heroes the report mentions, when the log printed them.
fn hero_names(report: &GameReport, names: &HashMap<String, String>) -> BTreeMap<String, String> {
    heroes_in(report)
        .filter_map(|card| Some((card.clone(), names.get(card)?.clone())))
        .take(MAX_HERO_MAPS)
        .collect()
}

/// Base hero links of the heroes the report mentions, when the log gives one
/// (and only one) for the card.
fn skin_parents(report: &GameReport, c: &Collector) -> BTreeMap<String, i64> {
    heroes_in(report)
        .filter_map(|card| Some((card.clone(), (*c.skin_parents.get(card)?)?)))
        .take(MAX_HERO_MAPS)
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
            triples: Vec::new(),
            trinkets: Vec::new(),
        })
        .map(|mut p| {
            let shown = c.leaderboard.of(p.player_id);
            p.triples = shown.triples;
            p.trinkets = shown.trinkets;
            p
        })
        .collect();
    base.shop_tribes = count_tribes(c);
    base.rounds = rounds_of(c, &heroes, complete.then(|| hero_health(own)));
    base.opening = opening_of(c, &heroes);
    base.start_health = c.lobby_health.get(&0).cloned().unwrap_or_default();
    base.shop = Some(c.shop.record(&c.board, c.seats()));
    base.hero_select = c.hero_select.record();
    base.discovers = c.discovers.record();
    base.old_god = game
        .opt_int(OLD_GOD)
        .filter(|v| (1..=collector::MAX_DBF_ID).contains(v));
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

/// The lobby seat whose hero has this card id (the latest one).
fn seat_by_hero(heroes: &[(i64, &Entity)], card: &Option<String>) -> Option<i64> {
    let card = card.as_ref()?;
    heroes
        .iter()
        .rev()
        .find(|(_, h)| h.card_id.as_ref() == Some(card))
        .map(|(pid, _)| *pid)
}

/// The legs whose start of combat is under way now (T-628, D-165), with
/// their seat as the round's entries get it.
fn opening_of(c: &Collector, heroes: &[(i64, &Entity)]) -> Vec<OpeningLeg> {
    c.opening()
        .into_iter()
        .map(|leg| OpeningLeg {
            player_id: seat_by_hero(heroes, &leg.hero).or(leg.player_id),
            ..leg
        })
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
    let by_card = |card: &Option<String>| seat_by_hero(heroes, card);
    c.entries
        .iter()
        .map(|(&n, entries)| Round {
            number: n,
            entries: entries
                .iter()
                .enumerate()
                .map(|(i, (side, card, hint, board))| CombatEntry {
                    side: *side,
                    player_id: by_card(card).or(*hint),
                    hero: card.clone(),
                    board: board.clone(),
                    values: c.entry_values.get(&(n, i)).copied(),
                    before: c.entry_before.get(&(n, i)).map(|b| b.0.clone()),
                    kit: c.entry_before.get(&(n, i)).map(|b| b.1.clone()),
                })
                .collect(),
            own_health_after: health.get(&n).copied().flatten(),
            health_after: lobby_health.get(&n).cloned().unwrap_or_default(),
            result: c
                .results
                .get(&n)
                .copied()
                .flatten()
                .filter(|_| c.result_tags_seen),
            attacks: c.attacks.get(&n).cloned().unwrap_or_default(),
            own_shop_board: c.shop_boards.get(&n).cloned(),
            hidden_triggers: c.hidden_triggers.get(&n).cloned().unwrap_or_default(),
            replay: c.replays.replay(n),
        })
        .collect()
}
