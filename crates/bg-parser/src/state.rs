//! Entity state replayed from packets, plus player name resolution.
//!
//! Mirrors what the Python prototype got from hslog's EntityTreeExporter and
//! PlayerManager (MIT): FULL_ENTITY creates or resets an entity, SHOW_ENTITY
//! and CHANGE_ENTITY set the card id and merge tags, HIDE_ENTITY changes
//! nothing, TAG_CHANGE sets one tag. An operation on an entity that was never
//! created is an error, as in hslog.

use std::collections::{BTreeSet, HashMap};

use crate::power::{EntityRef, ParseError, Value};

pub type Tags = HashMap<String, Value>;

/// Entities in one game: several times the most a long real game creates
/// (entity ids reach about 21,000 by turn 20), so a broken or hostile log
/// cannot grow the board without end.
pub const MAX_ENTITIES: usize = 200_000;
/// Different tags on one entity: far more than the game gives any entity.
pub const MAX_TAGS: usize = 1_000;
/// Tags on all the entities of one game together: many times what a long
/// real game holds, so the board's memory stays bounded however the
/// entities and their tags are spread.
pub const MAX_BOARD_TAGS: usize = 10_000_000;
/// Hero entities in one game (a 20-turn game has about 80, combat copies
/// included). The lobby is looked up among these on every leaderboard change.
pub const MAX_HEROES: usize = 5_000;

#[derive(Clone, Debug, Default)]
pub struct Entity {
    pub id: i64,
    pub card_id: Option<String>,
    pub tags: Tags,
}

impl Entity {
    pub fn int(&self, tag: &str) -> i64 {
        match self.tags.get(tag) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        }
    }

    pub fn opt_int(&self, tag: &str) -> Option<i64> {
        match self.int(tag) {
            0 => None,
            v => Some(v),
        }
    }

    pub fn is(&self, tag: &str, name: &str) -> bool {
        matches!(self.tags.get(tag), Some(Value::Name(v)) if v == name)
    }

    pub fn has(&self, tag: &str) -> bool {
        self.tags.contains_key(tag)
    }
}

/// All entities of one game, in creation order.
#[derive(Debug, Default)]
pub struct Board {
    entities: Vec<Entity>,
    index: HashMap<i64, usize>,
    /// Positions in `entities` of every entity that has been a hero.
    heroes: BTreeSet<usize>,
    /// Tags on all entities together, and how many there may be.
    tags: usize,
    tag_limit: Option<usize>,
    pub game_id: Option<i64>,
}

impl Board {
    pub fn get(&self, id: i64) -> Option<&Entity> {
        self.index.get(&id).map(|&i| &self.entities[i])
    }

