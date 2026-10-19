//! What one game looks like after parsing. Serializes to the same JSON as
//! `tools/parse_bg.py --json`, so both can be compared field by field.
//!
//! Privacy: only card ids and lobby player ids (1-8), never names or accounts.

use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::buffs::Buff;
use crate::discover::Discover;
use crate::hero_select::HeroSelect;
pub use crate::kit::{HeroKit, KitCard};
use crate::leaderboard::{TrinketSeen, TriplesSeen};
use crate::replay::CombatReplay;
use crate::shop::ShopRecord;

pub const NOT_IN_POWER_LOG: [&str; 2] = ["MMR", "available tribes (exact list)"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Incomplete,
    Unsupported,
    NotBattlegrounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Own,
    Opponent,
}

/// A combat's result for the local player (their team in Duos), as the game
/// records it (T-206, D-055).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatResult {
    Won,
    Lost,
    Tie,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Minion {
    pub card_id: Option<String>,
    pub atk: i64,
    pub health: i64,
    pub position: i64,
    pub golden: bool,
    /// Keywords the minion had as it entered the combat (T-208, parser
    /// revision 5), by name: see [`KEYWORDS`]. Rust-only.
    pub keywords: Vec<&'static str>,
    /// Lasting keywords the combat simulator reads that the saved report
    /// does not carry (T-604): see [`SIM_KEYWORDS`]. Never serialized, so
    /// neither the history nor the upload changes.
    #[serde(skip)]
    pub sim_keywords: Vec<&'static str>,
    /// The two numbers its text shows (`TAG_SCRIPT_DATA_NUM_1` and `_2`),
    /// for a card whose start-of-combat buff grows during the game
    /// (Hopebringer, Ultraviolet Ascendant; T-628); 0 where the game gives
    /// none. Never serialized.
    #[serde(skip)]
    pub counters: [i64; 2],
    /// The marks a hero power left on it that the combat simulator reads
    /// (T-631): see [`crate::sim_marks::SIM_MARKS`]. Only on the boards
    /// taken at the first attack ([`crate::collector::board_of`]); never
    /// serialized.
    #[serde(skip)]
    pub sim_marks: Vec<&'static str>,
    /// Dark Gifts on the minion as gift spell ids (`BG36_MidGameEffect_000tNN`,
    /// T-528, parser revision 9), at most [`crate::gifts::MAX_GIFTS`]. Left
    /// out of the JSON when empty: a revision 9 record without the key has
    /// none, an older one did not read them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gifts: Vec<&'static str>,
    /// The game marks the minion as gifted but no gift enchantment says
    /// which ("Dark Gift (unknown)"). Left out of the JSON when false.
    #[serde(skip_serializing_if = "is_false")]
    pub gift_unknown: bool,
    /// Buffs on the minion, by the card that gave them and how many (T-619,
    /// parser revision 14), at most [`crate::buffs::MAX_BUFF_KINDS`]. Left
    /// out of the JSON when empty: a revision 14 record without the key has
    /// none, an older one did not read them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub buffs: Vec<Buff>,
}

pub(crate) fn is_false(v: &bool) -> bool {
    !*v
}

/// Keyword tags read on board minions and the name each is saved under, in
/// this order (T-208). Golden is `Minion::golden`.
pub const KEYWORDS: [(&str, &str); 8] = [
    ("TAUNT", "taunt"),
    ("DIVINE_SHIELD", "divine_shield"),
    ("REBORN", "reborn"),
    ("VENOMOUS", "venomous"),
    ("WINDFURY", "windfury"),
    ("STEALTH", "stealth"),
    ("DEATHRATTLE", "deathrattle"),
    ("BACON_RALLY", "rally"),
];

/// Keyword tags read on board minions for the combat simulator only (T-604),
/// kept out of the saved report: upload-game checks every saved keyword
/// against [`KEYWORDS`]'s names.
pub const SIM_KEYWORDS: [(&str, &str); 2] = [
    ("POISONOUS", "poisonous"),
    ("MEGA_WINDFURY", "mega_windfury"),
];

/// A minion's place in one attack (T-603b): its side, its board position
/// (1 is the left-most) and its card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttackSlot {
    pub side: Side,
    pub position: i64,
    pub card_id: Option<String>,
}

