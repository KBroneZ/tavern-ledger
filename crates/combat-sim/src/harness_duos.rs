//! The harness on Duos combats (T-608): each team's boards in the order
//! they fought, read from the game's own logs; the engine scored on them;
//! and the odds the overlay would give in Duos, from the boards the player
//! had seen before each combat (D-054: only combats the game played on
//! screen).
//!
//! Local only, like the rest of the harness: counts, shares and card ids,
//! never a name, a hero or a player id.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Instant;

use bg_parser::report::{GameReport, Round, Side};
use serde_json::Value;

use crate::combat::CombatResult;
use crate::harness::{board_with_kit, result, CardDb};
use crate::harness_logs::{engine_board, entry_band};
use crate::model::{Team, Warband};
use crate::score::{score, Prediction};
use crate::trials::{simulate_teams, Options, Verdict};

/// One board that fought: whose (a lobby seat) and the board as it came in.
#[derive(Clone, Debug, PartialEq)]
pub struct Leg {
    pub seat: Option<i64>,
    pub board: Warband,
}

/// One Duos combat the log plays, with each team's boards in the order
/// they fought (one when the first board settled it).
#[derive(Clone, Debug, PartialEq)]
pub struct DuosCombat {
    pub game: usize,
    pub round: i64,
    pub own: Vec<Leg>,
    pub opponent: Vec<Leg>,
    /// The player's seat and their teammate's.
    pub local: Option<i64>,
    pub teammate: Option<i64>,
    /// The opposing team's two seats (the lobby's teams).
    pub opponent_seats: Vec<i64>,
    /// Seats with no health left before this combat: out of the game.
    pub out: Vec<i64>,
    /// The player's board as the shop turn before ended (the overlay's input).
    pub own_shop: Option<Warband>,
    pub result: Option<CombatResult>,
    /// Cards the game marks as gifted with no gift named (D-127).
    pub unknown_gifts: Vec<String>,
}

/// One side stepping into a combat, from the logs or the saved history.
struct Entry {
    side: Side,
    seat: Option<i64>,
    /// `None` when the log has no board for it.
    board: Option<Warband>,
}

/// The boards of `side` in the order they came in, at most one per seat.
///
/// The parser takes a board at the first attack after it comes in, a
/// minion's or, once the minions are done, the heroes': nothing happens in
/// between, so each is the board as it came in. A board with no attack after
/// it came in has none: an empty board replaced at once, the hero switching
/// back after the combat, or a combat the game does not play back (D-054),
/// whose boards are never read.
fn legs(entries: &[Entry], side: Side) -> Vec<Leg> {
    let mut out: Vec<Leg> = Vec::new();
    for e in entries.iter().filter(|e| e.side == side) {
        let Some(board) = &e.board else { continue };
        if out.iter().all(|l| l.seat != e.seat) {
            out.push(Leg {
                seat: e.seat,
                board: board.clone(),
            });
        }
    }
    out
}

/// What a game says about its seats.
struct Seats<'a> {
    local: Option<i64>,
    teammate: Option<i64>,
    /// (seat, Duos team) for every lobby seat.
    teams: &'a [(i64, Option<i64>)],
}

/// What one round gives besides its entries.
struct RoundFacts {
    game: usize,
    round: i64,
    out: Vec<i64>,
    own_shop: Option<Warband>,
    result: Option<CombatResult>,
    unknown_gifts: Vec<String>,
}

/// One combat from its entries; `None` without a board on each side.
fn combat(entries: &[Entry], seats: &Seats, facts: RoundFacts) -> Option<DuosCombat> {
    let (own, opponent) = (legs(entries, Side::Own), legs(entries, Side::Opponent));
    if own.is_empty() || opponent.is_empty() {
        return None;
    }
    let team = |seat: i64| seats.teams.iter().find(|(s, _)| *s == seat)?.1;
    let opponent_seats = match opponent[0].seat.and_then(team) {
        Some(t) => seats
            .teams
            .iter()
            .filter(|(_, team)| *team == Some(t))
            .map(|(s, _)| *s)
            .collect(),
        None => opponent.iter().filter_map(|l| l.seat).collect(),
    };
    Some(DuosCombat {
        game: facts.game,
        round: facts.round,
        own,
        opponent,
        local: seats.local,
        teammate: seats.teammate,
        opponent_seats,
        out: facts.out,
        own_shop: facts.own_shop,
        result: facts.result,
        unknown_gifts: facts.unknown_gifts,
    })
}

