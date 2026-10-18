//! Start of combat and deaths: Deathrattles (extra ones included), Avenge
//! and Reborn, resolved until nothing else dies.

use super::{Combat, Dead, Source};
use crate::cards::Trigger;
use crate::model::{Keywords, Tribes};

impl Combat<'_> {
    /// Start-of-combat effects of the boards that are coming in (`sides`),
    /// the first attacker's side first: every trinket, then every hero power
    /// (T-628), then each side's minions; each side's "attack immediately"
    /// comes after its other effects, before the other side's.
    pub(super) fn start_of_combat(&mut self, first: usize, sides: [bool; 2]) {
        let order: Vec<usize> = [first, 1 - first]
            .into_iter()
            .filter(|s| sides[*s])
            .collect();
        if order.iter().any(|s| self.has_kit_start(*s)) {
            for powers in [false, true] {
                for &side in &order {
                    self.play_kit(side, powers);
                }
            }
            self.process_deaths(first);
        }
        for side in order {
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
            if !self.immediate_attackers(side).is_empty() {
                // Its swings see the board after this side's effects.
                self.process_deaths(first);
                self.side_attacks_immediately(side);
            }
        }
        self.process_deaths(first);
    }

    /// "Start of Combat: attack immediately" of the boards that are coming
    /// in (`sides`), for boards taken after the other start-of-combat
    /// effects (start of combat off): the first attacker's side first, each
    /// side left to right.
    pub(super) fn attack_immediately(&mut self, first: usize, sides: [bool; 2]) {
        for side in [first, 1 - first].into_iter().filter(|s| sides[*s]) {
            self.side_attacks_immediately(side);
        }
    }

    /// The start-of-combat swings of `side`. They are not the side's turn:
    /// the attack order starts as if they had not happened (the user's logs,
    /// design notes).
    fn side_attacks_immediately(&mut self, side: usize) {
        for (uid, times) in self.immediate_attackers(side) {
            for _ in 0..times {
                if self.finished() || self.unit(side, uid).is_none_or(|u| u.attack <= 0) {
                    break;
                }
                self.swing(side, uid);
            }
        }
    }

    /// The minions of `side` that attack at the start of combat, left to
    /// right, with how many times. The replay's pending list
    /// (`replay::pending_immediate`) mirrors this: a minion that cannot
    /// swing loses all its swings in both.
    pub(super) fn immediate_attackers(&self, side: usize) -> Vec<(u32, u8)> {
        self.sides[side]
            .units
            .iter()
            .filter_map(|u| {
                let times: u8 = self
                    .triggers(u)
                    .iter()
                    .map(|t| match t {
                        Trigger::AttackImmediately(n) => *n,
                        _ => 0,
                    })
                    .sum();
                (times > 0).then_some((u.uid, times))
            })
            .collect()
    }

    pub(super) fn extra_deathrattles(&self, side: usize) -> u8 {
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
    pub(super) fn process_deaths(&mut self, first: usize) {
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
                // Hero powers that wait for space act once the deaths are
                // resolved (T-628 round 3).
                self.fill_both(first);
                return;
            }
            self.count_dead_knights(&dead);
            self.count_friendly_deaths(&dead);
            self.gain_deathrattles(&dead);
            for d in &dead {
                let times = 1 + usize::from(self.extra_deathrattles(d.side));
                let src = Source::Dead {
                    uid: d.unit.uid,
                    anchor: d.anchor,
                    killer: d.unit.hit_by,
                    stats: (d.unit.attack, d.unit.max_health),
                };
                // Its card's Deathrattles, then those it gained in combat.
                let cards = std::iter::once(d.unit.card).chain(d.unit.gained.iter().copied());
                for card in cards.flat_map(|c| std::iter::repeat_n(c, times)) {
                    let p = self.p;
                    let triggers = &p.cards[card as usize].triggers;
                    for t in triggers {
                        if let Trigger::Deathrattle(actions) = t {
                            for a in *actions {
                                self.act(d.side, src, None, None, a);
                            }
                            self.after_friendly_deathrattle(d.side);
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

    pub(super) fn avenge(&mut self, side: usize) {
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
    /// 1 Health and no Reborn (see `reborn_form` for a gifted one). Without
    /// the card's stats, its last Attack and keywords are used (an
    /// approximation the caller reports).
    pub(super) fn reborn(&mut self, d: &Dead) {
        let (card, mut attack, health, kw) = self.reborn_form(d);
        // A fresh copy of a card with known stats gets the side's game-long
        // Attack (its Health stays 1).
        let info = &self.p.cards[card as usize];
        if info.base.is_some() && !info.triggers.contains(&Trigger::RebornWithStats) {
            attack = attack.saturating_add(self.game_stats(d.side, card).0);
        }
        let unit = self.fresh(card, attack, health, kw.without(Keywords::REBORN));
        let uid = unit.uid;
        let at = self.anchor_pos(d.side, d.anchor);
        if self.sides[d.side].insert(at, unit) {
            self.after_summon(d.side, uid);
            self.after_reborn(d.side, uid);
        }
    }
}
