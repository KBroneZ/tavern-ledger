//! The game the app saves, read for the engine (T-346, T-549): each
//! board as a combat began, its game-long values, the result and the saved
//! combat replay (`rounds[].replay`, parser revision 8, D-099) rebuilt into
//! the logged steps the replay checks. The app's recap, the website's
//! engine and the validation harness read saved games with this one module.
//! The record is untrusted input: anything unexpected reads as unknown,
//! never as a panic.
//!
//! The saved replay names minions by index and gives each one's board slot
//! when it entered; it follows damage, stat changes, Divine Shield, deaths
//! and summons, but not other keywords. So the boards before each attack
//! are rebuilt from the events, and only Divine Shield is compared among
//! the keywords. A summon whose slot the log did not have yet (slot 0: a
//! Reborn copy or a teammate's board coming in) goes where a minion of the
//! same card last died on that side, else at the right end. The rebuilt
//! boards at the first attack must equal the boards the report saved for
//! the combat, or the combat is not replayed ("unreadable").

use std::cell::RefCell;

use serde_json::Value;

use crate::combat::replay::LoggedStep;
use crate::combat::CombatResult;
use crate::model::{BaseCard, GameValues, Keywords, Kit, KitCard, Minion, Team, Tribes, Warband};
use crate::trials::Options;
use crate::why::{explain, CombatFacts, Explained, Replayable};

/// The largest game-long value a saved report keeps (bg-parser's
/// `MAX_GAME_VALUE`; the harness tests check they agree).
pub const MAX_GAME_VALUE: i64 = 99_999;

/// Card facts a board does not show, from the card data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardFacts {
    pub tier: Option<u8>,
    pub tribes: Tribes,
    pub base: Option<BaseCard>,
}

/// The card data: a card id's facts, `None` when it has none.
pub type FactsOf<'a> = dyn Fn(&str) -> Option<CardFacts> + 'a;
/// A saved minion in the engine's form.
pub type MinionOf<'a> = dyn Fn(&Value) -> Option<Minion> + 'a;

/// A saved minion in the engine's form, with its card's facts when the
/// card data has them.
pub fn minion(v: &Value, facts: &FactsOf) -> Option<Minion> {
    let card_id = v.get("card_id")?.as_str()?.to_string();
    let attack = i32::try_from(v.get("atk")?.as_i64()?).ok()?;
    let health = i32::try_from(v.get("health")?.as_i64()?).ok()?;
    let keywords = v
        .get("keywords")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter_map(Keywords::from_name)
                .fold(Keywords::NONE, Keywords::union)
        })
        .unwrap_or(Keywords::NONE);
    let facts = facts(&card_id);
    // The parser does not save the Poisonous and Mega-Windfury tags (T-208);
    // both are lasting, so the card's own count.
    let unread = facts.and_then(|f| f.base).map_or(Keywords::NONE, |b| {
        [Keywords::POISONOUS, Keywords::MEGA_WINDFURY]
            .into_iter()
            .filter(|k| b.keywords.contains(*k))
            .fold(Keywords::NONE, Keywords::union)
    });
    Some(Minion {
        golden: v.get("golden").and_then(Value::as_bool).unwrap_or(false),
        attack,
        health,
        keywords: keywords.union(unread),
        tribes: facts.map_or(Tribes::NONE, |f| f.tribes),
        tier: facts.and_then(|f| f.tier),
        base: facts.and_then(|f| f.base),
        // Dark Gifts (parser revision 9, T-528); older records have none.
        // Then the marks a hero power left, as the buffs the record keeps
        // (revision 14, T-631).
        gifts: v
            .get("gifts")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .into_iter()
            .chain(saved_marks(v).map(String::from))
            .collect(),
        // Saved on replay minions since parser revision 16 (T-628), not read
        // here yet: the boards read here are taken at the first attack.
        counters: [0, 0],
        card_id,
    })
}

