//! Recap of one game (T-202) and the record against each opponent inside it
//! (T-203). The window only renders it.
//!
//! Red line (D-004, D-012): an opponent is a hero and a seat in *this* game,
//! never a name, a BattleTag or an account, and nothing links two games.
//!
//! The log never says who won a combat. It gives every lobby hero's health
//! (health + armor - damage) before and after each combat, so a result is
//! *worked out* from those changes and labelled inferred. When the numbers
//! cannot say, the result is unknown, never a guess.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::provenance::Source;
use crate::stats::{base_hero, counted_place, mode_of, rules, DUOS};

/// A recap never carries more than this many warnings, or this long a text.
const MAX_TEXTS: usize = 50;
const MAX_TEXT_LEN: usize = 500;

const WHY_NO_OPPONENT: &str = "the log does not say who you faced";
const WHY_MANY_OPPONENTS: &str = "the log names more than one possible opponent";
const WHY_NO_HEALTH: &str = "the log has no health for this combat";
const WHY_OUT: &str = "a side was already out of the game";
const WHY_HEALED: &str = "a side gained health, so health changes do not show the result";
const WHY_BOTH_LOST: &str = "both sides lost health, so the log cannot say who won";

const BASIS_SOLO: &str = "The log never says who won a combat. Results are worked out from health: you lost health and the opponent did not (lost), the opponent lost health and you did not (won), neither lost health (tie). If both lost health, or the log lacks a number, the result is unknown.";
const BASIS_DUOS: &str = "The log never says who won a combat. Results are worked out from health: your team lost health and the other team did not (lost), the other team lost health and yours did not (won), neither lost health (tie). If both lost health, or the log lacks a number, the result is unknown. In Duos each round counts once, for your team against the opposing team: both fights of the round together, because a team shares one health pool.";

/// A value and where it comes from: the log when there is a value, unknown
/// when there is none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Sourced<T> {
    pub value: Option<T>,
    pub source: Source,
}

impl<T> Sourced<T> {
    fn log(value: Option<T>) -> Self {
        Sourced {
            source: Source::log_if(value.is_some()),
            value,
        }
    }
}

/// A hero as the log printed it: the card id and, when the log names it, the name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeroRef {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Won,
    Lost,
    Tie,
    Unknown,
}

/// One combat (one round) against an opponent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Combat {
    pub round: i64,
    pub outcome: Outcome,
    /// Inferred for a result worked out from health, unknown for no result.
    pub source: Source,
    /// Why the result is unknown.
    pub reason: Option<&'static str>,
}

