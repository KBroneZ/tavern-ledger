//! Combat replay (T-215, D-099): what happened in each combat the game plays
//! back on screen, in the log's order. Attacks, damage, Divine Shields
//! popped, stat changes, deaths, summons, the triggers the log names
//! (start of combat, deathrattle, avenge, rally), a Duos teammate's board
//! coming in and the hero damage at the end.
//!
//! Only the two sides of the combat on screen are read: the local player's
//! and the opponent's (the game's "dummy" player). A combat the game does not
//! play back has no attack in the log and gets no replay ("not shown"); the
//! `SETASIDE` copies of a Duos teammate's hidden combat are never read
//! (D-010, D-054). Minions are kept by card id, side, board slot and golden
//! flag: nothing about other players beyond what the screen showed.
//!
//! This is what the log says happened, never a simulation.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::report::{Minion, Side};
use crate::state::{Board, Entity};

/// Minions kept per combat: both boards and every summon, far more than a
/// combat has (62 on the user's longest), so a strange log cannot grow the
/// record.
pub const MAX_REPLAY_MINIONS: usize = 200;
/// Events kept per combat: 466 on the user's longest. Past it the replay
/// stops and says it is incomplete.
pub const MAX_REPLAY_EVENTS: usize = 2000;

/// A minion as it enters the replay: on a board as the combat starts, or
/// when it is summoned. Its index in [`CombatReplay::minions`] is how the
/// events name it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReplayMinion {
    pub side: Side,
    pub card_id: Option<String>,
    pub atk: i64,
    pub health: i64,
    /// Board slot when it entered (1 is the left-most).
    pub position: i64,
    pub golden: bool,
    pub keywords: Vec<&'static str>,
}

impl ReplayMinion {
    fn new(side: Side, m: Minion) -> Self {
        ReplayMinion {
            side,
            card_id: m.card_id,
            atk: m.atk,
            health: m.health,
            position: m.position,
            golden: m.golden,
            keywords: m.keywords,
        }
    }
}

/// What a trigger the log names was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// Before the first attack, from a minion, hero power or trinket.
    StartOfCombat,
    Deathrattle,
    Avenge,
    Rally,
}

/// One thing that happened in a combat. Minions are named by their index in
/// [`CombatReplay::minions`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatEvent {
    /// `defender` is None when the log does not say whom it hit.
    Attack {
        attacker: usize,
        defender: Option<usize>,
    },
    /// Damage taken; `health` is what is left.
    Damage {
        minion: usize,
        amount: i64,
        health: i64,
    },
    /// Divine Shield lost (`on` false) or given.
    Shield {
        minion: usize,
        on: bool,
    },
    /// Attack or health changed other than by damage (a buff, a heal).
    Stats {
        minion: usize,
        atk: i64,
        health: i64,
    },
    Death {
        minion: usize,
    },
    /// The minion enters the board at its `position`.
    Summon {
        minion: usize,
    },
    /// `source` is the minion that triggered, when it is one of the
    /// replay's; a hero power or trinket has only its card.
    Trigger {
        trigger: TriggerKind,
        side: Side,
        card_id: String,
        source: Option<usize>,
    },
    /// Damage to the hero of `side` as the combat ends (the winner's hero
    /// attack).
    HeroDamage {
        side: Side,
        amount: i64,
    },
    /// Duos: the teammate's board comes in on `side`.
    BoardIn {
        side: Side,
        hero: Option<String>,
    },
}

/// One combat, attack by attack. The first `start` minions are both boards
/// as the combat starts (own side first, each in board order).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatReplay {
    pub minions: Vec<ReplayMinion>,
    pub start: usize,
    pub events: Vec<CombatEvent>,
    /// False when the log stops before the combat ends, or the combat had
    /// more events than are kept.
    pub complete: bool,
}

/// The player ids of the two sides on screen.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sides {
    pub own: Option<i64>,
    pub opponent: Option<i64>,
}

impl Sides {
    fn of(&self, e: &Entity) -> Option<Side> {
        let controller = Some(e.int("CONTROLLER"));
        if controller == self.own {
            Some(Side::Own)
        } else if controller == self.opponent {
            Some(Side::Opponent)
        } else {
            None
        }
    }
}

const TRIGGER_CARD_TYPES: [&str; 3] = ["MINION", "HERO_POWER", "BATTLEGROUND_TRINKET"];

