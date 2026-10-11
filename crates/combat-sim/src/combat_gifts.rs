//! Dark Gifts in combat (T-527, D-100): the card entry of a gifted minion
//! and the effects only gifts have. The gifts themselves are in
//! `crate::gifts`.

use super::{intern, CardInfo, Combat, Dead, Source, Unit};
use crate::cards::{self, Trigger};
use crate::gifts::{self, Gift};
use crate::model::{Keywords, Minion, Tribes};

/// The card entry for `m` (one per card and set of gifts) and the keywords
/// its gifts give. Gifts the engine does not know add nothing here: the
/// coverage blocks the odds for them (or flags them when asked to play).
pub(crate) fn card_of(cards: &mut Vec<CardInfo>, m: &Minion) -> (u16, Keywords) {
    let next = cards.len();
    let plain = intern(cards, &m.card_id, &[], || info(m, &[], next));
    // The same gift listed twice counts once (whether the game stacks a gift
    // a triple brings twice is not checked).
    let mut known: Vec<&'static Gift> = Vec::new();
    for g in m.gifts.iter().filter_map(|g| gifts::gift(g)) {
        if !known.iter().any(|k| k.id == g.id) {
            known.push(g);
        }
    }
    if known.is_empty() {
        return (plain, Keywords::NONE);
    }
    let ids: Vec<&'static str> = known.iter().map(|g| g.id).collect();
    let card = intern(cards, &m.card_id, &ids, || info(m, &known, plain as usize));
    let keywords = known
        .iter()
        .map(|g| g.keywords)
        .fold(Keywords::NONE, Keywords::union);
    (card, keywords)
}

/// The card's rules (its golden card's when a gift gilds it and that card
/// is modelled), then each gift's triggers in order.
fn info(m: &Minion, known: &[&'static Gift], plain: usize) -> CardInfo {
    let gilded = known.iter().any(|g| g.golden);
    let rules = gilded
        .then(|| cards::rules(&gifts::golden_id(&m.card_id)))
        .flatten()
        .or_else(|| cards::rules(&m.card_id));
    let mut triggers = rules.map_or(Vec::new(), |r| r.triggers.to_vec());
    for g in known {
        triggers.extend_from_slice(g.triggers);
    }
    let all_types = known.iter().any(|g| g.all_types);
    CardInfo {
        id: m.card_id.clone(),
        gifts: known.iter().map(|g| g.id).collect(),
        plain: plain as u16,
        triggers,
        tier: m.tier,
        tribes: if all_types {
            m.tribes.union(Tribes::ALL)
        } else {
            m.tribes
        },
        base: m.base,
    }
}

impl Combat<'_> {
    /// This minion's Attack and full Health: alive, or when it died.
    fn own_stats(&self, side: usize, src: Source) -> Option<(i32, i32)> {
        match src {
            Source::Unit(uid) => self.unit(side, uid).map(|u| (u.attack, u.max_health)),
            Source::Dead { stats, .. } => Some(stats),
        }
    }

    /// Gives this minion's Attack and/or full Health to another random
    /// friendly minion.
    pub(super) fn give_own_stats(&mut self, side: usize, src: Source, attack: bool, health: bool) {
        let Some((a, h)) = self.own_stats(side, src) else {
            return;
        };
        let me = match src {
            Source::Unit(uid) => Some(uid),
            Source::Dead { .. } => None,
        };
        let pool = self.friends(side, Tribes::NONE, me);
        for uid in self.pick(pool, 1) {
            let gain = (if attack { a } else { 0 }, if health { h } else { 0 });
            self.buff(side, uid, gain.0, gain.1);
        }
    }

    /// Multiplies this minion's Attack and Health (a gain of the difference,
    /// so what follows stat gains sees it).
    pub(super) fn multiply_stats(&mut self, side: usize, uid: u32, attack: i32, health: i32) {
        let Some(u) = self.unit(side, uid) else {
            return;
        };
        let gain_a = u.attack.saturating_mul(attack.saturating_sub(1));
        let gain_h = u.health.saturating_mul(health.saturating_sub(1));
        self.buff(side, uid, gain_a, gain_h);
    }

    /// Gains the Attack of the minion to the left, if there is one.
    pub(super) fn gain_left_attack(&mut self, side: usize, uid: u32) {
        let units = &self.sides[side].units;
        let Some(ix) = self.sides[side].pos(uid) else {
            return;
        };
        let Some(left) = ix.checked_sub(1).map(|i| units[i].attack) else {
            return;
        };
        self.buff(side, uid, left, 0);
    }

    /// Triggers this living minion's own Deathrattles (its card's, its
    /// gifts' and those it gained), once each.
    pub(super) fn trigger_own_deathrattles(&mut self, side: usize, uid: u32) {
        let Some(unit) = self.unit(side, uid) else {
            return;
        };
        for actions in self.deathrattles(unit) {
            for a in actions {
                self.act(side, Source::Unit(uid), None, None, a);
            }
            self.after_friendly_deathrattle(side);
        }
    }

    /// Summons a copy of this minion to its right: the same card and gifts,
    /// its current stats and keywords. The copy's own start-of-combat
    /// effects do not trigger (only the minions there at the start do).
    pub(super) fn summon_copy(&mut self, side: usize, uid: u32) {
        let Some(u) = self.unit(side, uid) else {
            return;
        };
        let (card, attack, health, max, kw) = (u.card, u.attack, u.health, u.max_health, u.kw);
        let mut copy = self.fresh(card, attack, health, kw);
        copy.max_health = max;
        let new = copy.uid;
        let at = self.summon_pos(side, Source::Unit(uid));
        if self.sides[side].insert(at, copy) {
            self.after_summon(side, new);
        }
    }

    /// How many hits this minion's Divine Shield takes to break (1 for most).
    pub(super) fn shield_hits(&self, unit: &Unit) -> u8 {
        self.triggers(unit)
            .iter()
            .find_map(|t| match t {
                Trigger::ShieldTakesHits(n) => Some(*n),
                _ => None,
            })
            .unwrap_or(1)
    }

    /// What a minion that comes back with Reborn is: a fresh copy of its card
    /// (no gifts) with 1 Health, or with the Persisting Horror gift, the
    /// card with its Attack, full Health and keywords when it died.
    pub(super) fn reborn_form(&self, d: &Dead) -> (u16, i32, i32, Keywords) {
        let info = &self.p.cards[d.unit.card as usize];
        if info.triggers.contains(&Trigger::RebornWithStats) {
            return (d.unit.card, d.unit.attack, d.unit.max_health, d.unit.kw);
        }
        let (attack, kw) = match info.base {
            Some(b) => (b.attack, b.keywords),
            None => (d.unit.attack, d.unit.kw),
        };
        (info.plain, attack, 1, kw)
    }
}