/// The combats against one opponent (Solo) or one opposing team (Duos).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpponentRecord {
    /// One hero in Solo, the team's heroes in Duos.
    pub heroes: Vec<HeroRef>,
    /// Lobby seats (player ids 1-8 of this game only).
    pub seats: Vec<i64>,
    /// Where they finished, if the game finished.
    pub final_place: Sourced<i64>,
    pub won: usize,
    pub lost: usize,
    pub tie: usize,
    pub unknown: usize,
    pub combats: Vec<Combat>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HealthPoint {
    /// 0 is before the first combat.
    pub round: i64,
    pub health: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TribeOffer {
    pub tribe: String,
    pub offers: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Recap {
    /// `GT_BATTLEGROUNDS`, `GT_BATTLEGROUNDS_DUO` or something else.
    pub game_type: Sourced<String>,
    pub is_duos: bool,
    /// `ok`, `incomplete`, `unsupported`...
    pub status: Sourced<String>,
    pub hero: Sourced<HeroRef>,
    /// Only meaningful in Duos.
    pub teammate_hero: Sourced<HeroRef>,
    /// Only a finished game with a place inside the mode's range.
    pub place: Sourced<i64>,
    /// Places that count as top half; None when the mode's rules are unknown.
    pub top_half: Option<i64>,
    pub final_health: Sourced<i64>,
    pub rounds_played: Sourced<usize>,
    /// Own health before the first combat and after each combat.
    pub health: Vec<HealthPoint>,
    pub health_source: Source,
    pub record: Vec<OpponentRecord>,
    /// Combats where the log does not say who was faced.
    pub unattributed: Vec<Combat>,
    /// Rounds between the first and the last that the log does not have.
    pub missing_rounds: Vec<i64>,
    /// How results are worked out, and how Duos counts.
    pub record_basis: &'static str,
    /// Tavern offers, most common first. Not the lobby's tribes.
    pub tribes: Vec<TribeOffer>,
    pub tribes_source: Source,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
}

struct LobbySeat {
    pid: i64,
    hero: Option<String>,
    team: Option<i64>,
    final_place: Option<i64>,
}

/// One report, read defensively: the history is a local file that can be old
/// or edited by hand.
struct View<'a> {
    report: &'a Value,
    lobby: Vec<LobbySeat>,
    /// Round number -> round, for the rounds the log has.
    rounds: BTreeMap<i64, &'a Value>,
    duos: bool,
}

impl<'a> View<'a> {
    fn new(report: &'a Value) -> Self {
        let lobby = list(report, "lobby")
            .iter()
            .filter_map(|p| {
                Some(LobbySeat {
                    pid: p.get("player_id")?.as_i64()?,
                    hero: text(p, "hero"),
                    team: p.get("duo_team").and_then(Value::as_i64),
                    final_place: p.get("final_place").and_then(Value::as_i64),
                })
            })
            .collect();
        let rounds = list(report, "rounds")
            .iter()
            .filter_map(|r| Some((r.get("number")?.as_i64()?, r)))
            .collect();
        View {
            report,
            lobby,
            rounds,
            duos: mode_of(report) == DUOS,
        }
    }

    fn seat(&self, pid: i64) -> Option<&LobbySeat> {
        self.lobby.iter().find(|s| s.pid == pid)
    }

    /// Everyone who shares this player's health: the team in Duos, the
    /// player alone otherwise (or when the lobby does not give the team).
    fn side_of(&self, pid: i64) -> Vec<i64> {
        let team = self.duos.then(|| self.seat(pid)?.team).flatten();
        let mut side: BTreeSet<i64> = BTreeSet::from([pid]);
        if let Some(team) = team {
            side.extend(
                self.lobby
                    .iter()
                    .filter(|s| s.team == Some(team))
                    .map(|s| s.pid),
            );
        }
        side.into_iter().collect()
    }

    fn own_side(&self) -> Option<Vec<i64>> {
        let local = self.report.get("local_player_id")?.as_i64()?;
        let mut side = self.side_of(local);
        if self.duos {
            side.extend(
                self.report
                    .get("teammate_player_id")
                    .and_then(Value::as_i64),
            );
            side.sort_unstable();
            side.dedup();
        }
        Some(side)
    }

    /// Health of a side at the end of `round` (0 = before the first combat):
    /// the number its players share. None when the log has none, or when the
    /// players disagree (then it is not one shared number).
    fn health_of(&self, side: &[i64], round: i64) -> Option<i64> {
        let map = if round == 0 {
            self.report.get("start_health")
        } else {
            self.rounds.get(&round)?.get("health_after")
        }?
        .as_object()?;
        let values: BTreeSet<i64> = side
            .iter()
            .filter_map(|pid| map.get(&pid.to_string())?.as_i64())
            .collect();
        match values.len() {
            1 => values.first().copied(),
            _ => None,
        }
    }

    fn hero_ref(&self, id: &str) -> HeroRef {
        let names = self.report.get("card_names").and_then(Value::as_object);
        let name = |key: &str| names?.get(key)?.as_str().map(String::from);
        HeroRef {
            id: id.to_string(),
            name: name(id).or_else(|| name(base_hero(id))),
        }
    }

    fn hero_in(&self, key: &str) -> Sourced<HeroRef> {
        Sourced::log(text(self.report, key).map(|id| self.hero_ref(&id)))
    }
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(String::from)
}

fn texts(value: &Value, key: &str) -> Vec<String> {
    list(value, key)
        .iter()
        .filter_map(Value::as_str)
        .take(MAX_TEXTS)
        .map(|t| t.chars().take(MAX_TEXT_LEN).collect())
        .collect()
}

/// Worked out from health alone (see the module docs).
fn judge(before: [Option<i64>; 2], after: [Option<i64>; 2]) -> (Outcome, Option<&'static str>) {
    let [Some(own_before), Some(opp_before), Some(own_after), Some(opp_after)] =
        [before[0], before[1], after[0], after[1]]
    else {
        return (Outcome::Unknown, Some(WHY_NO_HEALTH));
    };
    // A player already out comes back as a ghost with 0 health: nothing to read.
    if own_before == 0 || opp_before == 0 {
        return (Outcome::Unknown, Some(WHY_OUT));
    }
    // Armor or healing can hide a hit: the numbers would show a tie or a win.
    if own_after > own_before || opp_after > opp_before {
        return (Outcome::Unknown, Some(WHY_HEALED));
    }
    match (own_after < own_before, opp_after < opp_before) {
        (true, false) => (Outcome::Lost, None),
        (false, true) => (Outcome::Won, None),
        (false, false) => (Outcome::Tie, None),
        (true, true) => (Outcome::Unknown, Some(WHY_BOTH_LOST)),
    }
}

fn combat(round: i64, outcome: Outcome, reason: Option<&'static str>) -> Combat {
    Combat {
        round,
        outcome,
        source: if outcome == Outcome::Unknown {
            Source::Unknown
        } else {
            Source::Inferred
        },
        reason,
    }
}

/// The opposing sides a round names, each as the players who share its health.
fn opposing_sides(view: &View, round: &Value) -> Vec<Vec<i64>> {
    let pids: BTreeSet<i64> = list(round, "entries")
        .iter()
        .filter(|e| e.get("side").and_then(Value::as_str) == Some("opponent"))
        .filter_map(|e| e.get("player_id")?.as_i64())
        .collect();
    let sides: BTreeSet<Vec<i64>> = pids.into_iter().map(|pid| view.side_of(pid)).collect();
    sides.into_iter().collect()
}

#[derive(Default)]
struct Records {
    order: Vec<Vec<i64>>,
    combats: BTreeMap<Vec<i64>, Vec<Combat>>,
    unattributed: Vec<Combat>,
}

fn read_combats(view: &View) -> Records {
    let own = view.own_side();
    let mut records = Records::default();
    for (&number, round) in &view.rounds {
        let sides = opposing_sides(view, round);
        let [opp] = sides.as_slice() else {
            let why = if sides.is_empty() {
                WHY_NO_OPPONENT
            } else {
                WHY_MANY_OPPONENTS
            };
            records
                .unattributed
                .push(combat(number, Outcome::Unknown, Some(why)));
            continue;
        };
        let (outcome, reason) = match &own {
            Some(own) => judge(
                [
                    view.health_of(own, number - 1),
                    view.health_of(opp, number - 1),
                ],
                [view.health_of(own, number), view.health_of(opp, number)],
            ),
            None => (Outcome::Unknown, Some(WHY_NO_HEALTH)),
        };
        if !records.combats.contains_key(opp) {
            records.order.push(opp.clone());
        }
        records
            .combats
            .entry(opp.clone())
            .or_default()
            .push(combat(number, outcome, reason));
    }
    records
}

fn count(combats: &[Combat], outcome: Outcome) -> usize {
    combats.iter().filter(|c| c.outcome == outcome).count()
}

fn opponent_record(view: &View, seats: Vec<i64>, combats: Vec<Combat>) -> OpponentRecord {
    let heroes = seats
        .iter()
        .filter_map(|pid| view.seat(*pid)?.hero.as_deref())
        .map(|id| view.hero_ref(id))
        .collect();
    let finished = text(view.report, "status").as_deref() == Some("ok");
    // Teammates share a place; the first seat's is the team's.
    let place = finished
        .then(|| seats.iter().find_map(|pid| view.seat(*pid)?.final_place))
        .flatten();
    OpponentRecord {
        heroes,
        final_place: Sourced::log(place),
        won: count(&combats, Outcome::Won),
        lost: count(&combats, Outcome::Lost),
        tie: count(&combats, Outcome::Tie),
        unknown: count(&combats, Outcome::Unknown),
        seats,
        combats,
    }
}

fn missing_rounds(view: &View) -> Vec<i64> {
    let (Some(&first), Some(&last)) = (view.rounds.keys().next(), view.rounds.keys().next_back())
    else {
        return Vec::new();
    };
    (first.min(1)..=last)
        .filter(|n| !view.rounds.contains_key(n))
        .collect()
}

fn own_health(view: &View) -> Vec<HealthPoint> {
    let start = view
        .own_side()
        .and_then(|own| view.health_of(&own, 0))
        .map(|health| HealthPoint {
            round: 0,
            health: Some(health),
        });
    let rounds = view.rounds.iter().map(|(&round, r)| HealthPoint {
        round,
        health: r.get("own_health_after").and_then(Value::as_i64),
    });
    // A round the log lacks is a point with no health, so the chart shows the gap.
    let gaps = missing_rounds(view).into_iter().map(|round| HealthPoint {
        round,
        health: None,
    });
    let mut points: Vec<HealthPoint> = start.into_iter().chain(rounds).chain(gaps).collect();
    points.sort_by_key(|p| p.round);
    points
}

fn tribes(report: &Value) -> Vec<TribeOffer> {
    let mut offers: Vec<TribeOffer> = report
        .get("shop_tribes")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(tribe, n)| {
            n.as_u64().filter(|&n| n > 0).map(|offers| TribeOffer {
                tribe: tribe.clone(),
                offers,
            })
        })
        .collect();
    offers.sort_by(|a, b| b.offers.cmp(&a.offers).then_with(|| a.tribe.cmp(&b.tribe)));
    offers
}

