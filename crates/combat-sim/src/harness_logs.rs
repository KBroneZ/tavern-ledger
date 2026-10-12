//! The harness on the game's own logs (T-603b): reads each `Power.log` with
//! the parser, keeps every combat's attacks in order, replays them through
//! the engine and names the first rule that differs. Also scores the odds
//! the overlay would give (T-605): the player's board against the board of
//! that opponent the player saw last, not the one at the combat.
//!
//! Local only: the logs are read where the game keeps them and never copied;
//! the output has counts, rules, positions and card ids, never a name.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use bg_parser::report::{
    CombatEntry, CombatValues, GameReport, LoggedAttack, Minion as LogMinion, Round, Side,
};

use crate::combat::replay::{replay_teams, LoggedStep, Replay, Rule, Span};
use crate::combat::CombatResult;
use crate::harness::{CardDb, History, LoggedCombat, Scored};
use crate::harness_duos::{duos_combats, team_of, DuosCombat};
use crate::model::{GameValues, Keywords, Minion, Team, Warband};
use crate::score::{score, Prediction};
use crate::trials::{simulate, Options, Verdict};

/// The `Power.log` files under `path`: the file itself, or one per session
/// folder (`Hearthstone_<date>`) of a Logs folder, oldest first.
pub fn power_logs(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let entries = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut logs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path().join("Power.log"))
        .filter(|p| p.is_file())
        .collect();
    logs.sort();
    Ok(logs)
}

