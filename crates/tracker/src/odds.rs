//! Combat odds for the overlay (T-605, D-084): the player's board now, in
//! the shop, against the board of the next opponent the player last saw in a
//! combat, played many times by the simulator (D-081).
//!
//! Red line (D-004): only boards the player saw. The opponent's board is the
//! one that entered play in a combat the game has shown, never a later one;
//! the overlay says how old it is. When the odds cannot be trusted (Duos, no
//! board seen yet, a card the simulator does not know, no card data) the
//! overlay gets the reason, never a number.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use bg_parser::report::{GameReport, LobbySlot, Minion as LogMinion, Side};
use combat_sim::cards::has_start_of_combat;
use combat_sim::{simulate, BaseCard, CombatResult, Keywords, Minion, Odds, Options, Tribes, Verdict, Warband};
use serde::Serialize;

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
    pub own: Vec<LogMinion>,
    pub opponent: Vec<LogMinion>,
    pub own_tier: Option<i64>,
    /// The opponent's tavern tier now (it is shown on the leaderboard).
    pub opponent_tier: Option<i64>,
    pub damage_cap: Option<i64>,
}

/// Why there are no odds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gap {
    /// Duos combats are not in the simulator yet (T-608).
    Duos,
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
    pub seen_round: Option<i64>,
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

/// The newest board of `seat` that entered play in a combat (D-010), with
/// its combat number.
fn last_board(report: &GameReport, seat: i64) -> Option<(&[LogMinion], i64)> {
    report.rounds.iter().rev().find_map(|round| {
        round
            .entries
            .iter()
            .filter(|e| e.side == Side::Opponent && e.player_id == Some(seat))
            .find_map(|e| e.board.as_deref())
            .map(|b| (b, round.number))
    })
}

/// The input for the odds now, or why there are none (with the seat when
/// known). `own_now` is the player's board in the shop; `next` the seat the
/// game marks as the next opponent.
pub fn odds_input(
    report: &GameReport,
    own_now: Option<&[LogMinion]>,
    next: Option<i64>,
    lobby: &[LobbySlot],
    damage_cap: Option<i64>,
) -> Result<OddsInput, LiveOdds> {
    if report.game_type.as_deref() == Some("GT_BATTLEGROUNDS_DUO") {
        return Err(LiveOdds::gap(Gap::Duos, next, None));
    }
    let Some(seat) = next else {
        return Err(LiveOdds::gap(Gap::NoNextOpponent, None, None));
    };
    let Some(own) = own_now else {
        return Err(LiveOdds::gap(Gap::NotInShop, Some(seat), None));
    };
    let Some((opponent, seen_round)) = last_board(report, seat) else {
        return Err(LiveOdds::gap(Gap::NoBoardSeen, Some(seat), None));
    };
    let tier = |pid: Option<i64>| {
        lobby
            .iter()
            .find(|s| Some(s.player_id) == pid)
            .and_then(|s| s.tech_level)
    };
    Ok(OddsInput {
        opponent_seat: seat,
        seen_round,
        own: own.to_vec(),
        opponent: opponent.to_vec(),
        own_tier: tier(report.local_player_id),
        opponent_tier: tier(Some(seat)),
        damage_cap,
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
        card_id: id,
    })
}

fn warband(
    board: &[LogMinion],
    tier: Option<i64>,
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
    }
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
    let seen = Some(input.seen_round);
    let mut missing = Vec::new();
    let player = warband(&input.own, input.own_tier, facts, &mut missing);
    let opponent = warband(&input.opponent, input.opponent_tier, facts, &mut missing);
    if !missing.is_empty() {
        return LiveOdds {
            missing,
            ..LiveOdds::gap(Gap::NoCardData, seat, seen)
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
        match simulate(&player, &opponent, &options) {
            Ok(Verdict::Odds(o)) => match total.as_mut() {
                Some(t) => t.merge(&o),
                None => total = Some(o),
            },
            Ok(Verdict::NotModelled(c)) => {
                return LiveOdds {
                    missing: c.not_modelled,
                    approximate: c.approximate.into_iter().map(|(id, _)| id).collect(),
                    ..LiveOdds::gap(Gap::NotModelled, seat, seen)
                };
            }
            Err(_) => return LiveOdds::gap(Gap::BadBoard, seat, seen),
        }
        batch += 1;
        if started.elapsed() >= budget {
            break;
        }
    }
    let Some(o) = total else {
        return LiveOdds::gap(Gap::BadBoard, seat, seen);
    };
    let start_of_combat = player
        .minions
        .iter()
        .chain(&opponent.minions)
        .any(|m| has_start_of_combat(&m.card_id));
    let known = o.damage_unknown == 0;
    LiveOdds {
        seat,
        gap: None,
        seen_round: seen,
        won: o.share(CombatResult::Won),
        tied: o.share(CombatResult::Tie),
        lost: o.share(CombatResult::Lost),
        trials: o.trials,
        dealt: spread(&o.damage_dealt, input.damage_cap).filter(|_| known),
        taken: spread(&o.damage_taken, input.damage_cap).filter(|_| known),
        approximate: o.coverage.approximate.into_iter().map(|(id, _)| id).collect(),
        missing: Vec::new(),
        start_of_combat,
    }
}

#[cfg(test)]
#[path = "odds_tests.rs"]
mod tests;
