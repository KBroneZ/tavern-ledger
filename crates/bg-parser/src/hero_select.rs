//! The player's own hero pick (T-209, D-055): the heroes offered, the
//! rerolls used and the hero picked. Rust-only: the Python prototype does not
//! read it.
//!
//! What the log gives (checked on 11 real games, docs/research/parser-data-round.md):
//! - `DebugPrintEntityChoices` with `ChoiceType=MULLIGAN` lists the heroes the
//!   player is offered, one `Entities[i]=` line each;
//! - a reroll is a `CHANGE_ENTITY` of one of those heroes to a new card while
//!   the choice is open (its packet carries `BACON_NUM_MULLIGAN_REFRESH_USED`);
//! - `DebugPrintEntitiesChosen` with the same choice id names the hero picked.
//!
//! Only the local player's choice is in the log; other players' offers never are.

use serde::{Deserialize, Serialize};

use crate::power::{parse_entity, EntityRef};
use crate::state::Board;

/// Bounds, so a strange log cannot grow a report without end. The game
/// offers four heroes and allows a few rerolls.
pub const MAX_OFFERED: usize = 32;

const MULLIGAN: &str = "ChoiceType=MULLIGAN";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroSelect {
    /// Every hero the player was shown, by card id: the first offer in the
    /// log's order, then the hero each reroll brought.
    pub offered: Vec<String>,
    /// Heroes rerolled away.
    pub rerolls: u32,
    /// None when the log does not say (the game ended before the pick).
    pub picked: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Reading {
    #[default]
    Nothing,
    /// Inside the hero choice's entity list.
    Offer,
    /// Inside the list of what was chosen for the hero choice.
    Pick,
}

#[derive(Clone, Debug, Default)]
pub struct HeroSelectTracker {
    /// Id of the hero choice, once the log lists it.
    choice: Option<i64>,
    reading: Reading,
    /// Entity id of each hero offered (rerolls keep the entity).
    ids: Vec<i64>,
    record: Option<HeroSelect>,
    /// The game said what was chosen: no more offers or rerolls, even when
    /// the pick could not be read.
    closed: bool,
}

impl HeroSelectTracker {
    /// A `DebugPrintEntityChoices` line.
    pub fn on_choices(&mut self, body: &str, board: &Board, local_pid: Option<i64>) {
        if let Some(entity) = list_entity(body) {
            if self.reading == Reading::Offer {
                self.offer(entity, board, local_pid);
            }
            return;
        }
        if choice_id(body).is_none() {
            return; // "Source=GameEntity" and the like, inside the list
        }
        self.reading = Reading::Nothing;
        // Only the first hero choice of a game.
        if self.choice.is_none() && body.split(' ').any(|f| f == MULLIGAN) {
            self.choice = choice_id(body);
            if self.choice.is_some() {
                self.reading = Reading::Offer;
                self.record = Some(HeroSelect {
                    offered: Vec::new(),
                    rerolls: 0,
                    picked: None,
                });
            }
        }
    }

    /// A `DebugPrintEntitiesChosen` line.
    pub fn on_chosen(&mut self, body: &str, board: &Board) {
        if let Some(entity) = list_entity(body) {
            if self.reading == Reading::Pick {
                self.reading = Reading::Nothing;
                self.pick(entity, board);
            }
            return;
        }
        let open = self.choice.is_some() && !self.closed;
        self.reading = if open && choice_id(body) == self.choice {
            self.closed = true;
            Reading::Pick
        } else {
            Reading::Nothing
        };
    }

    /// Any other line ends a list.
    pub fn on_other_line(&mut self) {
        self.reading = Reading::Nothing;
    }

    /// A CHANGE_ENTITY to `card`: an offered hero rerolled before the pick.
    pub fn on_change(&mut self, id: i64, card: &str) {
        if !self.ids.contains(&id) || self.closed {
            return;
        }
        let Some(record) = self.record.as_mut() else {
            return;
        };
        if record.offered.len() < MAX_OFFERED && valid_card(card) {
            record.offered.push(card.to_string());
            record.rerolls += 1;
        }
    }

    pub fn record(&self) -> Option<HeroSelect> {
        self.record.clone()
    }

    fn offer(&mut self, entity: EntityRef, board: &Board, local_pid: Option<i64>) {
        let EntityRef::Id(id) = entity else { return };
        let Some(hero) = board.get(id) else { return };
        let own = local_pid.is_some_and(|pid| hero.int("CONTROLLER") == pid);
        let card = hero.card_id.as_deref().filter(|c| valid_card(c));
        let (Some(card), true, Some(record)) = (card, own, self.record.as_mut()) else {
            return;
        };
        if hero.is("CARDTYPE", "HERO")
            && !self.ids.contains(&id)
            && record.offered.len() < MAX_OFFERED
        {
            self.ids.push(id);
            record.offered.push(card.to_string());
        }
    }

    fn pick(&mut self, entity: EntityRef, board: &Board) {
        let EntityRef::Id(id) = entity else { return };
        if !self.ids.contains(&id) {
            return;
        }
        let card = board
            .get(id)
            .and_then(|h| h.card_id.clone())
            .filter(|c| valid_card(c));
        if let Some(record) = self.record.as_mut() {
            // A pick is one of the heroes shown (one past the bound is not).
            record.picked = card.filter(|c| record.offered.contains(c));
        }
    }
}

/// "Entities[3]=..." -> the entity; None for any other line.
fn list_entity(body: &str) -> Option<EntityRef> {
    let rest = body.strip_prefix("Entities[")?;
    let (index, entity) = rest.split_once("]=")?;
    index
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| parse_entity(entity.trim()))
}

