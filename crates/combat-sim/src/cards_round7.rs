//! Card rules of the accuracy rounds (T-609, session 106, engine `0.7.0`):
//! cards whose combat effect the replays of the user's logs showed was
//! missing, some of which need the side's game-long values
//! ([`crate::model::GameValues`]). From the card texts in HearthstoneJSON
//! (D-038), checked against the user's logs. Same rules as
//! [`crate::cards`].

use crate::cards::{exact, Action, CardRules, Trigger};

pub const EFFECTS_7: &[(&str, CardRules)] = &[
    // Golden Persistent Poet: "Adjacent Dragons permanently keep Bonus
    // Keywords and double stats gained in combat." The user's log (game 1
    // round 12) shows the doubling in combat: of a trinket's "Health equal
    // to half its Attack", the Dragon next to the Poet got twice the
    // others' share. The normal Poet only keeps them after combat.
    ("BG29_813_G", exact(&[Trigger::DoublesAdjacentDragonGains])),
    // Sanguine Refiner: Rally, your Blood Gems give an extra +1/+2 this game
    // (golden +2/+4), so the gems played later in this combat give more.
    (
        "BG33_885",
        exact(&[Trigger::Rally(&[Action::UpgradeGems {
            attack: 1,
            health: 2,
        }])]),
    ),
    (
        "BG33_885_G",
        exact(&[Trigger::Rally(&[Action::UpgradeGems {
            attack: 2,
            health: 4,
        }])]),
    ),
];
