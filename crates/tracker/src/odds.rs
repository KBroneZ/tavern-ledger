//! Combat odds for the overlay (T-605, D-152): the two boards the game
//! puts into play in the combat it is playing now, played many times by the
//! simulator (D-081). Only that combat's odds, only during that combat: no
//! odds in the shop from a board of an opponent seen turns ago. In Duos
//! (T-608) each team brings two boards: see [`crate::odds_duos`].
//!
//! Red line (D-004, D-054): only boards the player saw: the ones that
//! entered play in the combat on screen. When the odds cannot be trusted (a
//! board not in yet, a card the simulator does not know, no card data) the
//! overlay gets the reason, never a number.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use bg_parser::report::{CombatValues, GameReport, LobbySlot, Minion as LogMinion, Side};
use combat_sim::cards::{has_start_of_combat, unchecked_build};
use combat_sim::gifts;
use combat_sim::{
    simulate_teams, BaseCard, CombatResult, GameValues, Keywords, Minion, Odds, Options, Team,
    Tribes, Verdict, Warband,
};
use serde::Serialize;

use crate::odds_duos::{duos_combat, legs};
pub use crate::odds_duos::{BoardSeen, DuosInput, Role, Seen};

/// Trials for a full answer: the share's standard error is at most 0.5 point.
pub const TRIALS: u32 = 10_000;
/// Trials per batch; the time budget is checked between batches.
const BATCH: u32 = 1_000;
/// Fixed, so the same boards give the same odds.
const SEED: u64 = 0x0DD5;
/// The spread's ends: the 10th and 90th percentiles.
const LOW: f64 = 0.10;
const HIGH: f64 = 0.90;

/// Card facts a board does not show, from the card data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub tier: Option<u8>,
    pub tribes: Tribes,
    pub base: BaseCard,
}

impl Facts {
    /// From the card data's words: types as the log names them (`BEAST`,
    /// `MECHANICAL`, `ALL`) and keywords as HearthstoneJSON spells them
    /// (`DIVINE_SHIELD`). A word the simulator does not know is left out.
    pub fn from_names(
        attack: i32,
        health: i32,
        tier: Option<u8>,
        tribes: &[String],
        keywords: &[String],
    ) -> Facts {
        Facts {
            tier,
            tribes: tribes
                .iter()
                .filter_map(|t| Tribes::from_name(t))
                .fold(Tribes::NONE, Tribes::union),
            base: BaseCard {
                attack,
                health,
                keywords: keywords
                    .iter()
                    .filter_map(|k| Keywords::from_name(k))
                    .fold(Keywords::NONE, Keywords::union),
            },
        }
    }
}

/// What the odds are worked out from. Equal inputs give equal odds, so the
/// app runs the simulator only when this changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OddsInput {
    /// The opponent whose board fights first.
    pub opponent_seat: i64,
    /// The combat (round) the boards entered play in.
    pub round: i64,
    /// A card on a board of that combat acts at the start of combat, so the
    /// boards may already carry its effects (a minion killed or buffed
    /// before the first attack).
    pub seen_after_start_of_combat: bool,
    pub own: Vec<LogMinion>,
    pub opponent: Vec<LogMinion>,
    /// The player's game-long values as their board entered play (T-610);
    /// zero when the log gave none.
    pub own_values: CombatValues,
    /// The opponent's game-long values as their board entered play.
    pub opponent_values: CombatValues,
    pub own_tier: Option<i64>,
    /// The opponent's tavern tier now (it is shown on the leaderboard).
    pub opponent_tier: Option<i64>,
    pub damage_cap: Option<i64>,
    /// Duos: the teammate's board, the opposing teammate's and the order.
    pub duos: Option<DuosInput>,
    /// The game build the boards come from, when the log said (D-145).
    pub build: Option<i64>,
}

/// Why there are no odds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gap {
    /// Duos: a board of the combat is not in play yet (`boards` says which).
    MissingBoard,
    /// The game is not playing a combat: a shop turn, where there are no odds.
    NotInCombat,
    /// The combat's boards have not entered play yet.
    NoBoardSeen,
    /// The card data is not on the PC yet, or lacks a card on the boards.
    NoCardData,
    /// A card on the boards is not in the simulator.
    NotModelled,
    /// The boards are not ones the simulator accepts.
    BadBoard,
}

