//! The Deities (T-671): each player's C'Thun or Y'Shaarj waits hidden and
//! awakens in combat once enough friendly Aberrations died, entering with
//! the stats the game kept for it (the board's game-long values). C'Thun
//! then gives its stats split among the other friendly minions: each gets
//! the whole share and the rest goes one point at a time to different
//! random ones, Attack and Health apart (measured on the maintainer's
//! logged combats). Y'Shaarj's Deathrattle brings back the first
//! Aberrations that died this combat with the stats they died with.

use super::{Combat, Dead, Source};
use crate::cards;
use crate::model::Tribes;

impl Combat<'_> {
    /// Each friendly Aberration that died (a Deity aside) is counted toward
    /// the side's Deity and kept for Y'Shaarj.
    pub(super) fn count_aberrations(&mut self, dead: &[Dead]) {
        for d in dead {
            let info = &self.p.cards[d.unit.card as usize];
            if !info.tribes.is(Tribes::ABERRATION) || cards::is_deity(&info.id) {
                continue;
            }
            let entry = (d.unit.card, d.unit.attack, d.unit.max_health);
            let side = &mut self.sides[d.side];
            side.aberrations.push(entry);
            side.values.deity.progress = side.values.deity.progress.saturating_add(1);
        }
    }

    /// The side's Deity awakens when it is known, not awake yet, enough
    /// Aberrations died and the board has space; at the right end. It can
    /// enter a board its last minion just left (its awakening comes with the
    /// deaths). Not checked against logged combats yet: whether a Duos
    /// teammate's board counts the deaths of the first board; here each
    /// board counts its own, from the deaths the game counted for its Deity.
    pub(super) fn awaken(&mut self, side: usize) {
        let deity = self.sides[side].values.deity;
        if self.sides[side].deity.is_some() || !deity.known() || deity.progress < deity.needed {
            return;
        }
        let Some(id) = cards::deity_card(deity.dbf) else {
            return;
        };
        let Some(&(_, card)) = self.p.tokens.iter().find(|(t, _)| *t == id) else {
            debug_assert!(false, "Deity {id} was not prepared");
            return;
        };
        let kw = self.p.cards[card as usize]
            .base
            .map_or(crate::model::Keywords::NONE, |b| b.keywords);
        let unit = self.fresh(card, deity.attack, deity.health, kw);
        let uid = unit.uid;
        let at = self.sides[side].units.len();
        if !self.sides[side].insert(at, unit) {
            return;
        }
        self.sides[side].deity = Some(uid);
        self.after_summon(side, uid);
        if id == "BGFYM_000" {
            self.split_stats(side, uid);
        }
    }

    /// C'Thun gives its stats split among the other friendly minions.
    fn split_stats(&mut self, side: usize, uid: u32) {
        let Some((attack, health)) = self.unit(side, uid).map(|u| (u.attack, u.health)) else {
            return;
        };
        let others = self.friends(side, Tribes::NONE, Some(uid));
        if others.is_empty() {
            return;
        }
        let n = others.len() as i32;
        let attack_rest = self.pick(others.clone(), (attack % n) as u8);
        let health_rest = self.pick(others.clone(), (health % n) as u8);
        for o in others {
            let a = attack / n + i32::from(attack_rest.contains(&o));
            let h = health / n + i32::from(health_rest.contains(&o));
            self.buff(side, o, a, h);
        }
    }

    /// "Give your Deity +A/+H": the awakened one, else the hidden one.
    pub(super) fn buff_deity(&mut self, side: usize, attack: i32, health: i32) {
        // Once it awakened, the buff goes to it; after it died, nowhere.
        match self.sides[side].deity {
            Some(uid) => self.buff(side, uid, attack, health),
            None => {
                let d = &mut self.sides[side].values.deity;
                d.attack = d.attack.saturating_add(attack);
                d.health = d.health.saturating_add(health);
            }
        }
    }

    /// Y'Shaarj: the first `count` Aberrations that died this combat come
    /// back where it died, with the stats they died with.
    pub(super) fn summon_dead_aberrations(&mut self, side: usize, src: Source, count: u8) {
        let back: Vec<(u16, i32, i32)> = self.sides[side]
            .aberrations
            .iter()
            .take(usize::from(count))
            .copied()
            .collect();
        let at = self.summon_pos(side, src);
        for (k, (card, attack, health)) in back.into_iter().enumerate() {
            let kw = self.p.cards[card as usize]
                .base
                .map_or(crate::model::Keywords::NONE, |b| b.keywords);
            let unit = self.fresh(card, attack, health.max(1), kw);
            let uid = unit.uid;
            if !self.sides[side].insert(at + k, unit) {
                break;
            }
            self.after_summon(side, uid);
        }
    }
}
