//! Validation harness (T-603): plays every logged combat of the player's own
//! history through the engine and scores the real result against the odds.
//!
//! Inputs, read locally and never committed (fixtures rule): the app's
//! history (`games.jsonl`, one saved game per line) and HearthstoneJSON's
//! `cards.json` for card facts the board does not show (tier, tribes, the
//! card's own stats). Output: counts, shares and card ids only, never a
//! player name, BattleTag or account id; hero ids and player ids are not
//! printed either.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::time::Instant;

use serde_json::Value;

use crate::cards::{self, Fidelity};
use crate::combat::CombatResult;
use crate::harness_duos::{duos_from_saved, team_of, DuosCombat};
use crate::harness_logs::LogCombat;
use crate::harness_saved::{saved_combat, Saved};
use crate::model::{BaseCard, Keywords, Minion, Team, Tribes, Warband};
use crate::score::{score, Prediction, Report};
use crate::trials::{simulate, Coverage, Options, Verdict};

/// A combat whose real result got less than this is listed as a miss.
const MISS: f64 = 0.5;

pub use crate::saved::CardFacts;

pub type CardDb = HashMap<String, CardFacts>;

pub fn load_cards(json: &str) -> Result<CardDb, String> {
    let all: Value = serde_json::from_str(json).map_err(|e| format!("cards.json: {e}"))?;
    let list = all.as_array().ok_or("cards.json: expected an array")?;
    let mut db = CardDb::new();
    for c in list {
        let Some(id) = c.get("id").and_then(Value::as_str) else {
            continue;
        };
        if c.get("type").and_then(Value::as_str) != Some("MINION") {
            continue;
        }
        let int = |k: &str| c.get(k).and_then(Value::as_i64);
        let strings = |k: &str| -> Vec<String> {
            c.get(k)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default()
        };
        let mut races = strings("races");
        if let Some(r) = c.get("race").and_then(Value::as_str) {
            races.push(r.to_string());
        }
        let tribes = races
            .iter()
            .filter_map(|r| Tribes::from_name(r))
            .fold(Tribes::NONE, Tribes::union);
        let keywords = strings("mechanics")
            .iter()
            .filter_map(|m| Keywords::from_name(m))
            .fold(Keywords::NONE, Keywords::union);
        let base = match (int("attack"), int("health")) {
            (Some(a), Some(h)) => Some(BaseCard {
                attack: a as i32,
                health: h as i32,
                keywords,
            }),
            _ => None,
        };
        let tier = int("techLevel").and_then(|t| u8::try_from(t).ok());
        db.insert(id.to_string(), CardFacts { tier, tribes, base });
    }
    if db.is_empty() {
        return Err("cards.json: no minions".into());
    }
    Ok(db)
}

/// One Solo combat the log has from start to end, with both boards and the
/// game's own result.
#[derive(Clone, Debug, PartialEq)]
pub struct LoggedCombat {
    /// Position in the history (1-based game, round number): for reading the
    /// report against one's own history, not an id.
    pub game: usize,
    pub round: i64,
    /// The opponent's seat in this game (a lobby player id, 1-8), to find
    /// the board of theirs the player saw before (the overlay's input).
    pub opponent_seat: Option<i64>,
    pub player: Warband,
    pub opponent: Warband,
    pub result: CombatResult,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    pub games: usize,
    pub solo_games: usize,
    pub duos_games: usize,
    pub combats: Vec<LoggedCombat>,
    /// Solo rounds without both boards or without a result.
    pub solo_rounds_skipped: usize,
    /// Duos rounds, and those with every board and a result.
    pub duos_rounds: usize,
    pub duos_rounds_complete: usize,
    /// The Duos combats with each team's boards (T-608).
    pub duos: Vec<DuosCombat>,
    /// Every minion seen on a board entering combat (Solo and Duos): card id
    /// -> appearances. For the coverage list.
    pub card_counts: BTreeMap<String, u32>,
    pub bad_lines: usize,
    /// Combats with a saved replay rebuilt for the attack-by-attack replay
    /// (T-609), and the saved replays that could not be.
    pub saved: Vec<LogCombat>,
    pub saved_unreadable: usize,
    /// The latest record of each game, for why each result differed
    /// (T-346); empty when read from the logs.
    pub reports: Vec<Value>,
}

