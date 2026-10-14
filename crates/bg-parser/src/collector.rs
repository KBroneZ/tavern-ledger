//! Replays one game's packets and records what the report needs: the local
//! and shop (Bob) players, the tribes offered in the shop, who steps into each
//! combat with which board, the local health after each round, each combat's
//! result as the game records it, the hero pick and the skins' base heroes.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::discover::DiscoverTracker;
use crate::hero_select::HeroSelectTracker;
use crate::leaderboard::{is_leaderboard_tag, LeaderboardTracker};
use crate::power::{EntityRef, Packet, ParseError, Value};
use crate::replay::{board_entities, minion_of, ReplayTracker, Sides};
use crate::report::{
    AttackSlot, CombatResult, CombatValues, DefenderSource, LoggedAttack, Minion, Side,
};
use crate::shop::{Seats, ShopTracker};
use crate::state::{Board, Entity, Players, Tags, MAX_TAGS};

/// An entity packet whose `tag=` lines are still arriving.
#[derive(Debug)]
enum Pending {
    Game(i64),
    Player { id: i64, player_id: i64, lo: u64 },
    Full { id: i64, card_id: Option<String> },
    Show { id: i64, card_id: String },
    Change { id: i64, card_id: String },
}

impl Pending {
    fn id(&self) -> i64 {
        match self {
            Pending::Game(id)
            | Pending::Player { id, .. }
            | Pending::Full { id, .. }
            | Pending::Show { id, .. }
            | Pending::Change { id, .. } => *id,
        }
    }
}

#[derive(Clone, Debug)]
struct OpenLeg {
    explicit: bool, // false: the local hero by default, until a hero switch
    hero: Option<String>,
    hint: Option<i64>,
}

/// (side, hero card id, combat player id hint, board or None if not visible)
pub type RawEntry = (Side, Option<String>, Option<i64>, Option<Vec<Minion>>);

/// Hero card ids whose skin link is kept per game: far more than a lobby shows.
pub const MAX_SKIN_LINKS: usize = 256;
/// Database ids are positive and far below this.
pub const MAX_DBF_ID: i64 = 10_000_000;
const SKIN_PARENT: &str = "BACON_SKIN_PARENT_ID";
const RESULT_TAGS: [&str; 2] = ["BACON_WON_LAST_COMBAT", "DAMAGE_DEALT_TO_HERO_LAST_TURN"];
/// Attacks kept per combat: far more than a combat has (about 30), so a
/// strange log cannot grow the record.
pub const MAX_ATTACKS: usize = 300;
/// The game's `TURN` counts shop turns and combats, so it stays below twice
/// the most shop turns a game keeps. Any other value is not a game this
/// parser knows: the game is unsupported, never read with a wild number.
pub const MAX_GAME_TURN: i64 = 2 * crate::shop::MAX_TURNS as i64 + 1;

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
    /// (round, index in `entries`) -> the side's game-long values as its
    /// board was taken (T-609).
    pub entry_values: BTreeMap<(i64, usize), CombatValues>,
    pub health: BTreeMap<i64, Option<i64>>,
    /// Round -> health of every lobby hero by player id when its combat
    /// closed. Round 0 is the health before the first combat.
    pub lobby_health: BTreeMap<i64, BTreeMap<i64, i64>>,
    open_own: Option<OpenLeg>,
    open_opp: Option<OpenLeg>,
    /// The player's own shop and actions (T-204, T-205).
    pub shop: ShopTracker,
    /// The player's own hero pick (T-209).
    pub hero_select: HeroSelectTracker,
    /// The player's own discover picks (T-211, T-212).
    pub discovers: DiscoverTracker,
    /// Triples and trinkets on the leaderboard (T-207).
    pub leaderboard: LeaderboardTracker,
    /// Round -> the combat's result as the game records it on the local
    /// player (T-206); None when its tags do not agree with each other.
    pub results: BTreeMap<i64, Option<CombatResult>>,
    /// Hero card id -> database id of its base hero (`BACON_SKIN_PARENT_ID`,
    /// T-214); None when the log gives one card two different links.
    pub skin_parents: BTreeMap<String, Option<i64>>,
    /// The log set a combat result tag on the local player at least once. A
    /// game without any (a log that lacks them) has no read results: all
    /// ties would be a guess.
    pub result_tags_seen: bool,
    /// Round -> its attacks in the log's order (T-603b).
    pub attacks: BTreeMap<i64, Vec<LoggedAttack>>,
    /// The newest attack while its defender is not known yet: (round, index).
    open_attack: Option<(i64, usize)>,
    /// Round -> the player's own board as the shop turn before it ended
    /// (T-608).
    pub shop_boards: BTreeMap<i64, Vec<Minion>>,
    /// Each shown combat, event by event (T-215).
    pub replays: ReplayTracker,
    /// Time of the line being read, in ms from the game's first line.
    pub now_ms: i64,
    /// Nesting of the line being read (0 is the log's top level).
    pub indent: usize,
}