/// The marks on a saved minion: the record keeps a mark as a buff from the
/// card that gave it ([`crate::gifts::MARK_BUFFS`]).
fn saved_marks(v: &Value) -> impl Iterator<Item = &'static str> + '_ {
    v.get("buffs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|b| b.get("card_id").and_then(Value::as_str))
        .filter_map(crate::gifts::mark_of_buff)
}

/// A combat entry's board with its game-long values; `None` without one.
pub fn board(entry: &Value, facts: &FactsOf) -> Option<Warband> {
    let minions: Option<Vec<Minion>> = entry
        .get("board")?
        .as_array()?
        .iter()
        .map(|m| minion(m, facts))
        .collect();
    Some(Warband {
        kit: None,
        minions: minions?,
        tavern_tier: None,
        values: saved_values(entry.get("values")),
    })
}

/// The most hero powers or trinkets a saved board keeps of each kind
/// (bg-parser's `MAX_KIT_CARDS`).
const MAX_KIT_CARDS: usize = 4;

/// A combat entry's hero powers and trinkets as the combat began (parser
/// revision 16, T-628), in round `round`: `None` when the record has none
/// (an older parser, an entry without a board, or a side the parser does
/// not save) or anything in it is not what the parser writes (unknown, never
/// guessed). A named minion's board position becomes its index on the board
/// before start of combat (the replay's start; positions run 1 to n). Not
/// part of [`board`]: the boards it reads are taken at the first attack.
pub fn kit(entry: &Value, round: Option<u32>, facts: &FactsOf) -> Option<Kit> {
    let k = entry.get("kit")?;
    entry.get("board")?.as_array()?;
    // The parser never saves a minion the opponent holds (D-164).
    let own = entry.get("side").and_then(Value::as_str) == Some("own");
    let cards = |key: &str, power: bool| -> Option<Vec<KitCard>> {
        let list = k.get(key)?.as_array()?;
        if list.len() > MAX_KIT_CARDS {
            return None;
        }
        list.iter()
            .map(|c| kit_card(c, power, own, facts))
            .collect()
    };
    Some(Kit {
        powers: cards("powers", true)?,
        trinkets: cards("trinkets", false)?,
        round,
    })
}

fn kit_card(v: &Value, power: bool, own: bool, facts: &FactsOf) -> Option<KitCard> {
    let card_id = v.get("card_id")?.as_str()?.to_string();
    let slot = u8::try_from(v.get("slot")?.as_i64()?)
        .ok()
        .filter(|s| *s <= 3)?;
    let mut counters = [0i32; 5];
    if let Some(n) = v.get("counters") {
        let n = n.as_array().filter(|a| a.len() == counters.len())?;
        for (out, x) in counters.iter_mut().zip(n) {
            *out = x.as_i64().filter(|x| *x >= 0).and_then(to_i32)?;
        }
    }
    let target = match v.get("target") {
        None => None,
        Some(t) if power => Some(
            t.as_i64()
                .filter(|p| (1..=7).contains(p))
                .and_then(|p| usize::try_from(p - 1).ok())?,
        ),
        Some(_) => return None,
    };
    let held = match v.get("held") {
        None => None,
        Some(m) if power && own => Some(minion(m, facts)?),
        Some(_) => return None,
    };
    Some(KitCard {
        card_id,
        slot,
        counters,
        target,
        held,
    })
}

/// A saved board's game-long values (parser revision 10, T-610), by the
/// report's names. A name left out, or a value outside what the report
/// saves (1 to 99 999), is zero: unknown, the cards' printed numbers.
pub fn saved_values(v: Option<&Value>) -> GameValues {
    let n = |name: &str| {
        v.and_then(|v| v.get(name))
            .and_then(Value::as_i64)
            .filter(|n| (1..=MAX_GAME_VALUE).contains(n))
            .map_or(0, |n| n as i32)
    };
    GameValues {
        blood_gem: (n("blood_gem_atk"), n("blood_gem_health")),
        spell: (n("spell_atk"), n("spell_health")),
        knights_dead: n("knights_dead"),
        undead_attack: n("undead_atk"),
        beetle: (n("beetle_atk"), n("beetle_health")),
    }
}

/// The result the game recorded for the round; `None` when unknown.
pub fn result(round: &Value) -> Option<CombatResult> {
    match round.get("result")?.as_str()? {
        "won" => Some(CombatResult::Won),
        "lost" => Some(CombatResult::Lost),
        "tie" => Some(CombatResult::Tie),
        _ => None,
    }
}

/// One minion of a saved replay as it stands now.
#[derive(Clone)]
struct Live {
    /// Index in the replay's `minions`.
    index: usize,
    minion: Minion,
}

fn side_of(m: &Value) -> Option<usize> {
    match m.get("side")?.as_str()? {
        "own" => Some(0),
        "opponent" => Some(1),
        _ => None,
    }
}

fn int(v: &Value, k: &str) -> Option<i64> {
    v.get(k)?.as_i64()
}

fn to_i32(x: i64) -> Option<i32> {
    i32::try_from(x).ok()
}

/// A minion that died, for placing a Reborn copy.
struct Death {
    card_id: String,
    index: usize,
    right: Option<usize>,
}

/// Where a minion goes whose right-hand neighbour was `right`: before it if
/// it is still on the board, else before that one's own right-hand
/// neighbour when it died too, and so on; the right end when none is left.
fn place_of(board: &[Live], deaths: &[Death], mut right: Option<usize>) -> usize {
    for _ in 0..=deaths.len() {
        let Some(r) = right else { break };
        if let Some(p) = board.iter().position(|l| l.index == r) {
            return p;
        }
        right = deaths.iter().find(|d| d.index == r).and_then(|d| d.right);
    }
    board.len()
}

/// The saved replay's attacks as logged steps; `None` when the replay is
/// incomplete or an event names a minion that is not on a board.
pub fn saved_steps(replay: &Value, minion: &MinionOf) -> Option<Vec<LoggedStep>> {
    if replay.get("complete").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let minions = replay.get("minions")?.as_array()?;
    let start = usize::try_from(replay.get("start")?.as_u64()?).ok()?;
    let events = replay.get("events")?.as_array()?;
    let mut boards: [Vec<Live>; 2] = [Vec::new(), Vec::new()];
    // Each death on a side: its card, its replay index and the minion that
    // was to its right (`None`: the right end), for a summon with no slot (a
    // Reborn copy goes where its card died).
    let mut deaths: [Vec<Death>; 2] = [Vec::new(), Vec::new()];
    for (index, m) in minions.iter().enumerate().take(start) {
        boards[side_of(m)?].push(Live {
            index,
            minion: minion(m)?,
        });
    }
    let find = |boards: &[Vec<Live>; 2], index: usize| -> Option<(usize, usize)> {
        (0..2).find_map(|s| {
            boards[s]
                .iter()
                .position(|l| l.index == index)
                .map(|p| (s, p))
        })
    };
    let mut steps = Vec::new();
    for e in events {
        let at = |k: &str| int(e, k).and_then(|x| usize::try_from(x).ok());
        match e.get("kind")?.as_str()? {
            "attack" => {
                let attacker = find(&boards, at("attacker")?)?;
                let defender = match at("defender") {
                    None => None,
                    Some(d) => Some(find(&boards, d)?.1),
                };
                let before = [0, 1].map(|s| boards[s].iter().map(|l| l.minion.clone()).collect());
                steps.push(LoggedStep {
                    attacker,
                    defender,
                    before,
                    all_keywords: false,
                });
            }
            "damage" => {
                let (s, p) = find(&boards, at("minion")?)?;
                boards[s][p].minion.health = to_i32(int(e, "health")?)?;
            }
            "stats" => {
                let (s, p) = find(&boards, at("minion")?)?;
                let m = &mut boards[s][p].minion;
                m.attack = to_i32(int(e, "atk")?)?;
                m.health = to_i32(int(e, "health")?)?;
            }
            "shield" => {
                let (s, p) = find(&boards, at("minion")?)?;
                let m = &mut boards[s][p].minion;
                m.keywords = if e.get("on")?.as_bool()? {
                    m.keywords.union(Keywords::DIVINE_SHIELD)
                } else {
                    m.keywords.without(Keywords::DIVINE_SHIELD)
                };
            }
            "death" => {
                let (s, p) = find(&boards, at("minion")?)?;
                let dead = boards[s].remove(p);
                deaths[s].push(Death {
                    card_id: dead.minion.card_id,
                    index: dead.index,
                    right: boards[s].get(p).map(|l| l.index),
                });
            }
            "summon" => {
                let index = at("minion")?;
                let m = minions.get(index)?;
                let s = side_of(m)?;
                let new = minion(m)?;
                let slot = int(m, "position").unwrap_or(0);
                let place = if slot > 0 {
                    usize::try_from(slot - 1).ok()?
                } else {
                    match deaths[s].iter().rposition(|d| d.card_id == new.card_id) {
                        Some(k) => {
                            let d = deaths[s].remove(k);
                            place_of(&boards[s], &deaths[s], d.right)
                        }
                        None => boards[s].len(),
                    }
                };
                let place = place.min(boards[s].len());
                boards[s].insert(place, Live { index, minion: new });
            }
            // Triggers, a teammate's board coming in (its minions are
            // summons) and the hero damage change no board.
            _ => {}
        }
    }
    Some(steps)
}

/// The same minions with the same stats and Divine Shield: what a rebuilt
/// board can be checked on (it follows no other keyword).
pub fn same_board(saved: &[Minion], rebuilt: &[Minion]) -> bool {
    let shield = |m: &Minion| m.keywords.contains(Keywords::DIVINE_SHIELD);
    saved.len() == rebuilt.len()
        && saved.iter().zip(rebuilt).all(|(a, b)| {
            a.card_id == b.card_id
                && a.attack == b.attack
                && a.health == b.health
                && shield(a) == shield(b)
        })
}

/// What one combat of a saved game gives the odds.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
    /// The report lacks a board on a side (a combat the game did not show).
    NoBoards,
    /// The card data lacks these cards: no odds.
    NoCardData(Vec<String>),
    Explained(Explained),
}