/// One logged combat with its attacks, ready to replay.
#[derive(Clone, Debug, PartialEq)]
pub struct LogCombat {
    pub game: usize,
    pub round: i64,
    pub duos: bool,
    /// Each team's boards in the order they fought (one in Solo).
    pub player: Team,
    pub opponent: Team,
    pub steps: Vec<LoggedStep>,
    pub result: Option<CombatResult>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LogHistory {
    /// The Solo combats for the accuracy report, as from the saved history.
    pub history: History,
    /// Solo and Duos combats with attacks.
    pub combats: Vec<LogCombat>,
    /// The Duos combats with each team's boards (T-608).
    pub duos: Vec<DuosCombat>,
    /// Rounds with attacks that could not be turned into steps (a card id
    /// missing, an attacker or defender not on the board before).
    pub unreadable: usize,
}

/// A board minion in the engine's form, with the card facts and the
/// lasting keywords the log has (Poisonous, Mega-Windfury).
fn engine_minion(m: &LogMinion, db: &CardDb) -> Option<Minion> {
    let card_id = m.card_id.clone()?;
    let keywords = m
        .keywords
        .iter()
        .chain(&m.sim_keywords)
        .filter_map(|k| Keywords::from_name(k))
        .fold(Keywords::NONE, Keywords::union);
    let facts = db.get(&card_id);
    Some(Minion {
        golden: m.golden,
        attack: i32::try_from(m.atk).ok()?,
        health: i32::try_from(m.health).ok()?,
        keywords,
        tribes: facts.map_or(Default::default(), |f| f.tribes),
        tier: facts.and_then(|f| f.tier),
        base: facts.and_then(|f| f.base),
        gifts: m.gifts.iter().map(|g| g.to_string()).collect(),
        card_id,
    })
}

pub(crate) fn engine_board(board: &[LogMinion], db: &CardDb) -> Option<Vec<Minion>> {
    board.iter().map(|m| engine_minion(m, db)).collect()
}

/// The side's game-long values as the log gave them (T-609); zero when the
/// log had none.
pub(crate) fn game_values(v: Option<CombatValues>) -> GameValues {
    let v = v.unwrap_or_default();
    let int = |x: i64| i32::try_from(x).unwrap_or(0);
    GameValues {
        blood_gem: (int(v.blood_gem.0), int(v.blood_gem.1)),
        spell: (int(v.spell.0), int(v.spell.1)),
    }
}

/// A combat entry's board as a warband with its game-long values.
pub(crate) fn entry_band(e: &CombatEntry, db: &CardDb) -> Option<Option<Warband>> {
    match e.board.as_deref() {
        None => Some(None),
        Some(b) => Some(Some(Warband {
            values: game_values(e.values),
            ..Warband::new(engine_board(b, db)?)
        })),
    }
}

/// Index of the minion at `position` on a board sorted by position.
fn index_at(board: &[LogMinion], position: i64) -> Option<usize> {
    board.iter().position(|m| m.position == position)
}

fn step_of(a: &LoggedAttack, db: &CardDb) -> Option<LoggedStep> {
    let side = |s: Side| usize::from(s == Side::Opponent);
    let boards = [&a.own, &a.opponent];
    let attacker_side = side(a.attacker.side);
    let attacker = index_at(boards[attacker_side], a.attacker.position)?;
    let defender = match &a.defender {
        Some((slot, _)) => Some(index_at(boards[side(slot.side)], slot.position)?),
        None => None,
    };
    Some(LoggedStep {
        attacker: (attacker_side, attacker),
        defender,
        before: [engine_board(&a.own, db)?, engine_board(&a.opponent, db)?],
        all_keywords: true,
    })
}

fn result_of(r: bg_parser::report::CombatResult) -> CombatResult {
    match r {
        bg_parser::report::CombatResult::Won => CombatResult::Won,
        bg_parser::report::CombatResult::Lost => CombatResult::Lost,
        bg_parser::report::CombatResult::Tie => CombatResult::Tie,
    }
}

/// The own and opponent entries of a Solo round: one each, with boards.
fn solo_boards(round: &Round) -> Option<(&CombatEntry, &CombatEntry)> {
    let pick = |s: Side| -> Vec<_> { round.entries.iter().filter(|e| e.side == s).collect() };
    match (pick(Side::Own).as_slice(), pick(Side::Opponent).as_slice()) {
        ([o], [p]) if o.board.is_some() && p.board.is_some() => Some((o, p)),
        _ => None,
    }
}

/// Adds one game's combats.
fn add_game(out: &mut LogHistory, game: usize, report: &GameReport, db: &CardDb) {
    let is_duos = match report.game_type.as_deref() {
        Some("GT_BATTLEGROUNDS") => false,
        Some("GT_BATTLEGROUNDS_DUO") => true,
        _ => return,
    };
    out.history.games += 1;
    let duos: Vec<DuosCombat> = if is_duos {
        duos_combats(report, game, db)
    } else {
        Vec::new()
    };
    if is_duos {
        out.history.duos_games += 1;
    } else {
        out.history.solo_games += 1;
    }
    for round in &report.rounds {
        let result = round.result.map(result_of);
        if !is_duos {
            let scored = solo_boards(round).and_then(|(own, opp)| {
                Some((entry_band(own, db)??, entry_band(opp, db)??, opp.player_id))
            });
            match (scored, result) {
                (Some((player, opponent, seat)), Some(result)) => {
                    out.history.combats.push(LoggedCombat {
                        game,
                        round: round.number,
                        opponent_seat: seat,
                        player,
                        opponent,
                        result,
                    })
                }
                _ => out.history.solo_rounds_skipped += 1,
            }
        } else {
            out.history.duos_rounds += 1;
        }
        if round.attacks.is_empty() {
            continue;
        }
        let Some(steps) = round
            .attacks
            .iter()
            .map(|a| step_of(a, db))
            .collect::<Option<Vec<_>>>()
        else {
            out.unreadable += 1;
            continue;
        };
        let [p, o] = steps[0].before.clone();
        let (player, opponent) = if is_duos {
            // Each team's boards in the order they fought; the first ones
            // are the boards the attacks start from.
            let Some(c) = duos.iter().find(|c| c.round == round.number) else {
                out.unreadable += 1;
                continue;
            };
            let (player, opponent) = (team_of(&c.own), team_of(&c.opponent));
            if player.first.minions != p || opponent.first.minions != o {
                out.unreadable += 1;
                continue;
            }
            (player, opponent)
        } else {
            let values = |s: Side| {
                let e = round
                    .entries
                    .iter()
                    .find(|e| e.side == s && e.board.is_some());
                game_values(e.and_then(|e| e.values))
            };
            let band = |minions, s| Warband {
                values: values(s),
                ..Warband::new(minions)
            };
            (
                Team::solo(band(p, Side::Own)),
                Team::solo(band(o, Side::Opponent)),
            )
        };
        out.combats.push(LogCombat {
            game,
            round: round.number,
            duos: is_duos,
            player,
            opponent,
            steps,
            result,
        });
    }
    out.duos.extend(duos);
}

/// Reads every game of `logs`.
pub fn load_logs(logs: &[PathBuf], db: &CardDb) -> Result<LogHistory, String> {
    let mut out = LogHistory::default();
    let mut game = 0;
    for path in logs {
        let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let reports = bg_parser::parse_reader(std::io::BufReader::new(file))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        for report in &reports {
            game += 1;
            for round in &report.rounds {
                for e in &round.entries {
                    for m in e.board.iter().flatten() {
                        if let Some(id) = &m.card_id {
                            *out.history.card_counts.entry(id.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
            add_game(&mut out, game, report, db);
        }
    }
    Ok(out)
}

/// One replayed combat.
#[derive(Clone, Debug, PartialEq)]
pub struct Replayed {
    pub game: usize,
    pub round: i64,
    pub duos: bool,
    pub replay: Replay,
}

pub fn replay_all(combats: &[LogCombat], seed: u64) -> Vec<Replayed> {
    combats
        .iter()
        .map(|c| Replayed {
            game: c.game,
            round: c.round,
            duos: c.duos,
            replay: replay_teams(
                &c.player,
                &c.opponent,
                &c.steps,
                Span::Whole(c.result),
                seed,
            ),
        })
        .collect()
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

/// The replay report: how far the engine follows the log, and the first
/// rule that differs, overall and for each Solo combat the odds missed.
pub fn render_replays(rows: &[Replayed], scored: &[Scored], unreadable: usize) -> String {
    let mut out = String::new();
    let solo = rows.iter().filter(|r| !r.duos).count();
    let to_end = rows
        .iter()
        .filter(|r| r.replay.difference.is_none())
        .count();
    let attacks: usize = rows.iter().map(|r| r.replay.attacks).sum();
    let matched: usize = rows.iter().map(|r| r.replay.matched).sum();
    let boards_in: usize = rows.iter().map(|r| r.replay.boards_in).sum();
    let _ = writeln!(out, "\n## Attack order replay (T-603b)\n");
    let _ = writeln!(
        out,
        "Combats replayed: {} ({} Solo, {} Duos, both legs); {} rounds with attacks could not \
         be read. Replayed without a difference: {}. Attacks played as the log did before the \
         first difference: {} of {}. Teammates' boards that came in before the first \
         difference (Duos): {boards_in}.\n",
        rows.len(),
        solo,
        rows.len() - solo,
        unreadable,
        to_end,
        matched,
        attacks
    );
    let mut by_rule: BTreeMap<Rule, usize> = BTreeMap::new();
    for r in rows {
        if let Some(d) = &r.replay.difference {
            *by_rule.entry(d.rule).or_insert(0) += 1;
        }
    }
    let _ = writeln!(out, "| First difference | Combats |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| none | {to_end} |");
    for (rule, n) in &by_rule {
        let _ = writeln!(out, "| {} | {n} |", rule.label());
    }
    let _ = writeln!(
        out,
        "\nEach difference (game, round: attack, rule, detail):\n"
    );
    for r in rows.iter().filter(|r| r.replay.difference.is_some()) {
        let d = r.replay.difference.as_ref().expect("filtered");
        let _ = writeln!(
            out,
            "- game {} round {}{}: attack {} of {}, {}; {}",
            r.game,
            r.round,
            if r.duos { " (Duos)" } else { "" },
            d.attack,
            r.replay.attacks,
            d.rule.label(),
            d.detail
        );
    }
    let misses: Vec<&Scored> = scored
        .iter()
        .filter(|s| s.prediction.of(s.combat.result) < 0.5)
        .collect();
    let _ = writeln!(out, "\nSolo combats the odds missed, with the replay:\n");
    if misses.is_empty() {
        let _ = writeln!(out, "None.");
    }
    for s in misses {
        let replay = rows
            .iter()
            .find(|r| !r.duos && r.game == s.combat.game && r.round == s.combat.round);
        let words = match replay.map(|r| &r.replay.difference) {
            None => "no attacks in the log".to_string(),
            Some(None) => {
                "the replay follows the log to the end: the odds come from chance".to_string()
            }
            Some(Some(d)) => format!("attack {}: {}; {}", d.attack, d.rule.label(), d.detail),
        };
        let _ = writeln!(
            out,
            "- game {} round {}: real {:?}, said won {} / tie {} / lost {}; {}",
            s.combat.game,
            s.combat.round,
            s.combat.result,
            pct(s.prediction.won),
            pct(s.prediction.tied),
            pct(s.prediction.lost),
            words
        );
    }
    out
}

/// The overlay's input for a combat: the player's board against the board
/// of the same opponent the player saw last before this round (D-004);
/// `None` when they had not met yet. Returns the board and its age in rounds.
fn last_seen<'a>(all: &'a [LoggedCombat], c: &LoggedCombat) -> Option<(&'a Warband, i64)> {
    let seat = c.opponent_seat?;
    all.iter()
        .filter(|o| o.game == c.game && o.round < c.round && o.opponent_seat == Some(seat))
        .max_by_key(|o| o.round)
        .map(|o| (&o.opponent, c.round - o.round))
}

/// Accuracy of the odds the overlay would show (T-605): same scoring as the
/// main report, with the opponent's board as last seen and unknown cards
/// refused, as in the app.
pub fn render_overlay_path(history: &History, options: &Options) -> String {
    let opts = Options {
        allow_unmodelled: false,
        start_of_combat: false,
        ..*options
    };
    let mut items: Vec<(Prediction, CombatResult)> = Vec::new();
    let (mut unseen, mut not_modelled, mut refused) = (0, 0, 0);
    let mut ages: Vec<i64> = Vec::new();
    for c in &history.combats {
        let Some((opponent, age)) = last_seen(&history.combats, c) else {
            unseen += 1;
            continue;
        };
        match simulate(&c.player, opponent, &opts) {
            Ok(Verdict::Odds(o)) => {
                ages.push(age);
                items.push((
                    Prediction {
                        won: o.share(CombatResult::Won),
                        tied: o.share(CombatResult::Tie),
                        lost: o.share(CombatResult::Lost),
                    },
                    c.result,
                ));
            }
            Ok(Verdict::NotModelled(_)) => not_modelled += 1,
            Err(_) => refused += 1,
        }
    }
    let r = score(&items, 5);
    let mut out = String::new();
    let _ = writeln!(out, "\n## The overlay's inputs (T-605)\n");
    let _ = writeln!(
        out,
        "Solo combats: {}. No board of that opponent seen before: {unseen}. A card not \
         modelled (the overlay shows no number): {not_modelled}. Refused by the board check: \
         {refused}. Scored: {}.\n",
        history.combats.len(),
        r.combats
    );
    if r.combats > 0 {
        let mean_age = ages.iter().sum::<i64>() as f64 / ages.len() as f64;
        let _ = writeln!(
            out,
            "| n | Mean p(real result) | Brier | Most likely = real | Board age (rounds, mean) |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|");
        let _ = writeln!(
            out,
            "| {} | {} | {:.3} | {} of {} | {:.1} |",
            r.combats,
            pct(r.mean_p_actual),
            r.brier,
            r.top_pick_hits,
            r.combats,
            mean_age
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::load_cards;

    const CARDS: &str = include_str!("../tests/data/cards.json");

    /// A synthetic Solo game with one combat of three attacks the rules
    /// allow: 2/4 and 1/1 against a 1/5.
    fn log() -> String {
        include_str!("../tests/data/solo_attacks.log").replace("\r\n", "\n")
    }

    #[test]
    fn a_logged_solo_combat_is_read_and_replayed_to_the_end() {
        let db = load_cards(CARDS).unwrap();
        let dir = std::env::temp_dir().join(format!("combat-sim-logs-{}", std::process::id()));
        let session = dir.join("Hearthstone_2026_01_01_00_00_00");
        fs::create_dir_all(&session).unwrap();
        fs::write(session.join("Power.log"), log()).unwrap();
        let logs = power_logs(&dir).unwrap();
        assert_eq!(logs.len(), 1);
        let h = load_logs(&logs, &db).unwrap();
        fs::remove_dir_all(&dir).ok();
        assert_eq!((h.history.solo_games, h.history.combats.len()), (1, 1));
        assert_eq!(h.combats.len(), 1);
        let c = &h.combats[0];
        assert_eq!(c.steps.len(), 3);
        assert_eq!(c.steps[0].attacker, (0, 0));
        let rows = replay_all(&h.combats, 1);
        assert_eq!(rows[0].replay.difference, None, "{:?}", rows[0].replay);
        let text = render_replays(&rows, &[], h.unreadable);
        assert!(text.contains("| none | 1 |"), "{text}");
    }

    #[test]
    fn the_overlay_path_uses_the_board_seen_before_and_counts_the_rest() {
        let m = |id: &str, a, h| Minion::new(id, a, h).tier(1);
        let combat = |round, seat, opp: Vec<Minion>, result| LoggedCombat {
            game: 1,
            round,
            opponent_seat: Some(seat),
            player: Warband::new(vec![m("BGS_119", 5, 5)]),
            opponent: Warband::new(opp),
            result,
        };
        let history = History {
            combats: vec![
                combat(1, 3, vec![m("BG28_300", 1, 1)], CombatResult::Won),
                combat(2, 4, vec![m("SYNTH_UNKNOWN", 1, 1)], CombatResult::Won),
                // Met again: the round 1 board is the one the overlay has.
                combat(3, 3, vec![m("BG28_300", 9, 9)], CombatResult::Lost),
                combat(4, 4, vec![m("BG28_300", 1, 1)], CombatResult::Won),
            ],
            ..History::default()
        };
        let text = render_overlay_path(
            &history,
            &Options {
                trials: 200,
                ..Options::default()
            },
        );
        assert!(
            text.contains("No board of that opponent seen before: 2"),
            "{text}"
        );
        assert!(
            text.contains("not modelled (the overlay shows no number): 1"),
            "{text}"
        );
        // Said won (from the old 1/1), lost in fact.
        assert!(text.contains("| 1 | 0% | 2.000 | 0 of 1 | 2.0 |"), "{text}");
    }
}
