//! One combat, played to the end. The rules and their sources are in
//! design notes; the ones still to be checked against
//! the logs are marked there.

use crate::cards::{self, Action, Trigger};
use crate::kit_passive::{self, KitTrigger};
use crate::model::{BaseCard, GameValues, Keywords, Minion, Team, Tribes, Warband, MAX_BOARD};
use crate::rng::Rng;

/// Attack turns before a combat is called a tie. A real combat ends in far
/// fewer; this only stops a loop the rules do not end.
const MAX_ATTACK_TURNS: u32 = 1_000;

/// The result for the player (the first warband).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombatResult {
    Won,
    Tie,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub result: CombatResult,
    /// Damage to the losing hero before any cap: the winner's tavern tier
    /// plus the tiers of its surviving minions. `Some(0)` on a tie; `None`
    /// when a tier is unknown.
    pub damage: Option<i32>,
    /// True when the combat hit the turn limit and was called a tie.
    pub stalled: bool,
}

/// Card facts shared by every copy of a card in one combat.
#[derive(Clone, Debug)]
pub(crate) struct CardInfo {
    pub id: String,
    /// The Dark Gifts (`crate::gifts`) of the minions that share this entry;
    /// the same card with other gifts has an entry of its own.
    pub gifts: Vec<&'static str>,
    /// The entry of the same card without gifts, for a minion that comes
    /// back with Reborn as a fresh copy of its card.
    pub plain: u16,
    /// The card's triggers, then those of the minion's Dark Gifts.
    pub triggers: Vec<Trigger>,
    pub tier: Option<u8>,
    pub tribes: Tribes,
    pub base: Option<BaseCard>,
}

/// Both warbands turned into the engine's form once, then played many times.
#[derive(Clone, Debug)]
pub(crate) struct Prepared {
    pub cards: Vec<CardInfo>,
    /// Token id -> its own entry in `cards`, with the printed facts from
    /// `cards::TOKENS`, apart from any board minion with the same id (whose
    /// facts may be unknown).
    pub tokens: Vec<(&'static str, u16)>,
    pub start: [Vec<Unit>; 2],
    pub tavern_tier: [Option<u8>; 2],
    /// Duos: the teammate's board, which comes in once `start` is gone, and
    /// its hero's tavern tier. `None` in Solo or without a teammate.
    pub second: [Option<Vec<Unit>>; 2],
    pub second_tier: [Option<u8>; 2],
    /// Each side's game-long values, and its teammate's (T-609).
    pub values: [GameValues; 2],
    pub second_values: [GameValues; 2],
    /// Each side's played hero powers and trinkets, and its teammate's
    /// (T-628).
    pub kits: [Vec<KitUse>; 2],
    pub second_kits: [Vec<KitUse>; 2],
    /// Each side's played powers and trinkets that act during combat, and
    /// its teammate's (T-628 round 4).
    pub passives: [Vec<KitTrigger>; 2],
    pub second_passives: [Vec<KitTrigger>; 2],
    pub start_of_combat: bool,
    /// Some card here gains what the Amalgam gains (round 5): checked once,
    /// not on every buff.
    pub amalgam_followers: bool,
    /// Some card here doubles its neighbours' gains (T-609).
    pub gain_doublers: bool,
}

impl Prepared {
    /// Builds the engine's form. Cards not in the registry play as plain
    /// minions; the caller decides whether that is allowed (see `trials`).
    pub fn new(player: &Warband, opponent: &Warband, start_of_combat: bool) -> Self {
        Prepared::boards([player, opponent], [None, None], start_of_combat)
    }

    /// Both teams of a Duos combat (a team without a second board plays as
    /// in Solo).
    pub fn teams(player: &Team, opponent: &Team, start_of_combat: bool) -> Self {
        Prepared::boards(
            [&player.first, &opponent.first],
            [player.second.as_ref(), opponent.second.as_ref()],
            start_of_combat,
        )
    }