/// Seats with no health left once a combat closed: out of the game.
fn out_after(health: &BTreeMap<i64, i64>) -> Vec<i64> {
    health
        .iter()
        .filter(|(_, h)| **h <= 0)
        .map(|(s, _)| *s)
        .collect()
}

fn result_of(r: bg_parser::report::CombatResult) -> CombatResult {
    match r {
        bg_parser::report::CombatResult::Won => CombatResult::Won,
        bg_parser::report::CombatResult::Lost => CombatResult::Lost,
        bg_parser::report::CombatResult::Tie => CombatResult::Tie,
    }
}

/// The Duos combats of one game read from its log (a board on each side).
/// A card id missing drops the combat.
pub fn duos_combats(report: &GameReport, game: usize, db: &CardDb) -> Vec<DuosCombat> {
    let teams: Vec<(i64, Option<i64>)> = report
        .lobby
        .iter()
        .map(|p| (p.player_id, p.duo_team))
        .collect();
    let seats = Seats {
        local: report.local_player_id,
        teammate: report.teammate_player_id,
        teams: &teams,
    };
    let mut out = Vec::new();
    let mut previous: Option<&Round> = None;
    for round in &report.rounds {
        let before = previous.replace(round);
        let entries: Option<Vec<Entry>> = round
            .entries
            .iter()
            .map(|e| {
                Some(Entry {
                    side: e.side,
                    seat: e.player_id,
                    board: entry_band(e, db)?,
                })
            })
            .collect();
        let Some(entries) = entries else { continue };
        let facts = RoundFacts {
            game,
            round: round.number,
            out: before
                .map(|b| out_after(&b.health_after))
                .unwrap_or_default(),
            own_shop: round
                .own_shop_board
                .as_deref()
                .and_then(|b| engine_board(b, db))
                .map(Warband::new),
            result: round.result.map(result_of),
            unknown_gifts: crate::harness_logs::unknown_gifts(round),
        };
        out.extend(combat(&entries, &seats, facts));
    }
    out
}

/// The Duos combats of one game of the saved history (`games.jsonl`). The
/// saved report has no shop board: the player's board as it came into the
/// combat stands in for it, when the player's own board fought.
pub fn duos_from_saved(report: &Value, game: usize, db: &CardDb) -> Vec<DuosCombat> {
    let int = |v: &Value, k: &str| v.get(k).and_then(Value::as_i64);
    let teams: Vec<(i64, Option<i64>)> = report
        .get("lobby")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|p| Some((int(p, "player_id")?, int(p, "duo_team"))))
        .collect();
    let seats = Seats {
        local: int(report, "local_player_id"),
        teammate: int(report, "teammate_player_id"),
        teams: &teams,
    };
    let rounds = report.get("rounds").and_then(Value::as_array);
    let mut out = Vec::new();
    let mut gone: Vec<i64> = Vec::new();
    for round in rounds.into_iter().flatten() {
        let mut entries = Vec::new();
        let mut readable = true;
        let saved = round.get("entries").and_then(Value::as_array);
        for e in saved.into_iter().flatten() {
            let side = match e.get("side").and_then(Value::as_str) {
                Some("own") => Side::Own,
                Some("opponent") => Side::Opponent,
                _ => continue,
            };
            let board = match e.get("board") {
                Some(Value::Array(_)) => {
                    let b = board_with_kit(e, round, db);
                    readable &= b.is_some();
                    b
                }
                _ => None,
            };
            entries.push(Entry {
                side,
                seat: int(e, "player_id"),
                board,
            });
        }
        if readable {
            let own_shop = legs(&entries, Side::Own)
                .into_iter()
                .find(|l| l.seat.is_some() && l.seat == seats.local)
                .map(|l| l.board);
            let facts = RoundFacts {
                game,
                round: int(round, "number").unwrap_or(0),
                out: gone.clone(),
                own_shop,
                result: result(round),
                unknown_gifts: crate::saved::unknown_gifts(
                    round
                        .get("entries")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten(),
                ),
            };
            out.extend(combat(&entries, &seats, facts));
        }
        let health: BTreeMap<i64, i64> = round
            .get("health_after")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| Some((k.parse().ok()?, v.as_i64()?)))
            .collect();
        gone = out_after(&health);
    }
    out
}

