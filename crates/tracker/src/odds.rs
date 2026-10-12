//! Combat odds for the overlay (T-605, D-084): the player's board now, in
//! the shop, against the board of the next opponent the player last saw in a
//! combat, played many times by the simulator (D-081). In Duos (T-608) each
//! team brings two boards: see [`crate::odds_duos`].
//!
//! Red line (D-004, D-054): only boards the player saw. The opponent's board
//! is the one that entered play in a combat the game has shown, never a
//! later one; the overlay says how old it is. When the odds cannot be
//! trusted (no board seen yet, a card the simulator does not know, no card
//! data) the overlay gets the reason, never a number.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use bg_parser::report::{CombatValues, GameReport, LobbySlot, Minion as LogMinion, Side};
use combat_sim::cards::has_start_of_combat;
use combat_sim::gifts;
use combat_sim::{
    simulate_teams, BaseCard, CombatResult, GameValues, Keywords, Minion, Odds, Options, Team,
    Tribes, Verdict, Warband,
};
use serde::Serialize;

use crate::odds_duos::{duos_input, last_board, seen_after_start_of_combat};
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
    pub opponent_seat: i64,
    /// The combat (turn) the opponent's board was seen in.
    pub seen_round: i64,
    /// A card on either board of that combat acted at the start of combat,
    /// so the board seen may already carry its effects (a minion killed or
    /// buffed before the first attack).
    pub seen_after_start_of_combat: bool,
    pub own: Vec<LogMinion>,
    pub opponent: Vec<LogMinion>,
    /// The player's game-long values now (T-610); zero when unknown.
    pub own_values: CombatValues,
    /// The opponent's game-long values as their board was seen.
    pub opponent_values: CombatValues,
    pub own_tier: Option<i64>,
    /// The opponent's tavern tier now (it is shown on the leaderboard).
    pub opponent_tier: Option<i64>,
    pub damage_cap: Option<i64>,
    /// Duos: the teammate's board, the opposing teammate's and the order.
    pub duos: Option<DuosInput>,
}

/// Why there are no odds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gap {
    /// Duos: who fights first, or who is on which team, is not known yet.
    Order,
    /// Duos: a board the odds need was never seen (`boards` says which).
    MissingBoard,
    /// The game has not marked the next opponent.
    NoNextOpponent,
    /// Not a shop turn: the player's board is in a combat.
    NotInShop,
    /// The player has not seen a board of the next opponent yet.
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
    /// The opponent these odds are against; None without a next opponent.
    pub seat: Option<i64>,
    /// Set when there are no odds; the shares are then 0 and say nothing.
    pub gap: Option<Gap>,
    /// The combat the opponent's board was seen in; in Duos the oldest of
    /// the boards the odds use.
    pub seen_round: Option<i64>,
    /// Duos: every board but the player's own and when it was seen (with a
    /// `MissingBoard` gap, the ones never seen have no round). Empty in Solo.
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
}

impl LiveOdds {
    pub fn gap(gap: Gap, seat: Option<i64>, seen_round: Option<i64>) -> Self {
        LiveOdds {
            seat,
            gap: Some(gap),
            seen_round,
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
    /// The player's board in the shop; `None` outside a shop turn.
    pub own_now: Option<&'a [LogMinion]>,
    /// The player's game-long values now (T-610); `None` when not read.
    pub own_values: Option<CombatValues>,
    /// The seat the game marks as the next opponent.
    pub next: Option<i64>,
    pub lobby: &'a [LobbySlot],
    pub damage_cap: Option<i64>,
    /// Duos: the player's board fights before their teammate's.
    pub fights_first: Option<bool>,
}

/// The input for the odds now, or why there are none (with the seat when
/// known).
pub fn odds_input(report: &GameReport, live: LiveFacts) -> Result<OddsInput, NoOdds> {
    let Some(seat) = live.next else {
        return Err(NoOdds::new(Gap::NoNextOpponent, None));
    };
    let Some(own) = live.own_now else {
        return Err(NoOdds::new(Gap::NotInShop, Some(seat)));
    };
    let (duos, opponent) = if report.game_type.as_deref() == Some("GT_BATTLEGROUNDS_DUO") {
        let (duos, opponent) = duos_input(report, seat, live.fights_first, live.lobby)?;
        (Some(duos), opponent)
    } else {
        (None, last_board(report, seat, Side::Opponent))
    };
    let Some((opponent, seen_round, opponent_values)) = opponent else {
        return Err(NoOdds::new(Gap::NoBoardSeen, Some(seat)));
    };
    let seen_after_start_of_combat = start_of_combat_in(report, seen_round)
        || duos
            .as_ref()
            .is_some_and(|d| seen_after_start_of_combat(report, d));
    let tier = |pid: Option<i64>| {
        live.lobby
            .iter()
            .find(|s| Some(s.player_id) == pid)
            .and_then(|s| s.tech_level)
    };
    Ok(OddsInput {
        opponent_seat: seat,
        seen_round,
        seen_after_start_of_combat,
        own: own.to_vec(),
        opponent: opponent.to_vec(),
        own_values: live.own_values.unwrap_or_default().bounded(),
        opponent_values,
        own_tier: tier(report.local_player_id),
        opponent_tier: tier(Some(seat)),
        damage_cap: live.damage_cap,
        duos,
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
        .map(|d| d.boards(input.opponent_seat, input.seen_round))
        .unwrap_or_default();
    // In Duos, the oldest board the odds use.
    let seen = boards
        .iter()
        .filter_map(|b| b.round)
        .min()
        .or(Some(input.seen_round));
    let gap = |gap: Gap| LiveOdds {
        boards: boards.clone(),
        ..LiveOdds::gap(gap, seat, seen)
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
        seen_round: seen,
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