/// A combat being read.
#[derive(Debug)]
struct Run {
    replay: CombatReplay,
    /// Entity id -> index in `replay.minions`.
    refs: BTreeMap<i64, usize>,
    /// Indexes of the minions on the board now.
    alive: BTreeSet<usize>,
    attacked: bool,
    /// The newest attack while its defender is not known yet.
    open_attack: Option<usize>,
    /// Duos: the side whose next board is coming in, until an attack. Its
    /// minions may be the player's own, made before this combat.
    incoming: Option<Side>,
    /// The hero damage is in: what follows is the game cleaning up.
    ended: bool,
}

/// The combat of the turn being read.
#[derive(Debug)]
struct Live {
    round: i64,
    /// Entities created during this combat turn: a minion entering the board
    /// is a summon only if it is one of them (the shop's minions come back
    /// as the combat ends), or a Duos board is coming in on its side.
    created: BTreeSet<i64>,
    run: Option<Run>,
}

/// Collects one game's replays as its packets are read.
#[derive(Debug, Default)]
pub struct ReplayTracker {
    /// Round -> its replay, for every combat with at least one attack.
    done: BTreeMap<i64, CombatReplay>,
    live: Option<Live>,
}

impl ReplayTracker {
    /// The replay of `round`: finished, or the one being read (incomplete).
    pub fn replay(&self, round: i64) -> Option<CombatReplay> {
        if let Some(r) = self.done.get(&round) {
            return Some(r.clone());
        }
        let live = self.live.as_ref().filter(|l| l.round == round)?;
        let run = live.run.as_ref().filter(|r| r.attacked)?;
        let mut replay = run.replay.clone();
        replay.complete = false;
        Some(replay)
    }

    /// A new turn: the combat before it is over; a combat turn starts one.
    pub fn on_turn(&mut self, turn: i64) {
        self.finish();
        if turn >= 2 && turn % 2 == 0 {
            self.live = Some(Live {
                round: turn / 2,
                created: BTreeSet::new(),
                run: None,
            });
        }
    }

    /// The game is over: so is its last combat.
    pub fn on_game_over(&mut self) {
        self.finish();
    }

    fn finish(&mut self) {
        let Some(live) = self.live.take() else { return };
        let Some(mut run) = live.run.filter(|r| r.attacked) else {
            return;
        };
        run.drop_heroes_put_back();
        self.done.insert(live.round, run.replay);
    }

    /// An entity was created (`FULL_ENTITY`).
    pub fn on_created(&mut self, id: i64, board: &Board, sides: Sides) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        live.created.insert(id);
        live.summon(id, board, sides);
    }

    /// A hidden entity was shown or changed (`SHOW_ENTITY`,
    /// `CHANGE_ENTITY`): a summon can arrive this way.
    pub fn on_shown(&mut self, id: i64, board: &Board, sides: Sides) {
        if let Some(live) = self.live.as_mut() {
            live.summon(id, board, sides);
        }
    }

    /// A block starts: an attack, or a trigger the log may name.
    pub fn on_block(
        &mut self,
        kind: &str,
        entity: Option<i64>,
        keyword: Option<&str>,
        board: &Board,
        sides: Sides,
    ) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let Some(e) = entity.and_then(|id| board.get(id)) else {
            return;
        };
        match kind {
            "ATTACK" => live.attack(e, board, sides),
            "TRIGGER" => live.trigger(e, keyword, board, sides),
            _ => {}
        }
    }

    /// The game names the defender of the open attack.
    pub fn on_defender(&mut self, id: i64) {
        if let Some(run) = self.run_mut() {
            run.defender(id);
        }
    }

    /// A tag changed on entity `id` (already set on the board).
    pub fn on_tag(&mut self, id: i64, tag: &str, old: Option<i64>, board: &Board, sides: Sides) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        if tag == "ZONE" || tag == "CONTROLLER" {
            live.summon(id, board, sides);
        }
        let Some(run) = live.run.as_mut().filter(|r| !r.ended) else {
            return;
        };
        let Some(e) = board.get(id) else { return };
        let Some(&minion) = run.refs.get(&id) else {
            return;
        };
        if !run.alive.contains(&minion) {
            return;
        }
        match tag {
            "ZONE" if e.is("ZONE", "GRAVEYARD") => {
                run.alive.remove(&minion);
                run.push(CombatEvent::Death { minion });
            }
            "DAMAGE" => {
                let health = health_of(e);
                let amount = e.int("DAMAGE") - old.unwrap_or(0);
                if amount > 0 {
                    run.defender(id);
                    run.push(CombatEvent::Damage {
                        minion,
                        amount,
                        health,
                    });
                } else {
                    run.stats(minion, e.int("ATK"), health);
                }
            }
            "ATK" | "HEALTH" => run.stats(minion, e.int("ATK"), health_of(e)),
            "DIVINE_SHIELD" => {
                let on = e.int("DIVINE_SHIELD") > 0;
                if on != (old.unwrap_or(0) > 0) {
                    if !on {
                        run.defender(id);
                    }
                    run.push(CombatEvent::Shield { minion, on });
                }
            }
            _ => {}
        }
    }

    /// Duos: the hero on `side` changed during the combat.
    pub fn on_hero_switch(&mut self, side: Side, hero: Option<String>) {
        if let Some(run) = self.run_mut() {
            run.incoming = Some(side);
            run.push(CombatEvent::BoardIn { side, hero });
        }
    }

    fn run_mut(&mut self) -> Option<&mut Run> {
        self.live.as_mut()?.run.as_mut().filter(|r| !r.ended)
    }
}