/// How the defender of an attack was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefenderSource {
    /// The game entity's `PROPOSED_DEFENDER`.
    Game,
    /// The first enemy minion the attack damaged or stripped of its Divine
    /// Shield (a log trimmed of `PROPOSED_DEFENDER`).
    FirstHit,
}

/// One attack as the log plays it (T-603b), with both boards just before
/// it. Rust-only: not in the saved report nor the upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedAttack {
    pub attacker: AttackSlot,
    pub defender: Option<(AttackSlot, DefenderSource)>,
    pub own: Vec<Minion>,
    pub opponent: Vec<Minion>,
}

/// The largest game-long value the saved report keeps (parser revision 10);
/// upload-game refuses one past it.
pub const MAX_GAME_VALUE: i64 = 99_999;

/// Game-long values of one combat side that change what its cards do in
/// combat (T-609), read from the side's player entity when its board is
/// taken. Saved with the board since parser revision 10 (T-610): only the
/// values from 1 to [`MAX_GAME_VALUE`], by the names in [`Self::saved`];
/// zero, or a number the game would not show, is left out, so it reads as
/// unknown (the card's printed numbers), as in an older record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatValues {
    /// What the side's Blood Gems give beyond +1/+1
    /// (`BACON_BLOODGEMBUFFATKVALUE`, `BACON_BLOODGEMBUFFHEALTHVALUE`).
    pub blood_gem: (i64, i64),
    /// What the side's Tavern spells give beyond their text
    /// (`TAVERN_SPELL_ATTACK_INCREASE`, `TAVERN_SPELL_HEALTH_INCREASE`).
    pub spell: (i64, i64),
    /// Eternal Knights that died this game: the player enchantment
    /// `BG25_008pe` ("Counts the number of Eternal Knights that died").
    pub knights_dead: i64,
    /// What the side's Undead have on top this game: `BG25_011pe` ("Give
    /// Attack to Undead").
    pub undead_attack: i64,
    /// What the side's Beetles have on top this game: `BG31_808pe` ("Gives
    /// Beetle stats"), Attack and Health.
    pub beetle: (i64, i64),
}

impl CombatValues {
    /// Each value by the name the saved report gives it.
    pub fn saved(&self) -> [(&'static str, i64); 8] {
        [
            ("blood_gem_atk", self.blood_gem.0),
            ("blood_gem_health", self.blood_gem.1),
            ("spell_atk", self.spell.0),
            ("spell_health", self.spell.1),
            ("knights_dead", self.knights_dead),
            ("undead_atk", self.undead_attack),
            ("beetle_atk", self.beetle.0),
            ("beetle_health", self.beetle.1),
        ]
    }

    fn kept(&self) -> impl Iterator<Item = (&'static str, i64)> {
        self.saved()
            .into_iter()
            .filter(|(_, v)| (1..=MAX_GAME_VALUE).contains(v))
    }

    /// True when no value would be saved.
    pub fn is_empty(&self) -> bool {
        self.kept().next().is_none()
    }

    /// The values as the saved report keeps them: one outside 1 to
    /// [`MAX_GAME_VALUE`] is zero (unknown), so the overlay plays what the
    /// saved game will.
    pub fn bounded(&self) -> CombatValues {
        let b = |v: i64| {
            if (1..=MAX_GAME_VALUE).contains(&v) {
                v
            } else {
                0
            }
        };
        CombatValues {
            blood_gem: (b(self.blood_gem.0), b(self.blood_gem.1)),
            spell: (b(self.spell.0), b(self.spell.1)),
            knights_dead: b(self.knights_dead),
            undead_attack: b(self.undead_attack),
            beetle: (b(self.beetle.0), b(self.beetle.1)),
        }
    }
}

impl Serialize for CombatValues {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        for (name, value) in self.kept() {
            map.serialize_entry(name, &value)?;
        }
        map.end()
    }
}

fn no_values(v: &Option<CombatValues>) -> bool {
    v.as_ref().is_none_or(CombatValues::is_empty)
}

/// Whether the opponent's hero powers and trinkets are saved with its boards
/// (parser revision 16; the user's choice, D-164). The player's own always
/// are. The opponent's are saved without a held minion.
pub const SAVE_OPPONENT_KIT: bool = true;

