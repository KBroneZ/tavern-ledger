//! Why a combat's result differed from its odds (T-346, D-127): for one
//! combat with its boards, its real result and, when the game saved it, the
//! attack-by-attack replay, the odds the engine gives on those boards and,
//! when their most likely result is not the one that happened, its cause
//! with the evidence. The app's recap, the website's game page and
//! `combat-validate --summary` all use this one function.
//!
//! The rules, in order (D-127):
//! 1. The most likely result happened: [`Cause::Matched`].
//! 2. No replay to check against (none saved, unreadable, or one that stops
//!    early): [`Cause::NoReplay`], never a guess.
//! 3. The replay follows the game move for move to the end: the engine
//!    plays this combat as the game did, so the odds missed on the game's
//!    random choices only: [`Cause::Chance`].
//! 4. Else the replay parts from the game at some attack, and the first
//!    of these that applies is the cause: a board brings something the log
//!    does not show (a game-long value, a minion from the hand, a Dark Gift
//!    it does not name: [`Cause::MissingInputs`]); a card or Dark Gift the
//!    engine plays approximately ([`Cause::ApproximateCard`]); else a rule
//!    of the engine differs from the game's ([`Cause::RuleDifference`]).
//!
//! Boards are taken at the first attack, after start of combat (as the
//! harness does), so the odds are the ones for the boards as they fought.

use crate::cards::{self, Fidelity};
use crate::combat::replay::{replay_teams, Difference, LoggedStep, Rule, Span};
use crate::combat::CombatResult;
use crate::model::Team;
use crate::score::Prediction;
use crate::trials::{simulate_teams, Coverage, Options, Verdict};
use crate::{cards_round1 as r1, cards_round2 as r2, cards_round3 as r3};
use crate::{cards_round4 as r4, cards_round5 as r5};

/// Something a board brings into combat that the log does not show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unseen {
    /// What the side's Blood Gems give beyond +1/+1.
    BloodGems,
    /// What the side's Tavern spells give beyond their text.
    Spells,
    /// The Eternal Knights that died this game.
    Knights,
    /// The Beetles' game-long buffs.
    Beetles,
    /// A minion summoned from the hand.
    Hand,
    /// What a card gained or grew to earlier in the game.
    Earlier,
    /// A Dark Gift the log does not name.
    Gift,
}

/// One thing not seen, and the card that needs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotSeen {
    pub what: Unseen,
    pub card: String,
}

/// Why a combat has no replay to check the odds against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoReplay {
    /// The game did not play the combat back, or the record is older.
    NotSaved,
    /// The saved replay cannot be rebuilt into the combat's boards.
    Unreadable,
    /// The replay stops before the end (an attack without a target).
    Stops,
}

/// How well the engine knows a combat's cards, said when there is no
/// replay to check: a card it does not know, one it plays approximately
/// (or a Dark Gift the log does not name), or every one exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cards {
    NotModelled,
    Approximate,
    Exact,
}

/// Why the result differed from the odds, or that it did not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// The most likely result happened.
    Matched,
    /// The replay follows the game to the end: the dice.
    Chance {
        attacks: usize,
    },
    /// A board brings something the log does not show; `parted` is where
    /// the replay leaves the game.
    MissingInputs {
        inputs: Vec<NotSeen>,
        parted: Difference,
    },
    /// Cards or Dark Gifts the engine plays approximately (ids).
    ApproximateCard {
        cards: Vec<String>,
        parted: Difference,
    },
    /// The engine's rules part from the game's at this step.
    RuleDifference(Difference),
    NoReplay {
        why: NoReplay,
        cards: Cards,
    },
}