/// No odds now, and why; the seat when known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoOdds {
    pub gap: Gap,
    pub seat: Option<i64>,
    /// Duos: the boards the odds need and which were never seen.
    pub boards: Vec<BoardSeen>,
}

impl NoOdds {
    pub(crate) fn new(gap: Gap, seat: Option<i64>) -> Self {
        NoOdds {
            gap,
            seat,
            boards: Vec::new(),
        }
    }

    /// As the overlay shows it.
    pub fn odds(&self) -> LiveOdds {
        LiveOdds {
            boards: self.boards.clone(),
            ..LiveOdds::gap(self.gap, self.seat, None)
        }
    }
}

/// The damage of the trials with that result, after the game's damage cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Spread {
    pub low: i32,
    /// The median.
    pub likely: i32,
    pub high: i32,
}

/// The odds as the overlay shows them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LiveOdds {
    /// The opponent whose board fights first in the combat; None when the
    /// combat's boards are not in yet.
    pub seat: Option<i64>,
    /// Set when there are no odds; the shares are then 0 and say nothing.
    pub gap: Option<Gap>,
    /// The combat these odds are for.
    pub round: Option<i64>,
    /// Duos: every board but the player's own and the combat it entered play
    /// in (with a `MissingBoard` gap, the ones not in yet have no round, and
    /// the player's own is listed too). Empty in Solo.
    pub boards: Vec<BoardSeen>,
    pub won: f64,
    pub tied: f64,
    pub lost: f64,
    pub trials: u32,
    /// Damage to the opponent when the player wins; None when unknown.
    pub dealt: Option<Spread>,
    /// Damage to the player when the player loses; None when unknown.
    pub taken: Option<Spread>,
    /// Card ids played with a known gap.
    pub approximate: Vec<String>,
    /// Card ids the simulator does not know, or the card data lacks.
    pub missing: Vec<String>,
    /// A card acts at the start of combat, which a shop board has not had yet.
    pub start_of_combat: bool,
    /// The game build when the simulator's rules were not checked against
    /// it: the odds are still given, with a warning (D-145).
    pub new_build: Option<i64>,
}

impl LiveOdds {
    pub fn gap(gap: Gap, seat: Option<i64>, round: Option<i64>) -> Self {
        LiveOdds {
            seat,
            gap: Some(gap),
            round,
            boards: Vec::new(),
            won: 0.0,
            tied: 0.0,
            lost: 0.0,
            trials: 0,
            dealt: None,
            taken: None,
            approximate: Vec::new(),
            missing: Vec::new(),
            start_of_combat: false,
            new_build: None,
        }
    }
}

/// True when a card on a board of combat `round` acts at the start of
/// combat: a board seen there may carry its effects.
pub(crate) fn start_of_combat_in(report: &GameReport, round: i64) -> bool {
    report
        .rounds
        .iter()
        .filter(|r| r.number == round)
        .flat_map(|r| r.entries.iter())
        .flat_map(|e| e.board.iter().flatten())
        .any(acts_at_start_of_combat)
}

/// True when the minion's card or one of its Dark Gifts acts at the start
/// of combat (T-528).
pub(crate) fn acts_at_start_of_combat(m: &LogMinion) -> bool {
    m.card_id.as_deref().is_some_and(has_start_of_combat)
        || m.gifts.iter().any(|g| gifts::has_start_of_combat(g))
}

/// Card ids of the minions the game marks as gifted with no gift the log
/// names (T-528): played as the plain card, so the odds say approximate.
fn unknown_gifts(input: &OddsInput) -> Vec<String> {
    let duos = input
        .duos
        .iter()
        .flat_map(|d| d.teammate.board.iter().chain(&d.opponent_teammate.board));
    let mut ids: Vec<String> = Vec::new();
    for m in input.own.iter().chain(&input.opponent).chain(duos) {
        if let (true, Some(id)) = (m.gift_unknown, &m.card_id) {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
    }
    ids
}

/// What the game shows now, beside the saved report, that the odds read.
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveFacts<'a> {
    /// The combat the game is playing now; `None` in a shop turn.
    pub combat_round: Option<i64>,
    pub lobby: &'a [LobbySlot],
    pub damage_cap: Option<i64>,
}

