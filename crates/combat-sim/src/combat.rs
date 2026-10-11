//! One combat, played to the end. The rules and their sources are in
//! `docs/research/combat-simulator.md`; the ones still to be checked against
//! the logs are marked there.

use crate::cards::{self, Action, Trigger};
use crate::model::{BaseCard, Keywords, Tribes, Warband, MAX_BOARD};
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
    pub triggers: &'static [Trigger],
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
    pub start_of_combat: bool,
}

impl Prepared {
    /// Builds the engine's form. Cards not in the registry play as plain
    /// minions; the caller decides whether that is allowed (see `trials`).
    pub fn new(player: &Warband, opponent: &Warband, start_of_combat: bool) -> Self {
        let mut cards: Vec<CardInfo> = Vec::new();
        let mut start = [Vec::new(), Vec::new()];
        let mut uid = 0;
        for (side, band) in [player, opponent].into_iter().enumerate() {
            for m in &band.minions {
                let card = intern(&mut cards, &m.card_id, || CardInfo {
                    id: m.card_id.clone(),
                    triggers: cards::rules(&m.card_id).map(|r| r.triggers).unwrap_or(&[]),
                    tier: m.tier,
                    tribes: m.tribes,
                    base: m.base,
                });
                uid += 1;
                start[side].push(Unit::new(uid, card, m.attack, m.health, m.keywords));
            }
        }
        // Tokens any of these cards can summon, and what those summon.
        let mut tokens: Vec<(&'static str, u16)> = Vec::new();
        let mut i = 0;
        while i < cards.len() {
            for t in summoned_tokens(cards[i].triggers) {
                let Some(token) = cards::token(t) else {
                    continue;
                };
                if tokens.iter().any(|(id, _)| *id == token.id) {
                    continue;
                }
                cards.push(CardInfo {
                    id: token.id.to_string(),
                    triggers: cards::rules(token.id).map(|r| r.triggers).unwrap_or(&[]),
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
            i += 1;
        }
        for side in &mut start {
            for unit in side.iter_mut() {
                let triggers = cards[unit.card as usize].triggers;
                unit.shield_done = triggers.iter().any(|t| match t {
                    Trigger::ShieldAtAttack(n) => unit.attack >= *n,
                    _ => false,
                });
            }
        }
        Prepared {
            cards,
            tokens,
            start,
            tavern_tier: [player.tavern_tier, opponent.tavern_tier],
            start_of_combat,
        }
    }
}

fn intern(cards: &mut Vec<CardInfo>, id: &str, make: impl FnOnce() -> CardInfo) -> u16 {
    if let Some(i) = cards.iter().position(|c| c.id == id) {
        return i as u16;
    }
    cards.push(make());
    (cards.len() - 1) as u16
}

pub(crate) fn summoned_tokens(triggers: &[Trigger]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for t in triggers {
        for a in t.actions() {
            if let Action::Summon { token, .. } | Action::SummonAndAttack { token } = a {
                out.push(*token);
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
        anchor: Option<u32>,
        killer: Option<u32>,
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
}

/// Plays one combat with `rng`.
pub(crate) fn play(p: &Prepared, rng: &mut Rng) -> Outcome {
    Combat::new(p, rng).run()
}

impl<'a> Combat<'a> {
    fn new(p: &'a Prepared, rng: &'a mut Rng) -> Self {
        let next_uid = p.start.iter().map(|s| s.len() as u32).sum::<u32>() + 1;
        Combat {
            p,
            rng,
            sides: [
                SideState {
                    units: p.start[0].clone(),
                    next: 0,
                },
                SideState {
                    units: p.start[1].clone(),
                    next: 0,
                },
            ],
            next_uid,
            forced_target: None,
        }
    }
}

impl Combat<'_> {
    fn run(&mut self) -> Outcome {
        let mut turn = match self.first_by_count() {
            Some(side) => side,
            None => usize::from(self.rng.coin()),
        };
        if self.p.start_of_combat {
            self.start_of_combat(turn);
        }
        let mut turns = 0;
        while !self.finished() {
            let attacked = self.attack_turn(turn);
            if !attacked && self.attacker_index(1 - turn).is_none() {
                return self.outcome(false);
            }
            turn = 1 - turn;
            turns += 1;
            if turns > MAX_ATTACK_TURNS {
                return self.outcome(true);
            }
        }
        self.outcome(false)
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
                let tiers: Option<i32> = self.sides[w]
                    .units
                    .iter()
                    .map(|u| self.p.cards[u.card as usize].tier.map(i32::from))
                    .sum();
                let hero = self.p.tavern_tier[w].map(i32::from);
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

    fn triggers(&self, unit: &Unit) -> &'static [Trigger] {
        self.p.cards[unit.card as usize].triggers
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

    fn start_of_combat(&mut self, first: usize) {
        for side in [first, 1 - first] {
            let uids: Vec<u32> = self.sides[side].units.iter().map(|u| u.uid).collect();
            for uid in uids {
                let Some(unit) = self.unit(side, uid) else {
                    continue;
                };
                for t in self.triggers(unit) {
                    if let Trigger::StartOfCombat(actions) = t {
                        for a in *actions {
                            self.act(side, Source::Unit(uid), None, None, a);
                        }
                    }
                }
            }
        }
        self.process_deaths(first);
    }

    /// The next minion of `side` that can attack (Attack above 0), from the
    /// pointer, wrapping to the left-most.
    fn attacker_index(&self, side: usize) -> Option<usize> {
        let s = &self.sides[side];
        let n = s.units.len();
        let start = if s.next < n { s.next } else { 0 };
        (0..n)
            .map(|k| (start + k) % n)
            .find(|&i| s.units[i].attack > 0)
    }

    /// One attack turn of `side` (two swings with Windfury, four with
    /// Mega-Windfury). False when no minion of `side` can attack.
    fn attack_turn(&mut self, side: usize) -> bool {
        let Some(ix) = self.attacker_index(side) else {
            return false;
        };
        let attacker = &self.sides[side].units[ix];
        let uid = attacker.uid;
        let swings = if attacker.kw.contains(Keywords::MEGA_WINDFURY) {
            4
        } else if attacker.kw.contains(Keywords::WINDFURY) {
            2
        } else {
            1
        };
        self.sides[side].next = ix + 1;
        for _ in 0..swings {
            if self.finished() {
                break;
            }
            match self.unit(side, uid) {
                Some(u) if u.attack > 0 => self.swing(side, uid),
                _ => break,
            }
        }
        true
    }

    fn is_taunt(&self, unit: &Unit) -> bool {
        unit.kw.contains(Keywords::TAUNT)
            || (unit.kw.contains(Keywords::DIVINE_SHIELD)
                && self
                    .triggers(unit)
                    .contains(&Trigger::TauntWhileDivineShield))
    }

    /// A random enemy to attack from [`Combat::target_pool`], unless the
    /// replay forces one.
    fn choose_target(&mut self, foe: usize) -> Option<u32> {
        if let Some(t) = self.forced_target.take() {
            return Some(t);
        }
        let pool = self.target_pool(foe);
        if pool.is_empty() {
            return None;
        }
        Some(pool[self.rng.below(pool.len())])
    }

    /// The enemies an attack may hit: the Taunt minions that are not
    /// Stealthed (Stealth hides its Taunt), else the minions that are not
    /// Stealthed, else all.
    fn target_pool(&self, foe: usize) -> Vec<u32> {
        let units = &self.sides[foe].units;
        let taunts: Vec<u32> = units
            .iter()
            .filter(|u| self.is_taunt(u) && !u.kw.contains(Keywords::STEALTH))
            .map(|u| u.uid)
            .collect();
        if !taunts.is_empty() {
            return taunts;
        }
        let open: Vec<u32> = units
            .iter()
            .filter(|u| !u.kw.contains(Keywords::STEALTH))
            .map(|u| u.uid)
            .collect();
        if open.is_empty() {
            units.iter().map(|u| u.uid).collect()
        } else {
            open
        }
    }

    fn swing(&mut self, side: usize, uid: u32) {
        let foe = 1 - side;
        let Some(target) = self.choose_target(foe) else {
            return;
        };
        if let Some(a) = self.unit_mut(side, uid) {
            a.kw = a.kw.without(Keywords::STEALTH);
        }
        self.before_attack(side, uid, foe, target);
        let (Some(a), Some(t)) = (self.unit(side, uid), self.unit(foe, target)) else {
            self.process_deaths(side);
            return;
        };
        if !a.alive() || !t.alive() {
            self.process_deaths(side);
            return;
        }
        let (a_atk, a_kw, t_atk, t_kw) = (a.attack, a.kw, t.attack, t.kw);
        let a_triggers = self.triggers(a);
        let (a_tribes, t_tribes) = (self.tribes(a), self.tribes(t));
        let t_health = t.health;
        let t_shield = t.kw.contains(Keywords::DIVINE_SHIELD);
        let t_ix = self.sides[foe].pos(target).unwrap_or(0);
        let neighbours = self.neighbours(foe, t_ix);

        // Both hits land at once.
        let killed_venom = self.damage(foe, target, a_atk, a_kw, uid);
        if a_triggers.contains(&Trigger::Cleave) {
            for n in &neighbours {
                self.damage(foe, *n, a_atk, a_kw, uid);
            }
        }
        let hit_back_venom = self.damage(side, uid, t_atk, t_kw, target);
        if killed_venom {
            self.lose_venomous(side, uid);
        }
        if hit_back_venom {
            self.lose_venomous(foe, target);
        }

        self.after_damage_dealt([
            (side, uid, a_tribes, a_atk > 0 && !t_shield),
            (
                foe,
                target,
                t_tribes,
                t_atk > 0 && !a_kw.contains(Keywords::DIVINE_SHIELD),
            ),
        ]);
        let attacker_alive = self.unit(side, uid).is_some_and(Unit::alive);
        if a_triggers.contains(&Trigger::ExcessToAdjacent) && !t_shield && a_atk > t_health {
            let open: Vec<u32> = neighbours
                .iter()
                .copied()
                .filter(|n| self.unit(foe, *n).is_some_and(Unit::alive))
                .collect();
            if !open.is_empty() {
                let n = open[self.rng.below(open.len())];
                self.damage(foe, n, a_atk.saturating_sub(t_health), Keywords::NONE, uid);
            }
        }
        for t in a_triggers {
            if let Trigger::AfterAttackHitHighestHealth { times } = t {
                if attacker_alive {
                    let atk = self.unit(side, uid).map_or(0, |u| u.attack);
                    if let Some(h) = self.highest_health(foe) {
                        self.damage(foe, h, atk.saturating_mul(*times), Keywords::NONE, uid);
                    }
                }
            }
        }
        if a_triggers.iter().any(|t| matches!(t, Trigger::Rally(_))) {
            self.after_rally_attack(side);
        }
        self.process_deaths(side);
    }

    /// Rally of the attacker, then "whenever another friendly minion
    /// attacks" of its friends, left to right.
    fn before_attack(&mut self, side: usize, uid: u32, foe: usize, target: u32) {
        let Some(attacker) = self.unit(side, uid) else {
            return;
        };
        let attacker_tribes = self.tribes(attacker);
        for t in self.triggers(attacker) {
            if let Trigger::Rally(actions) = t {
                for a in *actions {
                    self.act(side, Source::Unit(uid), Some((foe, target)), None, a);
                }
            }
        }
        let friends: Vec<u32> = self.sides[side].units.iter().map(|u| u.uid).collect();
        for f in friends {
            let Some(friend) = self.unit(side, f) else {
                continue;
            };
            for t in self.triggers(friend) {
                let (tribe, actions) = match *t {
                    Trigger::FriendlyAttacks { tribe, actions } if f != uid => (tribe, actions),
                    Trigger::WheneverFriendlyAttacks { tribe, actions } => (tribe, actions),
                    _ => continue,
                };
                if tribe == Tribes::NONE || attacker_tribes.is(tribe) {
                    for a in actions {
                        self.act(side, Source::Unit(f), Some((foe, target)), Some(uid), a);
                    }
                }
            }
        }
    }

    fn neighbours(&self, side: usize, ix: usize) -> Vec<u32> {
        let units = &self.sides[side].units;
        let mut out = Vec::new();
        if ix > 0 {
            out.push(units[ix - 1].uid);
        }
        if ix + 1 < units.len() {
            out.push(units[ix + 1].uid);
        }
        out
    }

    fn highest_health(&mut self, side: usize) -> Option<u32> {
        let alive: Vec<&Unit> = self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive())
            .collect();
        let top = alive.iter().map(|u| u.health).max()?;
        let ties: Vec<u32> = alive
            .iter()
            .filter(|u| u.health == top)
            .map(|u| u.uid)
            .collect();
        Some(ties[self.rng.below(ties.len())])
    }

    /// Deals `amount` to a minion. Divine Shield takes the whole hit. Returns
    /// true when the source's Venomous (not Poisonous) killed it.
    fn damage(&mut self, side: usize, uid: u32, amount: i32, source: Keywords, by: u32) -> bool {
        if amount <= 0 {
            return false;
        }
        let Some(u) = self.unit_mut(side, uid) else {
            return false;
        };
        u.hit_by = Some(by);
        if u.kw.contains(Keywords::DIVINE_SHIELD) {
            u.kw = u.kw.without(Keywords::DIVINE_SHIELD);
            return false;
        }
        u.health = u.health.saturating_sub(amount);
        let poisonous = source.contains(Keywords::POISONOUS);
        let venomous = source.contains(Keywords::VENOMOUS);
        if (poisonous || venomous) && u.health > 0 {
            u.health = 0;
        }
        venomous && !poisonous
    }

    fn lose_venomous(&mut self, side: usize, uid: u32) {
        if let Some(u) = self.unit_mut(side, uid) {
            u.kw = u.kw.without(Keywords::VENOMOUS);
        }
    }

    /// Gives `uid` stats, then checks "once this reaches N Attack".
    fn buff(&mut self, side: usize, uid: u32, attack: i32, health: i32) {
        let Some(u) = self.unit_mut(side, uid) else {
            return;
        };
        u.attack = u.attack.saturating_add(attack);
        u.health = u.health.saturating_add(health);
        let (atk, done, card) = (u.attack, u.shield_done, u.card);
        if done {
            return;
        }
        let reaches = self.p.cards[card as usize]
            .triggers
            .iter()
            .any(|t| matches!(t, Trigger::ShieldAtAttack(n) if atk >= *n));
        if reaches {
            if let Some(u) = self.unit_mut(side, uid) {
                u.kw = u.kw.union(Keywords::DIVINE_SHIELD);
                u.shield_done = true;
            }
        }
    }

    /// Living friendly minions of `tribe` (any when NONE), without `skip`.
    fn friends(&self, side: usize, tribe: Tribes, skip: Option<u32>) -> Vec<u32> {
        self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive() && Some(u.uid) != skip)
            .filter(|u| tribe == Tribes::NONE || self.tribes(u).is(tribe))
            .map(|u| u.uid)
            .collect()
    }

    fn pick(&mut self, mut pool: Vec<u32>, count: u8) -> Vec<u32> {
        let mut out = Vec::new();
        while out.len() < usize::from(count) && !pool.is_empty() {
            out.push(pool.swap_remove(self.rng.below(pool.len())));
        }
        out
    }

    fn act(
        &mut self,
        side: usize,
        src: Source,
        target: Option<(usize, u32)>,
        attacker: Option<u32>,
        action: &Action,
    ) {
        let (me, killer) = match src {
            Source::Unit(uid) => (Some(uid), None),
            Source::Dead { killer, .. } => (None, killer),
        };
        match *action {
            Action::Summon { token, count } => self.summon(side, src, token, count),
            Action::BuffAll {
                attack,
                health,
                tribe,
                others,
            } => {
                let skip = if others { me } else { None };
                for uid in self.friends(side, tribe, skip) {
                    self.buff(side, uid, attack, health);
                }
            }
            Action::BuffRandom {
                attack,
                health,
                tribe,
                count,
            } => {
                let pool = self.friends(side, tribe, me);
                for uid in self.pick(pool, count) {
                    self.buff(side, uid, attack, health);
                }
            }
            Action::GainStats { attack, health } => {
                if let Some(uid) = me {
                    self.buff(side, uid, attack, health);
                }
            }
            Action::GainKeyword(k) => {
                if let Some(u) = me.and_then(|uid| self.unit_mut(side, uid)) {
                    u.kw = u.kw.union(k);
                }
            }
            Action::GiveKeyword {
                keyword,
                tribe,
                count,
            } => {
                let pool: Vec<u32> = self
                    .friends(side, tribe, me)
                    .into_iter()
                    .filter(|uid| {
                        self.unit(side, *uid)
                            .is_some_and(|u| !u.kw.contains(keyword))
                    })
                    .collect();
                for uid in self.pick(pool, count) {
                    if let Some(u) = self.unit_mut(side, uid) {
                        u.kw = u.kw.union(keyword);
                    }
                }
            }
            Action::DamageTargetAndAdjacent => {
                let (Some(uid), Some((foe, t))) = (me, target) else {
                    return;
                };
                let atk = self.unit(side, uid).map_or(0, |u| u.attack);
                let Some(t_ix) = self.sides[foe].pos(t) else {
                    return;
                };
                let adjacent = self.neighbours(foe, t_ix);
                self.damage(foe, t, atk, Keywords::NONE, uid);
                if !adjacent.is_empty() {
                    let n = adjacent[self.rng.below(adjacent.len())];
                    self.damage(foe, n, atk, Keywords::NONE, uid);
                }
            }
            Action::GainTargetAttack { times } => {
                let (Some(uid), Some((foe, t))) = (me, target) else {
                    return;
                };
                let gain = self
                    .unit(foe, t)
                    .map_or(0, |u| u.attack)
                    .saturating_mul(times);
                self.buff(side, uid, gain, 0);
            }
            Action::StripTarget(k) => {
                if let Some(u) = target.and_then(|(foe, t)| self.unit_mut(foe, t)) {
                    u.kw = u.kw.without(k);
                }
            }
            Action::DestroyKiller => {
                if let Some(u) = killer.and_then(|k| self.unit_mut(1 - side, k)) {
                    u.health = u.health.min(0);
                }
            }
            Action::BuffAttacker { attack, health } => {
                if let Some(a) = attacker {
                    self.buff(side, a, attack, health);
                }
            }
            Action::BuffAllWithKeyword {
                attack,
                health,
                keyword,
            } => {
                let shielded: Vec<u32> = self
                    .friends(side, Tribes::NONE, None)
                    .into_iter()
                    .filter(|f| self.unit(side, *f).is_some_and(|u| u.kw.contains(keyword)))
                    .collect();
                for f in shielded {
                    self.buff(side, f, attack, health);
                }
            }
            Action::SummonAndAttack { token } => self.summon_and_attack(side, src, token),
            Action::BuffOnePerTribe { attack, health } => {
                self.buff_one_per_tribe(side, attack, health)
            }
            // Needs the reborn minion: played by `after_reborn`.
            Action::GiveRebornAttackToRightmost { .. } => {}
        }
    }

    fn summon(&mut self, side: usize, src: Source, token: &str, count: u8) {
        // Every token a card can summon is entered in `Prepared::new`, with
        // its printed facts; a test in `cards` checks the token table is complete.
        let Some(&(_, card)) = self.p.tokens.iter().find(|(id, _)| *id == token) else {
            debug_assert!(false, "token {token} was not prepared");
            return;
        };
        let Some(base) = self.p.cards[card as usize].base else {
            return;
        };
        let at = match src {
            Source::Unit(uid) => self.sides[side].pos(uid).map_or(0, |i| i + 1),
            Source::Dead { anchor, .. } => self.anchor_pos(side, anchor),
        };
        for k in 0..usize::from(count) {
            let unit = self.fresh(card, base.attack, base.health, base.keywords);
            if !self.sides[side].insert(at + k, unit) {
                break;
            }
        }
    }

    fn anchor_pos(&self, side: usize, anchor: Option<u32>) -> usize {
        anchor
            .and_then(|a| self.sides[side].pos(a))
            .unwrap_or(self.sides[side].units.len())
    }

    fn fresh(&mut self, card: u16, attack: i32, health: i32, kw: Keywords) -> Unit {
        self.next_uid += 1;
        let mut unit = Unit::new(self.next_uid, card, attack, health, kw);
        unit.shield_done = self.p.cards[card as usize]
            .triggers
            .iter()
            .any(|t| matches!(t, Trigger::ShieldAtAttack(n) if attack >= *n));
        unit
    }

    fn extra_deathrattles(&self, side: usize) -> u8 {
        self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive())
            .flat_map(|u| self.triggers(u).iter())
            .map(|t| match t {
                Trigger::ExtraDeathrattles(n) => *n,
                _ => 0,
            })
            .sum()
    }

    /// Removes the dead and resolves what their deaths cause, until nothing
    /// else dies: deathrattles (the attacking side first, each side left to
    /// right), then Avenge, then Reborn.
    fn process_deaths(&mut self, first: usize) {
        loop {
            let mut dead: Vec<Dead> = Vec::new();
            for side in [first, 1 - first] {
                let units = &self.sides[side].units;
                let mut found = Vec::new();
                for (i, u) in units.iter().enumerate() {
                    if !u.alive() {
                        let anchor = units[i + 1..].iter().find(|v| v.alive()).map(|v| v.uid);
                        found.push((i, anchor));
                    }
                }
                for &(i, anchor) in found.iter().rev() {
                    let unit = self.sides[side].remove(i);
                    dead.push(Dead { side, unit, anchor });
                }
                // Removed right to left; put this side's dead back in order.
                let start = dead.len() - found.len();
                dead[start..].reverse();
            }
            if dead.is_empty() {
                return;
            }
            for d in &dead {
                let times = 1 + usize::from(self.extra_deathrattles(d.side));
                let src = Source::Dead {
                    anchor: d.anchor,
                    killer: d.unit.hit_by,
                };
                for _ in 0..times {
                    for t in self.triggers(&d.unit) {
                        if let Trigger::Deathrattle(actions) = t {
                            for a in *actions {
                                self.act(d.side, src, None, None, a);
                            }
                        }
                    }
                }
            }
            for d in &dead {
                self.avenge(d.side);
            }
            for d in &dead {
                if d.unit.kw.contains(Keywords::REBORN) {
                    self.reborn(d);
                }
            }
        }
    }

    fn avenge(&mut self, side: usize) {
        let uids: Vec<u32> = self.friends(side, Tribes::NONE, None);
        for uid in uids {
            let Some(unit) = self.unit(side, uid) else {
                continue;
            };
            for t in self.triggers(unit) {
                if let Trigger::Avenge(n, actions) = t {
                    let Some(u) = self.unit_mut(side, uid) else {
                        continue;
                    };
                    u.avenge += 1;
                    if u.avenge >= *n {
                        u.avenge = 0;
                        for a in *actions {
                            self.act(side, Source::Unit(uid), None, None, a);
                        }
                    }
                }
            }
        }
    }

    /// The minion comes back where it died as a fresh copy of its card with
    /// 1 Health and no Reborn. Without the card's stats, its last Attack and
    /// keywords are used (an approximation the caller reports).
    fn reborn(&mut self, d: &Dead) {
        let card = d.unit.card;
        let (attack, kw) = match self.p.cards[card as usize].base {
            Some(b) => (b.attack, b.keywords),
            None => (d.unit.attack, d.unit.kw),
        };
        let unit = self.fresh(card, attack, 1, kw.without(Keywords::REBORN));
        let uid = unit.uid;
        let at = self.anchor_pos(d.side, d.anchor);
        if self.sides[d.side].insert(at, unit) {
            self.after_reborn(d.side, uid);
        }
    }
}

#[path = "replay.rs"]
pub mod replay;

#[path = "combat_round2.rs"]
mod round2;

#[cfg(test)]
#[path = "combat_tests.rs"]
mod tests;
