//! Card round 8 (T-616): minions that grow with the side's deaths, give a
//! summoned minion this one's stats, or summon random Deathrattle minions;
//! since engine 0.23.0 also minions made Golden in combat (Sr. Tomb Diver)
//! and minions that follow the side's spells (Timewarped Nalaa).

use super::{Combat, Dead, Source, Unit};
use crate::cards::{self, Trigger};

impl Combat<'_> {
    /// Attack a minion has per friendly death this combat (Rot Hide Gnoll).
    fn attack_per_death(&self, unit: &Unit) -> i32 {
        self.triggers(unit)
            .iter()
            .map(|t| match t {
                Trigger::AttackPerFriendlyDeath(n) => *n,
                _ => 0,
            })
            .sum()
    }

    /// Counts the minions that just died on each side and raises the Attack
    /// of the friendly minions that grow with deaths.
    pub(super) fn count_friendly_deaths(&mut self, dead: &[Dead]) {
        for side in 0..2 {
            let died = dead.iter().filter(|d| d.side == side).count() as u32;
            if died == 0 {
                continue;
            }
            self.sides[side].deaths += died;
            let growers: Vec<(u32, i32)> = self.sides[side]
                .units
                .iter()
                .filter(|u| u.alive())
                .map(|u| (u.uid, self.attack_per_death(u)))
                .filter(|(_, per)| *per > 0)
                .collect();
            for (uid, per) in growers {
                self.buff(side, uid, per.saturating_mul(died as i32), 0);
            }
        }
    }

    /// A minion that comes into the combat after some friends died (summoned,
    /// reborn, or a teammate's board) has the Attack for all of them.
    pub(super) fn catch_up_deaths(&mut self, side: usize, uid: u32) {
        let died = self.sides[side].deaths as i32;
        let per = self.unit(side, uid).map_or(0, |u| self.attack_per_death(u));
        if died > 0 && per > 0 {
            self.buff(side, uid, per.saturating_mul(died), 0);
        }
    }

    /// Stalwart Kodo: gives the minion just summoned this minion's Attack and
    /// full Health, `factor` times over, up to `times` per combat.
    pub(super) fn give_summoned_own_stats(
        &mut self,
        side: usize,
        me: Option<u32>,
        summoned: Option<u32>,
        factor: i32,
        times: u8,
    ) {
        let (Some(me), Some(summoned)) = (me, summoned) else {
            return;
        };
        let Some(kodo) = self.unit_mut(side, me) else {
            return;
        };
        if kodo.uses >= times {
            return;
        }
        kodo.uses += 1;
        let (attack, health) = (kodo.attack, kodo.max_health);
        self.buff(
            side,
            summoned,
            attack.saturating_mul(factor),
            health.saturating_mul(factor),
        );
    }

    /// Ghastcoiler: `count` Deathrattle minions picked at random, each from
    /// the whole pool, side by side where the card was.
    pub(super) fn summon_random_deathrattle(&mut self, side: usize, src: Source, count: u8) {
        let at = self.summon_pos(side, src);
        for k in 0..usize::from(count) {
            let id = cards::DEATHRATTLE_POOL[self.rng.below(cards::DEATHRATTLE_POOL.len())];
            let Some(&(_, card)) = self.p.tokens.iter().find(|(t, _)| *t == id) else {
                debug_assert!(false, "pool minion {id} was not prepared");
                return;
            };
            let Some(base) = self.p.cards[card as usize].base else {
                return;
            };
            let (more_attack, more_health) = self.game_stats(side, card);
            let unit = self.fresh(
                card,
                base.attack.saturating_add(more_attack),
                base.health.saturating_add(more_health),
                base.keywords,
            );
            let uid = unit.uid;
            if !self.insert_summon(side, src, at + k, unit) {
                break;
            }
            self.after_summon(side, uid);
        }
    }

    /// Sr. Tomb Diver: the `count` right-most living minions of `side` that
    /// are not Golden become their golden card, with its rules, and keep
    /// their buffs: their stats grow by the normal card's printed stats (or
    /// stay as they are when the board did not give those). A Golden minion
    /// is passed over for the next one to its left; a minion whose golden
    /// card is not modelled counts as made Golden and stays as it is.
    pub(super) fn make_rightmost_golden(&mut self, side: usize, count: u8) {
        let mut left = count;
        let mut ix = self.sides[side].units.len();
        while left > 0 && ix > 0 {
            ix -= 1;
            let unit = &self.sides[side].units[ix];
            let info = &self.p.cards[unit.card as usize];
            if !unit.alive() || info.golden {
                continue;
            }
            left -= 1;
            let Some(&(_, golden)) = self.p.gilded.iter().find(|(n, _)| *n == unit.card) else {
                continue;
            };
            let (attack, health) = info.base.map_or((0, 0), |b| (b.attack, b.health));
            let unit = &mut self.sides[side].units[ix];
            unit.card = golden;
            unit.attack = unit.attack.saturating_add(attack);
            unit.health = unit.health.saturating_add(health);
            unit.max_health = unit.max_health.saturating_add(health);
        }
    }

    /// After `side` casts a spell in combat: each of its living minions that
    /// follows spells acts (Timewarped Nalaa), left to right.
    pub(super) fn after_friendly_spell(&mut self, side: usize) {
        let uids: Vec<u32> = self.sides[side].units.iter().map(|u| u.uid).collect();
        for uid in uids {
            let Some(unit) = self.unit(side, uid).filter(|u| u.alive()) else {
                continue;
            };
            for t in self.triggers(unit) {
                if let Trigger::AfterFriendlySpell(actions) = t {
                    for a in *actions {
                        self.act(side, Source::Unit(uid), None, None, a);
                    }
                }
            }
        }
    }
}
