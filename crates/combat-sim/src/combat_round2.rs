//! The effects card coverage round 2 added (T-604, session 071): Reborn
//! followers, damage dealt by a friend, a Rally attack that triggers a
//! Deathrattle, a summon that attacks at once and one buff per type.

use super::{Combat, Source, Unit};
use crate::cards::{Action, Trigger};
use crate::model::Tribes;

/// The minion types a "one of each type" effect walks, in this order.
const TYPES: [Tribes; 11] = [
    Tribes::BEAST,
    Tribes::DEMON,
    Tribes::DRAGON,
    Tribes::ELEMENTAL,
    Tribes::MECH,
    Tribes::MURLOC,
    Tribes::NAGA,
    Tribes::PIRATE,
    Tribes::QUILBOAR,
    Tribes::UNDEAD,
    Tribes::ABERRATION,
];

impl Combat<'_> {
    /// Summons one token from `src` and has it attack at once.
    pub(super) fn summon_and_attack(&mut self, side: usize, src: Source, token: &str) {
        let before = self.next_uid;
        self.summon(side, src, token, 1);
        let uid = self.next_uid;
        if uid != before && self.unit(side, uid).is_some() && !self.finished() {
            self.swing(side, uid);
        }
    }

    pub(super) fn buff_one_per_tribe(&mut self, side: usize, attack: i32, health: i32) {
        for tribe in TYPES {
            let pool = self.friends(side, tribe, None);
            for uid in self.pick(pool, 1) {
                self.buff(side, uid, attack, health);
            }
        }
    }

    /// "After another friendly minion of a type deals damage": each entry is
    /// (side, the minion, its types, whether its hit got through).
    pub(super) fn after_damage_dealt(&mut self, hits: [(usize, u32, Tribes, bool); 2]) {
        for (side, dealer, tribes, dealt) in hits {
            if !dealt {
                continue;
            }
            let friends: Vec<u32> = self.sides[side]
                .units
                .iter()
                .filter(|u| u.uid != dealer)
                .map(|u| u.uid)
                .collect();
            for f in friends {
                let Some(unit) = self.unit(side, f) else {
                    continue;
                };
                for t in self.triggers(unit) {
                    if let Trigger::AfterFriendlyDealsDamage { tribe, actions } = *t {
                        if tribe == Tribes::NONE || tribes.is(tribe) {
                            for a in actions {
                                self.act(side, Source::Unit(f), None, None, a);
                            }
                        }
                    }
                }
            }
        }
    }

    /// After a friendly Rally minion attacks: the left-most friendly
    /// Deathrattle, once per trigger and `times` each.
    pub(super) fn after_rally_attack(&mut self, side: usize) {
        let times: u32 = self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive())
            .flat_map(|u| self.triggers(u).iter())
            .map(|t| match t {
                Trigger::AfterFriendlyRallyAttacks { times } => u32::from(*times),
                _ => 0,
            })
            .sum();
        if times == 0 {
            return;
        }
        let has_deathrattle =
            |c: &Self, u: &Unit| c.triggers(u).iter().any(|t| matches!(t, Trigger::Deathrattle(_)));
        let Some(uid) = self.sides[side]
            .units
            .iter()
            .find(|u| u.alive() && has_deathrattle(self, u))
            .map(|u| u.uid)
        else {
            return;
        };
        for _ in 0..times {
            let Some(unit) = self.unit(side, uid) else {
                return;
            };
            for t in self.triggers(unit) {
                if let Trigger::Deathrattle(actions) = t {
                    for a in *actions {
                        self.act(side, Source::Unit(uid), None, None, a);
                    }
                }
            }
        }
    }

    /// After a friendly minion comes back with Reborn.
    pub(super) fn after_reborn(&mut self, side: usize, reborn: u32) {
        let attack = self.unit(side, reborn).map_or(0, |u| u.attack);
        let friends: Vec<u32> = self.sides[side].units.iter().map(|u| u.uid).collect();
        for f in friends {
            let Some(unit) = self.unit(side, f) else {
                continue;
            };
            for t in self.triggers(unit) {
                let Trigger::AfterFriendlyReborn(actions) = t else {
                    continue;
                };
                for a in *actions {
                    match *a {
                        Action::GiveRebornAttackToRightmost { tribe, times } => {
                            let gain = attack.saturating_mul(times);
                            if let Some(&target) = self.friends(side, tribe, None).last() {
                                self.buff(side, target, gain, gain);
                            }
                        }
                        _ => self.act(side, Source::Unit(f), None, None, a),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "combat_round2_tests.rs"]
mod tests;
