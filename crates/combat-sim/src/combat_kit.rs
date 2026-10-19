//! Hero powers and trinkets in combat (T-628 rounds 2 and 3): the actions of
//! [`crate::kit_effects`] at the start of combat, played before the minions'
//! (the user's logs: trinkets, then hero powers, then minions), and what the
//! played hero powers do once their side has space.

use super::{intern, CardInfo, Combat, Unit};
use crate::gifts;
use crate::kit_effects::{self, KitAction, KitPlay, Pick, SpaceAction};
use crate::model::{Keywords, Kit, Minion, Tribes, MAX_BOARD};

/// A hero power or trinket the engine plays, as one board brings it.
#[derive(Clone, Debug)]
pub(crate) struct KitUse {
    pub power: bool,
    pub counters: [i32; 5],
    pub play: &'static KitPlay,
    /// The uid of the friendly minion it names, from its index on the board
    /// as it was taken.
    pub target: Option<u32>,
    /// The minion it holds, in the engine's form (its uid is not used).
    pub held: Option<Unit>,
    /// It acts this combat ([`kit_effects::acts`]; false when unknown).
    pub acts: bool,
    /// The card entry of the mark it gives a minion ([`kit_effects::mark`]).
    pub mark: Option<u16>,
}

/// What a hero power does once its side has space, waiting for it.
#[derive(Clone, Debug)]
pub(super) enum Pending {
    /// Copy the highest-Attack (or Health) friendly minion.
    Copy { health: bool },
    /// Summon this held minion at the right end; it attacks at once.
    Fire(Unit),
    /// Summon this exact copy back at board index `at`.
    Resummon { unit: Unit, at: usize },
}

/// A board's played powers and trinkets: trinkets first, then hero powers.
/// `held` turns a held minion into the engine's form.
pub(crate) fn kit_uses(
    kit: Option<&Kit>,
    board: &[Unit],
    cards: &mut Vec<CardInfo>,
    held: impl Fn(&Minion, &mut Vec<CardInfo>) -> Unit,
) -> Vec<KitUse> {
    let Some(kit) = kit else {
        return Vec::new();
    };
    let tagged = kit
        .trinkets
        .iter()
        .map(|c| (false, c))
        .chain(kit.powers.iter().map(|c| (true, c)));
    let mut out = Vec::new();
    for (power, c) in tagged {
        let Some(play) = kit_effects::played(&c.card_id) else {
            continue;
        };
        out.push(KitUse {
            power,
            counters: c.counters,
            play,
            target: c.target.and_then(|ix| board.get(ix)).map(|u| u.uid),
            held: c.held.as_ref().map(|m| held(m, cards)),
            acts: kit_effects::acts(play, &c.counters, kit.round).unwrap_or(false),
            mark: kit_effects::mark(play).and_then(|m| mark_card(cards, m)),
        });
    }
    out
}

/// The card entry of a mark: its triggers only, which a minion gains with it.
fn mark_card(cards: &mut Vec<CardInfo>, mark: &str) -> Option<u16> {
    let gift = gifts::gift(mark)?;
    let next = cards.len() as u16;
    Some(intern(cards, gift.id, &[], || CardInfo {
        id: gift.id.to_string(),
        gifts: Vec::new(),
        plain: next,
        triggers: gift.triggers.to_vec(),
        tier: None,
        tribes: Tribes::NONE,
        base: None,
    }))
}

