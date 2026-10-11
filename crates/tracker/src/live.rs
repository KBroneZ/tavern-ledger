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

use crate::game_values::{GameTribes, ReaderStatus};
use crate::live_seats::{seats_shown, LiveSeat};
use crate::odds::LiveOdds;
use crate::possible::{LobbyTribes, Possible};
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
    /// Keywords as it entered the combat (T-208); None when the record is
    /// older than parser revision 5 (not recorded), never "none".
    pub keywords: Option<Vec<String>>,
    /// Dark Gifts as gift card ids (T-528, parser revision 9); empty when
    /// none or when the record is older.
    pub gifts: Vec<String>,
    /// The game marks a Dark Gift the log does not name.
    pub gift_unknown: bool,
}

/// A board that entered play in a combat, with the combat it was seen in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveBoard {
    /// The seat (player id) whose board it is.
    pub seat: i64,
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
    /// Minions each hero could have (T-307, D-049, D-050), added by the app
    /// from the card data on the turns the rule lists; empty otherwise.
    pub possible: Vec<Possible>,
    /// Each hero's triples and trinkets as the game's leaderboard shows them
    /// now (T-312), in the order of `seats`; empty when not recorded.
    pub shown: Vec<LiveSeat>,
    /// Log when the report has the leaderboard's values, else unknown.
    pub shown_source: Source,
    /// The game marks this opponent (a team in Duos) as the next one on its
    /// leaderboard (T-313), from the log.
    pub next: bool,
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
    /// The five tribes read from the game (D-063), when the user turned that
    /// on, with the reader's status; empty otherwise. Never merged with the
    /// other tribes, never saved in the report.
    pub game_tribes: GameTribes,
    /// The game build from the log, for the reader's checked builds.
    pub build: Option<i64>,
    /// The lobby's tribes as far as the app knows them (from the game, else
    /// entered, else confirmed by the tavern with the card data, else seen in
    /// the tavern), added by the app with the possible minions (D-050).
    pub lobby_tribes: LobbyTribes,
    /// Every pool minion Bob offered so far (card ids, repeats kept), from
    /// the shop record; read by the app with the card data, not shown.
    #[serde(skip)]
    pub offered: Vec<String>,
    pub opponents: Vec<LiveOpponent>,
    pub record_basis: &'static str,
    /// The words of every source mark, from the app (T-109): the overlay shows
    /// them and chooses none.
    pub legend: Vec<LegendEntry>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
    /// The most damage a lost combat can deal now, which the game shows the
    /// player ("Max damage: 15", T-315); None when no cap applies or the
    /// log has none. Live only, from the game entity.
    pub damage_cap: Option<i64>,
    /// The combat odds against the next opponent (T-605), added by the app
    /// from its own thread; None when switched off or not worked out yet.
    pub odds: Option<LiveOdds>,
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
    /// Adds the tribes the reader got from the game for this game (D-063),
    /// with its status. Checked here; checked against the tavern in
    /// [`LiveGame::with_possible`].
    pub fn with_game_tribes(mut self, tribes: Option<&[String]>, status: ReaderStatus) -> Self {
        self.game_tribes = GameTribes::new(tribes, status, self.build);
        self
    }
}

impl LiveGame {
    /// Adds the combat damage cap the game shows now (T-315); none once the
    /// game is over.
    pub fn with_damage_cap(mut self, cap: Option<i64>) -> Self {
        self.damage_cap = cap.filter(|_| self.phase == Phase::Playing);
        self
    }

    /// Adds the combat odds (T-605) while the game is on, only when they
    /// are against the opponent marked next now (odds worked out for the
    /// previous one never show under the new one's line).
    pub fn with_odds(mut self, odds: Option<LiveOdds>) -> Self {
        let next: Vec<i64> = self
            .opponents
            .iter()
            .filter(|o| o.next)
            .flat_map(|o| o.seats.iter().copied())
            .collect();
        let playing = self.phase == Phase::Playing;
        self.odds = odds.filter(|o| playing && o.seat.is_some_and(|s| next.contains(&s)));
        self
    }

    /// Marks the opponent holding `seat`, the one the game marks as next on
    /// its leaderboard (T-313); in Duos that marks the whole opposing team.
    /// Our own seats are never marked.
    pub fn with_next_opponent(mut self, seat: Option<i64>) -> Self {
        let seat = seat.filter(|s| !self.own_seats.contains(s));
        for opponent in &mut self.opponents {
            opponent.next = seat.is_some_and(|s| opponent.seats.contains(&s));
        }
        self
    }

    /// Adds the card each trinket is, from the card data (the log gives a
    /// database id); a trinket the card data does not know keeps None.
    pub fn with_trinket_cards(mut self, card_of: impl Fn(i64) -> Option<String>) -> Self {
        for trinket in self
            .opponents
            .iter_mut()
            .flat_map(|o| o.shown.iter_mut())
            .flat_map(|s| s.trinkets.iter_mut())
        {
            trinket.card_id = card_of(trinket.dbf_id);
        }
        self
    }

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
            .into_iter()
            .map(|o| with_shown(o, report))
            .collect()
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
        game_tribes: GameTribes::default(),
        build: report.get("build").and_then(Value::as_i64),
        lobby_tribes: LobbyTribes::default(),
        offered: offered(report),
        opponents,
        record_basis: recap.record_basis,
        legend: legend(),
        warnings: texts(report, "warnings"),
        problems: texts(report, "problems"),
        damage_cap: None,
        odds: None,
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

/// The card ids of every offer in the report's shop record.
fn offered(report: &Value) -> Vec<String> {
    list(report.get("shop").unwrap_or(&Value::Null), "turns")
        .iter()
        .flat_map(|turn| list(turn, "offers"))
        .filter_map(|offer| text(offer, "card_id"))
        .collect()
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
        shown: Vec::new(),
        shown_source: Source::Unknown,
        next: false,
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

/// The leaderboard's triples and trinkets of the opponent's heroes (T-312).
fn with_shown(mut opponent: LiveOpponent, report: &Value) -> LiveOpponent {
    match seats_shown(report, &opponent.seats) {
        Some(shown) => {
            opponent.shown = shown;
            opponent.shown_source = Source::Log;
        }
        None => {
            opponent.shown = Vec::new();
            opponent.shown_source = Source::Unknown;
        }
    }
    opponent
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
                    seat: pid,
                    hero: text(e, "hero").map(|id| view.hero_ref(&id)),
                    round: n,
                    minions: minions_of(minions),
                })
            })
    })
}

/// The parser writes at most this many keywords per minion.
const MAX_KEYWORDS: usize = 8;

pub(crate) fn minions_of(board: &[Value]) -> Vec<LiveMinion> {
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
                    keywords: m.get("keywords").and_then(Value::as_array).map(|k| {
                        k.iter()
                            .filter_map(Value::as_str)
                            .take(MAX_KEYWORDS)
                            .map(String::from)
                            .collect()
                    }),
                    gifts: m
                        .get("gifts")
                        .and_then(Value::as_array)
                        .map(|g| {
                            g.iter()
                                .filter_map(Value::as_str)
                                .take(bg_parser::gifts::MAX_GIFTS)
                                .map(String::from)
                                .collect()
                        })
                        .unwrap_or_default(),
                    gift_unknown: m.get("gift_unknown").and_then(Value::as_bool) == Some(true),
                },
            )
        })
        .collect();
    placed.sort_by_key(|(position, _)| *position);
    placed.into_iter().map(|(_, minion)| minion).collect()
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;
