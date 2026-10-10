//! What the overlay shows about the game being played (T-301). Built from
//! the parser's report of the game so far, by the same reading that the
//! recap uses on a saved game, so live and saved agree.
//!
//! Red line (D-004, D-006, D-010, D-012): only what the player could see on
//! their own screen. Boards are the ones that entered play in a combat the
//! log replays; opponents are a hero (and a seat of this game), never a name,
//! a BattleTag or an account. A value the log does not give is unknown, never
//! zero and never guessed.

use std::collections::BTreeSet;

use bg_parser::report::LobbySlot;
use serde::Serialize;
use serde_json::Value;

use crate::possible::Possible;
use crate::provenance::{legend, LegendEntry, Source};
use crate::recap::{list, recap, text, texts, tribes, HeroRef, Sourced, TribeOffer, View};

/// How far the game is, as far as the log says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// The log has not shown the lobby yet (hero select).
    Starting,
    Playing,
    /// The log says the game is over.
    Over,
    /// The parser could not read this game (e.g. a game patch changed the
    /// log): the overlay says so instead of showing guesses.
    Unreadable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveMinion {
    pub card_id: Option<String>,
    /// None when the report has no number: unknown, not 0.
    pub atk: Option<i64>,
    pub health: Option<i64>,
    pub golden: bool,
}

/// A board that entered play in a combat, with the combat it was seen in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveBoard {
    pub hero: Option<HeroRef>,
    pub round: i64,
    pub minions: Vec<LiveMinion>,
}