    pub fn game(&self) -> Option<&Entity> {
        self.game_id.and_then(|id| self.get(id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    /// The entities that are heroes now, in creation order, without going
    /// through every entity of the game.
    pub fn heroes(&self) -> impl Iterator<Item = &Entity> {
        self.heroes
            .iter()
            .map(|&i| &self.entities[i])
            .filter(|e| e.is("CARDTYPE", "HERO"))
    }

    /// FULL_ENTITY: an id seen again (GAME_RESET) keeps its place but is reset.
    pub fn create(&mut self, entity: Entity) -> Result<(), ParseError> {
        check_tags(&entity.tags)?;
        let i = match self.index.get(&entity.id) {
            Some(&i) => {
                self.count_tags(entity.tags.len(), self.entities[i].tags.len())?;
                self.entities[i] = entity;
                i
            }
            None => {
                if self.entities.len() >= MAX_ENTITIES {
                    return Err(ParseError("TooManyEntities"));
                }
                self.count_tags(entity.tags.len(), 0)?;
                self.index.insert(entity.id, self.entities.len());
                self.entities.push(entity);
                self.entities.len() - 1
            }
        };
        self.note_hero(i)
    }

    /// Keeps the board's tag count, refusing to grow past [`MAX_BOARD_TAGS`].
    fn count_tags(&mut self, added: usize, removed: usize) -> Result<(), ParseError> {
        let total = (self.tags + added).saturating_sub(removed);
        if added > removed && total > self.tag_limit.unwrap_or(MAX_BOARD_TAGS) {
            return Err(ParseError("TooManyTags"));
        }
        self.tags = total;
        Ok(())
    }

    fn get_mut(&mut self, id: i64) -> Result<&mut Entity, ParseError> {
        let i = *self.index.get(&id).ok_or(ParseError("EntityNotFound"))?;
        Ok(&mut self.entities[i])
    }

    pub fn exists(&self, id: i64) -> Result<(), ParseError> {
        self.get(id).map(|_| ()).ok_or(ParseError("EntityNotFound"))
    }

    pub fn show(&mut self, id: i64, card_id: String, tags: Tags) -> Result<(), ParseError> {
        self.merge(id, tags)?;
        self.get_mut(id)?.card_id = Some(card_id);
        self.note_hero_id(id)
    }

    pub fn change(&mut self, id: i64, card_id: String, tags: Tags) -> Result<(), ParseError> {
        if self.get_mut(id)?.card_id.is_none() {
            return Err(ParseError("ExporterError"));
        }
        self.merge(id, tags)?;
        self.get_mut(id)?.card_id = Some(card_id);
        self.note_hero_id(id)
    }

    /// Adds a packet's tags to an entity, within the per-entity and the
    /// board's limits.
    fn merge(&mut self, id: i64, tags: Tags) -> Result<(), ParseError> {
        let entity = self.get_mut(id)?;
        let new = tags
            .keys()
            .filter(|t| !entity.tags.contains_key(*t))
            .count();
        if entity.tags.len() + new > MAX_TAGS {
            return Err(ParseError("TooManyTags"));
        }
        self.count_tags(new, 0)?;
        self.get_mut(id)?.tags.extend(tags);
        Ok(())
    }

    pub fn set_tag(&mut self, id: i64, tag: String, value: Value) -> Result<(), ParseError> {
        let is_type = tag == "CARDTYPE";
        let entity = self.get_mut(id)?;
        if !entity.tags.contains_key(&tag) {
            if entity.tags.len() >= MAX_TAGS {
                return Err(ParseError("TooManyTags"));
            }
            self.count_tags(1, 0)?;
        }
        self.get_mut(id)?.tags.insert(tag, value);
        if is_type {
            self.note_hero_id(id)?;
        }
        Ok(())
    }

    fn note_hero_id(&mut self, id: i64) -> Result<(), ParseError> {
        match self.index.get(&id) {
            Some(&i) => self.note_hero(i),
            None => Ok(()),
        }
    }

    fn note_hero(&mut self, i: usize) -> Result<(), ParseError> {
        if !self.entities[i].is("CARDTYPE", "HERO") || self.heroes.contains(&i) {
            return Ok(());
        }
        if self.heroes.len() >= MAX_HEROES {
            return Err(ParseError("TooManyHeroes"));
        }
        self.heroes.insert(i);
        Ok(())
    }
}

/// A packet's tags fit on one entity.
pub fn check_tags(tags: &Tags) -> Result<(), ParseError> {
    if tags.len() > MAX_TAGS {
        return Err(ParseError("TooManyTags"));
    }
    Ok(())
}

const UNKNOWN_HUMAN_PLAYER: &str = "UNKNOWN HUMAN PLAYER";

#[derive(Debug, Default, Clone, Copy)]
struct PlayerSlot {
    entity_id: Option<i64>,
    player_id: Option<i64>,
}

/// Maps player names to player entities, like hslog's PlayerManager does for
/// Battlegrounds: the names announced by DebugPrintGame belong to their
/// players; any other name (the opponent shown on Bob's side during a
/// combat) belongs to the AI player. Names live only in memory.
#[derive(Debug, Default)]
pub struct Players {
    slots: Vec<PlayerSlot>,
    by_name: HashMap<String, usize>,
    aliases: HashMap<String, String>,
    ai: Option<usize>,
}

impl Players {
    pub fn register_entity(&mut self, entity_id: i64, player_id: i64, is_ai: bool) {
        let slot = self
            .find(|s| s.entity_id == Some(entity_id) || s.player_id == Some(player_id))
            .unwrap_or_else(|| self.push(PlayerSlot::default()));
        self.slots[slot].entity_id.get_or_insert(entity_id);
        self.slots[slot].player_id.get_or_insert(player_id);
        if is_ai {
            self.ai = Some(slot);
        }
    }

    /// "PlayerID=N, PlayerName=..." from DebugPrintGame.
    pub fn register_name(&mut self, player_id: i64, name: &str) {
        if name == UNKNOWN_HUMAN_PLAYER || self.lookup(name).is_some() {
            return;
        }
        let slot = match self.find(|s| s.player_id == Some(player_id)) {
            Some(slot) => slot,
            None => self.guess(name).unwrap_or_else(|| {
                self.push(PlayerSlot {
                    entity_id: None,
                    player_id: Some(player_id),
                })
            }),
        };
        self.remember(name, slot);
    }

    /// Entity id for a name used in a packet; MissingPlayerData if the name
    /// belongs to no player entity (as hslog would raise).
    pub fn resolve(&mut self, name: &str) -> Result<i64, ParseError> {
        let missing = ParseError("MissingPlayerData");
        if name == UNKNOWN_HUMAN_PLAYER {
            return Err(missing);
        }
        let slot = match self.lookup(name) {
            Some(slot) => slot,
            None => {
                let slot = self
                    .guess(name)
                    .unwrap_or_else(|| self.push(PlayerSlot::default()));
                self.remember(name, slot);
                slot
            }
        };
        self.slots[slot].entity_id.ok_or(missing)
    }

    /// A third (or later) name in a game with an AI player is the AI player.
    fn guess(&self, _name: &str) -> Option<usize> {
        if self.by_name.len() > 1 {
            self.ai
        } else {
            None
        }
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.by_name
            .get(name)
            .or_else(|| {
                self.aliases
                    .get(name)
                    .and_then(|full| self.by_name.get(full))
            })
            .copied()
    }

    fn remember(&mut self, name: &str, slot: usize) {
        self.by_name.insert(name.to_string(), slot);
        if let Some((alias, _)) = name.split_once('#') {
            self.aliases
                .entry(alias.to_string())
                .or_insert_with(|| name.to_string());
        }
    }

    fn find(&self, pred: impl Fn(&PlayerSlot) -> bool) -> Option<usize> {
        self.slots.iter().position(pred)
    }

    fn push(&mut self, slot: PlayerSlot) -> usize {
        self.slots.push(slot);
        self.slots.len() - 1
    }

    /// Entity id for any entity reference.
    pub fn entity_id(&mut self, entity: &EntityRef, board: &Board) -> Result<i64, ParseError> {
        match entity {
            EntityRef::Id(id) => Ok(*id),
            EntityRef::Game => board.game_id.ok_or(ParseError("EntityNotFound")),
            EntityRef::Name(name) => self.resolve(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players() -> Players {
        let mut p = Players::default();
        p.register_entity(2, 5, false);
        p.register_entity(3, 13, true);
        p.register_name(5, "Local#1234");
        p.register_name(13, "Bob");
        p
    }

    #[test]
    fn announced_names_resolve_to_their_players() {
        let mut p = players();
        assert_eq!(p.resolve("Local#1234"), Ok(2));
        assert_eq!(p.resolve("Local"), Ok(2));
        assert_eq!(p.resolve("Bob"), Ok(3));
    }

    #[test]
    fn other_names_are_the_ai_player() {
        let mut p = players();
        assert_eq!(p.resolve("Opponent#999"), Ok(3));
        assert_eq!(p.resolve("Opponent#999"), Ok(3));
    }

    #[test]
    fn unknown_human_player_is_missing() {
        let mut p = players();
        assert_eq!(
            p.resolve(UNKNOWN_HUMAN_PLAYER),
            Err(ParseError("MissingPlayerData"))
        );
    }

    #[test]
    fn a_name_before_two_are_known_is_missing() {
        let mut p = Players::default();
        p.register_entity(2, 5, false);
        assert_eq!(p.resolve("Someone"), Err(ParseError("MissingPlayerData")));
    }

    #[test]
    fn tags_spread_over_entities_stop_at_the_board_s_limit() {
        let mut b = Board {
            tag_limit: Some(10),
            ..Board::default()
        };
        let tags = |n: usize| {
            (0..n)
                .map(|t| (format!("T{t}"), Value::Int(1)))
                .collect::<Tags>()
        };
        let entity = |id, n| Entity {
            id,
            tags: tags(n),
            ..Entity::default()
        };
        b.create(entity(1, 4)).unwrap();
        b.create(entity(2, 4)).unwrap();
        // A reset entity gives its old tags back.
        b.create(entity(1, 2)).unwrap();
        b.set_tag(2, "T9".into(), Value::Int(1)).unwrap();
        b.show(1, "X".into(), tags(3)).unwrap();
        assert_eq!(b.tags, 8);
        b.set_tag(2, "T9".into(), Value::Int(2)).unwrap();
        b.create(entity(3, 2)).unwrap();
        assert_eq!(
            b.set_tag(3, "NEW".into(), Value::Int(1)),
            Err(ParseError("TooManyTags"))
        );
        assert_eq!(b.create(entity(4, 1)), Err(ParseError("TooManyTags")));
        assert_eq!(
            b.show(2, "X".into(), tags(9)),
            Err(ParseError("TooManyTags"))
        );
    }

    #[test]
    fn heroes_are_listed_in_creation_order_while_they_are_heroes() {
        let mut b = Board::default();
        let hero = |id| Entity {
            id,
            tags: Tags::from([("CARDTYPE".to_string(), Value::Name("HERO".into()))]),
            ..Entity::default()
        };
        b.create(hero(7)).unwrap();
        b.create(Entity {
            id: 3,
            ..Entity::default()
        })
        .unwrap();
        b.create(hero(9)).unwrap();
        b.set_tag(3, "CARDTYPE".into(), Value::Name("HERO".into()))
            .unwrap();
        let ids = |b: &Board| b.heroes().map(|e| e.id).collect::<Vec<_>>();
        assert_eq!(ids(&b), [7, 3, 9]);
        b.set_tag(9, "CARDTYPE".into(), Value::Name("MINION".into()))
            .unwrap();
        assert_eq!(ids(&b), [7, 3]);
    }

    #[test]
    fn change_entity_without_card_id_is_an_error() {
        let mut b = Board::default();
        b.create(Entity {
            id: 5,
            ..Entity::default()
        })
        .unwrap();
        assert_eq!(
            b.change(5, "X".into(), Tags::new()),
            Err(ParseError("ExporterError"))
        );
        assert_eq!(
            b.set_tag(6, "ZONE".into(), Value::Int(1)),
            Err(ParseError("EntityNotFound"))
        );
    }
}