impl Cause {
    /// The cause's id, shared with the accuracy export and the website's
    /// cause list (one vocabulary): `chance`, `replay_<rule>` for the first
    /// rule that differs, `missing_inputs`, `approximate_card`, and
    /// `no_replay_<cards>` without a replay. `matched` is not a miss.
    pub fn id(&self) -> &'static str {
        match self {
            Cause::Matched => "matched",
            Cause::Chance { .. } => "chance",
            Cause::MissingInputs { .. } => "missing_inputs",
            Cause::ApproximateCard { .. } => "approximate_card",
            Cause::RuleDifference(d) => rule_id(d.rule),
            Cause::NoReplay {
                why: NoReplay::Stops,
                ..
            } => rule_id(Rule::NoDefender),
            Cause::NoReplay { cards, .. } => no_replay_id(*cards),
        }
    }
}

/// The id of a miss whose replay parts from the game at `rule`.
pub fn rule_id(rule: Rule) -> &'static str {
    match rule {
        Rule::FirstAttacker => "replay_first_attacker",
        Rule::AfterTeammate => "replay_after_teammate",
        Rule::AttackOrder => "replay_attack_order",
        Rule::Target => "replay_target",
        Rule::Minions => "replay_minions",
        Rule::Stats => "replay_stats",
        Rule::End => "replay_end",
        Rule::NoDefender => "replay_no_defender",
    }
}

/// The id of a miss with no replay, by how well the engine knows its cards.
pub fn no_replay_id(cards: Cards) -> &'static str {
    match cards {
        Cards::NotModelled => "no_replay_not_modelled",
        Cards::Approximate => "no_replay_approximate",
        Cards::Exact => "no_replay_exact",
    }
}

/// What the game saved of the combat's replay.
#[derive(Clone, Copy, Debug)]
pub enum Replayable<'a> {
    NotSaved,
    Unreadable,
    /// The attacks with the boards before each, from the first attack.
    Steps(&'a [LoggedStep]),
}

/// One combat to explain.
#[derive(Clone, Copy, Debug)]
pub struct CombatFacts<'a> {
    /// Each team's boards in the order they fought (one in Solo).
    pub player: &'a Team,
    pub opponent: &'a Team,
    pub result: CombatResult,
    pub replay: Replayable<'a>,
    /// Cards the game marks as gifted with no gift the log names.
    pub unknown_gifts: &'a [String],
}

/// What the engine says about the combat's boards.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    Odds(Prediction),
    /// A card or Dark Gift it does not know: no number, by design.
    NotModelled(Vec<String>),
    /// The boards are not ones the engine accepts.
    BadBoard,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Explained {
    pub said: Said,
    /// `None` when there are no odds.
    pub cause: Option<Cause>,
}

/// The odds for the combat's boards and, against its real result, why
/// they missed (see the module's rules).
pub fn explain(c: &CombatFacts, options: &Options) -> Explained {
    let opts = Options {
        start_of_combat: false,
        allow_unmodelled: false,
        ..*options
    };
    let odds = match simulate_teams(c.player, c.opponent, &opts) {
        Ok(Verdict::Odds(o)) => o,
        Ok(Verdict::NotModelled(cov)) => {
            return Explained {
                said: Said::NotModelled(cov.not_modelled),
                cause: None,
            }
        }
        Err(_) => {
            return Explained {
                said: Said::BadBoard,
                cause: None,
            }
        }
    };
    let prediction = Prediction {
        won: odds.share(CombatResult::Won),
        tied: odds.share(CombatResult::Tie),
        lost: odds.share(CombatResult::Lost),
    };
    let cause = if prediction.top() == c.result {
        Cause::Matched
    } else {
        missed(c, &odds.coverage, opts.seed)
    };
    Explained {
        said: Said::Odds(prediction),
        cause: Some(cause),
    }
}

fn missed(c: &CombatFacts, coverage: &Coverage, seed: u64) -> Cause {
    let cards = cards_of(coverage, !c.unknown_gifts.is_empty());
    let steps = match c.replay {
        Replayable::NotSaved => {
            return Cause::NoReplay {
                why: NoReplay::NotSaved,
                cards,
            }
        }
        Replayable::Unreadable | Replayable::Steps([]) => {
            return Cause::NoReplay {
                why: NoReplay::Unreadable,
                cards,
            }
        }
        Replayable::Steps(s) => s,
    };
    let replay = replay_teams(
        c.player,
        c.opponent,
        steps,
        Span::Whole(Some(c.result)),
        seed,
    );
    match replay.difference {
        None => Cause::Chance {
            attacks: replay.attacks,
        },
        Some(d) => parted(d, not_seen(c.player, c.opponent, c.unknown_gifts), coverage),
    }
}