impl Live {
    /// The run, started now with both boards as they are.
    fn run(&mut self, board: &Board, sides: Sides) -> &mut Run {
        self.run.get_or_insert_with(|| {
            let mut minions = Vec::new();
            let mut refs = BTreeMap::new();
            for (side, pid) in [(Side::Own, sides.own), (Side::Opponent, sides.opponent)] {
                let Some(pid) = pid else { continue };
                for e in board_entities(board, pid) {
                    if minions.len() < MAX_REPLAY_MINIONS {
                        refs.insert(e.id, minions.len());
                        minions.push(ReplayMinion::new(side, minion_of(e)));
                    }
                }
            }
            Run {
                alive: (0..minions.len()).collect(),
                replay: CombatReplay {
                    start: minions.len(),
                    complete: minions.len() < MAX_REPLAY_MINIONS,
                    minions,
                    events: Vec::new(),
                },
                refs,
                attacked: false,
                open_attack: None,
                incoming: None,
                ended: false,
            }
        })
    }

    fn attack(&mut self, e: &Entity, board: &Board, sides: Sides) {
        let Some(side) = sides.of(e) else { return };
        let run = self.run(board, sides);
        if run.ended {
            return;
        }
        run.attacked = true;
        run.open_attack = None;
        run.incoming = None;
        if e.is("CARDTYPE", "HERO") {
            // The winner's hero hits the loser's as the combat ends.
            let amount = e.int("ATK");
            if amount > 0 {
                let hurt = match side {
                    Side::Own => Side::Opponent,
                    Side::Opponent => Side::Own,
                };
                run.push(CombatEvent::HeroDamage { side: hurt, amount });
                run.ended = true;
            }
        } else if let Some(&attacker) = run.refs.get(&e.id) {
            run.push(CombatEvent::Attack {
                attacker,
                defender: None,
            });
            run.open_attack = Some(run.replay.events.len() - 1);
        }
    }

    fn trigger(&mut self, e: &Entity, keyword: Option<&str>, board: &Board, sides: Sides) {
        let Some(side) = sides.of(e) else { return };
        let Some(card_id) = e.card_id.clone() else {
            return;
        };
        if !TRIGGER_CARD_TYPES.iter().any(|t| e.is("CARDTYPE", t)) {
            return;
        }
        let attacked = self.run.as_ref().is_some_and(|r| r.attacked);
        let trigger = match keyword {
            Some("DEATHRATTLE") => TriggerKind::Deathrattle,
            Some("AVENGE") => TriggerKind::Avenge,
            Some("BACON_RALLY") => TriggerKind::Rally,
            _ if !attacked => TriggerKind::StartOfCombat,
            _ => return,
        };
        let run = self.run(board, sides);
        if run.ended {
            return;
        }
        let source = run.refs.get(&e.id).copied();
        run.push(CombatEvent::Trigger {
            trigger,
            side,
            card_id,
            source,
        });
    }