pub(crate) fn minion(v: &Value, db: &CardDb) -> Option<Minion> {
    crate::saved::minion(v, &|id| db.get(id).copied())
}

pub(crate) fn board(entry: &Value, db: &CardDb) -> Option<Warband> {
    crate::saved::board(entry, &|id| db.get(id).copied())
}

pub(crate) use crate::saved::result;

/// Reads the history: the last record of each game wins, as in the app.
pub fn load_history(jsonl: &str, db: &CardDb) -> History {
    let mut latest: Vec<((String, u64), Value)> = Vec::new();
    let mut h = History::default();
    for line in jsonl.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            h.bad_lines += 1;
            continue;
        };
        let session = v.get("session").and_then(Value::as_str);
        let index = v.get("index").and_then(Value::as_u64);
        let (Some(session), Some(index), Some(report)) = (session, index, v.get("report")) else {
            h.bad_lines += 1;
            continue;
        };
        let key = (session.to_string(), index);
        let report = report.clone();
        match latest.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = report,
            None => latest.push((key, report)),
        }
    }
    h.games = latest.len();
    for (game, (_, report)) in latest.iter().enumerate() {
        let solo = match report.get("game_type").and_then(Value::as_str) {
            Some("GT_BATTLEGROUNDS") => true,
            Some("GT_BATTLEGROUNDS_DUO") => false,
            _ => continue,
        };
        let duos = if solo {
            h.solo_games += 1;
            Vec::new()
        } else {
            h.duos_games += 1;
            duos_from_saved(report, game + 1, db)
        };
        let rounds = report.get("rounds").and_then(Value::as_array);
        for round in rounds.into_iter().flatten() {
            let number = round.get("number").and_then(Value::as_i64).unwrap_or(0);
            let teams = if solo {
                solo_teams(round, db)
            } else {
                duos.iter()
                    .find(|c| c.round == number)
                    .map(|c| (team_of(&c.own), team_of(&c.opponent)))
            };
            match saved_combat(round, game + 1, !solo, teams, db) {
                Saved::None => {}
                Saved::Unreadable => h.saved_unreadable += 1,
                Saved::Combat(c) => h.saved.push(*c),
            }
            let entries: Vec<&Value> = round
                .get("entries")
                .and_then(Value::as_array)
                .map(|e| e.iter().collect())
                .unwrap_or_default();
            for e in &entries {
                for m in e
                    .get("board")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(id) = m.get("card_id").and_then(Value::as_str) {
                        *h.card_counts.entry(id.to_string()).or_insert(0) += 1;
                    }
                }
            }
            if !solo {
                h.duos_rounds += 1;
                let all_known = entries.iter().all(|e| board(e, db).is_some());
                if all_known && result(round).is_some() {
                    h.duos_rounds_complete += 1;
                }
                continue;
            }
            let side = |s: &str| -> Vec<&Value> {
                entries
                    .iter()
                    .copied()
                    .filter(|e| e.get("side").and_then(Value::as_str) == Some(s))
                    .collect()
            };
            let (own, opp) = (side("own"), side("opponent"));
            let combat = match (own.as_slice(), opp.as_slice(), result(round)) {
                ([o], [p], Some(r)) => board(o, db).zip(board(p, db)).map(|(a, b)| (a, b, r)),
                _ => None,
            };
            match combat {
                Some((player, opponent, result)) => h.combats.push(LoggedCombat {
                    game: game + 1,
                    round: round.get("number").and_then(Value::as_i64).unwrap_or(0),
                    opponent_seat: opp.first().and_then(|e| e.get("player_id")?.as_i64()),
                    player,
                    opponent,
                    result,
                }),
                None => h.solo_rounds_skipped += 1,
            }
        }
        h.duos.extend(duos);
    }
    h.reports = latest.into_iter().map(|(_, report)| report).collect();
    h
}

/// A Solo round's two boards, one entry a side.
fn solo_teams(round: &Value, db: &CardDb) -> Option<(Team, Team)> {
    let entries = round.get("entries")?.as_array()?;
    let side = |s: &str| -> Vec<&Value> {
        entries
            .iter()
            .filter(|e| e.get("side").and_then(Value::as_str) == Some(s))
            .collect()
    };
    match (side("own").as_slice(), side("opponent").as_slice()) {
        ([o], [p]) => Some((Team::solo(board(o, db)?), Team::solo(board(p, db)?))),
        _ => None,
    }
}