/// Combats won, lost or tied against an opponent (a team in Duos), worked out
/// from health (D-028): always inferred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveRecord {
    pub won: usize,
    pub lost: usize,
    pub tie: usize,
    pub unknown: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveOpponent {
    /// One hero in Solo, the team's heroes in Duos.
    pub heroes: Vec<HeroRef>,
    pub seats: Vec<i64>,
    /// Health after the last combat the log closed.
    pub health: Sourced<i64>,
    /// The last board of each of them that entered play; empty until one did.
    pub boards: Vec<LiveBoard>,
    pub boards_source: Source,
    /// None until we have fought them.
    pub record: Option<LiveRecord>,
    pub record_source: Source,
    /// Their place on the game's leaderboard now (top is 1; a team's place in
    /// Duos), from the log (T-306).
    pub place: Sourced<i64>,
    /// Each hero's tavern tier now, in the order of `seats`, from the log.
    pub tiers: Vec<Sourced<i64>>,
    /// Minions each hero at tier 1 or 2 could have (T-307), added by the app
    /// from the card data; empty otherwise.
    pub possible: Vec<Possible>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LiveGame {
    pub phase: Phase,
    pub is_duos: bool,
    pub game_type: Sourced<String>,
    pub hero: Sourced<HeroRef>,
    pub teammate_hero: Sourced<HeroRef>,
    /// The number of the last combat the log has.
    pub last_combat: Sourced<i64>,
    /// The game turn now: the number of the newest shop phase in the shop
    /// record (1 for the first shop; its combat keeps the same number), the
    /// same turn the shop record and the rounds use (T-309, D-049).
    pub turn: Sourced<i64>,
    /// Our own seats (ours and, in Duos, the teammate's): the leaderboard
    /// slot that holds them shows nothing on hover.
    pub own_seats: Vec<i64>,
    /// Our place on the game's leaderboard now, from the log.
    pub own_place: Sourced<i64>,
    /// How many slots the game's leaderboard has (8 in Solo, 4 teams in
    /// Duos), from the places the log gives; None until every place is known.
    pub leaderboard_slots: Option<i64>,
    pub own_health: Sourced<i64>,
    /// Tavern offers, most common first. Not the lobby's tribes.
    pub tribes: Vec<TribeOffer>,
    pub tribes_source: Source,
    /// The five tribes the user picked (T-303), never merged with `tribes`.
    pub entered_tribes: Vec<String>,
    pub entered_tribes_source: Source,
    pub opponents: Vec<LiveOpponent>,
    pub record_basis: &'static str,
    /// The words of every source mark, from the app (T-109): the overlay shows
    /// them and chooses none.
    pub legend: Vec<LegendEntry>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
}

impl LiveGame {
    /// Adds the tribes the user entered for this game, labelled as such.
    pub fn with_entered_tribes(mut self, entered: Option<&[String]>) -> Self {
        self.entered_tribes = entered.map(<[String]>::to_vec).unwrap_or_default();
        self.entered_tribes_source = if self.entered_tribes.is_empty() {
            Source::Unknown
        } else {
            Source::Entered
        };
        self
    }
}

impl LiveGame {
    /// Adds the leaderboard as the log has it now (T-306): every opponent's
    /// place and tiers and our own place, and sorts the opponents in the
    /// leaderboard's order (unknown places last).
    pub fn with_leaderboard(mut self, lobby: &[LobbySlot]) -> Self {
        let slot = |pid: i64| lobby.iter().find(|s| s.player_id == pid);
        let place_of = |seats: &[i64]| seats.iter().find_map(|&pid| slot(pid)?.place);
        for opponent in &mut self.opponents {
            opponent.place = Sourced::log(place_of(&opponent.seats));
            opponent.tiers = opponent
                .seats
                .iter()
                .map(|&pid| Sourced::log(slot(pid).and_then(|s| s.tech_level)))
                .collect();
        }
        self.own_place = Sourced::log(place_of(&self.own_seats));
        self.leaderboard_slots = leaderboard_slots(lobby);
        self.opponents.sort_by_key(|o| {
            (
                o.place.value.is_none(),
                o.place.value,
                std::cmp::Reverse(o.health.value),
                o.seats.clone(),
            )
        });
        self
    }
}

/// The number of leaderboard slots, when the places look like a whole
/// leaderboard: 1 to N with no gap, each held by one hero (Solo) or by the
/// two heroes of a team (Duos). Anything else is unknown.
fn leaderboard_slots(lobby: &[LobbySlot]) -> Option<i64> {
    let places: Option<Vec<i64>> = lobby.iter().map(|s| s.place).collect();
    let places = places?;
    let distinct: Vec<i64> = places
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let n = i64::try_from(distinct.len()).ok()?;
    let whole = distinct == (1..=n).collect::<Vec<_>>();
    let per_slot = places.len() / distinct.len().max(1);
    let even = places.len() % distinct.len().max(1) == 0 && (per_slot == 1 || per_slot == 2);
    (n > 0 && whole && even).then_some(n)
}

/// A parser problem that is a failure to read, not just "the lobby is not
/// there yet".
const PARSER_ERROR: &str = "parser error";

/// The live state from the report of the game so far (or of a saved game).
pub fn live_game(report: &Value) -> LiveGame {
    let view = View::new(report);
    let recap = recap(report);
    let tribes = tribes(report);
    let phase = phase_of(report);
    let own = view.own_side().unwrap_or_default();
    let opponents = if phase == Phase::Unreadable {
        Vec::new()
    } else {
        opponents_of(&view, &own, &recap.record)
    };
    LiveGame {
        phase,
        is_duos: view.duos,
        game_type: recap.game_type,
        hero: recap.hero,
        teammate_hero: recap.teammate_hero,
        last_combat: Sourced::log(view.rounds.keys().next_back().copied()),
        turn: Sourced::log(turn_now(report)),
        own_seats: own.clone(),
        own_place: Sourced::log(None),
        leaderboard_slots: None,
        own_health: Sourced::log(own_health_now(&recap.health)),
        tribes_source: if tribes.is_empty() {
            Source::Unknown
        } else {
            Source::Inferred
        },
        tribes,
        entered_tribes: Vec::new(),
        entered_tribes_source: Source::Unknown,
        opponents,
        record_basis: recap.record_basis,
        legend: legend(),
        warnings: texts(report, "warnings"),
        problems: texts(report, "problems"),
    }
}

/// The newest shop turn's number, from the report's shop record.
fn turn_now(report: &Value) -> Option<i64> {
    report
        .pointer("/shop/turns")?
        .as_array()?
        .last()?
        .get("turn")?
        .as_i64()
        .filter(|t| *t >= 1)
}

/// The same rule as [`current_health`]: the last point, or the one before it
/// when the last combat is still open.
fn own_health_now(points: &[crate::recap::HealthPoint]) -> Option<i64> {
    let mut last = points.iter().rev();
    let newest = last.next()?;
    newest
        .health
        .or_else(|| last.next().and_then(|before| before.health))
}

fn phase_of(report: &Value) -> Phase {
    match text(report, "status").as_deref() {
        Some("ok") => Phase::Over,
        Some("incomplete") => Phase::Playing,
        _ if texts(report, "problems")
            .iter()
            .any(|p| p.contains(PARSER_ERROR)) =>
        {
            Phase::Unreadable
        }
        _ => Phase::Starting,
    }
}

/// Everyone who is not on our side, grouped as the game groups them (a
/// team in Duos), with the record against them when we have fought them.
fn opponents_of(
    view: &View,
    own: &[i64],
    records: &[crate::recap::OpponentRecord],
) -> Vec<LiveOpponent> {
    if own.is_empty() {
        return Vec::new();
    }
    let sides: BTreeSet<Vec<i64>> = view
        .lobby
        .iter()
        .filter(|seat| !own.contains(&seat.pid))
        .map(|seat| view.side_of(seat.pid))
        .collect();
    let mut opponents: Vec<LiveOpponent> = sides
        .into_iter()
        .map(|seats| opponent_of(view, seats, records))
        .collect();
    // Healthiest first, like the game's leaderboard; unknown health last.
    opponents.sort_by_key(|o| (std::cmp::Reverse(o.health.value), o.seats.clone()));
    opponents
}

fn opponent_of(
    view: &View,
    seats: Vec<i64>,
    records: &[crate::recap::OpponentRecord],
) -> LiveOpponent {
    let heroes = seats
        .iter()
        .filter_map(|pid| view.seat(*pid)?.hero.as_deref())
        .map(|id| view.hero_ref(id))
        .collect();
    let boards: Vec<LiveBoard> = seats
        .iter()
        .filter_map(|&pid| last_board(view, pid))
        .collect();
    let record = records
        .iter()
        .find(|r| r.seats == seats)
        .map(|r| LiveRecord {
            won: r.won,
            lost: r.lost,
            tie: r.tie,
            unknown: r.unknown,
        });
    LiveOpponent {
        health: Sourced::log(current_health(view, &seats)),
        place: Sourced::log(None),
        tiers: seats.iter().map(|_| Sourced::log(None)).collect(),
        possible: Vec::new(),
        boards_source: Source::log_if(!boards.is_empty()),
        record_source: if record.is_some() {
            Source::Inferred
        } else {
            Source::Unknown
        },
        heroes,
        boards,
        record,
        seats,
    }
}

/// Health after the last closed combat (the start health before any). Only
/// the newest combat may lack a number, because it is still open; an older
/// round without one is a gap, and then the health is unknown, not a stale
/// number shown as current.
fn current_health(view: &View, seats: &[i64]) -> Option<i64> {
    let mut rounds = view.rounds.keys().rev();
    let Some(&newest) = rounds.next() else {
        return view.health_of(seats, 0);
    };
    view.health_of(seats, newest)
        .or_else(|| match rounds.next() {
            Some(&before) => view.health_of(seats, before),
            None => view.health_of(seats, 0),
        })
}

/// The newest board of this seat that entered play (D-010). A combat the log
/// never replayed has no board (null): it is skipped, not shown empty.
fn last_board(view: &View, pid: i64) -> Option<LiveBoard> {
    view.rounds.iter().rev().find_map(|(&n, round)| {
        list(round, "entries")
            .iter()
            .filter(|e| e.get("side").and_then(Value::as_str) == Some("opponent"))
            .filter(|e| e.get("player_id").and_then(Value::as_i64) == Some(pid))
            .find_map(|e| {
                let minions = e.get("board")?.as_array()?;
                Some(LiveBoard {
                    hero: text(e, "hero").map(|id| view.hero_ref(&id)),
                    round: n,
                    minions: minions_of(minions),
                })
            })
    })
}

fn minions_of(board: &[Value]) -> Vec<LiveMinion> {
    let mut placed: Vec<(i64, LiveMinion)> = board
        .iter()
        .map(|m| {
            let number = |key: &str| m.get(key).and_then(Value::as_i64);
            (
                number("position").unwrap_or(i64::MAX),
                LiveMinion {
                    card_id: text(m, "card_id"),
                    atk: number("atk"),
                    health: number("health"),
                    golden: m.get("golden").and_then(Value::as_bool).unwrap_or(false),
                },
            )
        })
        .collect();
    placed.sort_by_key(|(position, _)| *position);
    placed.into_iter().map(|(_, minion)| minion).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{DUOS, SOLO};
    use serde_json::json;

    fn minion(card: &str, atk: i64, health: i64, position: i64) -> Value {
        json!({"card_id": card, "atk": atk, "health": health, "position": position, "golden": false})
    }

    fn round(n: i64, opp: i64, board: Value, health: Value) -> Value {
        json!({
            "number": n,
            "own_health_after": health["1"],
            "health_after": health,
            "entries": [
                {"side": "own", "player_id": 1, "hero": "H_ME", "board": []},
                {"side": "opponent", "player_id": opp, "hero": format!("H_{opp}"), "board": board},
            ],
            "opponents": [opp],
        })
    }

    /// Solo, four players, we are seat 1.
    fn solo(rounds: Vec<Value>, status: &str) -> Value {
        json!({
            "status": status, "game_type": SOLO, "local_player_id": 1, "hero": "H_ME",
            "card_names": {"H_ME": "Me Hero", "H_2": "Second"},
            "lobby": [
                {"player_id": 1, "hero": "H_ME", "duo_team": null, "final_place": null, "final_health": 40},
                {"player_id": 2, "hero": "H_2", "duo_team": null, "final_place": null, "final_health": 40},
                {"player_id": 3, "hero": "H_3", "duo_team": null, "final_place": null, "final_health": 40},
                {"player_id": 4, "hero": "H_4", "duo_team": null, "final_place": null, "final_health": 40},
            ],
            "start_health": {"1": 40, "2": 40, "3": 40, "4": 40},
            "shop_tribes": {"BEAST": 4, "MURLOC": 9},
            "rounds": rounds, "warnings": [], "problems": [],
        })
    }

    fn opponent(live: &LiveGame, seat: i64) -> &LiveOpponent {
        live.opponents
            .iter()
            .find(|o| o.seats == [seat])
            .expect("opponent")
    }

    #[test]
    fn every_lobby_hero_but_us_is_an_opponent_even_before_a_combat() {
        let live = live_game(&solo(vec![], "incomplete"));
        let mut seats: Vec<_> = live.opponents.iter().map(|o| o.seats.clone()).collect();
        seats.sort();
        assert_eq!(seats, [[2], [3], [4]]);
        let first = opponent(&live, 2);
        assert_eq!(first.heroes[0].name.as_deref(), Some("Second"));
        assert_eq!(first.record, None, "not fought yet is not 0-0-0");
        assert_eq!(first.record_source, Source::Unknown);
        assert!(first.boards.is_empty());
        assert_eq!(first.boards_source, Source::Unknown);
        assert_eq!(first.health.value, Some(40), "start health from the log");
    }

    #[test]
    fn the_last_board_that_entered_play_comes_with_its_round() {
        let rounds = vec![
            round(
                1,
                2,
                json!([minion("OLD", 1, 1, 1)]),
                json!({"1": 40, "2": 40, "3": 40, "4": 40}),
            ),
            round(
                2,
                3,
                json!([minion("C3", 2, 2, 1)]),
                json!({"1": 35, "2": 40, "3": 40, "4": 40}),
            ),
            round(
                3,
                2,
                json!([minion("B", 5, 6, 2), minion("A", 3, 3, 1)]),
                json!({"1": 35, "2": 31, "3": 40, "4": 40}),
            ),
        ];
        let live = live_game(&solo(rounds, "incomplete"));
        let two = opponent(&live, 2);
        assert_eq!(two.boards.len(), 1);
        assert_eq!(two.boards[0].round, 3, "the newest board, not the first");
        let ids: Vec<_> = two.boards[0]
            .minions
            .iter()
            .map(|m| m.card_id.as_deref())
            .collect();
        assert_eq!(ids, [Some("A"), Some("B")], "in board order");
        assert_eq!(two.boards_source, Source::Log);
        assert_eq!(opponent(&live, 3).boards[0].round, 2);
        assert!(opponent(&live, 4).boards.is_empty());
    }

    #[test]
    fn a_board_the_log_never_replayed_is_unknown_not_empty() {
        let hidden = round(1, 2, Value::Null, json!({"1": 40, "2": 40}));
        let live = live_game(&solo(vec![hidden], "incomplete"));
        assert!(opponent(&live, 2).boards.is_empty());
        assert_eq!(opponent(&live, 2).boards_source, Source::Unknown);
    }

    #[test]
    fn an_empty_board_that_did_play_is_a_board() {
        let live = live_game(&solo(
            vec![round(1, 2, json!([]), json!({"1": 40, "2": 40}))],
            "incomplete",
        ));
        let two = opponent(&live, 2);
        assert_eq!(two.boards.len(), 1);
        assert!(two.boards[0].minions.is_empty());
        assert_eq!(two.boards_source, Source::Log);
    }

    #[test]
    fn the_record_is_worked_out_from_health_and_inferred() {
        let rounds = vec![
            round(1, 2, json!([]), json!({"1": 35, "2": 40, "3": 40, "4": 40})),
            round(2, 2, json!([]), json!({"1": 35, "2": 30, "3": 40, "4": 40})),
        ];
        let live = live_game(&solo(rounds, "incomplete"));
        let two = opponent(&live, 2);
        let record = two.record.as_ref().expect("fought");
        assert_eq!((record.won, record.lost, record.tie), (1, 1, 0));
        assert_eq!(two.record_source, Source::Inferred);
        assert_eq!(two.health.value, Some(30));
        assert_eq!(live.own_health.value, Some(35));
        assert_eq!(live.last_combat.value, Some(2));
    }

    #[test]
    fn health_during_a_combat_is_the_one_after_the_last_closed_combat() {
        // Round 2 is on but its health is not known yet (the combat is open).
        let mut open = round(2, 3, json!([]), json!({}));
        open["own_health_after"] = Value::Null;
        let rounds = vec![
            round(1, 2, json!([]), json!({"1": 35, "2": 40, "3": 40, "4": 40})),
            open,
        ];
        let live = live_game(&solo(rounds, "incomplete"));
        assert_eq!(live.own_health.value, Some(35));
        assert_eq!(opponent(&live, 3).health.value, Some(40));
    }

    #[test]
    fn an_older_round_without_health_is_a_gap_not_a_stale_number() {
        let mut gap = round(2, 3, json!([]), json!({}));
        gap["own_health_after"] = Value::Null;
        let rounds = vec![
            round(1, 2, json!([]), json!({"1": 35, "2": 40, "3": 40, "4": 40})),
            gap,
            round(3, 4, json!([]), json!({})),
        ];
        let live = live_game(&solo(rounds, "incomplete"));
        assert_eq!(opponent(&live, 3).health.value, None);
        assert_eq!(opponent(&live, 3).health.source, Source::Unknown);
    }

    #[test]
    fn a_minion_number_the_log_lacks_is_unknown_not_zero() {
        let mut broken = minion("A", 1, 1, 1);
        broken.as_object_mut().unwrap().remove("health");
        let live = live_game(&solo(
            vec![round(1, 2, json!([broken]), json!({"1": 40, "2": 40}))],
            "incomplete",
        ));
        let m = &opponent(&live, 2).boards[0].minions[0];
        assert_eq!((m.atk, m.health), (Some(1), None));
    }

    #[test]
    fn health_the_log_does_not_give_is_unknown_not_zero() {
        let mut report = solo(vec![], "incomplete");
        report["start_health"] = json!({});
        let live = live_game(&report);
        assert_eq!(opponent(&live, 2).health.value, None);
        assert_eq!(opponent(&live, 2).health.source, Source::Unknown);
        assert_eq!(live.own_health.value, None);
        assert_eq!(live.own_health.source, Source::Unknown);
    }

    #[test]
    fn tavern_tribes_and_entered_tribes_stay_apart_and_labelled() {
        let live = live_game(&solo(vec![], "incomplete"));
        assert_eq!(live.tribes[0].tribe, "MURLOC");
        assert_eq!(live.tribes_source, Source::Inferred);
        assert!(live.entered_tribes.is_empty());
        assert_eq!(live.entered_tribes_source, Source::Unknown);
        let tribes: Vec<String> = ["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]
            .map(String::from)
            .to_vec();
        let live = live.with_entered_tribes(Some(&tribes));
        assert_eq!(live.entered_tribes, tribes);
        assert_eq!(live.entered_tribes_source, Source::Entered);
        assert_eq!(
            live.tribes[0].tribe, "MURLOC",
            "the tavern list is untouched"
        );
    }

    #[test]
    fn no_tribes_seen_is_unknown() {
        let mut report = solo(vec![], "incomplete");
        report["shop_tribes"] = json!({});
        let live = live_game(&report);
        assert!(live.tribes.is_empty());
        assert_eq!(live.tribes_source, Source::Unknown);
    }

    #[test]
    fn phase_follows_what_the_parser_says() {
        assert_eq!(live_game(&solo(vec![], "incomplete")).phase, Phase::Playing);
        assert_eq!(live_game(&solo(vec![], "ok")).phase, Phase::Over);
        let starting = json!({"status": "unsupported", "game_type": SOLO,
            "problems": ["could not identify the local player and the shop player"]});
        assert_eq!(live_game(&starting).phase, Phase::Starting);
        let broken = json!({"status": "unsupported", "game_type": SOLO,
            "problems": ["parser error: RegexParsingError"]});
        let live = live_game(&broken);
        assert_eq!(live.phase, Phase::Unreadable);
        assert!(live.opponents.is_empty());
        assert_eq!(live.problems, ["parser error: RegexParsingError"]);
    }

    #[test]
    fn without_our_own_seat_nobody_is_called_an_opponent() {
        let mut report = solo(vec![], "incomplete");
        report.as_object_mut().unwrap().remove("local_player_id");
        assert!(live_game(&report).opponents.is_empty());
    }

    /// Duos: we are seats 1 and 2 (team 1); 3 and 4 are team 2; 5 and 6 team 3.
    fn duos(rounds: Vec<Value>) -> Value {
        let seat = |pid: i64, team: i64| {
            json!({"player_id": pid, "hero": format!("H_{pid}"), "duo_team": team,
                   "final_place": null, "final_health": 30})
        };
        json!({
            "status": "incomplete", "game_type": DUOS, "local_player_id": 1, "hero": "H_1",
            "teammate_player_id": 2, "teammate_hero": "H_2",
            "lobby": [seat(1, 1), seat(2, 1), seat(3, 2), seat(4, 2), seat(5, 3), seat(6, 3)],
            "start_health": {"1": 30, "2": 30, "3": 30, "4": 30, "5": 30, "6": 30},
            "shop_tribes": {}, "rounds": rounds, "warnings": [], "problems": [],
        })
    }

    #[test]
    fn in_duos_an_opponent_is_a_team_with_a_board_per_hero() {
        let both_legs = json!({
            "number": 2,
            "own_health_after": 30,
            "health_after": {"1": 30, "2": 30, "3": 30, "4": 30, "5": 30, "6": 30},
            "entries": [
                {"side": "own", "player_id": 1, "hero": "H_1", "board": []},
                {"side": "opponent", "player_id": 3, "hero": "H_3", "board": [minion("X", 1, 1, 1)]},
                {"side": "own", "player_id": 2, "hero": "H_2", "board": []},
                {"side": "opponent", "player_id": 4, "hero": "H_4", "board": [minion("Y", 2, 2, 1)]},
            ],
            "opponents": [3, 4],
        });
        let live = live_game(&duos(vec![both_legs]));
        assert!(live.is_duos);
        assert_eq!(
            live.opponents.len(),
            2,
            "two opposing teams, not four heroes"
        );
        let team = live.opponents.iter().find(|o| o.seats == [3, 4]).unwrap();
        assert_eq!(team.heroes.len(), 2);
        let boards: Vec<_> = team
            .boards
            .iter()
            .map(|b| b.minions[0].card_id.as_deref())
            .collect();
        assert_eq!(boards, [Some("X"), Some("Y")]);
        let other = live.opponents.iter().find(|o| o.seats == [5, 6]).unwrap();
        assert!(other.boards.is_empty());
        assert_eq!(live.teammate_hero.value.as_ref().unwrap().id, "H_2");
    }

    fn slot(player_id: i64, place: Option<i64>, tech_level: Option<i64>) -> LobbySlot {
        LobbySlot {
            player_id,
            place,
            tech_level,
        }
    }

    #[test]
    fn the_leaderboard_gives_places_tiers_and_its_order() {
        let lobby = [
            slot(1, Some(2), Some(3)),
            slot(2, Some(4), Some(1)),
            slot(3, Some(1), Some(2)),
            slot(4, Some(3), None),
        ];
        let live = live_game(&solo(vec![], "incomplete")).with_leaderboard(&lobby);
        let order: Vec<_> = live.opponents.iter().map(|o| o.seats[0]).collect();
        assert_eq!(order, [3, 4, 2], "top of the leaderboard first");
        assert_eq!(opponent(&live, 3).place.value, Some(1));
        assert_eq!(opponent(&live, 2).tiers[0].value, Some(1));
        assert_eq!(opponent(&live, 4).tiers[0].source, Source::Unknown);
        assert_eq!(live.own_place.value, Some(2));
        assert_eq!(live.own_seats, [1]);
        assert_eq!(live.leaderboard_slots, Some(4));
    }

    #[test]
    fn without_the_leaderboard_places_and_tiers_are_unknown() {
        let live = live_game(&solo(vec![], "incomplete"));
        let two = opponent(&live, 2);
        assert_eq!(two.place.source, Source::Unknown);
        assert_eq!(two.tiers.len(), 1);
        assert_eq!(two.tiers[0].value, None);
        assert_eq!(live.leaderboard_slots, None);
        let live = live.with_leaderboard(&[]);
        assert_eq!(live.own_place.value, None);
    }

    #[test]
    fn a_leaderboard_with_gaps_or_missing_places_has_no_slot_count() {
        assert_eq!(
            leaderboard_slots(&[slot(1, Some(1), None), slot(2, Some(3), None)]),
            None
        );
        assert_eq!(
            leaderboard_slots(&[slot(1, Some(1), None), slot(2, None, None)]),
            None
        );
        assert_eq!(leaderboard_slots(&[]), None);
        let three_on_one = [
            slot(1, Some(1), None),
            slot(2, Some(1), None),
            slot(3, Some(1), None),
        ];
        assert_eq!(leaderboard_slots(&three_on_one), None);
    }

    #[test]
    fn in_duos_a_team_has_one_place_and_a_tier_per_hero() {
        let lobby = [
            slot(1, Some(2), Some(2)),
            slot(2, Some(2), Some(3)),
            slot(3, Some(1), Some(1)),
            slot(4, Some(1), Some(2)),
            slot(5, Some(3), Some(4)),
            slot(6, Some(3), Some(5)),
        ];
        let live = live_game(&duos(vec![])).with_leaderboard(&lobby);
        assert_eq!(live.leaderboard_slots, Some(3));
        assert_eq!(live.own_place.value, Some(2));
        let first = &live.opponents[0];
        assert_eq!(first.seats, [3, 4]);
        assert_eq!(first.place.value, Some(1));
        let tiers: Vec<_> = first.tiers.iter().map(|t| t.value).collect();
        assert_eq!(tiers, [Some(1), Some(2)]);
    }

    /// A shop record whose newest turn is `turn` (none for 0).
    fn at_turn(mut report: Value, turn: i64) -> Value {
        let turns: Vec<Value> = (1..=turn).map(|t| json!({"turn": t})).collect();
        report["shop"] = json!({"turns": turns});
        report
    }

    fn pool() -> Vec<crate::possible::PoolCard> {
        vec![crate::possible::PoolCard {
            id: "NEUTRAL_1".into(),
            tier: 1,
            tribes: Vec::new(),
            duos_only: false,
        }]
    }

    #[test]
    fn the_game_turn_is_the_newest_shop_turn() {
        assert_eq!(live_game(&solo(vec![], "incomplete")).turn.value, None);
        assert_eq!(
            live_game(&at_turn(solo(vec![], "incomplete"), 0))
                .turn
                .value,
            None
        );
        let live = live_game(&at_turn(solo(vec![], "incomplete"), 5));
        assert_eq!(live.turn.value, Some(5));
        assert_eq!(live.turn.source, Source::Log);
    }

    #[test]
    fn possible_minions_only_on_turns_two_and_three_in_solo_and_duos() {
        let solo_lobby = [
            slot(1, Some(1), Some(1)),
            slot(2, Some(2), Some(1)),
            slot(3, Some(3), Some(2)),
            slot(4, Some(4), Some(1)),
        ];
        let duos_lobby = [
            slot(1, Some(1), Some(1)),
            slot(2, Some(1), Some(1)),
            slot(3, Some(2), Some(2)),
            slot(4, Some(2), Some(1)),
            slot(5, Some(3), Some(1)),
            slot(6, Some(3), Some(1)),
        ];
        let tribes = ["BEAST".to_string()];
        for (name, report, lobby) in [
            ("solo", solo(vec![], "incomplete"), &solo_lobby[..]),
            ("duos", duos(vec![]), &duos_lobby[..]),
        ] {
            for turn in 1..=4 {
                let live = live_game(&at_turn(report.clone(), turn))
                    .with_leaderboard(lobby)
                    .with_entered_tribes(Some(&tribes))
                    .with_possible(&pool());
                let listed = live.opponents.iter().all(|o| {
                    o.possible.len() == o.seats.len()
                        && o.possible.iter().all(|p| p.source == Source::Possible)
                });
                let none = live.opponents.iter().all(|o| o.possible.is_empty());
                if turn == 2 || turn == 3 {
                    assert!(listed, "{name} turn {turn}: every opponent hero listed");
                } else {
                    assert!(none, "{name} turn {turn}: nothing listed");
                }
            }
        }
    }

    #[test]
    fn the_words_of_every_source_mark_travel_with_the_state() {
        let live = live_game(&solo(vec![], "incomplete"));
        let labels: Vec<_> = live.legend.iter().map(|e| (e.source, e.label)).collect();
        assert!(labels.contains(&(Source::Inferred, "inferred")));
        assert!(labels.contains(&(Source::Entered, "entered by you")));
        assert!(labels.contains(&(Source::Unknown, "unknown")));
    }

    #[test]
    fn nothing_in_the_live_state_can_hold_a_name() {
        // Opponents are heroes and seats; the serialized state has no field
        // for a player name, BattleTag or account.
        let mut report = solo(vec![], "incomplete");
        report["player_name"] = json!("Someone#4321");
        report["battletag"] = json!("Someone#4321");
        report["lobby"][1]["player_name"] = json!("Other#1234");
        report["lobby"][1]["game_account_id"] = json!("hi=1 lo=2");
        report["rounds"] = json!([round(1, 2, json!([]), json!({"1": 40, "2": 40}))]);
        report["rounds"][0]["entries"][1]["player_name"] = json!("Other#1234");
        let text = serde_json::to_string(&live_game(&report)).unwrap();
        for forbidden in ["Someone", "Other#", "1234", "4321", "hi=1", "account"] {
            assert!(
                !text.contains(forbidden),
                "{forbidden} reached the live state"
            );
        }
    }
}
