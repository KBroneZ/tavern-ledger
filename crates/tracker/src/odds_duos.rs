//! Duos odds for the overlay (T-608): the player's board now and their
//! teammate's, in the order the game has set, against the next opposing
//! team's two boards (the one the game marks as the next opponent first).
//!
//! Red line (D-004, D-054): every board but the player's own is the last one
//! that entered play in a combat the game has shown. The teammate's board is
//! not in the log during the shop, so it is the one it last fought with on
//! screen; the copies the game makes of combats it does not play back are
//! never read (the parser gives no board for them). When a board was never
//! seen, the overlay names it instead of a number.

use bg_parser::report::{CombatValues, GameReport, LobbySlot, Minion as LogMinion, Side};
use serde::Serialize;

use crate::odds::{start_of_combat_in, Gap, NoOdds};

/// Whose board, besides the player's own, Duos odds use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Teammate,
    /// The board the game marks as the next opponent: it fights first.
    Opponent,
    OpponentTeammate,
}

/// A board the odds use and the combat it was seen in; `round` is `None`
/// when no board of that seat has been seen yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BoardSeen {
    pub role: Role,
    pub seat: i64,
    pub round: Option<i64>,
}

/// A board seen in an earlier combat, with what the odds need about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub seat: i64,
    pub round: i64,
    pub board: Vec<LogMinion>,
    /// The seat's tavern tier now (shown on the leaderboard).
    pub tier: Option<i64>,
    /// The seat's game-long values as that board was seen (T-610).
    pub values: CombatValues,
}

/// The boards Duos odds add to the player's own and the next opponent's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuosInput {
    /// The player's board fights before their teammate's.
    pub local_first: bool,
    pub teammate: Seen,
    pub opponent_teammate: Seen,
}

impl DuosInput {
    /// Every board but the player's, with the next opponent's.
    pub fn boards(&self, opponent: i64, seen: i64) -> Vec<BoardSeen> {
        vec![
            BoardSeen {
                role: Role::Teammate,
                seat: self.teammate.seat,
                round: Some(self.teammate.round),
            },
            BoardSeen {
                role: Role::Opponent,
                seat: opponent,
                round: Some(seen),
            },
            BoardSeen {
                role: Role::OpponentTeammate,
                seat: self.opponent_teammate.seat,
                round: Some(self.opponent_teammate.round),
            },
        ]
    }
}

/// A board, the combat it was seen in and the seat's game-long values then.
pub(crate) type LastBoard<'a> = (&'a [LogMinion], i64, CombatValues);

/// The newest board of `seat` on `side` that entered play in a combat the
/// game has shown, with its combat number and the seat's values then (zero
/// when the log gave none).
pub(crate) fn last_board(report: &GameReport, seat: i64, side: Side) -> Option<LastBoard<'_>> {
    report.rounds.iter().rev().find_map(|round| {
        round
            .entries
            .iter()
            .filter(|e| e.side == side && e.player_id == Some(seat))
            .find_map(|e| Some((e.board.as_deref()?, e.values.unwrap_or_default())))
            .map(|(b, values)| (b, round.number, values.bounded()))
    })
}

/// The other seat of `seat`'s Duos team, from the lobby the game showed at
/// hero select.
fn partner(report: &GameReport, seat: i64) -> Option<i64> {
    let team = report
        .lobby
        .iter()
        .find(|p| p.player_id == seat)?
        .duo_team?;
    report
        .lobby
        .iter()
        .find(|p| p.duo_team == Some(team) && p.player_id != seat)
        .map(|p| p.player_id)
}

/// The Duos boards for odds against the next opposing team, whose first
/// board is `next`; or why there are none. `fights_first` is the game's flag
/// on the player's own hero.
pub fn duos_input<'a>(
    report: &'a GameReport,
    next: i64,
    fights_first: Option<bool>,
    lobby: &[LobbySlot],
) -> Result<(DuosInput, Option<LastBoard<'a>>), NoOdds> {
    let order = || NoOdds::new(Gap::Order, Some(next));
    let local_first = fights_first.ok_or_else(order)?;
    let mate = report.teammate_player_id.ok_or_else(order)?;
    let foe_mate = partner(report, next).ok_or_else(order)?;
    let tier = |seat: i64| {
        lobby
            .iter()
            .find(|s| s.player_id == seat)
            .and_then(|s| s.tech_level)
    };
    let seen = |seat: i64, side: Side| {
        last_board(report, seat, side).map(|(board, round, values)| Seen {
            seat,
            round,
            board: board.to_vec(),
            tier: tier(seat),
            values,
        })
    };
    let teammate = seen(mate, Side::Own);
    let opponent = last_board(report, next, Side::Opponent);
    let opponent_teammate = seen(foe_mate, Side::Opponent);
    match (teammate, opponent_teammate) {
        (Some(teammate), Some(opponent_teammate)) if opponent.is_some() => Ok((
            DuosInput {
                local_first,
                teammate,
                opponent_teammate,
            },
            opponent,
        )),
        (teammate, opponent_teammate) => {
            let boards = vec![
                BoardSeen {
                    role: Role::Teammate,
                    seat: mate,
                    round: teammate.map(|s| s.round),
                },
                BoardSeen {
                    role: Role::Opponent,
                    seat: next,
                    round: opponent.map(|(_, r, _)| r),
                },
                BoardSeen {
                    role: Role::OpponentTeammate,
                    seat: foe_mate,
                    round: opponent_teammate.map(|s| s.round),
                },
            ];
            Err(NoOdds {
                boards,
                ..NoOdds::new(Gap::MissingBoard, Some(next))
            })
        }
    }
}

/// True when a card acted at the start of combat in the combat any of the
/// Duos boards was seen in.
pub(crate) fn seen_after_start_of_combat(report: &GameReport, duos: &DuosInput) -> bool {
    [duos.teammate.round, duos.opponent_teammate.round]
        .into_iter()
        .any(|round| start_of_combat_in(report, round))
}