/// The "id=N" field of a choice header.
fn choice_id(body: &str) -> Option<i64> {
    body.split(' ')
        .find_map(|f| f.strip_prefix("id="))
        .and_then(|v| v.parse().ok())
}

fn valid_card(card: &str) -> bool {
    !card.is_empty()
        && card.len() <= 64
        && card.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::Value;
    use crate::state::{Entity, Tags};

    fn hero(id: i64, card: &str, controller: i64) -> Entity {
        let mut tags = Tags::new();
        tags.insert("CARDTYPE".into(), Value::Name("HERO".into()));
        tags.insert("CONTROLLER".into(), Value::Int(controller));
        Entity {
            id,
            card_id: Some(card.into()),
            tags,
        }
    }

    fn board() -> Board {
        let mut b = Board::default();
        b.create(hero(101, "H_A", 5));
        b.create(hero(102, "H_B_SKIN_C", 5));
        b.create(hero(103, "H_OTHER", 9));
        b
    }

    fn offer_all(t: &mut HeroSelectTracker, b: &Board) {
        t.on_choices(
            "id=1 Player=Player1 TaskList=7 ChoiceType=MULLIGAN CountMin=1 CountMax=1",
            b,
            Some(5),
        );
        t.on_choices("Source=GameEntity", b, Some(5));
        t.on_choices("Entities[0]=101", b, Some(5));
        t.on_choices(
            "Entities[1]=[entityName=B id=102 zone=HAND zonePos=2 cardId=H_B_SKIN_C player=5]",
            b,
            Some(5),
        );
        t.on_choices("Entities[2]=103", b, Some(5));
    }

    #[test]
    fn offered_heroes_rerolls_and_the_pick_are_read() {
        let mut b = board();
        let mut t = HeroSelectTracker::default();
        offer_all(&mut t, &b);
        t.on_other_line();
        t.on_change(101, "H_NEW");
        b.change(101, "H_NEW".into(), Tags::new()).unwrap();
        t.on_chosen("id=1 Player=Player1 EntitiesCount=1", &b);
        t.on_chosen("Entities[0]=101", &b);
        let r = t.record().expect("a hero choice");
        // Another player's hero is never an offer.
        assert_eq!(r.offered, ["H_A", "H_B_SKIN_C", "H_NEW"]);
        assert_eq!(r.rerolls, 1);
        assert_eq!(r.picked.as_deref(), Some("H_NEW"));
    }

    #[test]
    fn a_game_without_a_hero_choice_has_no_record() {
        let mut t = HeroSelectTracker::default();
        t.on_choices("id=2 ChoiceType=GENERAL", &board(), Some(5));
        t.on_choices("Entities[0]=101", &board(), Some(5));
        assert_eq!(t.record(), None);
    }

    #[test]
    fn later_choices_and_changes_do_not_touch_the_hero_pick() {
        let b = board();
        let mut t = HeroSelectTracker::default();
        offer_all(&mut t, &b);
        t.on_chosen("id=1 EntitiesCount=1", &b);
        t.on_chosen("Entities[0]=102", &b);
        // A discover later, another choice list, a hero that changes later.
        t.on_choices("id=2 ChoiceType=GENERAL", &b, Some(5));
        t.on_choices("Entities[0]=101", &b, Some(5));
        t.on_chosen("id=2 EntitiesCount=1", &b);
        t.on_chosen("Entities[0]=101", &b);
        t.on_change(102, "H_LATER");
        let r = t.record().unwrap();
        assert_eq!(r.offered, ["H_A", "H_B_SKIN_C"]);
        assert_eq!((r.rerolls, r.picked.as_deref()), (0, Some("H_B_SKIN_C")));
    }

    #[test]
    fn a_pick_that_cannot_be_read_still_closes_the_choice() {
        let b = board();
        let mut t = HeroSelectTracker::default();
        offer_all(&mut t, &b);
        t.on_chosen("id=1 EntitiesCount=1", &b);
        t.on_chosen("Entities[0]=999", &b); // not an offered hero
        t.on_change(101, "H_LATER");
        let r = t.record().unwrap();
        assert_eq!((r.rerolls, r.picked), (0, None));
        assert_eq!(r.offered.len(), 2);
    }

    #[test]
    fn a_pick_is_always_one_of_the_heroes_shown() {
        // The picked entity's card changed without a CHANGE_ENTITY.
        let mut b = board();
        let mut t = HeroSelectTracker::default();
        offer_all(&mut t, &b);
        b.show(101, "H_SHOWN".into(), Tags::new()).unwrap();
        t.on_chosen("id=1 EntitiesCount=1", &b);
        t.on_chosen("Entities[0]=101", &b);
        assert_eq!(t.record().unwrap().picked, None);
    }

    #[test]
    fn a_pick_the_log_never_gives_is_unknown() {
        let b = board();
        let mut t = HeroSelectTracker::default();
        offer_all(&mut t, &b);
        assert_eq!(t.record().unwrap().picked, None);
    }

    #[test]
    fn offers_are_bounded() {
        let mut b = Board::default();
        let mut t = HeroSelectTracker::default();
        t.on_choices("id=1 ChoiceType=MULLIGAN", &b, Some(5));
        for id in 1..=MAX_OFFERED as i64 + 10 {
            b.create(hero(id, "H_X", 5));
            t.on_choices(&format!("Entities[0]={id}"), &b, Some(5));
        }
        for _ in 0..10 {
            t.on_change(1, "H_Y");
        }
        assert_eq!(t.record().unwrap().offered.len(), MAX_OFFERED);
    }
}
