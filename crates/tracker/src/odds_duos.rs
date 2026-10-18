//! The boards of one combat, for the overlay's odds (D-152): the boards the
//! game has put into play in the combat it is playing now, read from the
//! report's round for that combat. In Solo one board each side; in Duos each
//! team brings two, and a team's second board enters play only when its
//! first one is gone, so the odds come once all four have entered.
//!
//! Red line (D-004, D-054): only boards that entered play in a combat the
//! game is showing; the copies the game makes of combats it does not play
//! back are never read (the parser gives no board for them). When a board is
//! not in yet, the overlay names it instead of a number.
//!
//! Each board is read at one of two moments (T-628, D-165): at its first
//! attack (the odds), or as its leg began, before its start of combat (the
//! early estimate), with its hero's powers and trinkets then.

use bg_parser::report::{CombatValues, GameReport, HeroKit, LobbySlot, Minion as LogMinion, Side};
use serde::Serialize;

use crate::odds::{Gap, NoOdds};

/// Whose board the odds use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The player's own board.
    Own,
    Teammate,
    /// The board that fights first on the other team.
    Opponent,
    OpponentTeammate,
}

/// A board the odds use and the combat it entered play in; `round` is `None`
/// when the board has not entered play yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BoardSeen {
    pub role: Role,
    pub seat: i64,
    pub round: Option<i64>,
}

/// A board of the combat, with what the odds need about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub seat: i64,
    pub round: i64,
    pub board: Vec<LogMinion>,
    /// The seat's tavern tier now (shown on the leaderboard).
    pub tier: Option<i64>,
    /// The seat's game-long values as the board entered play (T-610).
    pub values: CombatValues,
    /// The seat's hero powers and trinkets as the leg began (T-628); None
    /// when the log gave none.
    pub kit: Option<HeroKit>,
}

/// The boards Duos odds add to the player's own and the first opponent's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuosInput {
    /// The player's board fights before their teammate's.
    pub local_first: bool,
    pub teammate: Seen,
    pub opponent_teammate: Seen,
}

impl DuosInput {
    /// Every board but the player's, with the first opponent's.
    pub fn boards(&self, opponent: i64, round: i64) -> Vec<BoardSeen> {
        vec![
            BoardSeen {
                role: Role::Teammate,
                seat: self.teammate.seat,
                round: Some(self.teammate.round),
            },
            BoardSeen {
                role: Role::Opponent,
                seat: opponent,
                round: Some(round),
            },
            BoardSeen {
                role: Role::OpponentTeammate,
                seat: self.opponent_teammate.seat,
                round: Some(self.opponent_teammate.round),
            },
        ]
    }
}

/// Which board of each leg the odds read (D-165).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Moment {
    /// At the leg's first attack, its start of combat played: the odds.
    FirstAttack,
    /// As the leg began, before its start of combat: the early estimate.
    BeforeStart,
}

/// A board that entered play, whose it is, the seat's game-long values as
/// it entered (zero when the log gave none) and its hero's powers and
/// trinkets as the leg began.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Leg<'a> {
    pub seat: i64,
    pub board: &'a [LogMinion],
    pub values: CombatValues,
    pub kit: Option<&'a HeroKit>,
}

/// The boards of `side` in combat `number` at `moment`, in the order they
/// came in, one per seat. Before start of combat: the legs that reached
/// their first attack, then the ones between their start of combat and it
/// (the report's `opening`).
pub(crate) fn legs(report: &GameReport, number: i64, side: Side, moment: Moment) -> Vec<Leg<'_>> {
    let mut out: Vec<Leg<'_>> = Vec::new();
    let entries = report
        .rounds
        .iter()
        .find(|r| r.number == number)
        .into_iter()
        .flat_map(|r| r.entries.iter())
        .filter(|e| e.side == side);
    for e in entries {
        let board = match moment {
            Moment::FirstAttack => e.board.as_deref(),
            Moment::BeforeStart => e.board.as_ref().and(e.before.as_deref()),
        };
        let (Some(seat), Some(board)) = (e.player_id, board) else {
            continue;
        };
        if out.iter().all(|l| l.seat != seat) {
            out.push(Leg {
                seat,
                board,
                values: e.values.unwrap_or_default().bounded(),
                kit: e.kit.as_ref(),
            });
        }
    }
    if moment == Moment::BeforeStart {
        let opening = report
            .opening
            .iter()
            .filter(|o| o.round == number && o.side == side);
        for o in opening {
            let Some(seat) = o.player_id else { continue };
            if out.iter().all(|l| l.seat != seat) {
                out.push(Leg {
                    seat,
                    board: &o.board,
                    values: o.values.bounded(),
                    kit: Some(&o.kit),
                });
            }
        }
    }
    out
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

/// The boards of Duos combat `number` at `moment`: the player's, the first
/// opponent's and the Duos extras; or why there are none yet. The boards
/// that have not entered play are named in the reason.
pub(crate) fn duos_combat<'a>(
    report: &'a GameReport,
    number: i64,
    lobby: &[LobbySlot],
    moment: Moment,
) -> Result<(Leg<'a>, Leg<'a>, DuosInput), NoOdds> {
    let bad = || NoOdds::new(Gap::BadBoard, None);
    let local = report.local_player_id.ok_or_else(bad)?;
    let mate = report.teammate_player_id.ok_or_else(bad)?;
    let own_legs = legs(report, number, Side::Own, moment);
    let foe_legs = legs(report, number, Side::Opponent, moment);
    let Some(foe) = foe_legs.first().map(|l| l.seat) else {
        return Err(NoOdds::new(Gap::NoBoardSeen, None));
    };
    let foe_mate = partner(report, foe).ok_or_else(|| NoOdds::new(Gap::BadBoard, Some(foe)))?;
    let find = |legs: &[Leg<'a>], seat: i64| legs.iter().copied().find(|l| l.seat == seat);
    let (own, teammate) = (find(&own_legs, local), find(&own_legs, mate));
    let (first, second) = (find(&foe_legs, foe), find(&foe_legs, foe_mate));
    let (Some(own), Some(teammate), Some(first), Some(second)) = (own, teammate, first, second)
    else {
        let seen = |role, seat, leg: Option<Leg<'_>>| BoardSeen {
            role,
            seat,
            round: leg.map(|_| number),
        };
        let boards = vec![
            seen(Role::Own, local, own),
            seen(Role::Teammate, mate, teammate),
            seen(Role::Opponent, foe, first),
            seen(Role::OpponentTeammate, foe_mate, second),
        ];
        return Err(NoOdds {
            boards,
            ..NoOdds::new(Gap::MissingBoard, Some(foe))
        });
    };
    let tier = |seat: i64| {
        lobby
            .iter()
            .find(|s| s.player_id == seat)
            .and_then(|s| s.tech_level)
    };
    let seen = |leg: Leg<'_>| Seen {
        seat: leg.seat,
        round: number,
        board: leg.board.to_vec(),
        tier: tier(leg.seat),
        values: leg.values,
        kit: leg.kit.cloned(),
    };
    let duos = DuosInput {
        local_first: own_legs.first().is_some_and(|l| l.seat == local),
        teammate: seen(teammate),
        opponent_teammate: seen(second),
    };
    Ok((own, first, duos))
}
