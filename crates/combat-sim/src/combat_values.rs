//! Game-long values in combat (T-609): Blood Gems with the side's upgrades
//! and Tavern spells with the side's spell upgrades.

use super::{Combat, Source};
use crate::cards::{Action, GemTo};
use crate::model::Tribes;

impl Combat<'_> {
    /// What `count` Blood Gems give on `side`: +1/+1 each plus its upgrades.
    pub(super) fn gem_stats(&self, side: usize, count: i32) -> (i32, i32) {
        let (a, h) = self.sides[side].values.blood_gem;
        (
            count.saturating_mul(a.saturating_add(1)),
            count.saturating_mul(h.saturating_add(1)),
        )
    }

    pub(super) fn play_gems(
        &mut self,
        side: usize,
        me: Option<u32>,
        attacker: Option<u32>,
        to: GemTo,
        count: i32,
    ) {
        let (attack, health) = self.gem_stats(side, count);
        let targets: Vec<u32> = match to {
            GemTo::Itself => me.into_iter().collect(),
            GemTo::Attacker => attacker.into_iter().collect(),
            GemTo::Others => self.friends(side, Tribes::NONE, me),
        };
        for uid in targets {
            self.buff(side, uid, attack, health);
        }
    }

    /// A Tavern spell: its buffs, each with the side's spell upgrades.
    pub(super) fn cast_spell(
        &mut self,
        side: usize,
        src: Source,
        target: Option<(usize, u32)>,
        attacker: Option<u32>,
        actions: &[Action],
    ) {
        let (more_attack, more_health) = self.sides[side].values.spell;
        let up = |a: i32, h: i32| (a.saturating_add(more_attack), h.saturating_add(more_health));
        for action in actions {
            let upgraded = match *action {
                Action::BuffAll {
                    attack,
                    health,
                    tribe,
                    others,
                } => {
                    let (attack, health) = up(attack, health);
                    Action::BuffAll {
                        attack,
                        health,
                        tribe,
                        others,
                    }
                }
                Action::BuffAllWithKeyword {
                    attack,
                    health,
                    keyword,
                } => {
                    let (attack, health) = up(attack, health);
                    Action::BuffAllWithKeyword {
                        attack,
                        health,
                        keyword,
                    }
                }
                other => other,
            };
            self.act(side, src, target, attacker, &upgraded);
        }
    }
}
