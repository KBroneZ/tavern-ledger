//! What a card does: the actions of `cards::Action` (buffs, keywords,
//! damage, summons) and the helpers they share.

use super::{Combat, Source, Unit};
use crate::cards::{Action, Trigger};
use crate::model::{Keywords, Tribes};

impl Combat<'_> {
    /// Gives `uid` stats, then checks "once this reaches N Attack"; the
    /// friends that follow an Amalgam gain them too.
    pub(super) fn buff(&mut self, side: usize, uid: u32, attack: i32, health: i32) {
        let factor = self.gain_factor(side, uid);
        let (attack, health) = (attack.saturating_mul(factor), health.saturating_mul(factor));
        self.buff_one(side, uid, attack, health);
        self.amalgam_gained(side, uid, attack, health);
    }

    fn buff_one(&mut self, side: usize, uid: u32, attack: i32, health: i32) {
        let Some(u) = self.unit_mut(side, uid) else {
            return;
        };
        u.attack = u.attack.saturating_add(attack);
        u.health = u.health.saturating_add(health);
        u.max_health = u.max_health.saturating_add(health);
        let (atk, done, card) = (u.attack, u.shield_done, u.card);
        if done {
            return;
        }
        let reaches = self.p.cards[card as usize]
            .triggers
            .iter()
            .any(|t| matches!(t, Trigger::ShieldAtAttack(n) if atk >= *n));
        if reaches {
            if let Some(u) = self.unit_mut(side, uid) {
                u.kw = u.kw.union(Keywords::DIVINE_SHIELD);
                u.shield_done = true;
            }
        }
    }

    /// Living friendly minions of `tribe` (any when NONE), without `skip`.
    pub(super) fn friends(&self, side: usize, tribe: Tribes, skip: Option<u32>) -> Vec<u32> {
        self.sides[side]
            .units
            .iter()
            .filter(|u| u.alive() && Some(u.uid) != skip)
            .filter(|u| tribe == Tribes::NONE || self.tribes(u).is(tribe))
            .map(|u| u.uid)
            .collect()
    }

    pub(super) fn pick(&mut self, mut pool: Vec<u32>, count: u8) -> Vec<u32> {
        let mut out = Vec::new();
        while out.len() < usize::from(count) && !pool.is_empty() {
            out.push(pool.swap_remove(self.rng.below(pool.len())));
        }
        out
    }

    pub(super) fn act(
        &mut self,
        side: usize,
        src: Source,
        target: Option<(usize, u32)>,
        attacker: Option<u32>,
        action: &Action,
    ) {
        let (me, killer) = match src {
            Source::Unit(uid) => (Some(uid), None),
            Source::Dead { killer, .. } => (None, killer),
        };
        match *action {
            Action::Summon { token, count } => self.summon(side, src, token, count),
            Action::BuffAll {
                attack,
                health,
                tribe,
                others,
            } => {
                let skip = if others { me } else { None };
                for uid in self.friends(side, tribe, skip) {
                    self.buff(side, uid, attack, health);
                }
            }
            Action::BuffRandom {
                attack,
                health,
                tribe,
                count,
            } => {
                let pool = self.friends(side, tribe, me);
                for uid in self.pick(pool, count) {
                    self.buff(side, uid, attack, health);
                }
            }
            Action::GainStats { attack, health } => {
                if let Some(uid) = me {
                    self.buff(side, uid, attack, health);
                }
            }
            Action::GainKeyword(k) => {
                if let Some(u) = me.and_then(|uid| self.unit_mut(side, uid)) {
                    u.kw = u.kw.union(k);
                }
            }
            Action::GiveKeyword {
                keyword,
                tribe,
                count,
            } => {
                let pool: Vec<u32> = self
                    .friends(side, tribe, me)
                    .into_iter()
                    .filter(|uid| {
                        self.unit(side, *uid)
                            .is_some_and(|u| !u.kw.contains(keyword))
                    })
                    .collect();
                for uid in self.pick(pool, count) {
                    if let Some(u) = self.unit_mut(side, uid) {
                        u.kw = u.kw.union(keyword);
                    }
                }
            }
            Action::DamageTargetAndAdjacent => {
                let (Some(uid), Some((foe, t))) = (me, target) else {
                    return;
                };
                let atk = self.unit(side, uid).map_or(0, |u| u.attack);
                let Some(t_ix) = self.sides[foe].pos(t) else {
                    return;
                };
                let adjacent = self.neighbours(foe, t_ix);
                self.damage(foe, t, atk, Keywords::NONE, uid);
                if !adjacent.is_empty() {
                    let n = adjacent[self.rng.below(adjacent.len())];
                    self.damage(foe, n, atk, Keywords::NONE, uid);
                }
            }
            Action::DamageTargetAndNeighbours => {
                let (Some(uid), Some((foe, t))) = (me, target) else {
                    return;
                };
                let atk = self.unit(side, uid).map_or(0, |u| u.attack);
                self.damage(foe, t, atk, Keywords::NONE, uid);
                self.damage_neighbours(foe, t, atk, uid);
            }
            Action::GainTargetAttack { times } => {
                let (Some(uid), Some((foe, t))) = (me, target) else {
                    return;
                };
                let gain = self
                    .unit(foe, t)
                    .map_or(0, |u| u.attack)
                    .saturating_mul(times);
                self.buff(side, uid, gain, 0);
            }
            Action::StripTarget(k) => {
                if let Some(u) = target.and_then(|(foe, t)| self.unit_mut(foe, t)) {
                    u.kw = u.kw.without(k);
                    if k.contains(Keywords::DIVINE_SHIELD) {
                        u.shield_taken = 0;
                    }
                }
            }
            Action::DestroyKiller => {
                if let Some(u) = killer.and_then(|k| self.unit_mut(1 - side, k)) {
                    u.health = u.health.min(0);
                }
            }
            Action::BuffAttacker { attack, health } => {
                if let Some(a) = attacker {
                    self.buff(side, a, attack, health);
                }
            }
            Action::BuffAllWithKeyword {
                attack,
                health,
                keyword,
            } => {
                let shielded: Vec<u32> = self
                    .friends(side, Tribes::NONE, None)
                    .into_iter()
                    .filter(|f| self.unit(side, *f).is_some_and(|u| u.kw.contains(keyword)))
                    .collect();
                for f in shielded {
                    self.buff(side, f, attack, health);
                }
            }
            Action::SummonAndAttack { token } => self.summon_and_attack(side, src, token),
            Action::BuffOnePerTribe { attack, health } => {
                self.buff_one_per_tribe(side, attack, health)
            }
            // Needs the reborn minion: played by `after_reborn`.
            Action::GiveRebornAttackToRightmost { .. } => {}
            Action::SummonWithStats {
                token,
                factor,
                attack_first,
            } => self.summon_with_stats(side, src, target, token, factor, attack_first),
            Action::BuffCards {
                cards,
                attack,
                health,
            } => self.buff_cards(side, cards, attack, health),
            Action::DamageTargetNeighbours(amount) => {
                if let (Some(uid), Some((foe, t))) = (me, target) {
                    self.damage_neighbours(foe, t, amount, uid);
                }
            }
            Action::TriggerAdjacentDeathrattle { both } => {
                self.trigger_adjacent_deathrattle(side, src, both)
            }
            Action::CopyLeftmostDeathrattles { count, times } => {
                if let Some(uid) = me {
                    self.copy_leftmost_deathrattles(side, uid, count, times);
                }
            }
            Action::GiveOwnStats { attack, health } => {
                self.give_own_stats(side, src, attack, health)
            }
            Action::MultiplyStats { attack, health } => {
                if let Some(uid) = me {
                    self.multiply_stats(side, uid, attack, health);
                }
            }
            Action::GainLeftAttack => {
                if let Some(uid) = me {
                    self.gain_left_attack(side, uid);
                }
            }
            Action::TriggerOwnDeathrattles => {
                if let Some(uid) = me {
                    self.trigger_own_deathrattles(side, uid);
                }
            }
            Action::SummonCopy => {
                if let Some(uid) = me {
                    self.summon_copy(side, uid);
                }
            }
            Action::MultiplyAttackerAttack { factor } => {
                if let Some(a) = attacker {
                    let gain = self
                        .unit(side, a)
                        .map_or(0, |u| u.attack.saturating_mul(factor.saturating_sub(1)));
                    self.buff(side, a, gain, 0);
                }
            }
            Action::Gems { to, count } => self.play_gems(side, me, attacker, to, count),
            Action::UpgradeGems { attack, health } => {
                let gem = &mut self.sides[side].values.blood_gem;
                *gem = (gem.0.saturating_add(attack), gem.1.saturating_add(health));
            }
            Action::Spell(actions) => self.cast_spell(side, src, target, attacker, actions),
            Action::SummonRandomDeathrattle { count } => {
                self.summon_random_deathrattle(side, src, count)
            }
            Action::GiveSummonedOwnStats { factor, times } => {
                self.give_summoned_own_stats(side, me, attacker, factor, times)
            }
            Action::BuffRandomWithKeyword {
                attack,
                health,
                keyword,
                tribe,
                count,
            } => {
                let pool = self.friends(side, tribe, me);
                for uid in self.pick(pool, count) {
                    self.buff(side, uid, attack, health);
                    if let Some(u) = self.unit_mut(side, uid) {
                        u.kw = u.kw.union(keyword);
                    }
                }
            }
            Action::BuffAllByCounters {
                attack,
                health,
                tribe,
                others,
            } => {
                let Some(uid) = me else { return };
                let own = self.unit(side, uid).map_or([0, 0], |u| u.counters);
                let (attack, health) = if own == [0, 0] {
                    (attack, health)
                } else {
                    (own[0], own[1])
                };
                let skip = if others { Some(uid) } else { None };
                for f in self.friends(side, tribe, skip) {
                    self.buff(side, f, attack, health);
                }
            }
            Action::DestroyNeighboursKeepCopies { both } => {
                if let Some(uid) = me {
                    self.destroy_neighbours_keep(side, uid, both);
                }
            }
            Action::SummonKept => self.summon_kept(side, src),
            Action::BecomeLeftCopy => {
                if let Some(uid) = me {
                    self.become_left_copy(side, uid);
                }
            }
            Action::DamageRandomEnemyByAttack { times } => {
                let (attack, by) = match src {
                    Source::Unit(uid) => (self.unit(side, uid).map_or(0, |u| u.attack), uid),
                    Source::Dead { uid, stats, .. } => (stats.0, uid),
                };
                let foe = 1 - side;
                for _ in 0..times {
                    let pool = self.friends(foe, Tribes::NONE, None);
                    if let Some(&t) = self.pick(pool, 1).first() {
                        self.damage(foe, t, attack, Keywords::NONE, by);
                    }
                }
            }
            Action::BecomeTeammateCopy => {
                if let Some(uid) = me {
                    self.become_teammate_copy(side, uid);
                }
            }
            Action::DamageAllOthers(amount) => {
                let Some(uid) = me else { return };
                for s in [side, 1 - side] {
                    for t in self.friends(s, Tribes::NONE, Some(uid)) {
                        self.damage(s, t, amount, Keywords::NONE, uid);
                    }
                }
            }
        }
    }

    pub(super) fn summon(&mut self, side: usize, src: Source, token: &str, count: u8) {
        // Every token a card can summon is entered in `Prepared::new`, with
        // its printed facts; a test in `cards` checks the token table is complete.
        let Some(&(_, card)) = self.p.tokens.iter().find(|(id, _)| *id == token) else {
            debug_assert!(false, "token {token} was not prepared");
            return;
        };
        let Some(base) = self.p.cards[card as usize].base else {
            return;
        };
        let at = self.summon_pos(side, src);
        let (more_attack, more_health) = self.game_stats(side, card);
        let (attack, health) = (
            base.attack.saturating_add(more_attack),
            base.health.saturating_add(more_health),
        );
        for k in 0..usize::from(count) {
            let unit = self.fresh(card, attack, health, base.keywords);
            let uid = unit.uid;
            if !self.sides[side].insert(at + k, unit) {
                break;
            }
            self.after_summon(side, uid);
        }
    }

    /// Where a summon from `src` goes: right of the card, or where it died.
    pub(super) fn summon_pos(&self, side: usize, src: Source) -> usize {
        match src {
            Source::Unit(uid) => self.sides[side].pos(uid).map_or(0, |i| i + 1),
            Source::Dead { anchor, .. } => self.anchor_pos(side, anchor),
        }
    }

    pub(super) fn anchor_pos(&self, side: usize, anchor: Option<u32>) -> usize {
        anchor
            .and_then(|a| self.sides[side].pos(a))
            .unwrap_or(self.sides[side].units.len())
    }

    pub(super) fn fresh(&mut self, card: u16, attack: i32, health: i32, kw: Keywords) -> Unit {
        self.next_uid += 1;
        let mut unit = Unit::new(self.next_uid, card, attack, health, kw);
        unit.shield_done = self.p.cards[card as usize]
            .triggers
            .iter()
            .any(|t| matches!(t, Trigger::ShieldAtAttack(n) if attack >= *n));
        unit
    }
}
