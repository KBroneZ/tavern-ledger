//! Entity state replayed from packets, plus player name resolution.
//!
//! Mirrors what the Python prototype got from hslog's EntityTreeExporter and
//! PlayerManager (MIT): FULL_ENTITY creates or resets an entity, SHOW_ENTITY
//! and CHANGE_ENTITY set the card id and merge tags, HIDE_ENTITY changes
//! nothing, TAG_CHANGE sets one tag. An operation on an entity that was never
//! created is an error, as in hslog.

use std::collections::HashMap;

use crate::power::{EntityRef, ParseError, Value};

pub type Tags = HashMap<String, Value>;

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

    /// FULL_ENTITY: an id seen again (GAME_RESET) keeps its place but is reset.
    pub fn create(&mut self, entity: Entity) {
        match self.index.get(&entity.id) {
            Some(&i) => self.entities[i] = entity,
            None => {
                self.index.insert(entity.id, self.entities.len());
                self.entities.push(entity);
            }
        }
    }

    fn get_mut(&mut self, id: i64) -> Result<&mut Entity, ParseError> {
        let i = *self.index.get(&id).ok_or(ParseError("EntityNotFound"))?;
        Ok(&mut self.entities[i])
    }

    pub fn exists(&self, id: i64) -> Result<(), ParseError> {
        self.get(id).map(|_| ()).ok_or(ParseError("EntityNotFound"))
    }

    pub fn show(&mut self, id: i64, card_id: String, tags: Tags) -> Result<(), ParseError> {
        let entity = self.get_mut(id)?;
        entity.card_id = Some(card_id);
        entity.tags.extend(tags);
        Ok(())
    }

    pub fn change(&mut self, id: i64, card_id: String, tags: Tags) -> Result<(), ParseError> {
        let entity = self.get_mut(id)?;
        if entity.card_id.is_none() {
            return Err(ParseError("ExporterError"));
        }
        entity.card_id = Some(card_id);
        entity.tags.extend(tags);
        Ok(())
    }

    pub fn set_tag(&mut self, id: i64, tag: String, value: Value) -> Result<(), ParseError> {
        self.get_mut(id)?.tags.insert(tag, value);
        Ok(())
    }
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
    fn change_entity_without_card_id_is_an_error() {
        let mut b = Board::default();
        b.create(Entity {
            id: 5,
            ..Entity::default()
        });
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