    fn boards(first: [&Warband; 2], second: [Option<&Warband>; 2], start_of_combat: bool) -> Self {
        let mut cards: Vec<CardInfo> = Vec::new();
        let mut uid = 0;
        let mut units = |band: &Warband, cards: &mut Vec<CardInfo>| -> Vec<Unit> {
            band.minions
                .iter()
                .map(|m| {
                    uid += 1;
                    unit_of(uid, m, cards)
                })
                .collect()
        };
        let mut start = [units(first[0], &mut cards), units(first[1], &mut cards)];
        let second_tier = second.map(|b| b.and_then(|b| b.tavern_tier));
        let second_values = second.map(|b| b.map_or(GameValues::default(), |b| b.values));
        let second_bands = second;
        let mut second = [
            second[0].map(|b| units(b, &mut cards)),
            second[1].map(|b| units(b, &mut cards)),
        ];
        // A held minion's uid is never used: it comes in as a fresh copy.
        let held = |m: &Minion, cards: &mut Vec<CardInfo>| unit_of(0, m, cards);
        let kits = [0, 1].map(|s| kit_uses(first[s].kit.as_ref(), &start[s], &mut cards, held));
        let second_kits = [0, 1].map(|s| {
            let kit = second_bands[s].and_then(|b| b.kit.as_ref());
            kit_uses(kit, second[s].as_deref().unwrap_or(&[]), &mut cards, held)
        });
        let passives = first.map(|b| kit_passives(b.kit.as_ref(), start_of_combat));
        let second_passives =
            second_bands.map(|b| kit_passives(b.and_then(|b| b.kit.as_ref()), start_of_combat));
        // Tokens the powers and trinkets or any of these cards can summon,
        // and what those summon.
        let mut tokens: Vec<(&'static str, u16)> = Vec::new();
        for t in passives.iter().chain(&second_passives).flatten() {
            if let KitTrigger::AvengeSummon { token, .. } = t {
                prepare_token(&mut cards, &mut tokens, token);
            }
        }
        // The Deities the boards bring can awaken (T-671).
        let values = [first[0].values, first[1].values]
            .into_iter()
            .chain(second_values);
        for v in values {
            if let Some(id) = cards::deity_card(v.deity.dbf) {
                prepare_token(&mut cards, &mut tokens, id);
            }
        }
        let mut i = 0;
        while i < cards.len() {
            for t in summoned_tokens(&cards[i].triggers) {
                prepare_token(&mut cards, &mut tokens, t);
            }
            i += 1;
        }
        let [s0, s1] = &mut second;
        let boards = start.iter_mut().chain([s0, s1].into_iter().flatten());
        for board in boards {
            for unit in board.iter_mut() {
                let triggers = &cards[unit.card as usize].triggers;
                unit.shield_done = triggers.iter().any(|t| match t {
                    Trigger::ShieldAtAttack(n) => unit.attack >= *n,
                    _ => false,
                });
            }
        }
        let amalgam_followers = cards.iter().any(|c| {
            c.triggers
                .iter()
                .any(|t| matches!(t, Trigger::GainsWithAmalgam(_)))
        });
        let gain_doublers = cards
            .iter()
            .any(|c| c.triggers.contains(&Trigger::DoublesAdjacentDragonGains));
        Prepared {
            amalgam_followers,
            gain_doublers,
            cards,
            tokens,
            start,
            tavern_tier: [first[0].tavern_tier, first[1].tavern_tier],
            second,
            second_tier,
            values: [first[0].values, first[1].values],
            second_values,
            kits,
            second_kits,
            passives,
            second_passives,
            start_of_combat,
        }
    }
}

/// Enters token `id` with its printed facts, once.
fn prepare_token(cards: &mut Vec<CardInfo>, tokens: &mut Vec<(&'static str, u16)>, id: &str) {
    let Some(token) = cards::token(id) else {
        return;
    };
    if tokens.iter().any(|(t, _)| *t == token.id) {
        return;
    }
    cards.push(CardInfo {
        id: token.id.to_string(),
        gifts: Vec::new(),
        plain: cards.len() as u16,
        triggers: cards::rules(token.id).map_or(Vec::new(), |r| r.triggers.to_vec()),
        tier: Some(token.tier),
        tribes: token.tribes,
        base: Some(BaseCard {
            attack: token.attack,
            health: token.health,
            keywords: token.keywords,
        }),
    });
    tokens.push((token.id, (cards.len() - 1) as u16));
}

/// A board's played powers and trinkets that act during combat; one played
/// only before start of combat is left out on a board taken after it.
fn kit_passives(kit: Option<&crate::model::Kit>, start_of_combat: bool) -> Vec<KitTrigger> {
    kit.into_iter()
        .flat_map(|k| k.cards())
        .filter_map(|c| kit_passive::passive(&c.card_id))
        .filter(|p| start_of_combat || !p.before_start_only)
        .map(|p| p.trigger)
        .collect()
}

/// A board minion in the engine's form, with uid `uid`.
fn unit_of(uid: u32, m: &Minion, cards: &mut Vec<CardInfo>) -> Unit {
    let (card, keywords) = gifts::card_of(cards, m);
    let mut unit = Unit::new(uid, card, m.attack, m.health, m.keywords.union(keywords));
    unit.counters = m.counters;
    unit
}

pub(crate) fn intern(
    cards: &mut Vec<CardInfo>,
    id: &str,
    gifts: &[&'static str],
    make: impl FnOnce() -> CardInfo,
) -> u16 {
    if let Some(i) = cards.iter().position(|c| c.id == id && c.gifts == gifts) {
        return i as u16;
    }
    cards.push(make());
    (cards.len() - 1) as u16
}

pub(crate) fn summoned_tokens(triggers: &[Trigger]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for t in triggers {
        for a in t.actions() {
            out.extend(a.token());
            if matches!(a, Action::SummonRandomDeathrattle { .. }) {
                out.extend(cards::DEATHRATTLE_POOL);
            }
        }
    }
    out
}

#[derive(Clone, Debug)]
pub(crate) struct Unit {
    uid: u32,
    card: u16,
    attack: i32,
    health: i32,
    kw: Keywords,
    /// Friendly deaths counted toward this unit's Avenge.
    avenge: u8,
    /// Its "once this reaches N Attack" effect has happened.
    shield_done: bool,
    /// The enemy minion that last damaged it, for "destroy the minion that
    /// killed this".
    hit_by: Option<u32>,
    /// Its Health before damage: buffs raise it, damage does not.
    max_health: i32,
    /// Cards whose Deathrattles it gained in this combat.
    gained: Vec<u16>,
    /// Hits its current Divine Shield has taken (a gift's shield takes 3).
    shield_taken: u8,
    /// Times its "n times per combat" effect has happened.
    uses: u8,
    /// The two numbers its text shows (T-628); 0 when not given.
    counters: [i32; 2],
}

impl Unit {
    fn new(uid: u32, card: u16, attack: i32, health: i32, kw: Keywords) -> Self {
        Unit {
            uid,
            card,
            attack,
            health,
            kw,
            avenge: 0,
            shield_done: false,
            hit_by: None,
            max_health: health,
            gained: Vec::new(),
            shield_taken: 0,
            uses: 0,
            counters: [0, 0],
        }
    }

    fn alive(&self) -> bool {
        self.health > 0
    }
}

#[derive(Clone, Debug, Default)]
struct SideState {
    units: Vec<Unit>,
    /// Index of the next attacker (wraps to 0 past the end). Removals and
    /// summons to its left move it, so it keeps pointing at the same slot.
    next: usize,
    /// The game-long values of the board fighting now (T-609).
    values: GameValues,
    /// Friendly minions that died so far in this combat, both boards of a
    /// Duos team counted (T-616).
    deaths: u32,
    /// The Aberrations (not Deities) that died this combat, in order: card,
    /// Attack and Health as they died (T-671).
    aberrations: Vec<(u16, i32, i32)>,
    /// The awakened Deity on the board.
    deity: Option<u32>,
}

impl SideState {
    fn pos(&self, uid: u32) -> Option<usize> {
        self.units.iter().position(|u| u.uid == uid)
    }

    fn remove(&mut self, at: usize) -> Unit {
        if at < self.next {
            self.next -= 1;
        }
        self.units.remove(at)
    }

    fn insert(&mut self, at: usize, unit: Unit) -> bool {
        if self.units.len() >= MAX_BOARD {
            return false;
        }
        let at = at.min(self.units.len());
        if at < self.next {
            self.next += 1;
        }
        self.units.insert(at, unit);
        true
    }
}

/// Where an effect comes from: a minion on the board, or one that just died
/// (its summons go where it was: before `anchor`, or at the end).
#[derive(Clone, Copy, Debug)]
enum Source {
    Unit(u32),
    Dead {
        uid: u32,
        anchor: Option<u32>,
        killer: Option<u32>,
        /// Its Attack and full Health when it died.
        stats: (i32, i32),
    },
}

struct Dead {
    side: usize,
    unit: Unit,
    anchor: Option<u32>,
}

struct Combat<'a> {
    p: &'a Prepared,
    rng: &'a mut Rng,
    sides: [SideState; 2],
    next_uid: u32,
    /// The target of the next swing when it is not the engine's to choose
    /// (the replay of a logged combat, `replay`).
    forced_target: Option<u32>,
    legs: Legs,
    /// What each side's hero powers do once it has space, waiting for it
    /// (T-628 round 3).
    pending: [Vec<Pending>; 2],
    /// The exact copies a minion kept of the minions it destroyed, by its
    /// uid, for its Deathrattle (Stitched Salvager).
    kept: Vec<(u32, Vec<Unit>)>,
    /// What each side's powers and trinkets that act during combat counted
    /// (T-628 round 4).
    kit_counts: [KitCounts; 2],
}

/// Plays one combat with `rng`.
pub(crate) fn play(p: &Prepared, rng: &mut Rng) -> Outcome {
    Combat::new(p, rng).run()
}

impl<'a> Combat<'a> {
    fn new(p: &'a Prepared, rng: &'a mut Rng) -> Self {
        let waiting: usize = p.second.iter().flatten().map(Vec::len).sum();
        let next_uid = p.start.iter().map(Vec::len).sum::<usize>() + waiting + 1;
        let next_uid = next_uid as u32;
        Combat {
            p,
            rng,
            sides: [
                SideState {
                    units: p.start[0].clone(),
                    next: 0,
                    values: p.values[0],
                    deaths: 0,
                    ..SideState::default()
                },
                SideState {
                    units: p.start[1].clone(),
                    next: 0,
                    values: p.values[1],
                    deaths: 0,
                    ..SideState::default()
                },
            ],
            next_uid,
            forced_target: None,
            legs: Legs::of(p),
            pending: [Vec::new(), Vec::new()],
            kept: Vec::new(),
            kit_counts: Default::default(),
        }
    }
}

impl<'a> Combat<'a> {
    fn run(&mut self) -> Outcome {
        let mut turn = self.open();
        let mut turns = 0;
        loop {
            if let Some(first) = self.bring_in_teammates() {
                turn = first;
            }
            if self.finished() {
                return self.outcome(false);
            }
            let attacked = self.attack_turn(turn);
            if !attacked && !self.finished() && self.attacker_index(1 - turn).is_none() {
                return self.outcome(false);
            }
            turn = 1 - turn;
            turns += 1;
            if turns > MAX_ATTACK_TURNS {
                return self.outcome(true);
            }
        }
    }

    /// The combat's opening: start of combat (or, on boards taken after it,
    /// only its "attack immediately" swings), then who attacks first.
    pub(crate) fn open(&mut self) -> usize {
        let turn = self.first_attacker();
        if self.p.start_of_combat {
            let swings = self.has_immediate_attackers([true, true]);
            self.start_of_combat(turn, [true, true]);
            self.first_after_start(turn, swings)
        } else {
            self.attack_immediately(turn, [true, true]);
            turn
        }
    }

    /// The side with more minions, else a coin flip.
    fn first_attacker(&mut self) -> usize {
        match self.first_by_count() {
            Some(side) => side,
            None => usize::from(self.rng.coin()),
        }
    }

    /// Who attacks first once start of combat is played: the side with more
    /// minions after its effects (T-628: the user's logs, 187 of 187 combat
    /// legs where the counts differ), else `before` (the coin already
    /// flipped). With "attack immediately" on a board the count before them
    /// is kept (D-115, one combat seen).
    pub(crate) fn first_after_start(&self, before: usize, swings: bool) -> usize {
        if swings {
            return before;
        }
        self.first_by_count().unwrap_or(before)
    }

    /// Some board of `sides` has a minion, hero power or trinket that
    /// attacks at the start of combat.
    pub(crate) fn has_immediate_attackers(&self, sides: [bool; 2]) -> bool {
        (0..2).any(|s| sides[s] && (!self.immediate_attackers(s).is_empty() || self.kit_swings(s)))
    }

    /// The side with more minions attacks first; `None` on equal counts (a
    /// coin flip).
    fn first_by_count(&self) -> Option<usize> {
        let (a, b) = (self.sides[0].units.len(), self.sides[1].units.len());
        match a.cmp(&b) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// A board is gone. In Duos its teammate's may still come in (see
    /// [`Combat::bring_in_teammates`]); the combat is over when none is left.
    fn finished(&self) -> bool {
        self.sides.iter().any(|s| s.units.is_empty())
    }

    fn outcome(&self, stalled: bool) -> Outcome {
        let alive = [
            !self.sides[0].units.is_empty(),
            !self.sides[1].units.is_empty(),
        ];
        let winner = match (stalled, alive) {
            (false, [true, false]) => Some(0),
            (false, [false, true]) => Some(1),
            _ => None,
        };
        match winner {
            None => Outcome {
                result: CombatResult::Tie,
                damage: Some(0),
                stalled,
            },
            Some(w) => {
                // In Duos a teammate's board that never came in survives whole.
                let waiting = self.p.second[w]
                    .iter()
                    .flatten()
                    .filter(|_| self.legs.waiting[w]);
                let tiers: Option<i32> = self.sides[w]
                    .units
                    .iter()
                    .chain(waiting)
                    .map(|u| self.p.cards[u.card as usize].tier.map(i32::from))
                    .sum();
                // The hero whose board is on the field at the end.
                let hero = self.legs.tier[w].map(i32::from);
                Outcome {
                    result: if w == 0 {
                        CombatResult::Won
                    } else {
                        CombatResult::Lost
                    },
                    damage: tiers.zip(hero).map(|(a, b)| a + b),
                    stalled,
                }
            }
        }
    }

    fn triggers(&self, unit: &Unit) -> &'a [Trigger] {
        let p: &'a Prepared = self.p;
        &p.cards[unit.card as usize].triggers
    }

    fn tribes(&self, unit: &Unit) -> Tribes {
        self.p.cards[unit.card as usize].tribes
    }

    fn unit(&self, side: usize, uid: u32) -> Option<&Unit> {
        self.sides[side].units.iter().find(|u| u.uid == uid)
    }

    fn unit_mut(&mut self, side: usize, uid: u32) -> Option<&mut Unit> {
        self.sides[side].units.iter_mut().find(|u| u.uid == uid)
    }
}

#[path = "combat_attack.rs"]
mod attack;

#[path = "combat_effects.rs"]
mod effects;

#[path = "combat_deaths.rs"]
mod deaths;

#[path = "replay.rs"]
pub mod replay;

#[path = "combat_round2.rs"]
mod round2;

#[path = "combat_round3.rs"]
mod round3;

#[path = "combat_round4.rs"]
mod round4;

#[path = "combat_round5.rs"]
mod round5;

#[path = "combat_round8.rs"]
mod round8;

#[path = "combat_duos.rs"]
mod duos;

#[path = "combat_gifts.rs"]
mod gifts;

#[path = "combat_values.rs"]
mod values;

#[path = "combat_kit.rs"]
mod kit;

#[path = "combat_copies.rs"]
mod copies;

#[path = "combat_deity.rs"]
mod deity;
#[cfg(test)]
#[path = "combat_deity_tests.rs"]
mod deity_tests;
#[path = "combat_passive.rs"]
mod passive;
use duos::Legs;
use kit::{kit_uses, KitUse, Pending};
use passive::KitCounts;

#[cfg(test)]
#[path = "combat_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "combat_duos_tests.rs"]
mod duos_tests;

#[cfg(test)]
#[path = "combat_gifts_tests.rs"]
mod gifts_tests;

#[cfg(test)]
#[path = "combat_t609_tests.rs"]
mod t609_tests;

#[cfg(test)]
#[path = "combat_golden_tests.rs"]
mod golden_tests;

#[cfg(test)]
#[path = "combat_round8_tests.rs"]
mod round8_tests;

#[cfg(test)]
#[path = "combat_stat_cap_tests.rs"]
mod stat_cap_tests;

#[cfg(test)]
#[path = "combat_kit_tests.rs"]
mod kit_tests;

#[cfg(test)]
#[path = "combat_space_tests.rs"]
mod space_tests;

#[cfg(test)]
#[path = "combat_passive_tests.rs"]
mod passive_tests;

#[cfg(test)]
#[path = "combat_t631_tests.rs"]
mod t631_tests;