impl Combat<'_> {
    /// The powers and trinkets of the board `side` fights with now.
    fn kit_now(&self, side: usize) -> &[KitUse] {
        let second = self.p.second[side].is_some() && !self.legs.waiting[side];
        if second {
            &self.p.second_kits[side]
        } else {
            &self.p.kits[side]
        }
    }

    /// Plays `side`'s trinkets (`powers` false) or hero powers, the deaths
    /// each causes resolved before the next (`first` attacks first); a hero
    /// power that acts when there is space acts then if there is.
    pub(super) fn play_kit(&mut self, side: usize, powers: bool, first: usize) {
        let uses: Vec<KitUse> = self
            .kit_now(side)
            .iter()
            .filter(|k| k.power == powers && k.acts)
            .cloned()
            .collect();
        for k in uses {
            for action in k.play.start {
                self.kit_act(side, &k, action);
            }
            let pending = match (k.play.space, k.held) {
                (Some(SpaceAction::CopyHighest { health, .. }), _) => {
                    Some(Pending::Copy { health })
                }
                (Some(SpaceAction::FireHeld), Some(h)) => Some(Pending::Fire(h)),
                _ => None,
            };
            if let Some(p) = pending {
                self.pending[side].push(p);
            }
            self.process_deaths(first);
        }
    }

    fn kit_act(&mut self, side: usize, k: &KitUse, action: &KitAction) {
        let value = |a| kit_effects::value(a, &k.counters);
        match *action {
            KitAction::Buff { to, attack, health } => {
                for uid in self.picked(side, to) {
                    self.buff(side, uid, value(attack), value(health));
                }
            }
            KitAction::BuffOnePerTribe(a) => {
                let n = value(a);
                self.buff_one_per_tribe(side, n, n);
            }
            KitAction::Keywords { to, keywords } => {
                for uid in self.picked(side, to) {
                    if let Some(u) = self.unit_mut(side, uid) {
                        u.kw = u.kw.union(keywords);
                    }
                }
            }
            KitAction::DoubleAttack(to) => {
                for uid in self.picked(side, to) {
                    let gain = self.unit(side, uid).map_or(0, |u| u.attack);
                    self.buff(side, uid, gain, 0);
                }
            }
            KitAction::DoubleStats(to) => {
                for uid in self.picked(side, to) {
                    let (a, h) = self
                        .unit(side, uid)
                        .map_or((0, 0), |u| (u.attack, u.health));
                    self.buff(side, uid, a, h);
                }
            }
            KitAction::HealthFromHalfAttack(to) => {
                for uid in self.picked(side, to) {
                    let gain = self.unit(side, uid).map_or(0, |u| u.attack / 2);
                    self.buff(side, uid, 0, gain);
                }
            }
            KitAction::DamageRandomEnemies { amount, count } => {
                let foe = 1 - side;
                for _ in 0..count {
                    let pool = self.friends(foe, Tribes::NONE, None);
                    if let Some(&t) = self.pick(pool, 1).first() {
                        self.damage(foe, t, amount, Keywords::NONE, 0);
                    }
                }
            }
            KitAction::TriggerDeathrattles => {
                for uid in self.friends(side, Tribes::NONE, None) {
                    self.trigger_own_deathrattles(side, uid);
                }
            }
            KitAction::AttackImmediately(to) => {
                for uid in self.picked(side, to) {
                    if self.finished() || self.unit(side, uid).is_none_or(|u| u.attack <= 0) {
                        continue;
                    }
                    self.swing(side, uid);
                }
            }
            KitAction::Mark { to, mark } => {
                let Some(card) = k.mark else {
                    return;
                };
                // A board that already shows the mark had this start of
                // combat: it is not given twice.
                if self.shows_mark(side, mark) {
                    return;
                }
                for uid in self.picked(side, to) {
                    if let Some(u) = self.unit_mut(side, uid) {
                        u.gained.push(card);
                    }
                }
            }
            KitAction::DestroyTarget => self.destroy_target(side, k.target),
            KitAction::TransformTargetIntoTeammates => {
                let Some(uid) = self.target_uid(side, k.target) else {
                    return;
                };
                if let Some(model) = self.teammates_highest_tier(side) {
                    self.become_copy(side, uid, &model);
                }
            }
        }
    }

    /// True when a minion of `side` carries `mark` as one of its gifts.
    fn shows_mark(&self, side: usize, mark: &str) -> bool {
        self.sides[side]
            .units
            .iter()
            .any(|u| self.p.cards[u.card as usize].gifts.contains(&mark))
    }

    /// The named minion, when it is still on the board and alive.
    fn target_uid(&self, side: usize, target: Option<u32>) -> Option<u32> {
        self.unit(side, target?)
            .filter(|u| u.alive())
            .map(|u| u.uid)
    }

    /// Destroys the named minion and keeps its exact copy, summoned back at
    /// its place once there is space (after its death is resolved).
    fn destroy_target(&mut self, side: usize, target: Option<u32>) {
        let Some(uid) = self.target_uid(side, target) else {
            return;
        };
        let at = self.sides[side].pos(uid).unwrap_or(0);
        let Some(u) = self.unit_mut(side, uid) else {
            return;
        };
        let unit = u.clone();
        u.health = u.health.min(0);
        self.pending[side].push(Pending::Resummon { unit, at });
    }

    /// Plays what waits for space on `side`, oldest first, while it has
    /// space and the combat goes on.
    pub(super) fn fill_space(&mut self, side: usize) {
        while self.sides[side].units.len() < MAX_BOARD
            && !self.pending[side].is_empty()
            && !self.finished()
        {
            match self.pending[side].remove(0) {
                Pending::Copy { health } => self.copy_highest(side, health),
                Pending::Fire(held) => {
                    let unit = self.copy_of(&held);
                    let uid = unit.uid;
                    let end = self.sides[side].units.len();
                    if self.sides[side].insert(end, unit) {
                        self.after_summon(side, uid);
                        if !self.finished() && self.unit(side, uid).is_some_and(|u| u.attack > 0) {
                            self.swing(side, uid);
                        }
                    }
                }
                Pending::Resummon { unit, at } => {
                    let unit = self.copy_of(&unit);
                    let uid = unit.uid;
                    if self.sides[side].insert(at, unit) {
                        self.after_summon(side, uid);
                    }
                }
            }
        }
    }

    /// Both sides' waiting powers, the first attacker's side first.
    pub(super) fn fill_both(&mut self, first: usize) {
        for side in [first, 1 - first] {
            if !self.pending[side].is_empty() {
                self.fill_space(side);
            }
        }
    }

    /// Summons an exact copy of the friendly minion with the highest Attack
    /// (or Health), the left-most on ties, to its right.
    fn copy_highest(&mut self, side: usize, health: bool) {
        let mut best: Option<(i32, usize)> = None;
        for (ix, u) in self.sides[side].units.iter().enumerate() {
            let v = if health { u.health } else { u.attack };
            if u.alive() && best.is_none_or(|(b, _)| v > b) {
                best = Some((v, ix));
            }
        }
        let Some((_, ix)) = best else {
            return;
        };
        let model = self.sides[side].units[ix].clone();
        let copy = self.copy_of(&model);
        let uid = copy.uid;
        if self.sides[side].insert(ix + 1, copy) {
            self.after_summon(side, uid);
        }
    }

    /// The living friendly minions `pick` names, left to right.
    fn picked(&mut self, side: usize, pick: Pick) -> Vec<u32> {
        let alive = self.friends(side, Tribes::NONE, None);
        match pick {
            Pick::LeftMost => alive.first().copied().into_iter().collect(),
            Pick::RightMost => alive.last().copied().into_iter().collect(),
            Pick::LeftAndRightMost => {
                let mut ends: Vec<u32> = alive.first().copied().into_iter().collect();
                if let Some(&last) = alive.last() {
                    if !ends.contains(&last) {
                        ends.push(last);
                    }
                }
                ends
            }
            Pick::Random { tribe, count } => {
                let pool = self.friends(side, tribe, None);
                self.pick(pool, count)
            }
            Pick::Cards(ids) => alive
                .into_iter()
                .filter(|uid| {
                    self.unit(side, *uid)
                        .is_some_and(|u| ids.contains(&self.p.cards[u.card as usize].id.as_str()))
                })
                .collect(),
            Pick::All => alive,
            Pick::LowestAttack(count) => {
                let mut by_attack: Vec<(i32, usize, u32)> = alive
                    .iter()
                    .enumerate()
                    .filter_map(|(ix, uid)| self.unit(side, *uid).map(|u| (u.attack, ix, *uid)))
                    .collect();
                by_attack.sort_unstable();
                by_attack
                    .into_iter()
                    .take(usize::from(count))
                    .map(|(_, _, uid)| uid)
                    .collect()
            }
        }
    }

    /// True when the board `side` fights with now has a power or trinket
    /// whose minions attack at the start of combat (Wingmen; Lock and Load
    /// when its side has space for the minion it fires).
    pub(super) fn kit_swings(&self, side: usize) -> bool {
        let space = self.sides[side].units.len() < MAX_BOARD;
        self.kit_now(side).iter().filter(|k| k.acts).any(|k| {
            let fires = space && k.held.is_some() && k.play.space == Some(SpaceAction::FireHeld);
            fires
                || k.play
                    .start
                    .iter()
                    .any(|a| matches!(a, KitAction::AttackImmediately(_)))
        })
    }

    /// True when the board `side` fights with now plays a power or trinket
    /// at the start of combat.
    pub(super) fn has_kit_start(&self, side: usize) -> bool {
        self.kit_now(side).iter().any(|k| k.acts)
    }
}