impl Collector {
    pub fn feed(&mut self, packet: Packet) -> Result<(), ParseError> {
        if let Packet::Tag { tag, value } = packet {
            return match self.pending.as_mut() {
                Some((_, tags)) => {
                    if tags.len() >= MAX_TAGS && !tags.contains_key(&tag) {
                        return Err(ParseError("TooManyTags"));
                    }
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
            Packet::BlockStart {
                block_type,
                entity,
                target,
                trigger,
            } => {
                self.mention(&entity);
                if block_type == "ATTACK" && self.in_combat() {
                    self.snapshot_open_legs();
                    self.on_attack(&entity);
                }
                // Every block: an attack or a trigger is a step, and any
                // block groups the steps inside it into one beat.
                let id = match entity {
                    EntityRef::Id(id) => Some(id),
                    _ => None,
                };
                let sides = self.sides();
                self.replays.on_block_start(
                    &block_type,
                    id,
                    trigger.as_deref(),
                    self.indent,
                    &self.board,
                    sides,
                );
                if self.indent == 0 {
                    let seats = self.seats();
                    // A player is named, not numbered (the turn's own gold
                    // is set in a block of the player entity).
                    let entity = match &entity {
                        EntityRef::Id(id) => Some(*id),
                        EntityRef::Name(_) => self.players.entity_id(&entity, &self.board).ok(),
                        EntityRef::Game => None,
                    };
                    let by_id = |r: &EntityRef| match r {
                        EntityRef::Id(id) => self.board.get(*id),
                        _ => None,
                    };
                    let entity = entity.and_then(|id| self.board.get(id));
                    let target = target.as_ref().and_then(by_id);
                    self.shop
                        .on_top_block(&block_type, entity, target, &self.board, seats);
                }
            }
            Packet::BlockEnd => {
                self.replays.on_block_end(self.indent);
                if self.indent == 0 {
                    let seats = self.seats();
                    self.shop.on_top_block_end(&self.board, seats);
                }
            }
            Packet::Mention(entity) => self.mention(&entity),
            Packet::Ignored | Packet::Tag { .. } => {}
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

    /// The two sides of a combat on screen (T-215).
    fn sides(&self) -> Sides {
        Sides {
            own: self.local_pid,
            opponent: self.dummy_pid,
        }
    }

    pub fn seats(&self) -> Seats {
        Seats {
            local: self.local,
            local_pid: self.local_pid,
            dummy_pid: self.dummy_pid,
        }
    }

    fn shop_sees(&mut self, id: i64) {
        let seats = self.seats();
        if let Some(entity) = self.board.get(id) {
            self.shop.on_maybe_offer(entity, seats);
        }
    }

    fn commit(&mut self) -> Result<(), ParseError> {
        let Some((pending, tags)) = self.pending.take() else {
            return Ok(());
        };
        if matches!(pending, Pending::Game(_)) || Some(pending.id()) == self.board.game_id {
            if let Some(turn) = tags.get("TURN") {
                check_turn(turn)?;
            }
        }
        match pending {
            Pending::Game(id) => self.board.create(Entity {
                id,
                card_id: None,
                tags,
            })?,
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
                self.board.create(player)?;
            }
            Pending::Full { id, card_id } => {
                let parent = skin_parent(&tags);
                let shown = leaderboard_values(&tags);
                self.board.create(Entity { id, card_id, tags })?;
                self.note_skin(id, parent);
                self.note_leaderboard_packet(id, shown);
                self.track_shop(id);
                self.shop_sees(id);
                let sides = self.sides();
                self.replays.on_created(id, &self.board, sides);
            }
            Pending::Show { id, card_id } => {
                let parent = skin_parent(&tags);
                let shown = leaderboard_values(&tags);
                self.board.show(id, card_id, tags)?;
                self.note_skin(id, parent);
                self.note_leaderboard_packet(id, shown);
                self.shop_sees(id);
                let sides = self.sides();
                self.replays.on_shown(id, &self.board, sides);
            }
            Pending::Change { id, card_id } => {
                let parent = skin_parent(&tags);
                self.hero_select.on_change(id, &card_id);
                let shown = leaderboard_values(&tags);
                self.board.change(id, card_id, tags)?;
                self.note_skin(id, parent);
                self.note_leaderboard_packet(id, shown);
                self.shop_sees(id);
                let sides = self.sides();
                self.replays.on_shown(id, &self.board, sides);
            }
        }
        Ok(())
    }

    /// Keeps the base hero link a hero packet carries for its card. Read from
    /// the packet's own tags: an entity changed to another card keeps the old
    /// card's tags.
    fn note_skin(&mut self, id: i64, parent: Option<i64>) {
        let Some(parent) = parent else { return };
        let Some(card) = self
            .board
            .get(id)
            .filter(|e| e.is("CARDTYPE", "HERO"))
            .and_then(|e| e.card_id.clone())
        else {
            return;
        };
        let full = self.skin_parents.len() >= MAX_SKIN_LINKS;
        match self.skin_parents.get_mut(&card) {
            Some(seen) if *seen != Some(parent) => *seen = None,
            Some(_) => {}
            None if !full => {
                self.skin_parents.insert(card, Some(parent));
            }
            None => {}
        }
    }

    /// A leaderboard tag changed on entity `id`: kept when it is a lobby
    /// hero, never a combat copy.
    fn note_leaderboard_packet(&mut self, id: i64, shown: Vec<(String, i64)>) {
        for (tag, value) in shown {
            self.note_leaderboard(id, &tag, value);
        }
    }

    fn note_leaderboard(&mut self, id: i64, tag: &str, value: i64) {
        let pid = leaderboard_heroes(&self.board)
            .into_iter()
            .find(|(_, h)| h.id == id)
            .map(|(pid, _)| pid);
        if let Some(pid) = pid {
            let turn = self.turn();
            self.leaderboard.on_tag(pid, tag, value, turn);
        }
    }

    /// The last combat's result as the game records it on the local player:
    /// `BACON_WON_LAST_COMBAT` 1 is a win, else `DAMAGE_DEALT_TO_HERO_LAST_TURN`
    /// above 0 a loss, else a tie. Both are set back to 0 when a combat starts
    /// (the log leaves out tags that are 0). Anything else is unknown.
    fn combat_result(&self) -> Option<CombatResult> {
        let player = self.local.and_then(|id| self.board.get(id))?;
        match (player.int(RESULT_TAGS[0]), player.int(RESULT_TAGS[1])) {
            (1, 0) => Some(CombatResult::Won),
            (0, d) if d > 0 => Some(CombatResult::Lost),
            (0, 0) => Some(CombatResult::Tie),
            _ => None,
        }
    }

    pub fn turn(&self) -> i64 {
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
        if is_turn {
            check_turn(&value)?;
        }
        let is_hero = tag == "HERO_ENTITY";
        let is_shop = tag == "ZONE" || tag == "CONTROLLER";
        let is_skin_parent = tag == SKIN_PARENT;
        // A value that is not a number is not one the leaderboard shows.
        let leaderboard_tag = match &value {
            Value::Int(v) if is_leaderboard_tag(&tag) => Some((tag.clone(), *v)),
            _ => None,
        };
        if Some(id) == self.local && RESULT_TAGS.contains(&tag.as_str()) {
            self.result_tags_seen = true;
        }
        let is_over = Some(id) == self.board.game_id
            && tag == "STATE"
            && value == Value::Name("COMPLETE".into());
        let new_int = if let Value::Int(v) = value { v } else { 0 };
        let old = self.board.get(id).and_then(|e| e.opt_int(&tag));
        let shop_tag = matches!(
            tag.as_str(),
            "FROZEN"
                | "PLAYER_TECH_LEVEL"
                | "ZONE"
                | "CONTROLLER"
                | "RESOURCES"
                | "TEMP_RESOURCES"
                | "RESOURCES_USED"
                | "BACON_FREE_REFRESH_COUNT"
        )
        .then(|| tag.clone());
        self.board.set_tag(id, tag.clone(), value)?;
        self.note_defender(id, &tag, old, new_int);
        let sides = self.sides();
        self.replays.on_tag(id, &tag, old, &self.board, sides);
        if is_skin_parent {
            self.note_skin(id, Some(new_int).filter(|v| (1..=MAX_DBF_ID).contains(v)));
        }
        if let Some((tag, v)) = leaderboard_tag {
            self.note_leaderboard(id, &tag, v);
        }
        if is_turn {
            self.on_turn(new_int, old);
            let seats = self.seats();
            self.shop.on_turn(new_int, self.now_ms, &self.board, seats);
        } else if is_hero && self.in_combat() {
            self.on_hero_switch(id, new_int);
        } else if is_shop {
            self.track_shop(id);
        }
        if is_over {
            self.shop.on_game_over(self.now_ms);
            self.replays.on_game_over();
            // The game ends right after its last combat: no shop turn follows.
            if self.in_combat() {
                let result = self.combat_result();
                self.results.entry(self.turn() / 2).or_insert(result);
            }
        }
        if let (Some(tag), Some(entity)) = (shop_tag, self.board.get(id)) {
            let seats = self.seats();
            self.shop.on_tag(entity, &tag, old, &self.board, seats);
        }
        Ok(())
    }

    fn on_turn(&mut self, turn: i64, old: Option<i64>) {
        self.open_attack = None;
        self.replays.on_turn(turn);
        // The previous turn's legs that never reached an attack were not visible.
        let previous = turn.saturating_sub(1).div_euclid(2);
        // A combat the log has from start to end: its result is read as the
        // next shop turn starts.
        let combat_seen = old.is_some_and(|o| o >= 2 && o % 2 == 0 && o + 1 == turn);
        if combat_seen {
            let result = self.combat_result();
            self.results.insert(previous, result);
        }
        self.close_unseen(Side::Own, previous);
        self.close_unseen(Side::Opponent, previous);
        if self.in_combat() {
            let shop_seen = old.is_some_and(|o| o % 2 == 1 && o + 1 == turn);
            if let Some(pid) = self.local_pid.filter(|_| shop_seen) {
                self.shop_boards
                    .insert(turn / 2, board_of(&self.board, pid));
            }
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
        self.replays.on_hero_switch(side, hero.clone());
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

    /// The game-long values on the player entity of `side` (T-609): the
    /// local player, or the dummy player that stands for the opponent.
    pub(crate) fn combat_values(&self, side: Side) -> CombatValues {
        let player = match side {
            Side::Own => self.local,
            Side::Opponent => self.dummy,
        };
        let Some(p) = player.and_then(|id| self.board.get(id)) else {
            return CombatValues::default();
        };
        // A player enchantment's two numbers (`TAG_SCRIPT_DATA_NUM_1`, `_2`),
        // when one with this card id is in play on the player.
        let enchant = |card: &str| -> (i64, i64) {
            self.board
                .iter()
                .find(|e| {
                    e.card_id.as_deref() == Some(card)
                        && e.int("ATTACHED") == p.id
                        && e.is("ZONE", "PLAY")
                })
                .map_or((0, 0), |e| {
                    (
                        e.int("TAG_SCRIPT_DATA_NUM_1"),
                        e.int("TAG_SCRIPT_DATA_NUM_2"),
                    )
                })
        };
        CombatValues {
            blood_gem: (
                p.int("BACON_BLOODGEMBUFFATKVALUE"),
                p.int("BACON_BLOODGEMBUFFHEALTHVALUE"),
            ),
            spell: (
                p.int("TAVERN_SPELL_ATTACK_INCREASE"),
                p.int("TAVERN_SPELL_HEALTH_INCREASE"),
            ),
            knights_dead: enchant("BG25_008pe").0,
            undead_attack: enchant("BG25_011pe").0,
            beetle: enchant("BG31_808pe"),
        }
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
                let values = self.combat_values(side);
                let list = self.entries.entry(round).or_default();
                if board.is_some() {
                    self.entry_values.insert((round, list.len()), values);
                }
                list.push((side, hero, hint, board));
            }
        }
        self.open_own = None;
        self.open_opp = None;
    }

    /// Which side of the combat a minion's controller is on.
    fn combat_side(&self, controller: i64) -> Option<Side> {
        if Some(controller) == self.local_pid {
            Some(Side::Own)
        } else if Some(controller) == self.dummy_pid {
            Some(Side::Opponent)
        } else {
            None
        }
    }

    fn slot(&self, id: i64) -> Option<AttackSlot> {
        let e = self.board.get(id).filter(|e| e.is("CARDTYPE", "MINION"))?;
        Some(AttackSlot {
            side: self.combat_side(e.int("CONTROLLER"))?,
            position: e.int("ZONE_POSITION"),
            card_id: e.card_id.clone(),
        })
    }

    /// An attack starts (T-603b): its attacker and both boards as they are
    /// now. A minion of neither combat side (a Duos teammate's copy) is left
    /// out.
    fn on_attack(&mut self, entity: &EntityRef) {
        self.open_attack = None;
        let EntityRef::Id(id) = entity else { return };
        let Some(attacker) = self.slot(*id) else {
            return;
        };
        let round = self.turn() / 2;
        let board = |pid: Option<i64>| pid.map(|p| board_of(&self.board, p)).unwrap_or_default();
        let attack = LoggedAttack {
            attacker,
            defender: None,
            own: board(self.local_pid),
            opponent: board(self.dummy_pid),
        };
        let list = self.attacks.entry(round).or_default();
        if list.len() < MAX_ATTACKS {
            list.push(attack);
            self.open_attack = Some((round, list.len() - 1));
        }
    }

    /// The defender of the open attack: the game's `PROPOSED_DEFENDER`, or,
    /// in a log without it, the first enemy minion the attack damages or
    /// strips of its Divine Shield.
    fn note_defender(&mut self, id: i64, tag: &str, old: Option<i64>, new: i64) {
        let Some((round, index)) = self.open_attack else {
            return;
        };
        let found = if Some(id) == self.board.game_id && tag == "PROPOSED_DEFENDER" && new > 0 {
            self.replays.on_defender(new);
            self.slot(new).map(|s| (s, DefenderSource::Game))
        } else {
            let old = old.unwrap_or(0);
            let hit =
                (tag == "DAMAGE" && new > old) || (tag == "DIVINE_SHIELD" && new == 0 && old > 0);
            hit.then(|| self.slot(id))
                .flatten()
                .map(|s| (s, DefenderSource::FirstHit))
        };
        let Some((slot, source)) = found else { return };
        let Some(attack) = self.attacks.get_mut(&round).and_then(|a| a.get_mut(index)) else {
            return;
        };
        if slot.side != attack.attacker.side {
            attack.defender = Some((slot, source));
            self.open_attack = None;
        }
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

/// A `TURN` value a real game can have (see [`MAX_GAME_TURN`]).
fn check_turn(value: &Value) -> Result<(), ParseError> {
    match value {
        Value::Int(turn) if (0..=MAX_GAME_TURN).contains(turn) => Ok(()),
        _ => Err(ParseError("UnsupportedTurn")),
    }
}

/// The leaderboard tags a packet carries with numbers, so a hero created
/// or shown with a count already set keeps it.
fn leaderboard_values(tags: &Tags) -> Vec<(String, i64)> {
    tags.iter()
        .filter(|(tag, _)| is_leaderboard_tag(tag))
        .filter_map(|(tag, value)| match value {
            Value::Int(v) => Some((tag.clone(), *v)),
            _ => None,
        })
        .collect()
}

/// A base hero link in a packet's tags: a database id, or None (0 is none).
fn skin_parent(tags: &Tags) -> Option<i64> {
    match tags.get(SKIN_PARENT) {
        Some(Value::Int(v)) if (1..=MAX_DBF_ID).contains(v) => Some(*v),
        _ => None,
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

/// Tripled minions have their own card id.
pub fn is_golden(card_id: Option<&str>) -> bool {
    card_id.is_some_and(|c| c.ends_with("_G") || c.starts_with("TB_BaconUps_"))
}

/// A board minion is golden when it has `PREMIUM` or a golden card id
/// (parser revision 6). `PREMIUM` also marks cards that are always golden and
/// golden tokens, which have no golden id; on the user's 11 games it is on
/// every golden-id board minion and on no other board minion at a combat. The
/// game sets it for a moment on minions set aside during a triple in the shop,
/// which never reach a board this way.
pub fn is_golden_minion(e: &Entity) -> bool {
    e.int("PREMIUM") > 0 || is_golden(e.card_id.as_deref())
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

/// The minions of `controller` on the board, left to right, with their
/// Dark Gifts (T-528) and buffs (T-619).
pub fn board_of(board: &Board, controller: i64) -> Vec<Minion> {
    let mut gifts = crate::gifts::gift_index(board);
    let mut buffs = crate::buffs::buff_index(board);
    board_entities(board, controller)
        .into_iter()
        .map(|e| {
            minion_of(
                e,
                gifts.remove(&e.id).unwrap_or_default(),
                buffs.remove(&e.id).unwrap_or_default(),
            )
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
        .heroes()
        .filter(|e| e.int("PLAYER_ID") != 0 && e.has("PLAYER_LEADERBOARD_PLACE"))
        .collect();
    let ids: HashSet<i64> = candidates.iter().map(|e| e.id).collect();
    let mut heroes: Vec<(i64, &Entity)> = Vec::new();
    let mut slots: HashMap<i64, usize> = HashMap::new();
    for e in candidates {
        if e.opt_int("COPIED_FROM_ENTITY_ID")
            .is_some_and(|src| ids.contains(&src))
        {
            continue;
        }
        let pid = e.int("PLAYER_ID");
        match slots.get(&pid) {
            Some(&slot) => heroes[slot].1 = e,
            None => {
                slots.insert(pid, heroes.len());
                heroes.push((pid, e));
            }
        }
    }
    heroes
}