/// The input for the odds of the combat the game is playing now, from the
/// boards that entered play in it, or why there are none (with the seat when
/// known). Never a board of an earlier combat.
pub fn odds_input(report: &GameReport, live: LiveFacts) -> Result<OddsInput, NoOdds> {
    let Some(number) = live.combat_round else {
        return Err(NoOdds::new(Gap::NotInCombat, None));
    };
    let Some(round) = report.rounds.iter().find(|r| r.number == number) else {
        return Err(NoOdds::new(Gap::NoBoardSeen, None));
    };
    let (own, opponent, duos) = if report.game_type.as_deref() == Some("GT_BATTLEGROUNDS_DUO") {
        let (own, opponent, duos) = duos_combat(report, round, live.lobby)?;
        (own, opponent, Some(duos))
    } else {
        let own = legs(round, Side::Own).into_iter().next();
        let opponent = legs(round, Side::Opponent).into_iter().next();
        let seat = opponent.map(|l| l.seat);
        let (Some(own), Some(opponent)) = (own, opponent) else {
            return Err(NoOdds::new(Gap::NoBoardSeen, seat));
        };
        (own, opponent, None)
    };
    let tier = |seat: i64| {
        live.lobby
            .iter()
            .find(|s| s.player_id == seat)
            .and_then(|s| s.tech_level)
    };
    Ok(OddsInput {
        opponent_seat: opponent.seat,
        round: number,
        seen_after_start_of_combat: start_of_combat_in(report, number),
        own: own.board.to_vec(),
        opponent: opponent.board.to_vec(),
        own_values: own.values,
        opponent_values: opponent.values,
        own_tier: tier(own.seat),
        opponent_tier: tier(opponent.seat),
        damage_cap: live.damage_cap,
        duos,
        build: report.build,
    })
}

fn engine_minion(m: &LogMinion, facts: &dyn Fn(&str) -> Option<Facts>) -> Result<Minion, String> {
    let id = m.card_id.clone().ok_or_else(|| "?".to_string())?;
    let f = facts(&id).ok_or_else(|| id.clone())?;
    let keywords = m
        .keywords
        .iter()
        .chain(&m.sim_keywords)
        .filter_map(|k| Keywords::from_name(k))
        .fold(Keywords::NONE, Keywords::union);
    Ok(Minion {
        golden: m.golden,
        attack: i32::try_from(m.atk).map_err(|_| id.clone())?,
        health: i32::try_from(m.health).map_err(|_| id.clone())?,
        keywords,
        tribes: f.tribes,
        tier: f.tier,
        base: Some(f.base),
        gifts: m.gifts.iter().map(|g| g.to_string()).collect(),
        card_id: id,
    })
}

/// The engine's form of values already bounded to at most 99 999
/// ([`CombatValues::bounded`]).
fn engine_values(v: &CombatValues) -> GameValues {
    let n = |x: i64| i32::try_from(x).unwrap_or(0);
    GameValues {
        blood_gem: (n(v.blood_gem.0), n(v.blood_gem.1)),
        spell: (n(v.spell.0), n(v.spell.1)),
        knights_dead: n(v.knights_dead),
        undead_attack: n(v.undead_attack),
        beetle: (n(v.beetle.0), n(v.beetle.1)),
    }
}

fn warband(
    board: &[LogMinion],
    tier: Option<i64>,
    values: &CombatValues,
    facts: &dyn Fn(&str) -> Option<Facts>,
    missing: &mut Vec<String>,
) -> Warband {
    let mut minions = Vec::new();
    for m in board {
        match engine_minion(m, facts) {
            Ok(minion) => minions.push(minion),
            Err(id) if !missing.contains(&id) => missing.push(id),
            Err(_) => {}
        }
    }
    Warband {
        minions,
        tavern_tier: tier.and_then(|t| u8::try_from(t).ok()),
        values: engine_values(values),
    }
}

/// Both teams in the engine's form: in Solo one board each; in Duos the
/// player's and the teammate's in the game's order, and the next opponent's
/// board before their teammate's. Cards without data go to `missing`.
fn teams(
    input: &OddsInput,
    facts: &dyn Fn(&str) -> Option<Facts>,
    missing: &mut Vec<String>,
) -> (Team, Team) {
    let own = warband(
        &input.own,
        input.own_tier,
        &input.own_values,
        facts,
        missing,
    );
    let opponent = warband(
        &input.opponent,
        input.opponent_tier,
        &input.opponent_values,
        facts,
        missing,
    );
    let Some(duos) = &input.duos else {
        return (Team::solo(own), Team::solo(opponent));
    };
    let seen =
        |s: &Seen, missing: &mut Vec<String>| warband(&s.board, s.tier, &s.values, facts, missing);
    let mate = seen(&duos.teammate, missing);
    let player = if duos.local_first {
        Team::duo(own, mate)
    } else {
        Team::duo(mate, own)
    };
    let foe_mate = seen(&duos.opponent_teammate, missing);
    (player, Team::duo(opponent, foe_mate))
}

