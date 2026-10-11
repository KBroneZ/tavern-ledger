//! The effects card coverage round 5 added (T-604, session 083): a minion
//! that gains what the Amalgam gains, and Deathrattles copied from the
//! left-most friends at the start of combat.

use super::{Combat, Unit};
use crate::cards::{Action, Trigger, AMALGAMS};
use crate::model::Tribes;

impl Combat<'_> {
    /// After `uid` gained stats: when it is an Amalgam, the friends that
    /// follow it gain them too, as many times over as their card says.
    pub(super) fn amalgam_gained(&mut self, side: usize, uid: u32, attack: i32, health: i32) {
        if !self.p.amalgam_followers || (attack <= 0 && health <= 0) {
            return;
        }
        let is_amalgam = self.unit(side, uid).is_some_and(|u| {
            u.alive() && AMALGAMS.contains(&self.p.cards[u.card as usize].id.as_str())
        });
        if !is_amalgam {
            return;
        }
        let followers: Vec<(u32, i32)> = self
            .friends(side, Tribes::NONE, Some(uid))
            .into_iter()
            .filter_map(|f| {
                let n: i32 = self
                    .triggers(self.unit(side, f)?)
                    .iter()
                    .map(|t| match t {
                        Trigger::GainsWithAmalgam(n) => *n,
                        _ => 0,
                    })
                    .sum();
                (n > 0).then_some((f, n))
            })
            .collect();
        for (f, n) in followers {
            self.buff(side, f, attack.saturating_mul(n), health.saturating_mul(n));
        }
    }

    /// `uid` gains the Deathrattles of the `count` left-most living friends
    /// that have one, `times` over; cards that copy Deathrattles themselves
    /// are left out (their own copies included). "Two left-most Deathrattles"
    /// is read as two minions, each with all its Deathrattles: a minion with
    /// two is not checked against the game (the card is approximate).
    pub(super) fn copy_leftmost_deathrattles(
        &mut self,
        side: usize,
        uid: u32,
        count: u8,
        times: u8,
    ) {
        let copied: Vec<u16> = self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive() && !self.copies_deathrattles(u))
            .filter(|u| !self.deathrattles(u).is_empty())
            .take(usize::from(count))
            .flat_map(|u| {
                let has_own = self
                    .triggers(u)
                    .iter()
                    .any(|t| matches!(t, Trigger::Deathrattle(_)));
                has_own
                    .then_some(u.card)
                    .into_iter()
                    .chain(u.gained.iter().copied())
            })
            .collect();
        if let Some(me) = self.unit_mut(side, uid) {
            for _ in 0..times {
                me.gained.extend(copied.iter().copied());
            }
        }
    }

    fn copies_deathrattles(&self, unit: &Unit) -> bool {
        self.triggers(unit).iter().any(|t| {
            t.actions()
                .iter()
                .any(|a| matches!(a, Action::CopyLeftmostDeathrattles { .. }))
        })
    }
}

#[cfg(test)]
#[path = "combat_round5_tests.rs"]
mod tests;
