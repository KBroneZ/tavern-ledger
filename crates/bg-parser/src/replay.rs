//! Combat replay (T-215, D-099): what happened in each combat the game plays
//! back on screen, in the log's order. Attacks, damage, Divine Shields
//! popped, stat changes, deaths, summons, the triggers the log names
//! (start of combat, deathrattle, avenge, rally), a Duos teammate's board
//! coming in and the hero damage at the end.
//!
//! Since parser revision 13 (T-575) the replay also says how the game groups
//! its steps: each run of steps that one log block gives is one beat
//! ([`CombatReplay::beats`]), so a board-wide buff or simultaneous deaths
//! play at once. Each damage names its source (the attack, or the card whose
//! effect dealt it), and keywords given or used mid-combat are steps too.
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

use crate::buffs::{buffs_on, Buff};
use crate::gifts::gifts_on;
use crate::report::{Minion, Side, KEYWORDS};
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
    /// Dark Gifts, as on a board minion (T-528); left out when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gifts: Vec<&'static str>,
    #[serde(skip_serializing_if = "crate::report::is_false")]
    pub gift_unknown: bool,
    /// Buffs, as on a board minion (T-619); left out when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub buffs: Vec<Buff>,
    /// The two numbers its text shows (`TAG_SCRIPT_DATA_NUM_1` and `_2`;
    /// parser revision 16, T-628), for a start-of-combat buff that grows
    /// during the game; left out when both are 0. Numbers below 0 or above
    /// [`crate::kit::MAX_KIT_NUMBER`] are kept as 0.
    #[serde(skip_serializing_if = "no_counters")]
    pub counters: [i64; 2],
}

fn no_counters(n: &[i64; 2]) -> bool {
    *n == [0, 0]
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
            gifts: m.gifts,
            gift_unknown: m.gift_unknown,
            buffs: m.buffs,
            counters: m.counters.map(|n| {
                if (0..=crate::kit::MAX_KIT_NUMBER).contains(&n) {
                    n
                } else {
                    0
                }
            }),
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

/// How a damage came about (parser revision 13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    Attack,
    StartOfCombat,
    Deathrattle,
    Avenge,
    Rally,
    HeroPower,
    Trinket,
    /// Any other effect of a card after the first attack.
    Effect,
}

