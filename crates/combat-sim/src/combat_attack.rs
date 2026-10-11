//! Attacks: who attacks next, whom, and the damage of a swing (Taunt,
//! Stealth, Divine Shield, Venomous, Cleave, Windfury).

use super::{Combat, Source, Unit};
use crate::cards::Trigger;
use crate::model::{Keywords, Tribes};

impl Combat<'_> {
    /// The next minion of `side` that can attack (Attack above 0), from the
    /// pointer, wrapping to the left-most.
    pub(super) fn attacker_index(&self, side: usize) -> Option<usize> {
        let s = &self.sides[side];
        let n = s.units.len();
        let start = if s.next < n { s.next } else { 0 };
        (0..n)
            .map(|k| (start + k) % n)
            .find(|&i| s.units[i].attack > 0)
    }

    /// One attack turn of `side` (two swings with Windfury, four with
    /// Mega-Windfury). False when no minion of `side` can attack.
    pub(super) fn attack_turn(&mut self, side: usize) -> bool {
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

    pub(super) fn is_taunt(&self, unit: &Unit) -> bool {
        unit.kw.contains(Keywords::TAUNT)
            || (unit.kw.contains(Keywords::DIVINE_SHIELD)
                && self
                    .triggers(unit)
                    .contains(&Trigger::TauntWhileDivineShield))
    }

    /// A random enemy to attack from [`Combat::target_pool`], unless the
    /// replay forces one.
    pub(super) fn choose_target(&mut self, foe: usize) -> Option<u32> {
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
    pub(super) fn target_pool(&self, foe: usize) -> Vec<u32> {
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

    pub(super) fn swing(&mut self, side: usize, uid: u32) {
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
    pub(super) fn before_attack(&mut self, side: usize, uid: u32, foe: usize, target: u32) {
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

    pub(super) fn neighbours(&self, side: usize, ix: usize) -> Vec<u32> {
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

    pub(super) fn highest_health(&mut self, side: usize) -> Option<u32> {
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
    pub(super) fn damage(
        &mut self,
        side: usize,
        uid: u32,
        amount: i32,
        source: Keywords,
        by: u32,
    ) -> bool {
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

    pub(super) fn lose_venomous(&mut self, side: usize, uid: u32) {
        if let Some(u) = self.unit_mut(side, uid) {
            u.kw = u.kw.without(Keywords::VENOMOUS);
        }
    }
}
