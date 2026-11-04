//! Hero powers and trinkets during combat (T-628 round 4): what
//! [`crate::kit_passive`] plays, hooked to kills, attacks, deaths and
//! summons of the board each hero's powers and trinkets belong to.

use super::{Combat, Dead, Source, Unit};
use crate::cards::Trigger;
use crate::kit_passive::KitTrigger;
use crate::model::Tribes;

/// What the powers and trinkets of the board fighting now counted this
/// combat (a Duos board that comes in starts again).
#[derive(Clone, Debug, Default)]
pub(crate) struct KitCounts {
    /// Friendly attacks.
    attacks: u32,
    /// Friendly deaths, for every Avenge (each fires on its multiples).
    deaths: u32,
    /// The first Deathrattle's extra trigger is used.
    first_deathrattle: bool,
    /// Times each limited one acted, by its index among the board's.
    uses: Vec<u32>,
    /// What the side's Eternal Knights gained this combat (Attack, Health).
    pub(super) knight_gain: (i32, i32),
}

impl Combat<'_> {
    /// The played powers and trinkets during combat of the board `side`
    /// fights with now.
    fn passives_now(&self, side: usize) -> &[KitTrigger] {
        let second = self.p.second[side].is_some() && !self.legs.waiting[side];
        if second {
            &self.p.second_passives[side]
        } else {
            &self.p.passives[side]
        }
    }

    /// A minion of `side` died, last hit by `killer` (an enemy's uid): the
    /// enemy's "after a friendly minion kills" powers act on the killer.
    pub(super) fn kit_kill(&mut self, side: usize, killer: Option<u32>) {
        let foe = 1 - side;
        let Some(killer) = killer.filter(|k| self.unit(foe, *k).is_some_and(Unit::alive)) else {
            return;
        };
        let gains: Vec<i32> = self
            .passives_now(foe)
            .iter()
            .filter_map(|t| match *t {
                KitTrigger::KillGainsAttack(n) => Some(n),
                _ => None,
            })
            .collect();
        for n in gains {
            self.buff(foe, killer, n, 0);
        }
    }

    /// After a friendly attack: Blood Gems every so many attacks.
    pub(super) fn kit_after_attack(&mut self, side: usize) {
        if self.passives_now(side).is_empty() {
            return;
        }
        self.kit_counts[side].attacks += 1;
        let done = self.kit_counts[side].attacks;
        let every: Vec<u32> = self
            .passives_now(side)
            .iter()
            .filter_map(|t| match *t {
                KitTrigger::GemsEveryAttacks(n) if n > 0 && done.is_multiple_of(n) => Some(n),
                _ => None,
            })
            .collect();
        for _ in every {
            let (attack, health) = self.gem_stats(side, 1);
            for uid in self.friends(side, Tribes::QUILBOAR, None) {
                self.buff(side, uid, attack, health);
            }
        }
    }

    /// Before a friendly attack hits: what the side's powers and trinkets
    /// give an attacker of their minion type.
    pub(super) fn kit_before_attack(&mut self, side: usize, uid: u32) {
        let Some(tribes) = self.unit(side, uid).map(|u| self.tribes(u)) else {
            return;
        };
        let passives: Vec<KitTrigger> = self.passives_now(side).to_vec();
        for (ix, t) in passives.iter().enumerate() {
            let KitTrigger::TribeAttacks {
                tribe,
                attack,
                health,
                keywords,
                limit,
            } = *t
            else {
                continue;
            };
            if !tribes.is(tribe) {
                continue;
            }
            let counts = &mut self.kit_counts[side];
            if counts.uses.len() <= ix {
                counts.uses.resize(ix + 1, 0);
            }
            if limit > 0 && counts.uses[ix] >= limit {
                continue;
            }
            counts.uses[ix] += 1;
            self.buff(side, uid, attack, health);
            if let Some(u) = self.unit_mut(side, uid) {
                u.kw = u.kw.union(keywords);
            }
        }
    }

    /// True when the side's first Deathrattle this combat is `unit`'s and a
    /// power or trinket makes it trigger an extra time (it is used then).
    pub(super) fn kit_first_deathrattle(&mut self, side: usize, unit: &Unit) -> bool {
        let twice = self
            .passives_now(side)
            .contains(&KitTrigger::FirstDeathrattleTwice);
        if !twice || self.kit_counts[side].first_deathrattle {
            return false;
        }
        let cards = std::iter::once(unit.card).chain(unit.gained.iter().copied());
        let has = cards
            .flat_map(|c| self.p.cards[c as usize].triggers.iter())
            .any(|t| matches!(t, Trigger::Deathrattle(_)));
        if has {
            self.kit_counts[side].first_deathrattle = true;
        }
        has
    }

    /// Minions died: on each side, every friendly Undead among them makes
    /// the side's Eternal Knights grow (Eternal Portrait).
    pub(super) fn kit_undead_died(&mut self, dead: &[Dead]) {
        for side in [0, 1] {
            let gains: Vec<(i32, i32)> = self
                .passives_now(side)
                .iter()
                .filter_map(|t| match *t {
                    KitTrigger::UndeadDeathBuffsKnights { attack, health } => {
                        Some((attack, health))
                    }
                    _ => None,
                })
                .collect();
            if gains.is_empty() {
                continue;
            }
            let undead = dead
                .iter()
                .filter(|d| d.side == side && self.tribes(&d.unit).is(Tribes::UNDEAD))
                .count();
            for _ in 0..undead {
                for &(attack, health) in &gains {
                    let gain = &mut self.kit_counts[side].knight_gain;
                    *gain = (gain.0.saturating_add(attack), gain.1.saturating_add(health));
                    for uid in self.knights(side) {
                        self.buff(side, uid, attack, health);
                    }
                }
            }
        }
    }

    /// A friendly minion of `side` died: the powers' and trinkets' Avenge.
    pub(super) fn kit_avenge(&mut self, side: usize) {
        if self.passives_now(side).is_empty() {
            return;
        }
        self.kit_counts[side].deaths += 1;
        let deaths = self.kit_counts[side].deaths;
        let passives: Vec<KitTrigger> = self.passives_now(side).to_vec();
        for t in passives {
            match t {
                KitTrigger::AvengeKeyword { n, tribe, keyword }
                    if n > 0 && deaths.is_multiple_of(n) =>
                {
                    let pool: Vec<u32> = self
                        .friends(side, tribe, None)
                        .into_iter()
                        .filter(|u| self.unit(side, *u).is_some_and(|u| !u.kw.contains(keyword)))
                        .collect();
                    if let Some(&uid) = self.pick(pool, 1).first() {
                        if let Some(u) = self.unit_mut(side, uid) {
                            u.kw = u.kw.union(keyword);
                        }
                    }
                }
                KitTrigger::AvengeBuffAll { n, attack, health }
                    if n > 0 && deaths.is_multiple_of(n) =>
                {
                    for uid in self.friends(side, Tribes::NONE, None) {
                        self.buff(side, uid, attack, health);
                    }
                }
                KitTrigger::AvengeSummon {
                    n,
                    token,
                    count,
                    keywords,
                } if n > 0 && deaths.is_multiple_of(n) => {
                    // At the right end: summoned by no minion.
                    let src = Source::Dead {
                        uid: 0,
                        anchor: None,
                        killer: None,
                        stats: (0, 0),
                    };
                    let before: Vec<u32> = self.sides[side].units.iter().map(|u| u.uid).collect();
                    self.summon(side, src, token, count);
                    for u in self.sides[side].units.iter_mut() {
                        if !before.contains(&u.uid) {
                            u.kw = u.kw.union(keywords);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A minion the side summoned in combat: what its powers and trinkets
    /// give it.
    pub(super) fn kit_after_summon(&mut self, side: usize, uid: u32) {
        let gains: Vec<KitTrigger> = self
            .passives_now(side)
            .iter()
            .copied()
            .filter(|t| matches!(t, KitTrigger::SummonedGain { .. }))
            .collect();
        for t in gains {
            if let KitTrigger::SummonedGain {
                attack,
                health,
                keywords,
            } = t
            {
                self.buff(side, uid, attack, health);
                if let Some(u) = self.unit_mut(side, uid) {
                    u.kw = u.kw.union(keywords);
                }
            }
        }
    }
}