/// One side stepping into a combat. `board` is None when the fight never
/// plays out in the local log: unknown, not empty. Serialized by hand
/// (below): a field added here is saved only once written there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatEntry {
    pub side: Side,
    pub player_id: Option<i64>,
    pub hero: Option<String>,
    pub board: Option<Vec<Minion>>,
    /// The side's game-long values as its board was taken (T-609); None
    /// without a board. Saved since parser revision 10 (T-610) when one is
    /// above zero: a record without the key read none, or is older.
    pub values: Option<CombatValues>,
    /// The board as the leg started, before any start-of-combat effect
    /// (T-628): read just before the leg's first effect or attack. None
    /// without a board. Rust-only and not serialized (the replay's start is
    /// that board).
    pub before: Option<Vec<Minion>>,
    /// The side's hero powers and trinkets then (T-628). Saved since parser
    /// revision 16 as `kit` with a board, for both sides (the opponent's
    /// while [`SAVE_OPPONENT_KIT`], without a held minion); a record without
    /// the key did not read them.
    pub kit: Option<HeroKit>,
}

impl Serialize for CombatEntry {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(None)?;
        map.serialize_entry("side", &self.side)?;
        map.serialize_entry("player_id", &self.player_id)?;
        map.serialize_entry("hero", &self.hero)?;
        map.serialize_entry("board", &self.board)?;
        if !no_values(&self.values) {
            map.serialize_entry("values", &self.values)?;
        }
        let own = self.side == Side::Own;
        if let Some(kit) = self
            .kit
            .as_ref()
            .filter(|_| (own || SAVE_OPPONENT_KIT) && self.board.is_some())
        {
            if own {
                map.serialize_entry("kit", kit)?;
            } else {
                // Never a minion the opponent holds (D-164).
                let mut kit = kit.clone();
                kit.powers.iter_mut().for_each(|p| p.held = None);
                map.serialize_entry("kit", &kit)?;
            }
        }
        map.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    pub number: i64,
    pub entries: Vec<CombatEntry>,
    pub own_health_after: Option<i64>,
    /// Health (health + armor - damage, never below 0) of every lobby hero by
    /// player id once the combat closed. Empty when the log did not give it
    /// (unknown, not zero). Rust-only: the Python prototype does not write it.
    pub health_after: BTreeMap<i64, i64>,
    /// The result the game records on the local player once the combat is
    /// over (parser revision 4). None when the log does not have the whole
    /// combat or its tags disagree: unknown, never a tie. Rust-only.
    pub result: Option<CombatResult>,
    /// Every attack of the combat in the log's order (T-603b). Rust-only and
    /// not serialized: the harness reads it straight from the logs.
    pub attacks: Vec<LoggedAttack>,
    /// The player's own board as the shop turn before this combat ended
    /// (T-608): what the overlay's odds start from. None when the log does
    /// not have that shop turn. Rust-only and not serialized.
    pub own_shop_board: Option<Vec<Minion>>,
    /// The combat event by event as the game played it back (T-215, parser
    /// revision 8). None when the game did not play it back on screen ("not
    /// shown"); a record from an older parser has no `replay` key at all
    /// ("not recorded").
    pub replay: Option<CombatReplay>,
    /// Effects of the combat from a card the screen does not show: one in a
    /// hand or a Secret (T-628), by side and card id, in the log's order.
    /// The simulator never plays them; the harness counts them. Rust-only
    /// and not serialized.
    pub hidden_triggers: Vec<(Side, String)>,
}

impl Round {
    pub fn opponents(&self) -> Vec<Option<i64>> {
        self.entries
            .iter()
            .filter(|e| e.side == Side::Opponent)
            .map(|e| e.player_id)
            .collect()
    }
}

impl Serialize for Round {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(7))?;
        map.serialize_entry("number", &self.number)?;
        map.serialize_entry("entries", &self.entries)?;
        map.serialize_entry("own_health_after", &self.own_health_after)?;
        map.serialize_entry("opponents", &self.opponents())?;
        map.serialize_entry("health_after", &self.health_after)?;
        map.serialize_entry("result", &self.result)?;
        map.serialize_entry("replay", &self.replay)?;
        map.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LobbyPlayer {
    pub player_id: i64,
    pub hero: Option<String>,
    pub duo_team: Option<i64>,
    pub final_place: Option<i64>,
    pub final_health: i64,
    /// The hero's triple count each time the leaderboard changed it (T-207,
    /// parser revision 5). Rust-only.
    pub triples: Vec<TriplesSeen>,
    /// The hero's trinkets as the leaderboard shows them, each time one is
    /// set (T-207, parser revision 5). Rust-only.
    pub trinkets: Vec<TrinketSeen>,
}

