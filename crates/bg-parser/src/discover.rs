//! The player's own discover picks: what they were offered and what they took
//! (T-212), which also gives the season choices the log holds (T-211):
//! trinket offers and picks, Dark Gifts, quests. Rust-only: the Python
//! prototype does not read them.
//!
//! What the log gives (checked on 11 real games, design notes):
//! - `DebugPrintEntityChoices` with `ChoiceType=GENERAL` lists one choice the
//!   player is offered: its `Source` (the card that discovers) and every
//!   offered card, one `Entities[i]=` line each. The client prints only its
//!   own choices (231 of 231);
//! - `DebugPrintEntitiesChosen` with the same choice id names what was taken
//!   (always one of the offered cards);
//! - an offer from the Dark Discovery button is a minion whose
//!   `DARK_GIFT_ENTITY` names the gift it comes with (105 of 105).
//!
//! What a choice was is left to its source card id: `BG30_Trinket_1st` and
//! `_2nd` are the trinket offers, `BG36_MidGameEffect_010` the Dark Gifts,
//! `TB_BaconShop_Triples_01` a triple reward, and so on.

use serde::{Deserialize, Serialize};

use crate::power::{parse_entity, EntityRef};
use crate::state::Board;

/// Bounds, so a strange log cannot grow a report without end. A game has
/// about 30 choices of 3 or 4 cards.
pub const MAX_DISCOVERS: usize = 200;
pub const MAX_OPTIONS: usize = 10;

const GENERAL: &str = "ChoiceType=GENERAL";
const GIFT: &str = "DARK_GIFT_ENTITY";

/// One card offered in a choice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverOption {
    /// None when the log hides the card.
    pub card_id: Option<String>,
    /// The Dark Gift the minion comes with, if any.
    pub gift: Option<String>,
}

/// One choice the player made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discover {
    /// The shop turn it was offered in.
    pub turn: i64,
    /// Card id of what offered the choice; None when the log does not say.
    pub source: Option<String>,
    pub offered: Vec<DiscoverOption>,
    /// Index in `offered` of each card taken; empty when the log does not
    /// say (the game ended first).
    pub picked: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Reading {
    #[default]
    Nothing,
    Offer,
    Pick,
}

#[derive(Clone, Debug, Default)]
pub struct DiscoverTracker {
    discovers: Vec<Discover>,
    /// Choice id and entity id of each option of every choice kept.
    ids: Vec<(i64, Vec<i64>)>,
    reading: Reading,
    /// Index in `discovers` of the choice being read.
    current: Option<usize>,
}

impl DiscoverTracker {
    /// A `DebugPrintEntityChoices` line.
    pub fn on_choices(&mut self, body: &str, board: &Board, local_pid: Option<i64>, turn: i64) {
        if let Some(raw) = body.strip_prefix("Source=") {
            if self.reading == Reading::Offer {
                let source = card_of(parse_entity(raw.trim()), board);
                if let Some(d) = self.current.and_then(|i| self.discovers.get_mut(i)) {
                    d.source = source;
                }
            }
            return;
        }
        if let Some(entity) = list_entity(body) {
            if self.reading == Reading::Offer {
                self.offer(entity, board, local_pid);
            }
            return;
        }
        self.reading = Reading::Nothing;
        self.current = None;
        let Some(id) = choice_id(body) else { return };
        // Past the shop record's bound on turns: not a turn the game has.
        let general = body.split(' ').any(|f| f == GENERAL) && turn <= crate::leaderboard::MAX_TURN;
        let seen = self.ids.iter().any(|(c, _)| *c == id);
        if general && !seen && self.discovers.len() < MAX_DISCOVERS {
            self.discovers.push(Discover {
                turn,
                source: None,
                offered: Vec::new(),
                picked: Vec::new(),
            });
            self.ids.push((id, Vec::new()));
            self.current = Some(self.discovers.len() - 1);
            self.reading = Reading::Offer;
        }
    }

    /// A `DebugPrintEntitiesChosen` line.
    pub fn on_chosen(&mut self, body: &str) {
        if let Some(entity) = list_entity(body) {
            if self.reading == Reading::Pick {
                self.pick(entity);
            }
            return;
        }
        self.reading = Reading::Nothing;
        self.current = choice_id(body).and_then(|id| {
            self.ids
                .iter()
                .position(|(c, _)| *c == id)
                .filter(|&i| self.discovers[i].picked.is_empty())
        });
        if self.current.is_some() {
            self.reading = Reading::Pick;
        }
    }

    /// Any other line ends a list.
    pub fn on_other_line(&mut self) {
        self.reading = Reading::Nothing;
    }

    /// Every choice the player was offered, without those whose cards are
    /// not the player's own.
    pub fn record(&self) -> Vec<Discover> {
        self.discovers
            .iter()
            .filter(|d| !d.offered.is_empty())
            .cloned()
            .collect()
    }

    fn offer(&mut self, entity: EntityRef, board: &Board, local_pid: Option<i64>) {
        let EntityRef::Id(id) = entity else { return };
        let Some(i) = self.current else { return };
        let Some(e) = board.get(id) else { return };
        // The player's own choice offers the player's own cards.
        let own = local_pid.is_some_and(|pid| e.int("CONTROLLER") == pid);
        let (ids, d) = (&mut self.ids[i].1, &mut self.discovers[i]);
        if !own || ids.contains(&id) || d.offered.len() >= MAX_OPTIONS {
            return;
        }
        let gift = e
            .opt_int(GIFT)
            .and_then(|g| board.get(g))
            .and_then(|g| g.card_id.clone())
            .filter(|c| valid_card(c));
        ids.push(id);
        d.offered.push(DiscoverOption {
            card_id: e.card_id.clone().filter(|c| valid_card(c)),
            gift,
        });
    }

