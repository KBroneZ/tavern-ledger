//! Replays one game's packets and records what the report needs: the local
//! and shop (Bob) players, the tribes offered in the shop, who steps into each
//! combat with which board, and the local health after each round.

use std::collections::BTreeMap;

use crate::power::{EntityRef, Packet, ParseError, Value};
use crate::report::{Minion, Side};
use crate::state::{Board, Entity, Players, Tags};

/// An entity packet whose `tag=` lines are still arriving.
#[derive(Debug)]
enum Pending {
    Game(i64),
    Player { id: i64, player_id: i64, lo: u64 },
    Full { id: i64, card_id: Option<String> },
    Show { id: i64, card_id: String },
    Change { id: i64, card_id: String },
}

#[derive(Clone, Debug)]
struct OpenLeg {
    explicit: bool, // false: the local hero by default, until a hero switch
    hero: Option<String>,
    hint: Option<i64>,
}

/// (side, hero card id, combat player id hint, board or None if not visible)
pub type RawEntry = (Side, Option<String>, Option<i64>, Option<Vec<Minion>>);

#[derive(Debug, Default)]
pub struct Collector {
    pub board: Board,
    pub players: Players,
    pub local: Option<i64>,
    pub dummy: Option<i64>,
    /// PlayerID from each player's CREATE_GAME line.
    pub local_pid: Option<i64>,
    pub dummy_pid: Option<i64>,
    dummy_initial_hero: i64,
    pending: Option<(Pending, Tags)>,
    started: bool,
    /// Shop minion entity id -> tribe, in first-seen order.
    pub shop_races: Vec<(i64, String)>,
    pub entries: BTreeMap<i64, Vec<RawEntry>>,
    pub health: BTreeMap<i64, Option<i64>>,
    /// Round -> health of every lobby hero by player id when its combat
    /// closed. Round 0 is the health before the first combat.
    pub lobby_health: BTreeMap<i64, BTreeMap<i64, i64>>,
    open_own: Option<OpenLeg>,
    open_opp: Option<OpenLeg>,
}

impl Collector {
    pub fn feed(&mut self, packet: Packet) -> Result<(), ParseError> {
        if let Packet::Tag { tag, value } = packet {
            return match self.pending.as_mut() {
                Some((_, tags)) => {
                    tags.insert(tag, value);
                    Ok(())
                }
                None => Ok(()), // hslog asserts; nothing to attach it to
            };
        }
        self.commit()?;
        match packet {
            Packet::CreateGame => self.started = true,
            Packet::GameEntity { id } => {
                self.board.game_id = Some(id);
                self.pending = Some((Pending::Game(id), Tags::new()));
            }
            Packet::Player { id, player_id, lo } => {
                self.players.register_entity(id, player_id, lo == 0);
                self.pending = Some((Pending::Player { id, player_id, lo }, Tags::new()));
            }
            Packet::FullEntity { id, card_id } => {
                self.pending = Some((Pending::Full { id, card_id }, Tags::new()));
            }
            Packet::ShowEntity { entity, card_id } => {
                let id = self.players.entity_id(&entity, &self.board)?;
                self.pending = Some((Pending::Show { id, card_id }, Tags::new()));
            }
            Packet::ChangeEntity { entity, card_id } => {
                let id = self.players.entity_id(&entity, &self.board)?;
                self.pending = Some((Pending::Change { id, card_id }, Tags::new()));
            }
            Packet::HideEntity { entity } => {
                let id = self.players.entity_id(&entity, &self.board)?;
                self.board.exists(id)?;
            }
            Packet::TagChange { entity, tag, value } => self.tag_change(&entity, tag, value)?,
            Packet::BlockStart { block_type, entity } => {
                self.mention(&entity);
                if block_type == "ATTACK" && self.in_combat() {
                    self.snapshot_open_legs();
                }
            }
            Packet::Mention(entity) => self.mention(&entity),
            Packet::BlockEnd | Packet::Ignored | Packet::Tag { .. } => {}
        }
        Ok(())
    }

    /// Names in detail lines get registered as hslog does; errors do not matter there.
    fn mention(&mut self, entity: &EntityRef) {
        if let EntityRef::Name(name) = entity {
            let _ = self.players.resolve(name);
        }
    }

    pub fn finish(&mut self) -> Result<(), ParseError> {
        self.commit()
    }