/// One lobby hero as the game's leaderboard shows it right now: its place
/// (top is 1; in Duos both heroes of a team share their team's place) and its
/// tavern tier. Live only (T-306): never part of the saved report, so the
/// report, the upload and the Python prototype stay as they are. `None` is
/// unknown, never 0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LobbySlot {
    pub player_id: i64,
    pub place: Option<i64>,
    pub tech_level: Option<i64>,
    /// The health the game shows on the hero (its health minus the damage
    /// taken, never below 0), armor not included.
    pub health: Option<i64>,
    /// The armor the game shows on the hero.
    pub armor: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameReport {
    pub index: usize,
    pub status: Status,
    pub game_type: Option<String>,
    pub build: Option<i64>,
    pub local_player_id: Option<i64>,
    pub hero: Option<String>,
    pub teammate_player_id: Option<i64>,
    pub teammate_hero: Option<String>,
    /// Card id -> name, for the heroes in this report whose name the log
    /// prints (in the game client's language). Not in the Python prototype.
    pub card_names: BTreeMap<String, String>,
    pub final_place: Option<i64>,
    pub final_health: Option<i64>,
    pub lobby: Vec<LobbyPlayer>,
    /// Tribe name and how many shop offers had it, most common first.
    #[serde(serialize_with = "pairs_as_map")]
    pub shop_tribes: Vec<(String, usize)>,
    pub rounds: Vec<Round>,
    /// Health of every lobby hero by player id before the first combat. Empty
    /// when the log did not give it. Rust-only, like `Round::health_after`.
    pub start_health: BTreeMap<i64, i64>,
    /// The player's own shop and logged actions (T-204, T-205, parser
    /// revision 3). None when the game could not be read. Rust-only.
    pub shop: Option<ShopRecord>,
    /// The player's own hero pick (T-209, parser revision 4). None when the
    /// log has no hero choice for this game. Rust-only.
    pub hero_select: Option<HeroSelect>,
    /// Card id -> database id of the base hero, for the skins in this report
    /// whose link the log gives (`BACON_SKIN_PARENT_ID`, T-214, parser
    /// revision 4). A hero missing here has no link in the log. Rust-only.
    pub skin_parents: BTreeMap<String, i64>,
    /// Every choice the player was offered and what they took: discovers,
    /// trinket offers, Dark Gifts, quests (T-211, T-212, parser revision 5).
    /// Rust-only.
    pub discovers: Vec<Discover>,
    /// Database id of the game's Old God (`BACON_GLOBAL_OLD_GOD_DBID`, T-211,
    /// parser revision 5); None when the log does not give one. Rust-only.
    pub old_god: Option<i64>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
    pub not_in_log: Vec<String>,
    /// The combat legs whose start of combat is under way now: begun, not at
    /// their first attack yet (T-628, D-165), for the overlay's early
    /// estimate. Empty in a shop turn and once a leg's first attack comes
    /// (its entry then has the board). Rust-only and not serialized.
    #[serde(skip)]
    pub opening: Vec<OpeningLeg>,
}

/// A combat leg as it began, before its first attack (T-628, D-165).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpeningLeg {
    /// The combat (round) number, as the round's entries have it.
    pub round: i64,
    pub side: Side,
    /// The seat, as the leg's entry will have it once it attacks.
    pub player_id: Option<i64>,
    pub hero: Option<String>,
    /// The board just before the leg's first start-of-combat effect.
    pub board: Vec<Minion>,
    /// The side's hero powers and trinkets then.
    pub kit: HeroKit,
    /// The side's game-long values then.
    pub values: CombatValues,
}