/// The damage spread after the cap, or None when there is none to give.
fn spread(counts: &BTreeMap<i32, u32>, cap: Option<i64>) -> Option<Spread> {
    let capped = |d: i32| cap.map_or(d, |c| d.min(i32::try_from(c).unwrap_or(i32::MAX)));
    let total: u32 = counts.values().sum();
    if total == 0 {
        return None;
    }
    let at = |share: f64| {
        let want = (share * f64::from(total)).ceil().max(1.0) as u32;
        let mut seen = 0;
        counts
            .iter()
            .find(|(_, n)| {
                seen += **n;
                seen >= want
            })
            .map_or(0, |(d, _)| capped(*d))
    };
    Some(Spread {
        low: at(LOW),
        likely: at(0.5),
        high: at(HIGH),
    })
}

/// Runs the simulator on `input` in batches until [`TRIALS`] or `budget`
/// (at least one batch). `facts` gives the card data for a card id.
pub fn run_odds(
    input: &OddsInput,
    facts: &dyn Fn(&str) -> Option<Facts>,
    budget: Duration,
) -> LiveOdds {
    let seat = Some(input.opponent_seat);
    let boards = input
        .duos
        .as_ref()
        .map(|d| d.boards(input.opponent_seat, input.round))
        .unwrap_or_default();
    let round = Some(input.round);
    let gap = |gap: Gap| LiveOdds {
        boards: boards.clone(),
        ..LiveOdds::gap(gap, seat, round)
    };
    let mut missing = Vec::new();
    let (player, opponent) = teams(input, facts, &mut missing);
    if !missing.is_empty() {
        return LiveOdds {
            missing,
            ..gap(Gap::NoCardData)
        };
    }
    let started = Instant::now();
    let mut total: Option<Odds> = None;
    let mut batch = 0u64;
    while total.as_ref().map_or(0, |o| o.trials) < TRIALS {
        let options = Options {
            trials: BATCH,
            seed: SEED ^ batch,
            start_of_combat: false,
            allow_unmodelled: false,
        };
        match simulate_teams(&player, &opponent, &options) {
            Ok(Verdict::Odds(o)) => match total.as_mut() {
                Some(t) => t.merge(&o),
                None => total = Some(o),
            },
            Ok(Verdict::NotModelled(c)) => {
                return LiveOdds {
                    missing: c.not_modelled,
                    approximate: c.approximate.into_iter().map(|(id, _)| id).collect(),
                    ..gap(Gap::NotModelled)
                };
            }
            Err(_) => return gap(Gap::BadBoard),
        }
        batch += 1;
        if started.elapsed() >= budget {
            break;
        }
    }
    let Some(o) = total else {
        return gap(Gap::BadBoard);
    };
    let start_of_combat = input.seen_after_start_of_combat
        || player
            .boards()
            .chain(opponent.boards())
            .flat_map(|b| &b.minions)
            .any(|m| {
                has_start_of_combat(&m.card_id)
                    || m.gifts.iter().any(|g| gifts::has_start_of_combat(g))
            });
    let known = o.damage_unknown == 0;
    LiveOdds {
        seat,
        gap: None,
        round,
        boards,
        won: o.share(CombatResult::Won),
        tied: o.share(CombatResult::Tie),
        lost: o.share(CombatResult::Lost),
        trials: o.trials,
        dealt: spread(&o.damage_dealt, input.damage_cap).filter(|_| known),
        taken: spread(&o.damage_taken, input.damage_cap).filter(|_| known),
        approximate: with_unknown_gifts(
            o.coverage
                .approximate
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
            input,
        ),
        missing: Vec::new(),
        start_of_combat,
        new_build: unchecked_build(input.build),
    }
}

fn with_unknown_gifts(mut approximate: Vec<String>, input: &OddsInput) -> Vec<String> {
    for id in unknown_gifts(input) {
        if !approximate.contains(&id) {
            approximate.push(id);
        }
    }
    approximate
}

#[cfg(test)]
#[path = "odds_tests.rs"]
mod tests;