/// A team from the boards that fought, in order. `legs` is never empty
/// ([`DuosCombat`] has a board on each side).
pub fn team_of(legs: &[Leg]) -> Team {
    Team {
        first: legs[0].board.clone(),
        second: legs.get(1).map(|l| l.board.clone()),
    }
}

/// Where a board the overlay needs comes from: none seen before, or seen
/// in an earlier combat (its age in rounds).
fn last_seen<'a>(
    all: &'a [DuosCombat],
    c: &DuosCombat,
    seat: i64,
    side: Side,
) -> Option<(&'a Warband, i64)> {
    all.iter()
        .filter(|o| o.game == c.game && o.round < c.round)
        .filter_map(|o| {
            let legs = match side {
                Side::Own => &o.own,
                Side::Opponent => &o.opponent,
            };
            legs.iter()
                .find(|l| l.seat == Some(seat))
                .map(|l| (&l.board, c.round - o.round))
        })
        .min_by_key(|(_, age)| *age)
}

/// A board or fact the overlay would not have for a Duos combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gap {
    OwnShop,
    Teammate,
    Opponent,
    Order,
}

/// Why the overlay would give no number, counted over the combats.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Missing {
    pub own_shop: usize,
    pub teammate: usize,
    pub opponent: usize,
    pub order: usize,
    pub not_modelled: usize,
    pub refused: usize,
}

impl Missing {
    fn count(&mut self, gap: Gap) {
        match gap {
            Gap::OwnShop => self.own_shop += 1,
            Gap::Teammate => self.teammate += 1,
            Gap::Opponent => self.opponent += 1,
            Gap::Order => self.order += 1,
        }
    }
}

/// The overlay's input for one Duos combat (T-608): the player's shop board,
/// the teammate's and both opponents' boards as last seen in an earlier
/// combat, each team in the order it fought. `Err` names what is missing.
fn overlay_teams(all: &[DuosCombat], c: &DuosCombat) -> Result<(Team, Team, Vec<i64>), Gap> {
    let own_shop = c.own_shop.clone().ok_or(Gap::OwnShop)?;
    let local = c.local.ok_or(Gap::Order)?;
    let local_first = c.own[0].seat.ok_or(Gap::Order)? == local;
    let mut ages = Vec::new();
    let mate = match c.teammate.filter(|t| !c.out.contains(t)) {
        None => None,
        Some(t) => {
            let (b, age) = last_seen(all, c, t, Side::Own).ok_or(Gap::Teammate)?;
            ages.push(age);
            Some(b.clone())
        }
    };
    let own = match (mate, local_first) {
        (None, _) => Team::solo(own_shop),
        (Some(m), true) => Team::duo(own_shop, m),
        (Some(m), false) => Team::duo(m, own_shop),
    };
    let first_seat = c.opponent[0].seat.ok_or(Gap::Order)?;
    let mut seats = vec![first_seat];
    seats.extend(
        c.opponent_seats
            .iter()
            .copied()
            .filter(|s| *s != first_seat && !c.out.contains(s)),
    );
    let mut boards = Vec::new();
    for seat in seats.iter().take(2) {
        let (b, age) = last_seen(all, c, *seat, Side::Opponent).ok_or(Gap::Opponent)?;
        ages.push(age);
        boards.push(b.clone());
    }
    let opponent = Team {
        first: boards[0].clone(),
        second: boards.get(1).cloned(),
    };
    Ok((own, opponent, ages))
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn prediction(o: &crate::trials::Odds) -> Prediction {
    Prediction {
        won: o.share(CombatResult::Won),
        tied: o.share(CombatResult::Tie),
        lost: o.share(CombatResult::Lost),
    }
}

/// Where the Duos combats were read from: the shop board is in the logs
/// only; the saved history has the player's board as it came into the
/// combat instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Logs,
    History,
}

