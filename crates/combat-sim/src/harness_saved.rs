//! Attack-by-attack replays from saved games (T-609): the combat replay the
//! app saves with each game (`rounds[].replay`, parser revision 8, D-099)
//! rebuilt into the same logged steps the game's own logs give
//! ([`crate::harness_logs`]), so a game with no `Power.log` left on the PC,
//! or a game someone else lent, can be replayed against the engine too.
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

use serde_json::Value;

use crate::combat::replay::LoggedStep;
use crate::harness::{minion, result, CardDb};
use crate::harness_logs::LogCombat;
use crate::model::{Keywords, Minion, Team};

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
pub fn saved_steps(replay: &Value, db: &CardDb) -> Option<Vec<LoggedStep>> {
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
            minion: minion(m, db)?,
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
                let new = minion(m, db)?;
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
fn same_board(saved: &[Minion], rebuilt: &[Minion]) -> bool {
    let shield = |m: &Minion| m.keywords.contains(Keywords::DIVINE_SHIELD);
    saved.len() == rebuilt.len()
        && saved.iter().zip(rebuilt).all(|(a, b)| {
            a.card_id == b.card_id
                && a.attack == b.attack
                && a.health == b.health
                && shield(a) == shield(b)
        })
}

/// What a saved round gives the replay.
pub enum Saved {
    /// No replay saved (the game did not play the combat back, or an older
    /// parser).
    None,
    /// A replay that cannot be rebuilt, or whose first boards are not the
    /// ones the report saved for the combat.
    Unreadable,
    Combat(Box<LogCombat>),
}

/// The saved replay of `round` as a combat to replay, from each team's
/// boards as they came in (`teams`: `None` when the report has no board on
/// each side).
pub fn saved_combat(
    round: &Value,
    game: usize,
    duos: bool,
    teams: Option<(Team, Team)>,
    db: &CardDb,
) -> Saved {
    let Some(replay) = round.get("replay").filter(|r| r.is_object()) else {
        return Saved::None;
    };
    let steps = saved_steps(replay, db).filter(|s| !s.is_empty());
    let (Some(steps), Some((player, opponent))) = (steps, teams) else {
        return Saved::Unreadable;
    };
    let [p, o] = &steps[0].before;
    if !same_board(&player.first.minions, p) || !same_board(&opponent.first.minions, o) {
        return Saved::Unreadable;
    }
    Saved::Combat(Box::new(LogCombat {
        game,
        round: int(round, "number").unwrap_or(0),
        duos,
        player,
        opponent,
        steps,
        result: result(round),
    }))
}

#[cfg(test)]
#[path = "harness_saved_tests.rs"]
mod tests;
