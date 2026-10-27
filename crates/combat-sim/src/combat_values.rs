//! Game-long values in combat (T-609): Blood Gems with the side's upgrades
//! and Tavern spells with the side's spell upgrades.

use super::{Combat, Dead, Source};
use crate::cards::{Action, GemTo, Trigger};
use crate::model::Tribes;

/// Eternal Knight and its golden card, with what each Knight that died this
/// game gives it ("Has +4/+2 for each friendly Eternal Knight that died
/// this game, wherever this is").
const KNIGHTS: [(&str, i32, i32); 2] = [("BG25_008", 4, 2), ("BG25_008_G", 8, 4)];
/// The Beetle and its golden card ("your Beetles have +N/+N this game").
const BEETLES: [&str; 2] = ["BG28_603t", "BG28_603t_G"];

impl Combat<'_> {
    /// What a minion summoned now has on top of its card's stats from the
    /// side's game-long values: the Undead Attack, the Eternal Knights that
    /// died, the Beetles' buffs (T-609). Zero for a value the board did not
    /// come with.
    pub(super) fn game_stats(&self, side: usize, card: u16) -> (i32, i32) {
        let info = &self.p.cards[card as usize];
        let v = self.sides[side].values;
        let mut out: (i32, i32) = (0, 0);
        if info.tribes.is(Tribes::UNDEAD) {
            out.0 = out.0.saturating_add(v.undead_attack);
        }
        if let Some((_, a, h)) = KNIGHTS.iter().find(|(id, ..)| *id == info.id) {
            let (ka, kh) = self.kit_counts[side].knight_gain;
            out.0 = out.0.saturating_add(a.saturating_mul(v.knights_dead));
            out.1 = out.1.saturating_add(h.saturating_mul(v.knights_dead));
            out = (out.0.saturating_add(ka), out.1.saturating_add(kh));
        }
        if BEETLES.contains(&info.id.as_str()) {
            out.0 = out.0.saturating_add(v.beetle.0);
            out.1 = out.1.saturating_add(v.beetle.1);
        }
        if info.tribes.is(Tribes::BEAST) {
            out.0 = out.0.saturating_add(self.sides[side].beast_attack);
        }
        out
    }

    /// On a board taken after the start of combat the Beasts there already
    /// show what each Humming Bird gave; the Beasts summoned later still get
    /// it (T-645).
    pub(super) fn beast_attack_on_board(&self, side: usize) -> i32 {
        self.sides[side]
            .units
            .iter()
            .flat_map(|u| self.triggers(u))
            .filter_map(|t| match t {
                Trigger::StartOfCombat(actions) => Some(*actions),
                _ => None,
            })
            .flatten()
            .map(|a| match a {
                Action::BeastAttackThisCombat(n) => *n,
                _ => 0,
            })
            .fold(0, i32::saturating_add)
    }

    /// How many times over a Dragon gains stats: twice for each neighbour
    /// that doubles its neighbours' gains (golden Persistent Poet).
    pub(super) fn gain_factor(&self, side: usize, uid: u32) -> i32 {
        if !self.p.gain_doublers {
            return 1;
        }
        let units = &self.sides[side].units;
        let Some(ix) = units.iter().position(|u| u.uid == uid) else {
            return 1;
        };
        if !self.tribes(&units[ix]).is(Tribes::DRAGON) {
            return 1;
        }
        let doublers = self
            .neighbours(side, ix)
            .into_iter()
            .filter_map(|n| self.unit(side, n))
            .filter(|n| n.alive())
            .filter(|n| {
                self.triggers(n)
                    .contains(&Trigger::DoublesAdjacentDragonGains)
            })
            .count();
        1 << doublers.min(2)
    }

    /// Each Eternal Knight that died adds to the side's count, and every
    /// friendly Knight still on the board grows with it.
    pub(super) fn count_dead_knights(&mut self, dead: &[Dead]) {
        for d in dead {
            let id = self.p.cards[d.unit.card as usize].id.as_str();
            if !KNIGHTS.iter().any(|(k, ..)| *k == id) {
                continue;
            }
            self.sides[d.side].values.knights_dead += 1;
            let knights: Vec<(u32, i32, i32)> = self.sides[d.side]
                .units
                .iter()
                .filter(|u| u.alive())
                .filter_map(|u| {
                    let id = self.p.cards[u.card as usize].id.as_str();
                    let (_, a, h) = KNIGHTS.iter().find(|(k, ..)| *k == id)?;
                    Some((u.uid, *a, *h))
                })
                .collect();
            for (uid, a, h) in knights {
                self.buff(d.side, uid, a, h);
            }
        }
    }

    /// The uids of the side's living Eternal Knights.
    pub(super) fn knights(&self, side: usize) -> Vec<u32> {
        self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive())
            .filter(|u| {
                let id = self.p.cards[u.card as usize].id.as_str();
                KNIGHTS.iter().any(|(k, ..)| *k == id)
            })
            .map(|u| u.uid)
            .collect()
    }

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
