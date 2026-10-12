//! Card rules of the accuracy rounds (T-609, session 106, engine `0.7.0`):
//! cards whose combat effect the board alone could not play before the
//! side's game-long values ([`crate::model::GameValues`]) were known. From
//! the card texts in HearthstoneJSON (D-038), checked against the user's
//! logs. Same rules as [`crate::cards`].

use crate::cards::{exact, Action, CardRules, Trigger};

pub const EFFECTS_7: &[(&str, CardRules)] = &[
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
