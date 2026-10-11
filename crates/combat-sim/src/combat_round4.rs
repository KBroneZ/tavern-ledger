//! The effects card coverage round 4 added (T-604, session 075): a token
//! with the card's own stats (one that can attack first), a buff for some
//! card ids, damage to the target's neighbours, an adjacent minion's
//! Deathrattle, "after a friendly minion attacks", and Deathrattles gained
//! in combat.

use super::{CardInfo, Combat, Dead, Source, Unit};
use crate::cards::{Action, Trigger};
use crate::model::{Keywords, Tribes};

impl Combat<'_> {
    /// The Deathrattle action lists of `unit`: its card's, then those it
    /// gained in this combat.
    pub(super) fn deathrattles(&self, unit: &Unit) -> Vec<&'static [Action]> {
        let own = std::iter::once(unit.card).chain(unit.gained.iter().copied());
        own.flat_map(|c| self.p.cards[c as usize].triggers)
            .filter_map(|t| match t {
                Trigger::Deathrattle(actions) => Some(*actions),
                _ => None,
            })
            .collect()
    }

    /// Summons `token` with the stats of the card behind `src`, `factor`
    /// times over; with `attack_first` it attacks the attack's target.
    pub(super) fn summon_with_stats(
        &mut self,
        side: usize,
        src: Source,
        target: Option<(usize, u32)>,
        token: &str,
        factor: i32,
        attack_first: bool,
    ) {
        let stats = match src {
            Source::Unit(uid) => self.unit(side, uid).map(|u| (u.attack, u.max_health)),
            Source::Dead { stats, .. } => Some(stats),
        };
        let Some((attack, health)) = stats else {
            return;
        };
        let Some(&(_, card)) = self.p.tokens.iter().find(|(id, _)| *id == token) else {
            debug_assert!(false, "token {token} was not prepared");
            return;
        };
        let kw = self.p.cards[card as usize]
            .base
            .map_or(Keywords::NONE, |b| b.keywords);
        let at = self.summon_pos(side, src);
        let unit = self.fresh(
            card,
            attack.saturating_mul(factor),
            health.saturating_mul(factor),
            kw,
        );
        let uid = unit.uid;
        if !self.sides[side].insert(at, unit) {
            return;
        }
        self.after_summon(side, uid);
        let Some((foe, t)) = target.filter(|_| attack_first) else {
            return;
        };
        if self.unit(foe, t).is_some_and(Unit::alive) && !self.finished() {
            self.forced_target = Some(t);
            self.swing(side, uid);
            self.forced_target = None;
        }
    }

    /// Gives every living friendly minion that is one of `cards` stats.
    pub(super) fn buff_cards(&mut self, side: usize, cards: &[&str], attack: i32, health: i32) {
        let targets: Vec<u32> = self
            .friends(side, Tribes::NONE, None)
            .into_iter()
            .filter(|f| {
                self.unit(side, *f)
                    .is_some_and(|u| cards.contains(&self.p.cards[u.card as usize].id.as_str()))
            })
            .collect();
        for f in targets {
            self.buff(side, f, attack, health);
        }
    }

    /// Deals `amount` to the minions next to `target` on `foe`.
    pub(super) fn damage_neighbours(&mut self, foe: usize, target: u32, amount: i32, by: u32) {
        let Some(ix) = self.sides[foe].pos(target) else {
            return;
        };
        for n in self.neighbours(foe, ix) {
            self.damage(foe, n, amount, Keywords::NONE, by);
        }
    }

    /// Triggers the Deathrattles of the living minions next to `src` (one at
    /// random unless `both`), leaving out cards that do this themselves.
    pub(super) fn trigger_adjacent_deathrattle(&mut self, side: usize, src: Source, both: bool) {
        let units = &self.sides[side].units;
        let (left, right) = match src {
            Source::Unit(uid) => match self.sides[side].pos(uid) {
                Some(i) => (i.checked_sub(1), Some(i + 1)),
                None => return,
            },
            Source::Dead { anchor, .. } => {
                let at = self.anchor_pos(side, anchor);
                (at.checked_sub(1), Some(at))
            }
        };
        let pool: Vec<u32> = [left, right]
            .into_iter()
            .flatten()
            .filter_map(|i| units.get(i))
            .filter(|u| u.alive() && !self.triggers_adjacent(u))
            .filter(|u| !self.deathrattles(u).is_empty())
            .map(|u| u.uid)
            .collect();
        let chosen = if both { pool } else { self.pick(pool, 1) };
        for uid in chosen {
            let Some(unit) = self.unit(side, uid) else {
                continue;
            };
            for actions in self.deathrattles(unit) {
                for a in actions {
                    self.act(side, Source::Unit(uid), None, None, a);
                }
                self.after_friendly_deathrattle(side);
            }
        }
    }

    fn triggers_adjacent(&self, unit: &Unit) -> bool {
        self.deathrattles(unit).iter().any(|actions| {
            actions
                .iter()
                .any(|a| matches!(a, Action::TriggerAdjacentDeathrattle { .. }))
        })
    }

    /// After a friendly minion of `side` attacks.
    pub(super) fn after_friendly_attack(&mut self, side: usize) {
        let has = |t: &Trigger| matches!(t, Trigger::AfterFriendlyAttack(_));
        let any = self.sides[side]
            .units
            .iter()
            .any(|u| self.triggers(u).iter().any(has));
        if !any {
            return;
        }
        for f in self.friends(side, Tribes::NONE, None) {
            let Some(unit) = self.unit(side, f) else {
                continue;
            };
            for t in self.triggers(unit) {
                if let Trigger::AfterFriendlyAttack(actions) = t {
                    for a in *actions {
                        self.act(side, Source::Unit(f), None, None, a);
                    }
                }
            }
        }
    }

    /// The minions that gain the Deathrattles of friends that die take them
    /// from `dead` (each dead minion's card and what it had gained), as many
    /// times over as their card says (round 5: golden Fish, twice).
    pub(super) fn gain_deathrattles(&mut self, dead: &[Dead]) {
        let times = |triggers: &[Trigger]| -> usize {
            triggers
                .iter()
                .map(|t| match t {
                    Trigger::GainsDeathrattles(n) => usize::from(*n),
                    _ => 0,
                })
                .sum()
        };
        if !self
            .p
            .cards
            .iter()
            .any(|c: &CardInfo| times(c.triggers) > 0)
        {
            return;
        }
        for d in dead {
            if self.deathrattles(&d.unit).is_empty() {
                continue;
            }
            let gainers: Vec<(u32, usize)> = self
                .friends(d.side, Tribes::NONE, None)
                .into_iter()
                .filter_map(|f| {
                    let n = times(self.triggers(self.unit(d.side, f)?));
                    (n > 0).then_some((f, n))
                })
                .collect();
            // Only cards that have a Deathrattle of their own: a dead Fish
            // passes on what it gained, not its own card.
            let has_own = self.p.cards[d.unit.card as usize]
                .triggers
                .iter()
                .any(|t| matches!(t, Trigger::Deathrattle(_)));
            let own = has_own.then_some(d.unit.card);
            for (g, n) in gainers {
                if let Some(u) = self.unit_mut(d.side, g) {
                    for _ in 0..n {
                        u.gained.extend(own);
                        u.gained.extend(d.unit.gained.iter().copied());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "combat_round4_tests.rs"]
mod tests;