    /// A minion made during this combat entering a board on screen.
    fn summon(&mut self, id: i64, board: &Board, sides: Sides) {
        let Some(run) = self.run.as_mut().filter(|r| !r.ended) else {
            return;
        };
        if run.refs.contains_key(&id) {
            return;
        }
        let Some(e) = board.get(id) else { return };
        if !(e.is("CARDTYPE", "MINION") && e.is("ZONE", "PLAY")) {
            return;
        }
        let Some(side) = sides.of(e) else { return };
        if !self.created.contains(&id) && run.incoming != Some(side) {
            return;
        }
        if run.replay.minions.len() >= MAX_REPLAY_MINIONS {
            run.replay.complete = false;
            return;
        }
        let minion = run.replay.minions.len();
        run.replay
            .minions
            .push(ReplayMinion::new(side, minion_of(e)));
        run.refs.insert(id, minion);
        run.alive.insert(minion);
        run.push(CombatEvent::Summon { minion });
    }
}

impl Run {
    /// Drops what follows the last hero switch when no fight came after it:
    /// the game putting the heroes and the shop boards back as a Duos combat
    /// ends in a tie, not a board coming in.
    fn drop_heroes_put_back(&mut self) {
        let events = &self.replay.events;
        let Some(last) = events
            .iter()
            .rposition(|e| matches!(e, CombatEvent::BoardIn { .. }))
        else {
            return;
        };
        let fought = events[last..].iter().any(|e| {
            matches!(
                e,
                CombatEvent::Attack { .. }
                    | CombatEvent::Damage { .. }
                    | CombatEvent::Death { .. }
                    | CombatEvent::HeroDamage { .. }
            )
        });
        if fought {
            return;
        }
        let first_new = events[last..]
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Summon { minion } => Some(*minion),
                _ => None,
            })
            .min();
        self.replay.events.truncate(last);
        if let Some(first_new) = first_new {
            self.replay.minions.truncate(first_new);
        }
    }

    fn push(&mut self, event: CombatEvent) {
        if self.replay.events.len() < MAX_REPLAY_EVENTS {
            self.replay.events.push(event);
        } else {
            self.replay.complete = false;
        }
    }

    /// A buff or heal: one event per minion for a run of changes in a row.
    fn stats(&mut self, minion: usize, atk: i64, health: i64) {
        if let Some(CombatEvent::Stats { minion: m, .. }) = self.replay.events.last() {
            if *m == minion {
                self.replay.events.pop();
            }
        }
        self.push(CombatEvent::Stats {
            minion,
            atk,
            health,
        });
    }

    /// The open attack's defender, when not known yet: the minion the game
    /// names, or the first enemy minion the attack hits.
    fn defender(&mut self, id: i64) {
        let Some(index) = self.open_attack else {
            return;
        };
        let Some(&target) = self.refs.get(&id) else {
            return;
        };
        let Some(CombatEvent::Attack { attacker, defender }) = self.replay.events.get_mut(index)
        else {
            return;
        };
        if defender.is_none()
            && self.replay.minions[*attacker].side != self.replay.minions[target].side
        {
            *defender = Some(target);
            self.open_attack = None;
        }
    }
}

/// Health left: `HEALTH - DAMAGE`.
fn health_of(e: &Entity) -> i64 {
    e.int("HEALTH").saturating_sub(e.int("DAMAGE"))
}

/// The minions of `controller` on the board, left to right.
pub(crate) fn board_entities(board: &Board, controller: i64) -> Vec<&Entity> {
    let mut minions: Vec<&Entity> = board
        .iter()
        .filter(|e| {
            e.is("CARDTYPE", "MINION") && e.is("ZONE", "PLAY") && e.int("CONTROLLER") == controller
        })
        .collect();
    minions.sort_by_key(|e| (e.int("ZONE_POSITION"), e.id));
    minions
}

/// A board minion as the report keeps it.
pub(crate) fn minion_of(e: &Entity) -> Minion {
    use crate::collector::is_golden_minion;
    use crate::report::{KEYWORDS, SIM_KEYWORDS};
    let named = |tags: &[(&str, &'static str)]| -> Vec<&'static str> {
        tags.iter()
            .filter(|(tag, _)| e.int(tag) > 0)
            .map(|(_, name)| *name)
            .collect()
    };
    Minion {
        card_id: e.card_id.clone(),
        atk: e.int("ATK"),
        health: health_of(e),
        position: e.int("ZONE_POSITION"),
        golden: is_golden_minion(e),
        keywords: named(&KEYWORDS),
        sim_keywords: named(&SIM_KEYWORDS),
    }
}