/// The recap of one game's latest report.
pub fn recap(report: &Value) -> Recap {
    let view = View::new(report);
    let mode = mode_of(report);
    let status = text(report, "status");
    let records = read_combats(&view);
    let tribes = tribes(report);
    Recap {
        game_type: Sourced::log(text(report, "game_type")),
        is_duos: view.duos,
        hero: view.hero_in("hero"),
        teammate_hero: view.hero_in("teammate_hero"),
        place: Sourced::log(counted_place(mode, status.as_deref(), report)),
        top_half: rules(mode).map(|(top, _)| top),
        final_health: Sourced::log(report.get("final_health").and_then(Value::as_i64)),
        rounds_played: Sourced::log((!view.rounds.is_empty()).then_some(view.rounds.len())),
        health: own_health(&view),
        health_source: Source::log_if(!view.rounds.is_empty()),
        record: records
            .order
            .into_iter()
            .map(|seats| {
                let combats = records.combats.get(&seats).cloned().unwrap_or_default();
                opponent_record(&view, seats, combats)
            })
            .collect(),
        unattributed: records.unattributed,
        missing_rounds: missing_rounds(&view),
        record_basis: if view.duos { BASIS_DUOS } else { BASIS_SOLO },
        tribes_source: if tribes.is_empty() {
            Source::Unknown
        } else {
            Source::Inferred
        },
        tribes,
        status: Sourced::log(status),
        warnings: texts(report, "warnings"),
        problems: texts(report, "problems"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::SOLO;
    use serde_json::json;

    /// A Solo game: we are player 1; each round is (opponent seat, own health
    /// after, opponent health after). Everyone starts at 40.
    fn solo(rounds: &[(i64, i64, i64)]) -> Value {
        let mut health = BTreeMap::from([(1, 40), (2, 40), (3, 40), (4, 40)]);
        let mut out = Vec::new();
        for (n, &(opp, own_after, opp_after)) in rounds.iter().enumerate() {
            health.insert(1, own_after);
            health.insert(opp, opp_after);
            out.push(json!({
                "number": n + 1,
                "own_health_after": own_after,
                "health_after": health.iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
                "entries": [
                    {"side": "own", "player_id": 1, "hero": "H_ME", "board": []},
                    {"side": "opponent", "player_id": opp, "hero": format!("H_{opp}"), "board": []},
                ],
                "opponents": [opp],
            }));
        }
        json!({
            "status": "ok", "game_type": SOLO, "local_player_id": 1, "hero": "H_ME",
            "final_place": 2, "final_health": 0,
            "card_names": {"H_ME": "Me Hero", "H_2": "Second"},
            "lobby": [
                {"player_id": 1, "hero": "H_ME", "duo_team": null, "final_place": 2, "final_health": 0},
                {"player_id": 2, "hero": "H_2", "duo_team": null, "final_place": 1, "final_health": 30},
                {"player_id": 3, "hero": "H_3", "duo_team": null, "final_place": 3, "final_health": 20},
                {"player_id": 4, "hero": "H_4", "duo_team": null, "final_place": 4, "final_health": 0},
            ],
            "start_health": {"1": 40, "2": 40, "3": 40, "4": 40},
            "shop_tribes": {"Beast": 4, "Murloc": 9, "Naga": 4},
            "rounds": out,
            "warnings": [], "problems": [],
        })
    }

    fn results(r: &Recap, seat: i64) -> Vec<(i64, Outcome)> {
        r.record
            .iter()
            .find(|o| o.seats == [seat])
            .expect("opponent")
            .combats
            .iter()
            .map(|c| (c.round, c.outcome))
            .collect()
    }

    #[test]
    fn health_changes_give_won_lost_and_tie() {
        // Round 1: we lose health. Round 2: they lose health. Round 3: nobody does.
        let r = recap(&solo(&[(2, 35, 40), (3, 35, 31), (4, 35, 40)]));
        assert_eq!(results(&r, 2), [(1, Outcome::Lost)]);
        assert_eq!(results(&r, 3), [(2, Outcome::Won)]);
        assert_eq!(results(&r, 4), [(3, Outcome::Tie)]);
        assert!(r.unattributed.is_empty() && r.missing_rounds.is_empty());
    }

    #[test]
    fn worked_out_results_are_inferred_never_straight_from_the_log() {
        let r = recap(&solo(&[(2, 35, 40), (3, 35, 31), (4, 35, 40)]));
        for record in &r.record {
            assert!(record.combats.iter().all(|c| c.source == Source::Inferred));
        }
    }

    #[test]
    fn both_sides_losing_health_is_unknown_with_a_reason() {
        let r = recap(&solo(&[(2, 35, 30)]));
        let c = &r.record[0].combats[0];
        assert_eq!(c.outcome, Outcome::Unknown);
        assert_eq!(c.source, Source::Unknown);
        assert_eq!(c.reason, Some(WHY_BOTH_LOST));
    }

    #[test]
    fn an_opponent_faced_twice_is_one_record_with_the_counts() {
        let r = recap(&solo(&[(2, 35, 40), (2, 35, 30), (2, 35, 30)]));
        assert_eq!(r.record.len(), 1);
        let o = &r.record[0];
        assert_eq!((o.won, o.lost, o.tie, o.unknown), (1, 1, 1, 0));
        assert_eq!(o.combats.len(), 3);
    }

    #[test]
    fn an_opponent_is_a_hero_and_a_seat_with_the_place_it_finished() {
        let r = recap(&solo(&[(2, 35, 40)]));
        let o = &r.record[0];
        assert_eq!(o.seats, [2]);
        assert_eq!(
            o.heroes,
            [HeroRef {
                id: "H_2".into(),
                name: Some("Second".into())
            }]
        );
        assert_eq!(o.final_place.value, Some(1));
        assert_eq!(o.final_place.source, Source::Log);
    }

    #[test]
    fn a_missing_round_is_listed_and_the_next_combat_is_unknown() {
        let mut report = solo(&[(2, 35, 40), (3, 35, 31), (4, 30, 40)]);
        report["rounds"].as_array_mut().unwrap().remove(1); // no round 2
        let r = recap(&report);
        assert_eq!(r.missing_rounds, [2]);
        assert_eq!(results(&r, 2), [(1, Outcome::Lost)]);
        // Round 3 needs the health after round 2, which the log lacks.
        assert_eq!(results(&r, 4), [(3, Outcome::Unknown)]);
        assert_eq!(r.rounds_played.value, Some(2));
    }

    #[test]
    fn a_ghost_of_a_player_already_out_is_unknown_not_a_tie() {
        // Round 1 knocks player 2 out (40 -> 0); in round 2 their ghost is faced.
        let r = recap(&solo(&[(2, 40, 0), (2, 40, 0)]));
        assert_eq!(results(&r, 2), [(1, Outcome::Won), (2, Outcome::Unknown)]);
        assert_eq!(r.record[0].combats[1].reason, Some(WHY_OUT));
    }

    #[test]
    fn a_side_that_gained_health_is_unknown() {
        let r = recap(&solo(&[(2, 45, 40), (3, 45, 45)]));
        assert_eq!(results(&r, 2), [(1, Outcome::Unknown)]);
        assert_eq!(results(&r, 3), [(2, Outcome::Unknown)]);
        assert_eq!(r.record[0].combats[0].reason, Some(WHY_HEALED));
    }

    #[test]
    fn a_missing_round_shows_as_a_gap_in_the_health() {
        let mut report = solo(&[(2, 35, 40), (3, 30, 40), (4, 25, 40)]);
        report["rounds"].as_array_mut().unwrap().remove(1);
        let r = recap(&report);
        let health: Vec<_> = r.health.iter().map(|h| (h.round, h.health)).collect();
        assert_eq!(
            health,
            [(0, Some(40)), (1, Some(35)), (2, None), (3, Some(25))]
        );
    }

    #[test]
    fn the_first_combat_needs_the_health_before_it() {
        let mut report = solo(&[(2, 35, 40)]);
        report.as_object_mut().unwrap().remove("start_health");
        let r = recap(&report);
        assert_eq!(results(&r, 2), [(1, Outcome::Unknown)]);
        assert_eq!(r.record[0].combats[0].reason, Some(WHY_NO_HEALTH));
    }

    #[test]
    fn a_game_saved_before_lobby_health_existed_is_all_unknown_not_guessed() {
        let mut report = solo(&[(2, 35, 40), (3, 35, 31)]);
        report.as_object_mut().unwrap().remove("start_health");
        for round in report["rounds"].as_array_mut().unwrap() {
            round.as_object_mut().unwrap().remove("health_after");
        }
        let r = recap(&report);
        assert!(r
            .record
            .iter()
            .flat_map(|o| &o.combats)
            .all(|c| c.outcome == Outcome::Unknown));
        // What the old record does have still shows.
        assert_eq!(r.health.len(), 2);
        assert_eq!(r.health[0].health, Some(35));
    }

    #[test]
    fn a_combat_with_no_opponent_named_is_not_attributed_to_anyone() {
        let mut report = solo(&[(2, 35, 40), (3, 35, 31)]);
        report["rounds"][0]["entries"] = json!([{"side": "own", "player_id": 1, "hero": "H_ME"}]);
        report["rounds"][1]["entries"] = json!([
            {"side": "opponent", "player_id": 2, "hero": "H_2"},
            {"side": "opponent", "player_id": 3, "hero": "H_3"},
        ]);
        let r = recap(&report);
        assert!(r.record.is_empty());
        let reasons: Vec<_> = r.unattributed.iter().map(|c| c.reason).collect();
        assert_eq!(reasons, [Some(WHY_NO_OPPONENT), Some(WHY_MANY_OPPONENTS)]);
        assert!(r.unattributed.iter().all(|c| c.outcome == Outcome::Unknown));
    }

    /// A Duos game: we are 1 (team 10) with 2; opponents 3 and 4 (team 20),
    /// 5 and 6 (team 30). Each round is (opposing seats, team health after,
    /// opposing team health after); a team shares one health number.
    fn duos(rounds: &[([i64; 2], i64, i64)]) -> Value {
        let mut health: BTreeMap<i64, i64> = (1..=6).map(|p| (p, 50)).collect();
        let mut out = Vec::new();
        for (n, &(opp, mine, theirs)) in rounds.iter().enumerate() {
            for p in [1, 2] {
                health.insert(p, mine);
            }
            for p in opp {
                health.insert(p, theirs);
            }
            out.push(json!({
                "number": n + 1,
                "own_health_after": mine,
                "health_after": health.iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
                "entries": [
                    {"side": "own", "player_id": 1, "hero": "H_1", "board": []},
                    {"side": "opponent", "player_id": opp[0], "hero": format!("H_{}", opp[0]), "board": []},
                    {"side": "own", "player_id": 2, "hero": "H_2", "board": null},
                    {"side": "opponent", "player_id": opp[1], "hero": format!("H_{}", opp[1]), "board": null},
                ],
            }));
        }
        let seat = |p: i64, team: i64, place: i64| json!({"player_id": p, "hero": format!("H_{p}"), "duo_team": team, "final_place": place, "final_health": 0});
        json!({
            "status": "ok", "game_type": DUOS, "local_player_id": 1, "teammate_player_id": 2,
            "hero": "H_1", "teammate_hero": "H_2", "final_place": 2, "final_health": 0,
            "card_names": {},
            "lobby": [seat(1, 10, 2), seat(2, 10, 2), seat(3, 20, 1), seat(4, 20, 1), seat(5, 30, 3), seat(6, 30, 3)],
            "start_health": {"1": 50, "2": 50, "3": 50, "4": 50, "5": 50, "6": 50},
            "rounds": out, "warnings": [], "problems": [],
        })
    }

    #[test]
    fn duos_counts_one_result_per_round_for_the_team_against_the_team() {
        let r = recap(&duos(&[
            ([3, 4], 45, 50),
            ([5, 6], 45, 40),
            ([3, 4], 45, 50),
        ]));
        assert!(r.is_duos);
        assert_eq!(r.record.len(), 2);
        let team20 = &r.record[0];
        assert_eq!(team20.seats, [3, 4]);
        assert_eq!(
            team20
                .heroes
                .iter()
                .map(|h| h.id.as_str())
                .collect::<Vec<_>>(),
            ["H_3", "H_4"]
        );
        assert_eq!((team20.lost, team20.tie, team20.won), (1, 1, 0));
        assert_eq!(team20.final_place.value, Some(1));
        let team30 = &r.record[1];
        assert_eq!(team30.seats, [5, 6]);
        assert_eq!((team30.won, team30.lost), (1, 0));
        assert_eq!(r.record_basis, BASIS_DUOS);
    }

    #[test]
    fn duos_with_one_opposing_player_seen_still_finds_the_team() {
        let mut report = duos(&[([3, 4], 45, 50)]);
        let entries = report["rounds"][0]["entries"].as_array_mut().unwrap();
        entries.remove(3); // the second opponent's leg is not in the log
        let r = recap(&report);
        assert_eq!(r.record[0].seats, [3, 4]);
        assert_eq!(r.record[0].lost, 1);
    }

    #[test]
    fn duos_teammates_that_disagree_on_health_give_unknown() {
        let mut report = duos(&[([3, 4], 45, 50)]);
        report["rounds"][0]["health_after"]["2"] = json!(44);
        let r = recap(&report);
        assert_eq!(r.record[0].combats[0].outcome, Outcome::Unknown);
    }

    #[test]
    fn duos_without_teams_in_the_lobby_cannot_group_two_opponents() {
        let mut report = duos(&[([3, 4], 45, 50)]);
        for seat in report["lobby"].as_array_mut().unwrap() {
            seat["duo_team"] = Value::Null;
        }
        let r = recap(&report);
        assert!(r.record.is_empty());
        assert_eq!(r.unattributed[0].reason, Some(WHY_MANY_OPPONENTS));
    }

    #[test]
    fn solo_basis_does_not_talk_about_teams() {
        assert_eq!(recap(&solo(&[])).record_basis, BASIS_SOLO);
        assert!(!BASIS_SOLO.contains("team"));
    }

    #[test]
    fn a_finished_game_has_hero_place_health_and_rounds_from_the_log() {
        let r = recap(&solo(&[(2, 35, 40), (3, 30, 40)]));
        assert_eq!(
            r.hero.value.as_ref().unwrap().name.as_deref(),
            Some("Me Hero")
        );
        assert_eq!(r.hero.source, Source::Log);
        assert_eq!((r.place.value, r.place.source), (Some(2), Source::Log));
        assert_eq!(r.top_half, Some(4));
        assert_eq!(r.final_health.value, Some(0));
        assert_eq!(r.rounds_played.value, Some(2));
        let health: Vec<_> = r.health.iter().map(|h| (h.round, h.health)).collect();
        assert_eq!(health, [(0, Some(40)), (1, Some(35)), (2, Some(30))]);
        assert_eq!(r.health_source, Source::Log);
    }

    #[test]
    fn an_unfinished_game_has_no_place_and_says_so() {
        let mut report = solo(&[(2, 35, 40)]);
        report["status"] = json!("incomplete");
        report["final_place"] = json!(5);
        let r = recap(&report);
        assert_eq!((r.place.value, r.place.source), (None, Source::Unknown));
        assert_eq!(r.status.value.as_deref(), Some("incomplete"));
        // Where the opponents finished is unknown too.
        assert_eq!(r.record[0].final_place.value, None);
    }

    #[test]
    fn tribes_are_inferred_most_offered_first() {
        let r = recap(&solo(&[]));
        let tribes: Vec<_> = r
            .tribes
            .iter()
            .map(|t| (t.tribe.as_str(), t.offers))
            .collect();
        assert_eq!(tribes, [("Murloc", 9), ("Beast", 4), ("Naga", 4)]);
        assert_eq!(r.tribes_source, Source::Inferred);
        let mut none = solo(&[]);
        none["shop_tribes"] = json!({});
        assert_eq!(recap(&none).tribes_source, Source::Unknown);
    }

    #[test]
    fn warnings_and_problems_come_through_bounded() {
        let mut report = solo(&[]);
        report["warnings"] = json!(["a".repeat(MAX_TEXT_LEN + 10), 3, "b"]);
        report["problems"] = json!((0..MAX_TEXTS + 5)
            .map(|i| i.to_string())
            .collect::<Vec<_>>());
        let r = recap(&report);
        assert_eq!(r.warnings.len(), 2, "non-text entries are dropped");
        assert_eq!(r.warnings[0].chars().count(), MAX_TEXT_LEN);
        assert_eq!(r.problems.len(), MAX_TEXTS);
    }

    #[test]
    fn nothing_but_known_fields_reaches_the_recap() {
        let mut report = solo(&[(2, 35, 40)]);
        report["player_name"] = json!("Someone#1234");
        report["rounds"][0]["entries"][1]["battletag"] = json!("Other#9999");
        report["lobby"][1]["account"] = json!("12345");
        let out = serde_json::to_string(&recap(&report)).unwrap();
        for leak in ["Someone", "Other#", "12345", "battletag", "account"] {
            assert!(!out.contains(leak), "{leak} leaked: {out}");
        }
    }

    #[test]
    fn a_record_that_is_not_a_game_gives_an_empty_recap() {
        for report in [
            json!(null),
            json!("x"),
            json!({}),
            json!({"rounds": 3, "lobby": {}}),
        ] {
            let r = recap(&report);
            assert!(r.record.is_empty() && r.health.is_empty());
            assert_eq!(r.hero.source, Source::Unknown);
            assert_eq!(r.rounds_played.source, Source::Unknown);
            assert_eq!(r.health_source, Source::Unknown);
        }
    }

    #[test]
    fn hand_edited_rounds_do_not_break_the_recap() {
        let report = json!({
            "status": "ok", "game_type": SOLO, "local_player_id": 1,
            "rounds": [
                {"number": "x", "entries": 4},
                {"number": 1, "entries": [{"side": "opponent", "player_id": "7"}], "health_after": []},
                {"number": 2, "own_health_after": "z", "health_after": {"1": "a"}},
            ],
        });
        let r = recap(&report);
        assert_eq!(r.rounds_played.value, Some(2));
        assert!(r.record.is_empty());
        assert_eq!(r.unattributed.len(), 2);
    }
}