/// Where a damage came from: a minion of the replay, or a card that is not
/// one (a hero power, a trinket) with its side.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DamageSource {
    pub how: Cause,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minion: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
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
    /// Damage taken; `health` is what is left. `source` is None when the
    /// log does not say (written as null, never left out).
    Damage {
        minion: usize,
        amount: i64,
        health: i64,
        source: Option<DamageSource>,
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
    /// A keyword of [`crate::report::KEYWORDS`] other than Divine Shield
    /// given (`on`) or lost: a Rally giving Venomous, Venomous used up, a
    /// Reborn spent.
    Keyword {
        minion: usize,
        keyword: &'static str,
        on: bool,
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
    /// The steps grouped as the game plays them (revision 13): the length of
    /// each run of events that one log block gives, in order (they add up to
    /// the number of events). An event outside any block is a beat alone.
    pub beats: Vec<usize>,
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

/// A Tavern spell cast for the next combat (Upper Hand, Brood of Nozdormu)
/// firing at its start (parser revision 16, T-628): the game keeps it in the
/// Secret zone and its block is an `OBJECTIVE` trigger. The Old God, kept
/// there too, fires its objective in the same way after attacks and with no
/// keyword before them; it never starts or joins the replay.
fn is_prepared_spell(e: &Entity, keyword: Option<&str>) -> bool {
    e.is("CARDTYPE", "SPELL")
        && e.is("ZONE", "SECRET")
        && keyword == Some("OBJECTIVE")
        && e.card_id.as_deref() != Some(OLD_GOD)
}

const OLD_GOD: &str = "BG_OldGod";

/// Cards a damage can be put down to when their block deals it. Anything
/// else (the game, a player, an enchantment) passes it to the block around.
const SOURCE_CARD_TYPES: [&str; 8] = [
    "MINION",
    "HERO",
    "HERO_POWER",
    "BATTLEGROUND_TRINKET",
    "SPELL",
    "BATTLEGROUND_SPELL",
    "BATTLEGROUND_QUEST_REWARD",
    "BATTLEGROUND_ANOMALY",
];

/// Blocks kept open at once; a deeper one is not tracked (no real log nests
/// this far).
const MAX_BLOCK_DEPTH: usize = 64;

/// What a block open in the log can be blamed for.
#[derive(Clone, Debug)]
enum Blame {
    /// Nothing: ask the block around it.
    None,
    /// An attack: its event in the replay names attacker and defender.
    Attack { event: usize },
    /// A card's effect.
    Effect {
        entity: i64,
        card_id: Option<String>,
        side: Side,
        how: Cause,
    },
}

/// A block open in the log.
#[derive(Clone, Debug)]
struct Open {
    /// Its line's indentation: a block end, or a block start at the same
    /// depth or above, closes it even when a trimmed log lost its end.
    depth: usize,
    id: u32,
    blame: Blame,
}

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
    /// Summons that entered with no slot (0): the game sets it in the tag
    /// change right after, before anything else happens.
    unplaced: BTreeSet<usize>,
    /// The log block each event came from (0: none), with the events.
    blocks: Vec<u32>,
    /// The block open now.
    block: u32,
}

/// The combat of the turn being read.
#[derive(Debug)]
struct Live {
    round: i64,
    /// The log block open now (0: none), for a run started later.
    block: u32,
    /// Entities created during this combat turn: a minion entering the board
    /// is a summon only if it is one of them (the shop's minions come back
    /// as the combat ends), or a Duos board is coming in on its side.
    created: BTreeSet<i64>,
    /// The game has announced the combat's action step (`STEP` or
    /// `NEXT_STEP` `MAIN_ACTION`): both boards are set up. Nothing before
    /// it starts the combat; a shop effect can fire while the shop board is
    /// still in play (T-560).
    action: bool,
    /// The game has announced a step this turn. A log without step tags
    /// (a trimmed fixture) takes the opponent's hero being set instead.
    steps: bool,
    /// The opponent's hero was set this turn.
    opponent_set: bool,
    run: Option<Run>,
}

/// Collects one game's replays as its packets are read.
#[derive(Debug, Default)]
pub struct ReplayTracker {
    /// Round -> its replay, for every combat with at least one attack.
    done: BTreeMap<i64, CombatReplay>,
    live: Option<Live>,
    /// The blocks open in the log, innermost last.
    open: Vec<Open>,
    /// The last block id given (ids start at 1; 0 is "no block").
    last_id: u32,
}

impl ReplayTracker {
    /// The replay of `round`: finished, or the one being read (incomplete).
    pub fn replay(&self, round: i64) -> Option<CombatReplay> {
        if let Some(r) = self.done.get(&round) {
            return Some(r.clone());
        }
        let live = self.live.as_ref().filter(|l| l.round == round)?;
        let run = live.run.as_ref().filter(|r| r.attacked)?;
        let mut replay = run.sealed();
        replay.complete = false;
        Some(replay)
    }

    /// A new turn: the combat before it is over; a combat turn starts one.
    pub fn on_turn(&mut self, turn: i64) {
        self.finish();
        if turn >= 2 && turn % 2 == 0 {
            let block = self.open.last().map_or(0, |o| o.id);
            self.live = Some(Live {
                round: turn / 2,
                block,
                created: BTreeSet::new(),
                action: false,
                steps: false,
                opponent_set: false,
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
        self.done.insert(live.round, run.sealed());
    }

    /// An entity was created (`FULL_ENTITY`).
    pub fn on_created(&mut self, id: i64, board: &Board, sides: Sides) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        live.created.insert(id);
        live.summon(id, board, sides);
        live.refresh_buffs(id, board);
    }

    /// A hidden entity was shown or changed (`SHOW_ENTITY`,
    /// `CHANGE_ENTITY`): a summon can arrive this way.
    pub fn on_shown(&mut self, id: i64, board: &Board, sides: Sides) {
        if let Some(live) = self.live.as_mut() {
            live.summon(id, board, sides);
            live.refresh_buffs(id, board);
        }
    }

    /// A block starts at indentation `depth` (0: nested in no other): an
    /// attack, a trigger the log may name, or any other block, which groups
    /// the steps inside it into one beat. `entity` is None for a block of
    /// the game or a player.
    pub fn on_block_start(
        &mut self,
        kind: &str,
        entity: Option<i64>,
        keyword: Option<&str>,
        depth: usize,
        board: &Board,
        sides: Sides,
    ) {
        self.close_to(depth);
        // Opened first: the attack or trigger step is the block's own.
        let tracked = self.open.len() < MAX_BLOCK_DEPTH;
        if tracked {
            self.last_id = self.last_id.wrapping_add(1).max(1);
            self.open.push(Open {
                depth,
                id: self.last_id,
                blame: Blame::None,
            });
        }
        self.sync_block();
        let attacked = self.live_run().is_some_and(|r| r.attacked);
        let events = self.events_now();
        self.on_block(kind, entity, keyword, depth == 0, board, sides);
        let pushed = self.events_now() > events;
        let blame = match (kind, entity.and_then(|id| board.get(id))) {
            ("ATTACK", Some(_)) if pushed => Blame::Attack { event: events },
            ("ATTACK", _) => Blame::None,
            (_, Some(e)) => blame_of(e, keyword, attacked, sides),
            (_, None) => Blame::None,
        };
        if let Some(open) = self.open.last_mut().filter(|_| tracked) {
            open.blame = blame;
        }
    }

    /// A block ends at indentation `depth`.
    pub fn on_block_end(&mut self, depth: usize) {
        self.close_to(depth);
        self.sync_block();
    }

    /// Closes the blocks at `depth` or deeper.
    fn close_to(&mut self, depth: usize) {
        while self.open.last().is_some_and(|o| o.depth >= depth) {
            self.open.pop();
        }
    }

    /// Tells the run which block is open now.
    fn sync_block(&mut self) {
        let id = self.open.last().map_or(0, |o| o.id);
        if let Some(live) = self.live.as_mut() {
            live.block = id;
            if let Some(run) = live.run.as_mut() {
                run.block = id;
            }
        }
    }

    fn live_run(&self) -> Option<&Run> {
        self.live.as_ref()?.run.as_ref()
    }

    fn events_now(&self) -> usize {
        self.live_run().map_or(0, |r| r.replay.events.len())
    }

    /// Where damage to `minion` came from: the innermost open block that
    /// can be blamed for it.
    fn source_of(&self, minion: usize) -> Option<DamageSource> {
        let run = self.live_run()?;
        for open in self.open.iter().rev() {
            match &open.blame {
                Blame::None => continue,
                Blame::Attack { event } => {
                    let Some(CombatEvent::Attack { attacker, defender }) =
                        run.replay.events.get(*event)
                    else {
                        return None;
                    };
                    // The attacker is hurt by its defender, anyone else by
                    // the attacker (Cleave's neighbours too).
                    let by = if *attacker == minion {
                        (*defender)?
                    } else {
                        *attacker
                    };
                    return Some(DamageSource {
                        how: Cause::Attack,
                        minion: Some(by),
                        card_id: None,
                        side: None,
                    });
                }
                Blame::Effect {
                    entity,
                    card_id,
                    side,
                    how,
                } => {
                    return match run.refs.get(entity) {
                        Some(&m) => Some(DamageSource {
                            how: *how,
                            minion: Some(m),
                            card_id: None,
                            side: None,
                        }),
                        // A card with no id cannot be named: unknown.
                        None => card_id.as_ref().map(|id| DamageSource {
                            how: *how,
                            minion: None,
                            card_id: Some(id.clone()),
                            side: Some(*side),
                        }),
                    };
                }
            }
        }
        None
    }

    /// An attack, or a trigger the log may name, for the replay's steps.
    fn on_block(
        &mut self,
        kind: &str,
        entity: Option<i64>,
        keyword: Option<&str>,
        top: bool,
        board: &Board,
        sides: Sides,
    ) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let Some(e) = entity.and_then(|id| board.get(id)) else {
            return;
        };
        // A trigger waits for the set-up; an attack is always the combat's,
        // so a log that never announces the action step still has a replay.
        if kind != "ATTACK" && !live.ready() {
            return;
        }
        match kind {
            "ATTACK" => live.attack(e, board, sides),
            "TRIGGER" => live.trigger(e, keyword, top, board, sides),
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
        let source = match (tag, self.live_run().and_then(|r| r.refs.get(&id))) {
            ("DAMAGE", Some(&minion)) => self.source_of(minion),
            _ => None,
        };
        let Some(live) = self.live.as_mut() else {
            return;
        };
        live.on_setup_tag(id, tag, board, sides);
        if tag == "ZONE" || tag == "CONTROLLER" {
            live.summon(id, board, sides);
        }
        if tag == "ZONE" || tag == "ATTACHED" {
            live.refresh_buffs(id, board);
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
                let amount = e.int("DAMAGE").saturating_sub(old.unwrap_or(0));
                if amount > 0 {
                    run.defender(id);
                    run.push(CombatEvent::Damage {
                        minion,
                        amount,
                        health,
                        source,
                    });
                } else {
                    run.stats(minion, e.int("ATK"), health);
                }
            }
            "ATK" | "HEALTH" => run.stats(minion, e.int("ATK"), health_of(e)),
            "ZONE_POSITION" => {
                let slot = e.int("ZONE_POSITION");
                if slot > 0 && run.unplaced.remove(&minion) {
                    run.replay.minions[minion].position = slot;
                }
            }
            "DIVINE_SHIELD" => {
                let on = e.int("DIVINE_SHIELD") > 0;
                if on != (old.unwrap_or(0) > 0) {
                    if !on {
                        run.defender(id);
                    }
                    run.push(CombatEvent::Shield { minion, on });
                }
            }
            _ => {
                let Some(&(_, keyword)) = KEYWORDS
                    .iter()
                    .find(|(name, _)| *name == tag && *name != "DIVINE_SHIELD")
                else {
                    return;
                };
                let on = e.int(tag) > 0;
                if on != (old.unwrap_or(0) > 0) {
                    run.push(CombatEvent::Keyword {
                        minion,
                        keyword,
                        on,
                    });
                }
            }
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
    /// Both boards are set up: the combat can start.
    fn ready(&self) -> bool {
        self.action || (!self.steps && self.opponent_set)
    }

    /// Notes the tags that tell the combat is set up.
    fn on_setup_tag(&mut self, id: i64, tag: &str, board: &Board, sides: Sides) {
        let Some(e) = board.get(id) else { return };
        if (tag == "STEP" || tag == "NEXT_STEP") && board.game().is_some_and(|g| g.id == id) {
            self.steps = true;
            self.action |= e.is(tag, "MAIN_ACTION");
        } else if tag == "HERO_ENTITY" && e.is("CARDTYPE", "PLAYER") {
            self.opponent_set |= Some(e.int("PLAYER_ID")) == sides.opponent;
        }
    }

    /// The run, started now with both boards as they are.
    fn run(&mut self, board: &Board, sides: Sides) -> &mut Run {
        let block = self.block;
        self.run.get_or_insert_with(|| {
            let mut minions = Vec::new();
            let mut refs = BTreeMap::new();
            let mut gifts = crate::gifts::gift_index(board);
            let mut buffs = crate::buffs::buff_index(board);
            for (side, pid) in [(Side::Own, sides.own), (Side::Opponent, sides.opponent)] {
                let Some(pid) = pid else { continue };
                for e in board_entities(board, pid) {
                    if minions.len() < MAX_REPLAY_MINIONS {
                        refs.insert(e.id, minions.len());
                        let on = gifts.remove(&e.id).unwrap_or_default();
                        let kinds = buffs.remove(&e.id).unwrap_or_default();
                        minions.push(ReplayMinion::new(side, minion_of(e, on, kinds)));
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
                    beats: Vec::new(),
                },
                refs,
                attacked: false,
                open_attack: None,
                incoming: None,
                ended: false,
                unplaced: BTreeSet::new(),
                blocks: Vec::new(),
                block,
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
            let attack = CombatEvent::Attack {
                attacker,
                defender: None,
            };
            if run.push(attack) {
                run.open_attack = Some(run.replay.events.len() - 1);
            }
        }
    }

    fn trigger(
        &mut self,
        e: &Entity,
        keyword: Option<&str>,
        top: bool,
        board: &Board,
        sides: Sides,
    ) {
        // The game sets both boards up in blocks of its own, and a trinket
        // or minion can fire inside them before every minion is placed: only
        // a trigger of its own, or an attack, starts the combat.
        if self.run.is_none() && !top {
            return;
        }
        let Some(side) = sides.of(e) else { return };
        let Some(card_id) = e.card_id.clone() else {
            return;
        };
        if !TRIGGER_CARD_TYPES.iter().any(|t| e.is("CARDTYPE", t)) && !is_prepared_spell(e, keyword)
        {
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

    /// An enchantment came to a minion summoned during this combat: its
    /// buffs arrive after the minion's own lines, so they are read again
    /// while it lives (a minion on the board as the combat began has them
    /// from the start).
    fn refresh_buffs(&mut self, id: i64, board: &Board) {
        let Some(run) = self.run.as_mut().filter(|r| !r.ended) else {
            return;
        };
        let Some(on) = board
            .get(id)
            .filter(|e| e.is("CARDTYPE", "ENCHANTMENT"))
            .and_then(|e| e.opt_int("ATTACHED"))
        else {
            return;
        };
        let Some(&minion) = run.refs.get(&on).filter(|&&m| m >= run.replay.start) else {
            return;
        };
        // A minion that died keeps what it had: the game then takes its
        // enchantments out of play one by one.
        if !run.alive.contains(&minion) {
            return;
        }
        run.replay.minions[minion].buffs = buffs_on(board, on);
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
        // A minion without its summon event would never be drawn.
        let full = run.replay.events.len() >= MAX_REPLAY_EVENTS;
        if run.replay.minions.len() >= MAX_REPLAY_MINIONS || full {
            run.replay.complete = false;
            return;
        }
        let minion = run.replay.minions.len();
        run.replay.minions.push(ReplayMinion::new(
            side,
            minion_of(e, gifts_on(board, id), buffs_on(board, id)),
        ));
        run.refs.insert(id, minion);
        run.alive.insert(minion);
        if run.push(CombatEvent::Summon { minion }) && e.int("ZONE_POSITION") <= 0 {
            run.unplaced.insert(minion);
        }
    }
}

impl Run {
    /// The replay with its beats: each run of events from one block. A stat,
    /// shield or keyword change outside any block is the game refreshing an
    /// aura after the block before it, so it plays with that block; any
    /// other event outside a block is a beat alone.
    fn sealed(&self) -> CombatReplay {
        let mut beats: Vec<usize> = Vec::new();
        let mut last = None;
        for (event, &block) in self.replay.events.iter().zip(&self.blocks) {
            let refresh = block == 0
                && matches!(
                    event,
                    CombatEvent::Stats { .. }
                        | CombatEvent::Shield { .. }
                        | CombatEvent::Keyword { .. }
                );
            match beats.last_mut() {
                Some(n) if refresh || (block != 0 && last == Some(block)) => *n += 1,
                _ => beats.push(1),
            }
            if !refresh {
                last = Some(block);
            }
        }
        CombatReplay {
            beats,
            ..self.replay.clone()
        }
    }

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
        self.blocks.truncate(last);
        if let Some(first_new) = first_new {
            self.replay.minions.truncate(first_new);
        }
    }

    /// Adds `event`; false (and the replay marked incomplete) past the bound.
    /// Anything but a summon settles the summons before it: a slot that
    /// comes later is the game shifting the board, not where they came in.
    fn push(&mut self, event: CombatEvent) -> bool {
        if !matches!(event, CombatEvent::Summon { .. }) {
            self.unplaced.clear();
        }
        if self.replay.events.len() < MAX_REPLAY_EVENTS {
            self.replay.events.push(event);
            self.blocks.push(self.block);
            true
        } else {
            self.replay.complete = false;
            false
        }
    }

    /// A buff or heal: one event per minion for a run of changes in a row
    /// in the same block.
    fn stats(&mut self, minion: usize, atk: i64, health: i64) {
        if let Some(CombatEvent::Stats { minion: m, .. }) = self.replay.events.last() {
            if *m == minion && self.blocks.last() == Some(&self.block) {
                self.replay.events.pop();
                self.blocks.pop();
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

/// What a block of card `e` is blamed for: its effect, named by how it
/// fired; a block of the game, a player or an enchantment passes the blame
/// to the block around it.
fn blame_of(e: &Entity, keyword: Option<&str>, attacked: bool, sides: Sides) -> Blame {
    let Some(side) = sides.of(e) else {
        return Blame::None;
    };
    if !SOURCE_CARD_TYPES.iter().any(|t| e.is("CARDTYPE", t)) {
        return Blame::None;
    }
    let how = match keyword {
        Some("DEATHRATTLE") => Cause::Deathrattle,
        Some("AVENGE") => Cause::Avenge,
        Some("BACON_RALLY") => Cause::Rally,
        _ if e.is("CARDTYPE", "HERO_POWER") => Cause::HeroPower,
        _ if e.is("CARDTYPE", "BATTLEGROUND_TRINKET") => Cause::Trinket,
        _ if !attacked => Cause::StartOfCombat,
        _ => Cause::Effect,
    };
    Blame::Effect {
        entity: e.id,
        card_id: e.card_id.clone(),
        side,
        how,
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

/// A board minion as the report keeps it, with `gifts` its Dark Gifts
/// (from [`crate::gifts::gift_index`] or [`crate::gifts::gifts_on`]) and
/// `buffs` its buffs ([`crate::buffs::buff_index`] or [`buffs_on`]).
pub(crate) fn minion_of(e: &Entity, gifts: Vec<&'static str>, buffs: Vec<Buff>) -> Minion {
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
        counters: [
            e.int("TAG_SCRIPT_DATA_NUM_1"),
            e.int("TAG_SCRIPT_DATA_NUM_2"),
        ],
        gift_unknown: crate::gifts::gift_unknown(e, &gifts),
        gifts,
        buffs,
    }
}