impl GameReport {
    pub fn new(
        index: usize,
        status: Status,
        game_type: Option<String>,
        build: Option<i64>,
    ) -> Self {
        GameReport {
            index,
            status,
            game_type,
            build,
            local_player_id: None,
            hero: None,
            teammate_player_id: None,
            teammate_hero: None,
            card_names: BTreeMap::new(),
            final_place: None,
            final_health: None,
            lobby: Vec::new(),
            shop_tribes: Vec::new(),
            rounds: Vec::new(),
            start_health: BTreeMap::new(),
            shop: None,
            hero_select: None,
            skin_parents: BTreeMap::new(),
            discovers: Vec::new(),
            old_god: None,
            warnings: Vec::new(),
            problems: Vec::new(),
            not_in_log: NOT_IN_POWER_LOG.iter().map(|s| s.to_string()).collect(),
            opening: Vec::new(),
        }
    }
}

fn pairs_as_map<S: Serializer>(pairs: &[(String, usize)], s: S) -> Result<S::Ok, S::Error> {
    let mut map = s.serialize_map(Some(pairs.len()))?;
    for (k, v) in pairs {
        map.serialize_entry(k, v)?;
    }
    map.end()
}

fn or_dash<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
}

fn opt<T: ToString>(value: &Option<T>) -> String {
    value
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "None".into())
}

fn fmt_board(board: &Option<Vec<Minion>>) -> String {
    match board {
        None => "(not visible in the log)".into(),
        Some(b) if b.is_empty() => "(empty)".into(),
        Some(b) => b
            .iter()
            .map(|m| {
                format!(
                    "{} {}/{}{}",
                    opt(&m.card_id),
                    m.atk,
                    m.health,
                    if m.golden { " golden" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn fmt_ids(ids: &[Option<i64>]) -> String {
    let items: Vec<String> = ids.iter().map(opt).collect();
    format!("[{}]", items.join(", "))
}

/// Human-readable summary, same wording as the Python prototype.
pub fn format_text(r: &GameReport) -> String {
    let status = serde_json::to_value(r.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from));
    let mut lines = vec![format!(
        "Game {}: {}, build {} - {}",
        r.index,
        r.game_type.as_deref().unwrap_or("unknown type"),
        r.build
            .map(|b| b.to_string())
            .unwrap_or_else(|| "unknown".into()),
        status.unwrap_or_default()
    )];
    lines.extend(r.warnings.iter().map(|w| format!("  warning: {w}")));
    lines.extend(r.problems.iter().map(|p| format!("  problem: {p}")));
    if !matches!(r.status, Status::Ok | Status::Incomplete) {
        return lines.join("\n");
    }
    let mate = match r.teammate_player_id {
        Some(pid) => format!("; teammate {} (player {pid})", opt(&r.teammate_hero)),
        None => String::new(),
    };
    let place = r
        .final_place
        .map(|p| p.to_string())
        .unwrap_or_else(|| "not available".into());
    lines.push(format!(
        "  Hero: {} (player {}){mate}",
        opt(&r.hero),
        opt(&r.local_player_id)
    ));
    lines.push(format!(
        "  Result: place {place}, health {}",
        opt(&r.final_health)
    ));
    let lobby: Vec<String> = r
        .lobby
        .iter()
        .map(|p| {
            format!(
                "p{} {} team {} place {}",
                p.player_id,
                opt(&p.hero),
                or_dash(p.duo_team),
                or_dash(p.final_place)
            )
        })
        .collect();
    lines.push(format!("  Lobby: {}", lobby.join("; ")));
    let tribes: Vec<String> = r
        .shop_tribes
        .iter()
        .map(|(n, c)| format!("{n} {c}"))
        .collect();
    let tribes = if tribes.is_empty() {
        "none seen".into()
    } else {
        tribes.join(", ")
    };
    lines.push(format!(
        "  Tribes seen in shop (inferred, not the exact lobby list): {tribes}"
    ));
    lines.push("  MMR: not available in Power.log".into());
    for round in &r.rounds {
        lines.push(format!(
            "  Round {}: health after {}, opponents {}",
            round.number,
            opt(&round.own_health_after),
            fmt_ids(&round.opponents())
        ));
        for e in &round.entries {
            let side = if e.side == Side::Own {
                "own"
            } else {
                "opponent"
            };
            lines.push(format!(
                "    {side} p{} {}: {}",
                opt(&e.player_id),
                opt(&e.hero),
                fmt_board(&e.board)
            ));
        }
    }
    lines.join("\n")
}