/// How well the engine knows a combat's cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Exact,
    Approximate,
    NotModelled,
}

impl Class {
    fn of(c: &Coverage) -> Class {
        if !c.not_modelled.is_empty() {
            Class::NotModelled
        } else if !c.approximate.is_empty() {
            Class::Approximate
        } else {
            Class::Exact
        }
    }

    fn label(self) -> &'static str {
        match self {
            Class::Exact => "exact",
            Class::Approximate => "approximate",
            Class::NotModelled => "not modelled",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scored {
    pub combat: LoggedCombat,
    pub prediction: Prediction,
    pub class: Class,
    pub coverage: Coverage,
    pub micros: u128,
}

/// Every logged combat played, or the reason the engine refused it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub scored: Vec<Scored>,
    /// Combats the engine refused, by reason (`BoardError`), never dropped
    /// without a count.
    pub rejected: BTreeMap<String, u32>,
}

pub fn run(history: &History, options: &Options) -> Run {
    let opts = Options {
        allow_unmodelled: true,
        ..*options
    };
    let mut out = Run::default();
    for c in &history.combats {
        let start = Instant::now();
        let verdict = simulate(&c.player, &c.opponent, &opts);
        let micros = start.elapsed().as_micros();
        let o = match verdict {
            Ok(Verdict::Odds(o)) => o,
            // `allow_unmodelled` is on, so this does not happen; counted if
            // it ever does.
            Ok(Verdict::NotModelled(_)) => {
                *out.rejected.entry("not modelled".into()).or_insert(0) += 1;
                continue;
            }
            Err(e) => {
                *out.rejected.entry(e.to_string()).or_insert(0) += 1;
                continue;
            }
        };
        out.scored.push(Scored {
            prediction: Prediction {
                won: o.share(CombatResult::Won),
                tied: o.share(CombatResult::Tie),
                lost: o.share(CombatResult::Lost),
            },
            class: Class::of(&o.coverage),
            coverage: o.coverage,
            combat: c.clone(),
            micros,
        });
    }
    out
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn report_line(out: &mut String, label: &str, r: &Report) {
    let _ = writeln!(
        out,
        "| {label} | {} | {} | {:.3} | {} |",
        r.combats,
        pct(r.mean_p_actual),
        r.brier,
        if r.combats > 0 {
            format!("{} of {}", r.top_pick_hits, r.combats)
        } else {
            "-".into()
        }
    );
}

fn result_word(r: CombatResult) -> &'static str {
    match r {
        CombatResult::Won => "won",
        CombatResult::Tie => "tie",
        CombatResult::Lost => "lost",
    }
}

/// The accuracy report, in Markdown: counts, shares and card ids only.
pub fn render(history: &History, run: &Run, options: &Options) -> String {
    let scored = &run.scored;
    let mut out = String::new();
    let _ = writeln!(out, "# Combat simulator accuracy\n");
    let _ = writeln!(
        out,
        "History: {} games ({} Solo, {} Duos). Solo combats with both boards and a result: {} \
         (skipped {}). Duos rounds: {} ({} with every board and a result), scored apart below \
         (T-608).\n",
        history.games,
        history.solo_games,
        history.duos_games,
        history.combats.len(),
        history.solo_rounds_skipped,
        history.duos_rounds,
        history.duos_rounds_complete,
    );
    let _ = writeln!(
        out,
        "{} trials per combat, seed {}, boards taken at the first attack (start of combat not \
         replayed). Unknown cards played as plain minions for this measurement only.\n",
        options.trials, options.seed
    );
    let rejected: u32 = run.rejected.values().sum();
    if rejected > 0 {
        let why: Vec<String> = run
            .rejected
            .iter()
            .map(|(reason, n)| format!("{n} ({reason})"))
            .collect();
        let _ = writeln!(
            out,
            "Refused by the engine's board check, not scored: {}.
",
            why.join(", ")
        );
    }
    let _ = writeln!(
        out,
        "| Combats | n | Mean p(real result) | Brier | Most likely = real |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    let all: Vec<(Prediction, CombatResult)> = scored
        .iter()
        .map(|s| (s.prediction, s.combat.result))
        .collect();
    report_line(&mut out, "all", &score(&all, 5));
    for class in [Class::Exact, Class::Approximate, Class::NotModelled] {
        let items: Vec<(Prediction, CombatResult)> = scored
            .iter()
            .filter(|s| s.class == class)
            .map(|s| (s.prediction, s.combat.result))
            .collect();
        report_line(&mut out, class.label(), &score(&items, 5));
    }
    let _ = writeln!(out, "\nCalibration (all combats, every result of each):\n");
    let _ = writeln!(out, "| Said | Predictions | Mean said | Happened |");
    let _ = writeln!(out, "|---|---|---|---|");
    for b in score(&all, 5).buckets {
        let _ = writeln!(
            out,
            "| {}-{} | {} | {} | {} |",
            pct(b.lo),
            pct(b.hi),
            b.count,
            if b.count > 0 {
                pct(b.mean_predicted)
            } else {
                "-".into()
            },
            if b.count > 0 {
                pct(b.observed)
            } else {
                "-".into()
            },
        );
    }
    let _ = writeln!(
        out,
        "\nMisses (real result given under 50%), with what the engine does not know:\n"
    );
    let mut misses: Vec<&Scored> = scored
        .iter()
        .filter(|s| s.prediction.of(s.combat.result) < MISS)
        .collect();
    misses.sort_by_key(|s| (s.combat.game, s.combat.round));
    if misses.is_empty() {
        let _ = writeln!(out, "None.");
    }
    for s in misses {
        let gaps = if !s.coverage.not_modelled.is_empty() {
            format!("not modelled: {}", s.coverage.not_modelled.join(", "))
        } else if !s.coverage.approximate.is_empty() {
            let ids: Vec<&str> = s
                .coverage
                .approximate
                .iter()
                .map(|(id, _)| id.as_str())
                .collect();
            format!("approximate: {}", ids.join(", "))
        } else {
            "every card exact: a rule differs (needs the combat's attack log)".into()
        };
        let _ = writeln!(
            out,
            "- game {} round {}: real {}, said won {} / tie {} / lost {}; {}",
            s.combat.game,
            s.combat.round,
            result_word(s.combat.result),
            pct(s.prediction.won),
            pct(s.prediction.tied),
            pct(s.prediction.lost),
            gaps
        );
    }
    let times: Vec<u128> = scored.iter().map(|s| s.micros).collect();
    if let (Some(max), false) = (times.iter().max(), times.is_empty()) {
        let mean = times.iter().sum::<u128>() / times.len() as u128;
        let _ = writeln!(
            out,
            "\nSpeed: {} trials per combat took {:.1} ms on average, {:.1} ms at most.",
            options.trials,
            mean as f64 / 1000.0,
            *max as f64 / 1000.0
        );
    }
    render_coverage(&mut out, history);
    out
}

fn render_coverage(out: &mut String, history: &History) {
    let mut by_class: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
    let mut missing: Vec<(&String, u32)> = Vec::new();
    for (id, n) in &history.card_counts {
        let class = match cards::rules(id) {
            None => {
                missing.push((id, *n));
                "not modelled"
            }
            Some(r) => match r.fidelity {
                Fidelity::Exact if r.triggers.is_empty() => "no combat effect",
                Fidelity::Exact => "modelled",
                Fidelity::Approximate(_) => "approximate",
            },
        };
        let e = by_class.entry(class).or_insert((0, 0));
        e.0 += 1;
        e.1 += n;
    }
    let total: u32 = history.card_counts.values().sum();
    let _ = writeln!(
        out,
        "\n## Card coverage\n\nEvery minion seen entering a combat (Solo and Duos): {} cards, {} \
         appearances.\n",
        history.card_counts.len(),
        total
    );
    let _ = writeln!(out, "| Class | Cards | Appearances |");
    let _ = writeln!(out, "|---|---|---|");
    for (class, (cards, n)) in &by_class {
        let _ = writeln!(
            out,
            "| {class} | {cards} | {n} ({}) |",
            pct(f64::from(*n) / f64::from(total.max(1)))
        );
    }
    missing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let _ = writeln!(
        out,
        "\nMost seen cards not modelled yet (card id: appearances):\n"
    );
    let top: Vec<String> = missing
        .iter()
        .take(40)
        .map(|(id, n)| format!("`{id}` {n}"))
        .collect();
    let _ = writeln!(out, "{}", top.join(", "));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GameValues;

    const CARDS: &str = include_str!("../tests/data/cards.json");
    const GAMES: &str = include_str!("../tests/data/games.jsonl");

    #[test]
    fn reads_card_facts() {
        let db = load_cards(CARDS).unwrap();
        let bonehead = &db["BG28_300"];
        assert_eq!(bonehead.tier, Some(1));
        assert!(bonehead.tribes.is(Tribes::UNDEAD));
        let warden = db["BGS_034"].base.unwrap();
        assert!(warden.keywords.contains(Keywords::REBORN));
        assert!(warden.keywords.contains(Keywords::DIVINE_SHIELD));
        assert!(load_cards("[]").is_err());
        assert!(load_cards("{").is_err());
    }

    #[test]
    fn reads_solo_combats_and_counts_the_rest() {
        let db = load_cards(CARDS).unwrap();
        let h = load_history(GAMES, &db);
        // Two records of game 1 (the second wins), one Duos game, a line with
        // no session or index and a line that is not JSON.
        assert_eq!(h.games, 2);
        assert_eq!((h.solo_games, h.duos_games), (1, 1));
        assert_eq!(h.bad_lines, 2);
        assert_eq!(h.combats.len(), 3);
        assert_eq!(h.solo_rounds_skipped, 1);
        assert_eq!((h.duos_rounds, h.duos_rounds_complete), (1, 1));
        let c = &h.combats[0];
        assert_eq!(c.result, CombatResult::Won);
        assert_eq!(c.player.minions[0].tier, Some(1));
        assert!(c.player.minions[0]
            .keywords
            .contains(Keywords::DIVINE_SHIELD));
        assert_eq!(h.card_counts["BG28_300"], 2);
    }

    #[test]
    fn reads_the_dark_gifts_a_saved_minion_has() {
        let db = load_cards(CARDS).unwrap();
        let gifted = serde_json::json!({
            "card_id": "BG28_300", "atk": 3, "health": 2,
            "gifts": ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"],
        });
        let m = minion(&gifted, &db).unwrap();
        assert_eq!(
            m.gifts,
            ["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"]
        );
        let older = serde_json::json!({"card_id": "BG28_300", "atk": 3, "health": 2});
        assert!(minion(&older, &db).unwrap().gifts.is_empty());
    }

    #[test]
    fn reads_the_game_long_values_a_saved_board_has() {
        // Parser revision 10 (T-610): only the values above zero are saved.
        let db = load_cards(CARDS).unwrap();
        let entry = serde_json::json!({
            "side": "own", "board": [{"card_id": "BG28_300", "atk": 3, "health": 2}],
            "values": {
                "blood_gem_atk": 2, "blood_gem_health": 1, "undead_atk": 5,
                "knights_dead": 3, "beetle_health": -4, "spell_atk": 100_000,
                "spell_health": "7",
            },
        });
        let got = board(&entry, &db).unwrap().values;
        assert_eq!(
            got,
            GameValues {
                blood_gem: (2, 1),
                knights_dead: 3,
                undead_attack: 5,
                ..GameValues::default()
            },
            "values past the saved bounds are unknown (zero)"
        );
        let older = serde_json::json!({"side": "own", "board": []});
        assert_eq!(board(&older, &db).unwrap().values, GameValues::default());
    }

    #[test]
    fn scores_and_renders_without_names() {
        let db = load_cards(CARDS).unwrap();
        let h = load_history(GAMES, &db);
        let opts = Options {
            trials: 500,
            ..Options::default()
        };
        let run = run(&h, &opts);
        let scored = &run.scored;
        assert_eq!(scored.len(), 2);
        // Round 4 has a minion with no health left: counted, not dropped.
        assert_eq!(run.rejected.values().sum::<u32>(), 1);
        // A 3/3 Divine Shield against a 1/1: certain win.
        assert_eq!(scored[0].prediction.won, 1.0);
        assert_eq!(scored[0].class, Class::Exact);
        assert_eq!(scored[1].class, Class::NotModelled);
        let text = render(&h, &run, &opts);
        assert!(text.contains("| exact | 1 | 100% |"), "{text}");
        assert!(text.contains("`SYNTH_UNKNOWN` 1"), "{text}");
        assert!(
            text.contains("not scored: 1 (minion 0 has no health left)"),
            "{text}"
        );
        assert!(!text.contains("SYNTH_HERO"), "hero ids are not printed");
    }
}
