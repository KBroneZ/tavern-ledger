//! Replays a logged combat attack by attack (T-603b): at each attack the
//! engine says who attacks next and whom it may hit, then plays the attack
//! the log has, and the boards are compared with the log's before the next
//! one. The first step where they part names the rule that differs.
//!
//! Random effects inside an attack (a random buff, a random neighbour) are
//! still rolled by the engine, so a "stats differ" step can be chance rather
//! than a wrong rule; the detail says what differs.

use super::{Combat, Prepared, Unit};
use crate::combat::CombatResult;
use crate::model::{Keywords, Minion, Team, Warband};
use crate::rng::Rng;

/// One logged attack: who attacked (side 0 is the player, 1 the opponent;
/// index from the left), whom (an index on the other side; `None` when the
/// log does not say) and both boards just before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedStep {
    pub attacker: (usize, usize),
    pub defender: Option<usize>,
    pub before: [Vec<Minion>; 2],
}

/// The rule a replay found to differ first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// The side that attacks first (more minions, else a coin flip).
    FirstAttacker,
    /// Duos: the side that attacks first once a teammate's board comes in
    /// (decided again as at the start).
    AfterTeammate,
    /// Which side attacks next (sides alternate; a side that cannot attack
    /// passes) or which of its minions (the pointer, Windfury).
    AttackOrder,
    /// The log's target is not one the engine allows (Taunt, Stealth).
    Target,
    /// Minions differ after an attack: deaths, Deathrattles, Reborn,
    /// summons or their places.
    Minions,
    /// The same minions with other Attack, Health or keywords.
    Stats,
    /// The log ends the combat (or goes on) where the engine does not, or
    /// the result differs.
    End,
    /// The log does not name the defender: the replay stops there.
    NoDefender,
}

impl Rule {
    pub fn label(self) -> &'static str {
        match self {
            Rule::FirstAttacker => "who attacks first",
            Rule::AfterTeammate => "who attacks after a teammate's board comes in",
            Rule::AttackOrder => "attack order",
            Rule::Target => "target choice (Taunt, Stealth)",
            Rule::Minions => "deaths, summons or their places",
            Rule::Stats => "stats or keywords after an attack",
            Rule::End => "end of combat or result",
            Rule::NoDefender => "defender not in the log",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// The logged attack (1-based) before or at which the engine parts.
    pub attack: usize,
    pub rule: Rule,
    /// Card ids, positions and numbers only.
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replay {
    pub attacks: usize,
    /// Attacks the engine played as the log did before the first difference.
    pub matched: usize,
    pub difference: Option<Difference>,
    /// Duos: teammates' boards that came in before the first difference.
    pub boards_in: usize,
}

/// Keywords compared between the engine and the log: the ones both track.
const COMPARED: [(Keywords, &str); 5] = [
    (Keywords::TAUNT, "taunt"),
    (Keywords::DIVINE_SHIELD, "divine shield"),
    (Keywords::REBORN, "reborn"),
    (Keywords::VENOMOUS, "venomous"),
    (Keywords::WINDFURY, "windfury"),
];

fn compared(k: Keywords) -> Keywords {
    COMPARED
        .iter()
        .filter(|(c, _)| k.contains(*c))
        .fold(Keywords::NONE, |all, (c, _)| all.union(*c))
}

fn keyword_names(k: Keywords) -> String {
    let names: Vec<&str> = COMPARED
        .iter()
        .filter(|(c, _)| k.contains(*c))
        .map(|(_, name)| *name)
        .collect();
    names.join(", ")
}

/// How much of a logged combat to replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Span {
    /// A Solo combat: every attack, then the end against the game's result.
    Whole(Option<CombatResult>),
    /// The first leg of a Duos combat: until one of the two boards is gone.
    /// For a combat whose teammate's board the log does not show.
    FirstLeg,
}

/// Replays `steps` from the boards of the first one (`player` and
/// `opponent`, with card facts).
pub fn replay(
    player: &Warband,
    opponent: &Warband,
    steps: &[LoggedStep],
    span: Span,
    seed: u64,
) -> Replay {
    let prepared = Prepared::new(player, opponent, false);
    replay_prepared(&prepared, steps, span, seed)
}

/// Replays a whole Duos combat (T-608): when a board is gone, the team's
/// second board comes in as the engine plays it, and the attacks go on.
pub fn replay_teams(
    player: &Team,
    opponent: &Team,
    steps: &[LoggedStep],
    span: Span,
    seed: u64,
) -> Replay {
    let prepared = Prepared::teams(player, opponent, false);
    replay_prepared(&prepared, steps, span, seed)
}

fn replay_prepared(prepared: &Prepared, steps: &[LoggedStep], span: Span, seed: u64) -> Replay {
    let mut rng = Rng::new(seed);
    let mut c = Combat::new(prepared, &mut rng);
    let mut out = Replay {
        attacks: steps.len(),
        matched: 0,
        difference: None,
        boards_in: 0,
    };
    let mut turn = Turn::default();
    if let Some(step) = steps.first() {
        // Counted before start of combat, as the engine does.
        turn.first = c.first_by_count();
        turn.pending = c.pending_immediate(turn.first.unwrap_or(step.attacker.0), [true, true]);
    }
    for (k, step) in steps.iter().enumerate() {
        if let Err(d) = c.check_step(k, step, &mut turn) {
            out.difference = Some(d);
            return out;
        }
        out.matched = k + 1;
        if span == Span::FirstLeg && c.finished() {
            return out;
        }
        let entered = if c.finished() {
            c.enter_waiting()
        } else {
            [false; 2]
        };
        if entered != [false, false] {
            out.boards_in += entered.iter().filter(|e| **e).count();
            // Who attacks next is decided again, as at the start.
            let first = c.first_by_count();
            let side = first
                .or(steps.get(k + 1).map(|s| s.attacker.0))
                .unwrap_or(0);
            turn = Turn {
                first,
                pending: c.pending_immediate(side, entered),
                after_teammate: true,
                ..Turn::default()
            };
        }
    }
    out.difference = match span {
        Span::Whole(result) => c.check_end(steps.len(), result),
        Span::FirstLeg => None,
    };
    out
}

/// Whose turn it is during a replay.
#[derive(Clone, Debug, Default)]
struct Turn {
    /// The attacker still swinging: side, uid, swings left.
    current: Option<(usize, u32, u8)>,
    /// The side of the last attack.
    last: Option<usize>,
    /// A teammate's board has just come in.
    after_teammate: bool,
    /// The side the minion count gives the first attack, counted before
    /// start of combat (`None`: equal counts).
    first: Option<usize>,
    /// Start-of-combat attacks still to come, in order: side and uid, one
    /// entry per swing. They are not the side's turn.
    pending: Vec<(usize, u32)>,
}

fn diff(attack: usize, rule: Rule, detail: String) -> Difference {
    Difference {
        attack,
        rule,
        detail,
    }
}

fn side_word(side: usize) -> &'static str {
    if side == 0 {
        "player"
    } else {
        "opponent"
    }
}

