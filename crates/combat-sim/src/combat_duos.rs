//! Duos combats (T-608): each team's second board comes in once its first
//! is gone. The rules and how they were found are in
//! `docs/research/combat-simulator.md`, section 3.

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
    /// `docs/research/combat-simulator.md`, section 3).
    pub(super) fn bring_in_teammates(&mut self) -> Option<usize> {
        let mut entered = self.enter_waiting();
        if entered == [false, false] {
            return None;
        }
        loop {
            // Counted before start of combat, as at the start (section 3).
            let first = self.first_attacker();
            if !self.p.start_of_combat {
                return Some(first);
            }
            self.start_of_combat(first, entered);
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
                self.legs.tier[side] = self.p.second_tier[side];
                self.sides[side] = SideState {
                    units: self.p.second[side].clone().unwrap_or_default(),
                    next: 0,
                };
            }
            enters
        })
    }
}
