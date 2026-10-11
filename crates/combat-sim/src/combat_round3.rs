//! The effects card coverage round 3 added (T-608, session 074): a friendly
//! Deathrattle that triggers, and a friendly minion summoned.

use super::{Combat, Source};
use crate::cards::Trigger;
use crate::model::Tribes;

impl Combat<'_> {
    /// After a friendly Deathrattle of `side` triggers: the "for each
    /// Deathrattle you've triggered" cards on the board.
    pub(super) fn after_friendly_deathrattle(&mut self, side: usize) {
        let friends: Vec<u32> = self.friends(side, Tribes::NONE, None);
        for f in friends {
            let Some(unit) = self.unit(side, f) else {
                continue;
            };
            for t in self.triggers(unit) {
                if let Trigger::FriendlyDeathrattle(actions) = t {
                    for a in *actions {
                        self.act(side, Source::Unit(f), None, None, a);
                    }
                }
            }
        }
    }

    /// After a friendly minion is summoned (a token or a Reborn copy).
    pub(super) fn after_summon(&mut self, side: usize, summoned: u32) {
        let Some(tribes) = self.unit(side, summoned).map(|u| self.tribes(u)) else {
            return;
        };
        let friends: Vec<u32> = self.friends(side, Tribes::NONE, Some(summoned));
        for f in friends {
            let Some(unit) = self.unit(side, f) else {
                continue;
            };
            for t in self.triggers(unit) {
                if let Trigger::AfterFriendlySummon { tribe, actions } = *t {
                    if tribe == Tribes::NONE || tribes.is(tribe) {
                        for a in actions {
                            self.act(side, Source::Unit(f), None, Some(summoned), a);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "combat_round3_tests.rs"]
mod tests;