    fn commit(&mut self) -> Result<(), ParseError> {
        let Some((pending, tags)) = self.pending.take() else {
            return Ok(());
        };
        match pending {
            Pending::Game(id) => self.board.create(Entity {
                id,
                card_id: None,
                tags,
            }),
            Pending::Player { id, player_id, lo } => {
                let player = Entity {
                    id,
                    card_id: None,
                    tags,
                };
                let is_dummy = player.int("BACON_DUMMY_PLAYER") != 0 || lo == 0;
                if is_dummy {
                    self.dummy = Some(id);
                    self.dummy_pid = Some(player_id);
                    self.dummy_initial_hero = player.int("HERO_ENTITY");
                } else {
                    self.local = Some(id);
                    self.local_pid = Some(player_id);
                }
                self.board.create(player);
            }
            Pending::Full { id, card_id } => {
                self.board.create(Entity { id, card_id, tags });
                self.track_shop(id);
            }
            Pending::Show { id, card_id } => self.board.show(id, card_id, tags)?,
            Pending::Change { id, card_id } => self.board.change(id, card_id, tags)?,
        }
        Ok(())
    }

    fn turn(&self) -> i64 {
        self.board.game().map(|g| g.int("TURN")).unwrap_or(0)
    }

    /// Odd turns are shop phases, even turns combats. Turn 0 is the hero pick.
    fn in_combat(&self) -> bool {
        let turn = self.turn();
        turn >= 2 && turn % 2 == 0
    }

    fn tag_change(
        &mut self,
        entity: &EntityRef,
        tag: String,
        value: Value,
    ) -> Result<(), ParseError> {
        let id = self.players.entity_id(entity, &self.board)?;
        let is_turn = tag == "TURN" && Some(id) == self.board.game_id;
        let is_hero = tag == "HERO_ENTITY";
        let is_shop = tag == "ZONE" || tag == "CONTROLLER";
        let new_int = if let Value::Int(v) = value { v } else { 0 };
        self.board.set_tag(id, tag, value)?;
        if is_turn {
            self.on_turn(new_int);
        } else if is_hero && self.in_combat() {
            self.on_hero_switch(id, new_int);
        } else if is_shop {
            self.track_shop(id);
        }
        Ok(())
    }

    fn on_turn(&mut self, turn: i64) {
        // The previous turn's legs that never reached an attack were not visible.
        let previous = turn.saturating_sub(1).div_euclid(2);
        self.close_unseen(Side::Own, previous);
        self.close_unseen(Side::Opponent, previous);
        if self.in_combat() {
            self.open_own = Some(OpenLeg {
                explicit: false,
                hero: None,
                hint: None,
            });
            self.open_opp = None;
        } else {
            self.open_own = None;
            self.open_opp = None;
            let snapshot = lobby_health_now(&self.board);
            if turn >= 1 && !snapshot.is_empty() {
                self.lobby_health.insert(previous, snapshot);
            }
            if turn > 1 {
                let health = self.own_leaderboard_hero().map(hero_health);
                self.health.insert(previous, health);
            }
        }
    }

    fn on_hero_switch(&mut self, player: i64, hero_id: i64) {
        let side = if Some(player) == self.local {
            Side::Own
        } else if Some(player) == self.dummy && hero_id != self.dummy_initial_hero {
            Side::Opponent
        } else {
            return;
        };
        self.close_unseen(side, self.turn() / 2);
        let (hero, hint) = self.current(side);
        *self.open_mut(side) = Some(OpenLeg {
            explicit: true,
            hero,
            hint,
        });
    }

    fn open_mut(&mut self, side: Side) -> &mut Option<OpenLeg> {
        match side {
            Side::Own => &mut self.open_own,
            Side::Opponent => &mut self.open_opp,
        }
    }

    fn current(&self, side: Side) -> (Option<String>, Option<i64>) {
        let player = match side {
            Side::Own => self.local,
            Side::Opponent => self.dummy,
        };
        let player = player.and_then(|id| self.board.get(id));
        let hero = player
            .and_then(|p| self.board.get(p.int("HERO_ENTITY")))
            .and_then(|h| h.card_id.clone());
        let hint = match side {
            Side::Opponent => self
                .dummy
                .and_then(|id| self.board.get(id))
                .and_then(|d| d.opt_int("BACON_CURRENT_COMBAT_PLAYER_ID")),
            Side::Own => None,
        };
        (hero, hint)
    }

    fn close_unseen(&mut self, side: Side, round: i64) {
        if let Some(leg) = self.open_mut(side).take() {
            if leg.explicit {
                self.entries
                    .entry(round)
                    .or_default()
                    .push((side, leg.hero, leg.hint, None));
            }
        }
    }

