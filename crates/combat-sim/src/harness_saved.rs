//! Attack-by-attack replays from saved games (T-609): the combat replay the
//! app saves with each game, rebuilt by [`crate::saved::saved_steps`], so a
//! game with no `Power.log` left on the PC, or a game someone else lent, can
//! be replayed against the engine too. The rebuilt boards at the first
//! attack must equal the boards the report saved for the combat, or the
//! combat is not replayed ("unreadable").

use serde_json::Value;

use crate::harness::{minion, result, CardDb};
use crate::harness_logs::LogCombat;
use crate::model::Team;
use crate::saved::{same_board, saved_steps};

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
    let steps = saved_steps(replay, &|m| minion(m, db)).filter(|s| !s.is_empty());
    let (Some(steps), Some((player, opponent))) = (steps, teams) else {
        return Saved::Unreadable;
    };
    let [p, o] = &steps[0].before;
    if !same_board(&player.first.minions, p) || !same_board(&opponent.first.minions, o) {
        return Saved::Unreadable;
    }
    Saved::Combat(Box::new(LogCombat {
        game,
        round: round.get("number").and_then(Value::as_i64).unwrap_or(0),
        duos,
        player,
        opponent,
        steps,
        result: result(round),
    }))
}
