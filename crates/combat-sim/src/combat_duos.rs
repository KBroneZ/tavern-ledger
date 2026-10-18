//! Duos combats (T-608): each team's second board comes in once its first
//! is gone. The rules and how they were found are in
//! design notes

use super::{Combat, Prepared, SideState};

/// Duos: whether each side's second board is still to come in, and the
/// tavern tier of the hero whose board fights now (part of the damage).
#[derive(Clone, Copy, Debug)]
pub(super) struct Legs {
    pub(super) waiting: [bool; 2],
    pub(super) tier: [Option<u8>; 2],
}

impl Legs {
    pub(super) fn of(p: &Prepared) -> Legs {
        Legs {
            waiting: [p.second[0].is_some(), p.second[1].is_some()],
            tier: p.tavern_tier,
        }
    }
}

impl Combat<'_> {
    /// Duos: a side whose board is gone brings in its teammate's board, which
    /// starts its attacks from its left-most minion; the other side keeps its
    /// place in its own order. Who attacks next is then decided as at the
    /// start of a combat (more minions, else a coin flip). Returns that side
    /// when a board came in (both user logs and the rule table in
    /// design notes).
    pub(super) fn bring_in_teammates(&mut self) -> Option<usize> {
        let mut entered = self.enter_waiting();
        if entered == [false, false] {
            return None;
        }
        loop {
            let mut first = self.first_attacker();
            if self.p.start_of_combat {
                // Only the board that came in plays its start of combat; then
                // who attacks next is decided again (T-628: 115 of 115 legs).
                let swings = self.has_immediate_attackers(entered);
                self.start_of_combat(first, entered);
                first = self.first_after_start(first, swings);
            } else {
                self.attack_immediately(first, entered);
            }
            // Its start of combat can empty a board again.
            entered = self.enter_waiting();
            if entered == [false, false] {
                return Some(first);
            }
        }
    }

    /// Puts each waiting teammate's board in place of a board that is gone;
    /// says which sides got one.
    pub(super) fn enter_waiting(&mut self) -> [bool; 2] {
        [0, 1].map(|side| {
            let enters = self.sides[side].units.is_empty() && self.legs.waiting[side];
            if enters {
                self.legs.waiting[side] = false;
                // The hero whose board is gone takes its waiting powers along.
                self.pending[side].clear();
                self.legs.tier[side] = self.p.second_tier[side];
                self.sides[side] = SideState {
                    units: self.p.second[side].clone().unwrap_or_default(),
                    next: 0,
                    values: self.p.second_values[side],
                    deaths: self.sides[side].deaths,
                };
                let entering: Vec<u32> = self.sides[side].units.iter().map(|u| u.uid).collect();
                for uid in entering {
                    self.catch_up_deaths(side, uid);
                }
            }
            enters
        })
    }
}
