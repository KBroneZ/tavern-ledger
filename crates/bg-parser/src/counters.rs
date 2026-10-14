//! The local player's game-long counters for the overlay (T-611, D-139):
//! running totals of things that happened to the player on their own screen.
//!
//! They are read from the player's own game-long values, the ones
//! [`Collector::combat_values`](crate::collector::Collector) already reads
//! for the simulator (T-609): nothing else is read and what the simulator
//! gets does not change. Only the local player's: an opponent's or a Duos
//! teammate's values are never looked at. Live only: not in the saved report.
//!
//! A kind is emitted only while it is in [`ENABLED_KINDS`]; one joins the list
//! after its log source is confirmed on the user's logs (D-139).

use serde::Serialize;

use crate::report::{CombatValues, MAX_GAME_VALUE};
use crate::state::Board;

/// One kind of counter, in the order the overlay lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterKind {
    BloodGem,
    TavernSpell,
    UndeadAttack,
    EternalKnights,
    Beetle,
    /// The Attack and Health the player's tavern Elementals get
    /// (`BG_ShopBuff_Elemental` on the player).
    ShopElemental,
}

impl CounterKind {
    /// Every kind this parser knows, in listing order.
    pub const ALL: [CounterKind; 6] = [
        CounterKind::BloodGem,
        CounterKind::TavernSpell,
        CounterKind::UndeadAttack,
        CounterKind::EternalKnights,
        CounterKind::Beetle,
        CounterKind::ShopElemental,
    ];

    /// The name the overlay's data gives it.
    pub fn name(self) -> &'static str {
        match self {
            CounterKind::BloodGem => "blood_gem",
            CounterKind::TavernSpell => "tavern_spell",
            CounterKind::UndeadAttack => "undead_attack",
            CounterKind::EternalKnights => "eternal_knights",
            CounterKind::Beetle => "beetle",
            CounterKind::ShopElemental => "shop_elemental",
        }
    }
}

/// The kinds the parser emits. A kind outside this list never appears, even
/// when the log has a value for it. The tavern's other buffs, the free rolls
/// and the gold for next turn are mapped but not here: their log source is
/// not a total that can be read for sure (see the research notes).
pub const ENABLED_KINDS: &[CounterKind] = &CounterKind::ALL;

/// One counter: its kind and only that kind's fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Counter {
    pub kind: CounterKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atk: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

impl Counter {
    pub fn of_stats(kind: CounterKind, atk: i64, health: i64) -> Self {
        Counter {
            kind,
            atk: Some(atk),
            health: Some(health),
            count: None,
        }
    }

    pub fn of_attack(kind: CounterKind, atk: i64) -> Self {
        Counter {
            kind,
            atk: Some(atk),
            health: None,
            count: None,
        }
    }

    pub fn of_count(kind: CounterKind, count: i64) -> Self {
        Counter {
            kind,
            atk: None,
            health: None,
            count: Some(count),
        }
    }

    /// Whether every field is a whole number the overlay may show: 0 to
    /// [`MAX_GAME_VALUE`].
    pub fn in_range(&self) -> bool {
        [self.atk, self.health, self.count]
            .into_iter()
            .flatten()
            .all(|v| (0..=MAX_GAME_VALUE).contains(&v))
    }
}

/// What the counters are built from: the player's game-long values (the
/// ones the simulator reads) and the tavern Elementals' buff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CounterInputs {
    pub values: CombatValues,
    /// `BG_ShopBuff_Elemental`'s two numbers on the player (Attack, Health).
    pub shop_elemental: (i64, i64),
}

/// The player enchantment `card` in play on `player`, with its two numbers
/// (`TAG_SCRIPT_DATA_NUM_1`, `_2`): the same rule as the simulator's values
/// (`Collector::combat_values`), `(0, 0)` when there is none.
pub fn player_enchantment(board: &Board, player: i64, card: &str) -> (i64, i64) {
    board
        .iter()
        .find(|e| {
            e.card_id.as_deref() == Some(card)
                && e.int("ATTACHED") == player
                && e.is("ZONE", "PLAY")
        })
        .map_or((0, 0), |e| {
            (
                e.int("TAG_SCRIPT_DATA_NUM_1"),
                e.int("TAG_SCRIPT_DATA_NUM_2"),
            )
        })
}

/// The counters of one player's values, only the `enabled` kinds, in the
/// kinds' order. A kind is listed when one of its fields is not zero (the
/// range is checked by the caller, which says so and drops it, never clamps);
/// a +0/+5 buff is listed.
pub fn counters_from(inputs: &CounterInputs, enabled: &[CounterKind]) -> Vec<Counter> {
    CounterKind::ALL
        .into_iter()
        .filter(|kind| enabled.contains(kind))
        .filter_map(|kind| counter_of(kind, inputs))
        .collect()
}

fn counter_of(kind: CounterKind, inputs: &CounterInputs) -> Option<Counter> {
    let v = &inputs.values;
    match kind {
        CounterKind::BloodGem => {
            let (atk, health) = v.blood_gem;
            (atk != 0 || health != 0)
                .then(|| Counter::of_stats(kind, gem_total(atk), gem_total(health)))
        }
        CounterKind::TavernSpell => stats_if_any(kind, v.spell),
        CounterKind::UndeadAttack => {
            (v.undead_attack != 0).then(|| Counter::of_attack(kind, v.undead_attack))
        }
        CounterKind::EternalKnights => {
            (v.knights_dead != 0).then(|| Counter::of_count(kind, v.knights_dead))
        }
        CounterKind::Beetle => stats_if_any(kind, v.beetle),
        CounterKind::ShopElemental => stats_if_any(kind, inputs.shop_elemental),
    }
}

fn stats_if_any(kind: CounterKind, (atk, health): (i64, i64)) -> Option<Counter> {
    (atk != 0 || health != 0).then(|| Counter::of_stats(kind, atk, health))
}

/// The Blood Gem's size: its base +1 plus what the player's extra adds. A
/// negative extra, which the game does not give, stays negative so the range
/// check refuses it instead of reading as a total. An extra of 99,999 gives a
/// total of 100,000, which the range check refuses too: the total is what
/// the overlay shows.
fn gem_total(extra: i64) -> i64 {
    if extra < 0 {
        extra
    } else {
        extra.saturating_add(1)
    }
}
