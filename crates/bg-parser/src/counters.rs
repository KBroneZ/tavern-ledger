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

/// The player's tags that carry counters. The game zeroes every tag on the
/// player entity for the whole combat and restores them at its end.
const VALUE_TAGS: [&str; 4] = [
    "BACON_BLOODGEMBUFFATKVALUE",
    "BACON_BLOODGEMBUFFHEALTHVALUE",
    "TAVERN_SPELL_ATTACK_INCREASE",
    "TAVERN_SPELL_HEALTH_INCREASE",
];
/// The tags of an enchantment that say it is in play with its numbers.
const ENCHANT_TAGS: [&str; 3] = ["ZONE", "TAG_SCRIPT_DATA_NUM_1", "TAG_SCRIPT_DATA_NUM_2"];

/// The counters' last good reading (T-611). On the user's logs the game
/// zeroes the local player's tags during each combat, and the Undead, Eternal
/// Knights and Beetle enchantments are only shown (in play) as copies for the
/// fight, so reading the board at an instant makes the counters blink. This
/// keeps each one's last good reading:
///
/// - the player's tags are taken only in shop turns (and when one starts, for
///   what the restore set during the combat), never from the wiped player;
/// - an enchantment is taken when the local player's one is in play with a
///   number above zero (a copy still being filled in, or hidden again, never
///   lowers a total); in a shop turn it is therefore the last combat's.
///
/// Only the local player's: an enchantment attached to anything else is
/// never read.
#[derive(Clone, Debug, Default)]
pub struct CounterMemory {
    inputs: CounterInputs,
}

impl CounterMemory {
    pub fn inputs(&self) -> &CounterInputs {
        &self.inputs
    }

    /// A tag was set on entity `id`; `shop_turn` is whether the game is in a
    /// shop turn.
    pub fn on_tag(
        &mut self,
        board: &Board,
        local: Option<i64>,
        id: i64,
        tag: &str,
        shop_turn: bool,
    ) {
        if Some(id) == local {
            if shop_turn && VALUE_TAGS.contains(&tag) {
                self.read_player(board, id);
            }
        } else if ENCHANT_TAGS.contains(&tag) {
            self.read_enchantment(board, local, id);
        }
    }

    /// An entity was created or shown.
    pub fn on_entity(&mut self, board: &Board, local: Option<i64>, id: i64) {
        self.read_enchantment(board, local, id);
    }

    /// A shop turn starts: the tags are back after the combat.
    pub fn on_shop_turn(&mut self, board: &Board, local: Option<i64>) {
        if let Some(player) = local {
            self.read_player(board, player);
        }
    }

    fn read_player(&mut self, board: &Board, player: i64) {
        let Some(p) = board.get(player) else { return };
        self.inputs.values.blood_gem = (
            p.int("BACON_BLOODGEMBUFFATKVALUE"),
            p.int("BACON_BLOODGEMBUFFHEALTHVALUE"),
        );
        self.inputs.values.spell = (
            p.int("TAVERN_SPELL_ATTACK_INCREASE"),
            p.int("TAVERN_SPELL_HEALTH_INCREASE"),
        );
    }

    fn read_enchantment(&mut self, board: &Board, local: Option<i64>, id: i64) {
        let Some(local) = local else { return };
        let Some(e) = board.get(id) else { return };
        if e.int("ATTACHED") != local || !e.is("ZONE", "PLAY") {
            return;
        }
        let first = e.int("TAG_SCRIPT_DATA_NUM_1");
        let second = e.int("TAG_SCRIPT_DATA_NUM_2");
        // Each number on its own: a copy gets its two numbers on separate
        // lines, and one still at zero must not lower the other's total.
        let keep = |old: i64, new: i64| if new != 0 { new } else { old };
        let values = &mut self.inputs.values;
        match e.card_id.as_deref() {
            Some("BG25_011pe") => values.undead_attack = keep(values.undead_attack, first),
            Some("BG25_008pe") => values.knights_dead = keep(values.knights_dead, first),
            Some("BG31_808pe") => {
                values.beetle = (keep(values.beetle.0, first), keep(values.beetle.1, second));
            }
            Some("BG_ShopBuff_Elemental") => {
                let old = self.inputs.shop_elemental;
                self.inputs.shop_elemental = (keep(old.0, first), keep(old.1, second));
            }
            _ => {}
        }
    }
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
