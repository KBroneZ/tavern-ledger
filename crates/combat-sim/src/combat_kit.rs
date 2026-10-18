//! Hero powers and trinkets at the start of combat (T-628 round 2): the
//! actions of [`crate::kit_effects`], played before the minions' (the user's
//! logs: trinkets, then hero powers, then minions).

use super::Combat;
use crate::kit_effects::{self, KitAction, KitPlay, Pick};
use crate::model::{Keywords, Kit, Tribes};

/// A hero power or trinket the engine plays, as one board brings it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KitUse {
    pub power: bool,
    pub counters: [i32; 5],
    pub play: &'static KitPlay,
}

/// A board's played powers and trinkets: trinkets first, then hero powers.
pub(crate) fn kit_uses(kit: Option<&Kit>) -> Vec<KitUse> {
    let Some(kit) = kit else {
        return Vec::new();
    };
    let tagged = kit
        .trinkets
        .iter()
        .map(|c| (false, c))
        .chain(kit.powers.iter().map(|c| (true, c)));
    tagged
        .filter_map(|(power, c)| {
            kit_effects::played(&c.card_id).map(|play| KitUse {
                power,
                counters: c.counters,
                play,
            })
        })
        .collect()
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

    /// Plays `side`'s trinkets (`powers` false) or hero powers.
    pub(super) fn play_kit(&mut self, side: usize, powers: bool) {
        let uses: Vec<KitUse> = self
            .kit_now(side)
            .iter()
            .filter(|k| k.power == powers)
            .copied()
            .collect();
        for k in uses {
            for action in k.play.start {
                self.kit_act(side, &k.counters, action);
            }
        }
    }

    fn kit_act(&mut self, side: usize, counters: &[i32; 5], action: &KitAction) {
        let value = |a| kit_effects::value(a, counters);
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
    /// whose minions attack at the start of combat (Wingmen).
    pub(super) fn kit_swings(&self, side: usize) -> bool {
        self.kit_now(side).iter().any(|k| {
            k.play
                .start
                .iter()
                .any(|a| matches!(a, KitAction::AttackImmediately(_)))
        })
    }

    /// True when the board `side` fights with now plays a power or trinket
    /// at the start of combat.
    pub(super) fn has_kit_start(&self, side: usize) -> bool {
        !self.kit_now(side).is_empty()
    }
}