    fn snapshot_open_legs(&mut self) {
        let round = self.turn() / 2;
        for side in [Side::Own, Side::Opponent] {
            if self.open_mut(side).is_some() {
                let pid = match side {
                    Side::Own => self.local_pid,
                    Side::Opponent => self.dummy_pid,
                };
                let (hero, hint) = self.current(side);
                let board = pid.map(|pid| board_of(&self.board, pid));
                self.entries
                    .entry(round)
                    .or_default()
                    .push((side, hero, hint, board));
            }
        }
        self.open_own = None;
        self.open_opp = None;
    }

    fn track_shop(&mut self, id: i64) {
        let Some(dummy_pid) = self.dummy_pid else {
            return;
        };
        if self.turn() % 2 != 1 {
            return;
        }
        let Some(e) = self.board.get(id) else { return };
        let is_offer = e.is("CARDTYPE", "MINION")
            && e.is("ZONE", "PLAY")
            && e.int("CONTROLLER") == dummy_pid
            && e.int("IS_BACON_POOL_MINION") != 0;
        if !is_offer {
            return;
        }
        let race = race_name(e.tags.get("CARDRACE"));
        match self.shop_races.iter_mut().find(|(eid, _)| *eid == id) {
            Some(slot) => slot.1 = race,
            None => self.shop_races.push((id, race)),
        }
    }

    pub fn own_leaderboard_hero(&self) -> Option<&Entity> {
        let local_pid = self.local_pid?;
        leaderboard_heroes(&self.board)
            .into_iter()
            .find(|(pid, _)| *pid == local_pid)
            .map(|(_, h)| h)
    }
}

pub fn race_name(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Int(0)) => "NEUTRAL".into(),
        Some(Value::Name(n)) if n == "INVALID" => "NEUTRAL".into(),
        Some(Value::Name(n)) => n.clone(),
        Some(Value::Int(v)) => format!("RACE_{v}"),
    }
}

/// Tripled minions have their own card id. The PREMIUM tag is only cosmetic.
pub fn is_golden(card_id: Option<&str>) -> bool {
    card_id.is_some_and(|c| c.ends_with("_G") || c.starts_with("TB_BaconUps_"))
}

/// Health + armor - damage, never below 0: the killing blow can overshoot.
pub fn hero_health(hero: &Entity) -> i64 {
    hero.int("HEALTH")
        .saturating_add(hero.int("ARMOR"))
        .saturating_sub(hero.int("DAMAGE"))
        .max(0)
}

/// Health of every leaderboard hero right now, by player id.
pub fn lobby_health_now(board: &Board) -> BTreeMap<i64, i64> {
    leaderboard_heroes(board)
        .into_iter()
        .map(|(pid, hero)| (pid, hero_health(hero)))
        .collect()
}

pub fn board_of(board: &Board, controller: i64) -> Vec<Minion> {
    let mut minions: Vec<&Entity> = board
        .iter()
        .filter(|e| {
            e.is("CARDTYPE", "MINION") && e.is("ZONE", "PLAY") && e.int("CONTROLLER") == controller
        })
        .collect();
    minions.sort_by_key(|e| (e.int("ZONE_POSITION"), e.id));
    minions
        .into_iter()
        .map(|e| Minion {
            card_id: e.card_id.clone(),
            atk: e.int("ATK"),
            health: e.int("HEALTH").saturating_sub(e.int("DAMAGE")),
            position: e.int("ZONE_POSITION"),
            golden: is_golden(e.card_id.as_deref()),
        })
        .collect()
}

/// Lobby heroes by player id, in first-seen order (a later entity replaces an
/// earlier one for the same player).
///
/// When the local player is eliminated the game copies its leaderboard hero
/// (same PLAYER_ID, COPIED_FROM_ENTITY_ID = the original, stale place, DAMAGE
/// back to 0). The original keeps the real damage and gets the final place, so
/// a copy of another leaderboard hero is skipped. Other copies stay: the
/// opponents' leaderboard heroes are copies of entities that are not.
pub fn leaderboard_heroes(board: &Board) -> Vec<(i64, &Entity)> {
    let candidates: Vec<&Entity> = board
        .iter()
        .filter(|e| {
            e.is("CARDTYPE", "HERO") && e.int("PLAYER_ID") != 0 && e.has("PLAYER_LEADERBOARD_PLACE")
        })
        .collect();
    let ids: Vec<i64> = candidates.iter().map(|e| e.id).collect();
    let mut heroes: Vec<(i64, &Entity)> = Vec::new();
    for e in candidates {
        if e.opt_int("COPIED_FROM_ENTITY_ID")
            .is_some_and(|src| ids.contains(&src))
        {
            continue;
        }
        let pid = e.int("PLAYER_ID");
        match heroes.iter_mut().find(|(p, _)| *p == pid) {
            Some(slot) => slot.1 = e,
            None => heroes.push((pid, e)),
        }
    }
    heroes
}