/// The Duos report: the engine on each combat's own boards (both legs, as
/// they came in), then the odds the overlay would show.
pub fn render_duos(combats: &[DuosCombat], options: &Options, source: Source) -> String {
    let mut out = String::new();
    let with_result: Vec<&DuosCombat> = combats.iter().filter(|c| c.result.is_some()).collect();
    let _ = writeln!(out, "\n## Duos combats (T-608)\n");
    let both = |c: &&DuosCombat| c.own.len() == 2 && c.opponent.len() == 2;
    let _ = writeln!(
        out,
        "Duos combats with a board on each side, every card id readable: {} ({} with the game's result). \
         Boards that fought: both teams' two boards {}, one team's first board settled it {}.\n",
        combats.len(),
        with_result.len(),
        combats.iter().filter(both).count(),
        combats.iter().filter(|c| !both(c)).count(),
    );
    // The engine on the boards as they came in. A team whose second board
    // did not fight is played without it: the result cannot depend on it
    // unless the engine's first board loses where the game's did not.
    let opts = Options {
        allow_unmodelled: true,
        ..*options
    };
    let mut rows: [(Vec<(Prediction, CombatResult)>, &str); 2] = [
        (Vec::new(), "every board that fought"),
        (Vec::new(), "both teams' two boards fought"),
    ];
    let mut micros: Vec<u128> = Vec::new();
    for c in &with_result {
        let start = Instant::now();
        let verdict = simulate_teams(&team_of(&c.own), &team_of(&c.opponent), &opts);
        micros.push(start.elapsed().as_micros());
        let Ok(Verdict::Odds(o)) = verdict else {
            continue;
        };
        let item = (prediction(&o), c.result.expect("filtered"));
        rows[0].0.push(item);
        if both(c) {
            rows[1].0.push(item);
        }
    }
    let _ = writeln!(
        out,
        "| Combats | n | Mean p(real result) | Brier | Most likely = real |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for (items, label) in &rows {
        let r = score(items, 5);
        let _ = writeln!(
            out,
            "| {label} | {} | {} | {:.3} | {} |",
            r.combats,
            pct(r.mean_p_actual),
            r.brier,
            if r.combats > 0 {
                format!("{} of {}", r.top_pick_hits, r.combats)
            } else {
                "-".into()
            }
        );
    }
    if let Some(max) = micros.iter().max() {
        let mean = micros.iter().sum::<u128>() / micros.len() as u128;
        let _ = writeln!(
            out,
            "
Speed: {} trials per Duos combat took {:.1} ms on average, {:.1} ms at most.",
            options.trials,
            mean as f64 / 1000.0,
            *max as f64 / 1000.0
        );
    }
    render_overlay_duos(&mut out, combats, options, source);
    out
}

/// The odds the overlay would give in Duos: unknown cards refused, start of
/// combat off, as in the app.
fn render_overlay_duos(
    out: &mut String,
    combats: &[DuosCombat],
    options: &Options,
    source: Source,
) {
    let opts = Options {
        allow_unmodelled: false,
        start_of_combat: false,
        ..*options
    };
    let mut missing = Missing::default();
    let mut items = Vec::new();
    let mut ages: Vec<i64> = Vec::new();
    let mut total = 0;
    for c in combats {
        let Some(result) = c.result else { continue };
        total += 1;
        let (own, opponent, seen) = match overlay_teams(combats, c) {
            Ok(t) => t,
            Err(gap) => {
                missing.count(gap);
                continue;
            }
        };
        match simulate_teams(&own, &opponent, &opts) {
            Ok(Verdict::Odds(o)) => {
                ages.extend(seen);
                items.push((prediction(&o), result));
            }
            Ok(Verdict::NotModelled(_)) => missing.not_modelled += 1,
            Err(_) => missing.refused += 1,
        }
    }
    let r = score(&items, 5);
    let _ = writeln!(out, "\n### The overlay's inputs in Duos\n");
    let shop = match source {
        Source::Logs => "No shop board before it",
        Source::History => {
            "The player's own board did not fight (the history has no shop board: the board \
             as it came into the combat stands in for it)"
        }
    };
    let _ = writeln!(
        out,
        "Duos combats with a result: {total}. {shop}: {}. Teammate's board \
         never seen before: {}. An opponent's board never seen before: {}. Order unknown: {}. \
         A card not modelled: {}. Refused by the board check: {}. Scored: {}.\n",
        missing.own_shop,
        missing.teammate,
        missing.opponent,
        missing.order,
        missing.not_modelled,
        missing.refused,
        r.combats
    );
    if r.combats > 0 {
        let mean_age = ages.iter().sum::<i64>() as f64 / ages.len().max(1) as f64;
        let _ = writeln!(
            out,
            "| n | Mean p(real result) | Brier | Most likely = real | Board age (rounds, mean) |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|");
        let _ = writeln!(
            out,
            "| {} | {} | {:.3} | {} of {} | {:.1} |",
            r.combats,
            pct(r.mean_p_actual),
            r.brier,
            r.top_pick_hits,
            r.combats,
            mean_age
        );
    }
}

#[cfg(test)]
#[path = "harness_duos_tests.rs"]
mod tests;