impl Combat<'_> {
    /// Checks one logged attack against the engine and plays it.
    fn check_step(
        &mut self,
        k: usize,
        step: &LoggedStep,
        turn: &mut Turn,
    ) -> Result<(), Difference> {
        let n = k + 1;
        self.compare_boards(n, &step.before)?;
        let (side, ix) = step.attacker;
        self.drop_spent(turn);
        let immediate = !turn.pending.is_empty();
        let expected = self.expected_attacker(turn, side);
        let Some((want_side, want_ix)) = expected else {
            return Err(diff(
                n,
                Rule::End,
                "the engine has no attacker left; the log attacks again".into(),
            ));
        };
        if side != want_side {
            let rule = if k == 0 {
                Rule::FirstAttacker
            } else if turn.after_teammate {
                Rule::AfterTeammate
            } else {
                Rule::AttackOrder
            };
            return Err(diff(
                n,
                rule,
                format!(
                    "engine: the {} attacks; log: the {}",
                    side_word(want_side),
                    side_word(side)
                ),
            ));
        }
        if ix != want_ix {
            let unit = &self.sides[side].units;
            let card = |i: usize| {
                unit.get(i)
                    .map_or("-", |u| self.p.cards[u.card as usize].id.as_str())
            };
            return Err(diff(
                n,
                Rule::AttackOrder,
                format!(
                    "{}: engine attacks with position {} ({}), log with position {} ({})",
                    side_word(side),
                    want_ix + 1,
                    card(want_ix),
                    ix + 1,
                    card(ix)
                ),
            ));
        }
        let uid = self.sides[side].units[ix].uid;
        let swings = match turn.current {
            _ if immediate => 1,
            Some((s, u, left)) if s == side && u == uid => left,
            _ => {
                self.sides[side].next = ix + 1;
                swings_of(&self.sides[side].units[ix])
            }
        };
        let Some(target_ix) = step.defender else {
            return Err(diff(n, Rule::NoDefender, String::new()));
        };
        let foe = 1 - side;
        let Some(target) = self.sides[foe].units.get(target_ix).map(|u| u.uid) else {
            return Err(diff(
                n,
                Rule::Target,
                format!("log target position {} is not on the board", target_ix + 1),
            ));
        };
        if !self.target_pool(foe).contains(&target) {
            let card = self.p.cards[self.sides[foe].units[target_ix].card as usize]
                .id
                .as_str();
            return Err(diff(
                n,
                Rule::Target,
                format!(
                    "log hits {} position {} ({card}), which the engine's rules do not allow",
                    side_word(foe),
                    target_ix + 1
                ),
            ));
        }
        self.forced_target = Some(target);
        self.swing(side, uid);
        self.forced_target = None;
        if immediate {
            turn.pending.remove(0);
            return Ok(());
        }
        let left = swings.saturating_sub(1);
        let still = self.unit(side, uid).is_some_and(|u| u.attack > 0);
        turn.current = (left > 0 && still && !self.finished()).then_some((side, uid, left));
        turn.last = Some(side);
        turn.after_teammate = false;
        Ok(())
    }

    /// The attacker the engine's rules give next: the same minion while it
    /// has swings left, else the other side's next attacker (a side that
    /// cannot attack passes). The first attack: the side with more minions,
    /// else a coin flip, which `logged` (the log's side) is taken to have won.
    fn expected_attacker(&self, turn: &Turn, logged: usize) -> Option<(usize, usize)> {
        if let Some(&(side, uid)) = turn.pending.first() {
            return self.sides[side].pos(uid).map(|ix| (side, ix));
        }
        if let Some((side, uid, _)) = turn.current {
            return self.sides[side].pos(uid).map(|ix| (side, ix));
        }
        let first = match turn.last {
            Some(last) => 1 - last,
            None => turn.first.unwrap_or(logged),
        };
        let order = [first, 1 - first];
        order
            .into_iter()
            .find_map(|s| self.attacker_index(s).map(|ix| (s, ix)))
    }

    /// The start-of-combat attacks of the boards coming in (`sides`), in the
    /// engine's order (see `Combat::attack_immediately`).
    fn pending_immediate(&self, first: usize, sides: [bool; 2]) -> Vec<(usize, u32)> {
        let mut out = Vec::new();
        for side in [first, 1 - first].into_iter().filter(|s| sides[*s]) {
            for (uid, times) in self.immediate_attackers(side) {
                out.extend(std::iter::repeat_n((side, uid), usize::from(times)));
            }
        }
        out
    }

    /// Drops the start-of-combat attacks the engine would not make: their
    /// minion is gone or has no Attack, or the combat is over.
    fn drop_spent(&self, turn: &mut Turn) {
        while let Some(&(side, uid)) = turn.pending.first() {
            let can = !self.finished() && self.unit(side, uid).is_some_and(|u| u.attack > 0);
            if can {
                return;
            }
            turn.pending.retain(|p| *p != (side, uid));
        }
    }

    fn compare_boards(&self, n: usize, logged: &[Vec<Minion>; 2]) -> Result<(), Difference> {
        let id = |u: &Unit| self.p.cards[u.card as usize].id.as_str();
        for (side, log) in logged.iter().enumerate() {
            let engine_ids: Vec<&str> = self.sides[side].units.iter().map(id).collect();
            let log_ids: Vec<&str> = log.iter().map(|m| m.card_id.as_str()).collect();
            if engine_ids != log_ids {
                return Err(diff(
                    n,
                    Rule::Minions,
                    format!(
                        "{} board: engine [{}], log [{}]",
                        side_word(side),
                        engine_ids.join(", "),
                        log_ids.join(", ")
                    ),
                ));
            }
        }
        // The same minions on both sides: now their numbers.
        for (side, log) in logged.iter().enumerate() {
            for (i, (u, m)) in self.sides[side].units.iter().zip(log).enumerate() {
                // A minion that counts as Taunt (while shielded) shows the tag.
                let mut engine_kw = u.kw;
                if self.is_taunt(u) {
                    engine_kw = engine_kw.union(Keywords::TAUNT);
                }
                let (engine_kw, log_kw) = (compared(engine_kw), compared(m.keywords));
                if u.attack != m.attack || u.health != m.health || engine_kw != log_kw {
                    return Err(diff(
                        n,
                        Rule::Stats,
                        format!(
                            "{} position {} ({}): engine {}/{} [{}], log {}/{} [{}]",
                            side_word(side),
                            i + 1,
                            m.card_id,
                            u.attack,
                            u.health,
                            keyword_names(engine_kw),
                            m.attack,
                            m.health,
                            keyword_names(log_kw)
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// After the last logged attack the engine must be done too, with the
    /// game's result.
    fn check_end(&mut self, attacks: usize, result: Option<CombatResult>) -> Option<Difference> {
        let n = attacks.max(1);
        let can_go_on = !self.finished()
            && (self.attacker_index(0).is_some() || self.attacker_index(1).is_some());
        if can_go_on {
            return Some(diff(
                n,
                Rule::End,
                "the log stops; the engine would attack again".into(),
            ));
        }
        let outcome = self.outcome(false).result;
        match result {
            Some(r) if r != outcome => Some(diff(
                n,
                Rule::End,
                format!("engine ends {outcome:?}, the game says {r:?}"),
            )),
            _ => None,
        }
    }
}

fn swings_of(u: &Unit) -> u8 {
    if u.kw.contains(Keywords::MEGA_WINDFURY) {
        4
    } else if u.kw.contains(Keywords::WINDFURY) {
        2
    } else {
        1
    }
}

#[cfg(test)]
#[path = "replay_tests.rs"]
mod tests;
