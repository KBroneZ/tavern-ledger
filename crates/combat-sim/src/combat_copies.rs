//! Exact copies (T-628 round 3): a minion summoned or transformed as an
//! exact copy of another, and the start-of-combat cards that make them
//! (Stitched Salvager, Timewarped Chameleon, Sandy). An exact copy has the
//! card (with its Dark Gifts), stats, full Health, keywords and numbers of
//! its model and the Deathrattles it gained, and none of what the model
//! counted this combat (the user's logs: Rapid Reanimation's copy kept its
//! damage, Frostwolf Fervor's and Sandy's copies their buffs).

use super::{Combat, Source, Unit};

/// The Stitched Salvager cards: their Deathrattle never copies a Salvager.
const SALVAGERS: &[&str] = &["BG31_999", "BG31_999_G"];

impl Combat<'_> {
    /// A new minion that is an exact copy of `model`.
    pub(super) fn copy_of(&mut self, model: &Unit) -> Unit {
        let mut unit = self.fresh(model.card, model.attack, model.health, model.kw);
        unit.max_health = model.max_health;
        unit.counters = model.counters;
        unit.gained = model.gained.clone();
        unit.shield_taken = model.shield_taken;
        unit
    }

    /// Turns `uid` into an exact copy of `model`, in its place.
    pub(super) fn become_copy(&mut self, side: usize, uid: u32, model: &Unit) {
        let copy = self.copy_of(model);
        if let Some(u) = self.unit_mut(side, uid) {
            *u = Unit { uid, ..copy };
        }
    }

    /// The teammate's board, as it was taken, of the board `side` fights
    /// with now: the second board for the first, the first for the second.
    fn teammate_units(&self, side: usize) -> Option<&[Unit]> {
        let on_second = self.p.second[side].is_some() && !self.legs.waiting[side];
        if on_second {
            Some(&self.p.start[side])
        } else {
            self.p.second[side].as_deref()
        }
    }

    /// The teammate's minion with the highest `key`, the left-most on ties.
    fn teammates_highest(&self, side: usize, key: impl Fn(&Unit) -> Option<i32>) -> Option<Unit> {
        let mut best: Option<(i32, &Unit)> = None;
        for u in self.teammate_units(side)? {
            let Some(v) = key(u) else { continue };
            if best.is_none_or(|(b, _)| v > b) {
                best = Some((v, u));
            }
        }
        best.map(|(_, u)| u.clone())
    }

    /// The teammate's highest-Tier minion (Glorious Gloop).
    pub(super) fn teammates_highest_tier(&self, side: usize) -> Option<Unit> {
        self.teammates_highest(side, |u| self.p.cards[u.card as usize].tier.map(i32::from))
    }

    /// Sandy: becomes an exact copy of the teammate's highest-Health minion.
    pub(super) fn become_teammate_copy(&mut self, side: usize, uid: u32) {
        if let Some(model) = self.teammates_highest(side, |u| Some(u.health)) {
            self.become_copy(side, uid, &model);
        }
    }

    /// Timewarped Chameleon: becomes an exact copy of the minion to its left.
    pub(super) fn become_left_copy(&mut self, side: usize, uid: u32) {
        let Some(ix) = self.sides[side].pos(uid) else {
            return;
        };
        let Some(left) = ix.checked_sub(1).map(|i| self.sides[side].units[i].clone()) else {
            return;
        };
        self.become_copy(side, uid, &left);
    }

    /// Stitched Salvager's start of combat: destroys the minion to its left
    /// (both neighbours with `both`) and keeps an exact copy of each that is
    /// not a Salvager, for its Deathrattle.
    pub(super) fn destroy_neighbours_keep(&mut self, side: usize, uid: u32, both: bool) {
        let Some(ix) = self.sides[side].pos(uid) else {
            return;
        };
        let right = if both { Some(ix + 1) } else { None };
        let around = ix.checked_sub(1).into_iter().chain(right);
        let mut kept = Vec::new();
        for i in around {
            let Some(u) = self.sides[side].units.get_mut(i).filter(|u| u.alive()) else {
                continue;
            };
            let copy = u.clone();
            u.health = u.health.min(0);
            if !SALVAGERS.contains(&self.p.cards[copy.card as usize].id.as_str()) {
                kept.push(copy);
            }
        }
        match self.kept.iter_mut().find(|(k, _)| *k == uid) {
            Some((_, list)) => list.extend(kept),
            None => self.kept.push((uid, kept)),
        }
    }

    /// Stitched Salvager's Deathrattle: summons the copies it kept, in their
    /// order, where the card's summons go.
    pub(super) fn summon_kept(&mut self, side: usize, src: Source) {
        let uid = match src {
            Source::Unit(uid) | Source::Dead { uid, .. } => uid,
        };
        let Some((_, kept)) = self.kept.iter().find(|(k, _)| *k == uid) else {
            return;
        };
        let kept = kept.clone();
        let at = self.summon_pos(side, src);
        for (k, model) in kept.iter().filter(|m| m.alive()).enumerate() {
            let unit = self.copy_of(model);
            let new = unit.uid;
            if !self.sides[side].insert(at + k, unit) {
                break;
            }
            self.after_summon(side, new);
        }
    }
}