    fn pick(&mut self, entity: EntityRef) {
        let EntityRef::Id(id) = entity else { return };
        let Some(i) = self.current else { return };
        let Some(index) = self.ids[i].1.iter().position(|&o| o == id) else {
            return;
        };
        let picked = &mut self.discovers[i].picked;
        if !picked.contains(&index) {
            picked.push(index);
        }
    }
}

fn card_of(entity: EntityRef, board: &Board) -> Option<String> {
    match entity {
        EntityRef::Id(id) => board
            .get(id)
            .and_then(|e| e.card_id.clone())
            .filter(|c| valid_card(c)),
        _ => None,
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

    fn card(id: i64, card: &str, controller: i64, gift: Option<i64>) -> Entity {
        let mut tags = Tags::new();
        tags.insert("CONTROLLER".into(), Value::Int(controller));
        if let Some(g) = gift {
            tags.insert(GIFT.into(), Value::Int(g));
        }
        Entity {
            id,
            card_id: Some(card.into()),
            tags,
        }
    }

    fn board() -> Board {
        let mut b = Board::default();
        b.create(card(10, "BG36_Button_DarkGift", 5, None)).unwrap();
        b.create(card(11, "BG36_MidGameEffect_010", 5, None))
            .unwrap();
        b.create(card(20, "M_A", 5, Some(30))).unwrap();
        b.create(card(21, "M_B", 5, Some(31))).unwrap();
        b.create(card(22, "M_C", 9, None)).unwrap(); // someone else's card
        b.create(card(30, "GIFT_A", 5, None)).unwrap();
        b.create(card(31, "GIFT_B", 5, None)).unwrap();
        b
    }

    fn offer(t: &mut DiscoverTracker, b: &Board, id: i64) {
        t.on_choices(
            &format!("id={id} Player=X TaskList=7 ChoiceType=GENERAL CountMin=1 CountMax=1"),
            b,
            Some(5),
            4,
        );
        t.on_choices(
            "Source=[entityName=X id=11 zone=PLAY zonePos=0 cardId=BG36_MidGameEffect_010 player=5]",
            b,
            Some(5),
            4,
        );
        t.on_choices("Entities[0]=20", b, Some(5), 4);
        t.on_choices(
            "Entities[1]=[entityName=X id=21 zone=SETASIDE zonePos=0 cardId=M_B player=5]",
            b,
            Some(5),
            4,
        );
        t.on_choices("Entities[2]=22", b, Some(5), 4);
        t.on_other_line();
    }

    #[test]
    fn offers_gifts_and_the_pick_are_read() {
        let b = board();
        let mut t = DiscoverTracker::default();
        offer(&mut t, &b, 3);
        t.on_chosen("id=3 Player=X EntitiesCount=1");
        t.on_chosen("Entities[0]=[entityName=X id=21 zone=SETASIDE zonePos=0 cardId=M_B player=5]");
        let r = t.record();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].turn, 4);
        assert_eq!(r[0].source.as_deref(), Some("BG36_MidGameEffect_010"));
        let cards: Vec<_> = r[0]
            .offered
            .iter()
            .map(|o| (o.card_id.as_deref(), o.gift.as_deref()))
            .collect();
        // Another player's card is never an offer.
        assert_eq!(
            cards,
            [(Some("M_A"), Some("GIFT_A")), (Some("M_B"), Some("GIFT_B"))]
        );
        assert_eq!(r[0].picked, [1]);
    }

    #[test]
    fn hero_choices_and_unknown_picks_are_left_out() {
        let b = board();
        let mut t = DiscoverTracker::default();
        t.on_choices(
            "id=1 Player=X ChoiceType=MULLIGAN CountMin=1 CountMax=1",
            &b,
            Some(5),
            0,
        );
        t.on_choices("Entities[0]=20", &b, Some(5), 0);
        offer(&mut t, &b, 2);
        t.on_chosen("id=2 Player=X EntitiesCount=1");
        t.on_chosen("Entities[0]=22"); // not an offer
        t.on_chosen("id=9 Player=X EntitiesCount=1");
        t.on_chosen("Entities[0]=20"); // another choice's pick
        let r = t.record();
        assert_eq!(r.len(), 1);
        assert!(r[0].picked.is_empty());
    }

    #[test]
    fn a_choice_with_no_own_card_is_dropped() {
        let b = board();
        let mut t = DiscoverTracker::default();
        t.on_choices("id=4 Player=X ChoiceType=GENERAL", &b, Some(5), 3);
        t.on_choices("Entities[0]=22", &b, Some(5), 3);
        assert!(t.record().is_empty());
    }

    #[test]
    fn choices_and_options_are_bounded() {
        let mut b = Board::default();
        for id in 1..=20 {
            b.create(card(id, "M_X", 5, None)).unwrap();
        }
        let mut t = DiscoverTracker::default();
        for n in 0..MAX_DISCOVERS as i64 + 5 {
            t.on_choices(
                &format!("id={} ChoiceType=GENERAL", n + 100),
                &b,
                Some(5),
                1,
            );
            for id in 1..=20 {
                t.on_choices(&format!("Entities[{id}]={id}"), &b, Some(5), 1);
            }
        }
        let r = t.record();
        assert_eq!(r.len(), MAX_DISCOVERS);
        assert!(r.iter().all(|d| d.offered.len() == MAX_OPTIONS));
    }
}