/// How well the engine knows the cards of a combat with this coverage
/// (`unknown_gift`: a Dark Gift the log does not name is on a board).
pub fn cards_of(coverage: &Coverage, unknown_gift: bool) -> Cards {
    if !coverage.not_modelled.is_empty() {
        Cards::NotModelled
    } else if !coverage.approximate.is_empty() || unknown_gift {
        Cards::Approximate
    } else {
        Cards::Exact
    }
}

/// The cause of a miss whose replay parts from the game at `d` (rule 4 of
/// the module): what the log does not show (`inputs`), else the cards the
/// engine does not know or plays approximately, else the rule. A replay
/// that stops at an attack without a target checks nothing.
pub fn parted(d: Difference, inputs: Vec<NotSeen>, coverage: &Coverage) -> Cause {
    if d.rule == Rule::NoDefender {
        return Cause::NoReplay {
            why: NoReplay::Stops,
            cards: cards_of(coverage, inputs.iter().any(|n| n.what == Unseen::Gift)),
        };
    }
    if !inputs.is_empty() {
        return Cause::MissingInputs { inputs, parted: d };
    }
    // Coverage names each id once.
    let cards: Vec<String> = coverage
        .not_modelled
        .iter()
        .cloned()
        .chain(coverage.approximate.iter().map(|(id, _)| id.clone()))
        .collect();
    if !cards.is_empty() {
        return Cause::ApproximateCard { cards, parted: d };
    }
    Cause::RuleDifference(d)
}

/// Which approximation reasons are something the log does not show. A
/// game-long value the board brings (above zero) is seen.
fn unseen_of(why: &str, values: &crate::model::GameValues) -> Option<Unseen> {
    let known = |v: bool| (!v).then_some(());
    match why {
        w if w == r1::BLOOD_GEM => known(values.blood_gem != (0, 0)).map(|_| Unseen::BloodGems),
        w if w == r2::SPELL => known(values.spell != (0, 0)).map(|_| Unseen::Spells),
        w if w == r2::KNIGHTS => known(values.knights_dead > 0).map(|_| Unseen::Knights),
        w if w == r4::BEETLE_BUFFS => known(values.beetle != (0, 0)).map(|_| Unseen::Beetles),
        w if w == r5::FROM_HAND => Some(Unseen::Hand),
        w if [
            r1::GAME_BUFFS,
            r1::SCALES,
            r3::IMPROVES,
            r4::EARLIER_GAINS,
            r4::PROTOSS,
            r5::EARLIER_GAINS,
            r5::COPIED_EARLIER,
        ]
        .contains(&w) =>
        {
            Some(Unseen::Earlier)
        }
        _ => None,
    }
}

/// Everything on both teams' boards that the log does not show;
/// `unknown_gifts`: cards marked gifted with no gift the log names.
pub fn not_seen(player: &Team, opponent: &Team, unknown_gifts: &[String]) -> Vec<NotSeen> {
    let mut out: Vec<NotSeen> = Vec::new();
    let mut add = |what: Unseen, card: &str| {
        if !out.iter().any(|n| n.what == what && n.card == card) {
            out.push(NotSeen {
                what,
                card: card.to_string(),
            });
        }
    };
    for band in player.boards().chain(opponent.boards()) {
        for m in &band.minions {
            let rules = cards::rules(&m.card_id);
            if let Some(Fidelity::Approximate(why)) = rules.map(|r| r.fidelity) {
                if let Some(what) = unseen_of(why, &band.values) {
                    add(what, &m.card_id);
                }
            }
        }
    }
    for card in unknown_gifts {
        add(Unseen::Gift, card);
    }
    out
}

#[cfg(test)]
#[path = "why_tests.rs"]
mod tests;