/// One combat with a result, and why it went as it did.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatWhy {
    pub round: i64,
    pub result: CombatResult,
    pub why: Why,
}

/// One side's boards in a round in the order they came in, one per seat
/// (two in Duos); `None` without a board, with a board that cannot be read,
/// or, in Solo, with more than one.
fn side_boards(entries: &[Value], side: &str, solo: bool, facts: &FactsOf) -> Option<Vec<Warband>> {
    let mut seats: Vec<Option<i64>> = Vec::new();
    let mut out = Vec::new();
    for e in entries {
        if e.get("side").and_then(Value::as_str) != Some(side) {
            continue;
        }
        if !matches!(e.get("board"), Some(Value::Array(_))) {
            continue;
        }
        let seat = e.get("player_id").and_then(Value::as_i64);
        if !solo && seats.contains(&seat) {
            continue;
        }
        seats.push(seat);
        out.push(board(e, facts)?);
    }
    let fits = if solo {
        out.len() == 1
    } else {
        (1..=2).contains(&out.len())
    };
    fits.then_some(out)
}

fn team(mut boards: Vec<Warband>) -> Team {
    let second = (boards.len() > 1).then(|| boards.remove(1));
    Team {
        first: boards.remove(0),
        second,
    }
}

/// Card ids of the minions the game marks as gifted with no gift named,
/// on the boards of a round's entries.
pub fn unknown_gifts<'a>(entries: impl IntoIterator<Item = &'a Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let boards = entries
        .into_iter()
        .filter_map(|e| e.get("board").and_then(Value::as_array));
    for m in boards.flatten() {
        let unknown = m.get("gift_unknown").and_then(Value::as_bool) == Some(true);
        if let (true, Some(id)) = (unknown, m.get("card_id").and_then(Value::as_str)) {
            if !out.iter().any(|o| o == id) {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// The round's saved replay as steps from the teams' first boards.
fn replay_steps(
    round: &Value,
    teams: &(Team, Team),
    facts: &FactsOf,
) -> Result<Vec<LoggedStep>, bool> {
    let Some(replay) = round.get("replay").filter(|r| r.is_object()) else {
        return Err(false);
    };
    let steps = saved_steps(replay, &|m| minion(m, facts))
        .filter(|s| !s.is_empty())
        .ok_or(true)?;
    let [p, o] = &steps[0].before;
    let fits = same_board(&teams.0.first.minions, p) && same_board(&teams.1.first.minions, o);
    fits.then_some(steps).ok_or(true)
}

fn round_why(
    round: &Value,
    result: CombatResult,
    solo: bool,
    facts: &FactsOf,
    options: &Options,
) -> Why {
    let entries: &[Value] = round
        .get("entries")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let lacking = RefCell::new(Vec::<String>::new());
    let checked = |id: &str| {
        let f = facts(id);
        let mut l = lacking.borrow_mut();
        if f.is_none() && !l.iter().any(|x| x == id) {
            l.push(id.to_string());
        }
        f
    };
    let sides = side_boards(entries, "own", solo, &checked)
        .zip(side_boards(entries, "opponent", solo, &checked));
    let lacking = lacking.into_inner();
    if !lacking.is_empty() {
        return Why::NoCardData(lacking);
    }
    let Some((own, opponent)) = sides else {
        return Why::NoBoards;
    };
    let teams = (team(own), team(opponent));
    let steps = replay_steps(round, &teams, facts);
    let replay = match &steps {
        Ok(s) => Replayable::Steps(s),
        Err(false) => Replayable::NotSaved,
        Err(true) => Replayable::Unreadable,
    };
    let gifts = unknown_gifts(entries);
    Why::Explained(explain(
        &CombatFacts {
            player: &teams.0,
            opponent: &teams.1,
            result,
            replay,
            unknown_gifts: &gifts,
        },
        options,
    ))
}

/// Every combat of a saved game (a report) that has a result, in round
/// order, with its odds and why they missed (D-127). A game that is not a
/// Battlegrounds game gives none.
pub fn explain_game(report: &Value, facts: &FactsOf, options: &Options) -> Vec<CombatWhy> {
    let solo = match report.get("game_type").and_then(Value::as_str) {
        Some("GT_BATTLEGROUNDS") => true,
        Some("GT_BATTLEGROUNDS_DUO") => false,
        _ => return Vec::new(),
    };
    let rounds = report.get("rounds").and_then(Value::as_array);
    rounds
        .into_iter()
        .flatten()
        .filter_map(|round| {
            let number = round.get("number").and_then(Value::as_i64)?;
            let result = result(round)?;
            Some(CombatWhy {
                round: number,
                result,
                why: round_why(round, result, solo, facts, options),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "saved_tests.rs"]
mod tests;
