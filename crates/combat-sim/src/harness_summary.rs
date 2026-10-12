//! The one-screen summary of `combat-validate --summary` (T-609): how many
//! combats a set of games gives, how many the app would block and why, how
//! good the odds are, how far the attack-by-attack replays follow the games
//! and the misses by cause. Counts, shares, rules and card ids only: no
//! names, players, heroes, games or rounds, so it can be shared for any set
//! of games the project may use.

use std::fmt::Write as _;

use crate::combat::replay::Rule;
use crate::harness::{History, Run};
use crate::harness_duos::{team_of, DuosCombat};
use crate::harness_export::{Accuracy, ModeAccuracy};
use crate::harness_logs::Replayed;
use crate::harness_misses::Miss;
use crate::harness_why::{render_why, Tally};
use crate::score::{score, Prediction, Report};
use crate::trials::{simulate_teams, Options, Verdict};
use crate::CombatResult;

/// Everything the summary is made from.
pub struct Summary<'a> {
    pub sources: usize,
    pub history: &'a History,
    pub run: &'a Run,
    pub accuracy: &'a Accuracy,
    pub replays: &'a [Replayed],
    pub unreadable: usize,
    pub misses: &'a [Miss],
    /// Every combat by why its result differed (D-127).
    pub why: &'a Tally,
    pub options: &'a Options,
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn score_cells(r: &Report) -> String {
    if r.combats == 0 {
        return "0 | - | - | -".into();
    }
    format!(
        "{} | {} | {:.3} | {} of {}",
        r.combats,
        pct(r.mean_p_actual),
        r.brier,
        r.top_pick_hits,
        r.combats
    )
}

fn app_row(mode: &str, m: &ModeAccuracy) -> String {
    format!(
        "| {mode} | {} | {} ({}) | {} | {} |",
        m.with_result,
        m.blocked,
        pct(m.blocked_share()),
        m.refused,
        score_cells(&m.report)
    )
}

/// The engine on every Duos board that fought, cards not modelled played
/// as plain minions (as in [`crate::harness_duos::render_duos`]).
fn duos_engine(duos: &[DuosCombat], options: &Options) -> Report {
    let opts = Options {
        allow_unmodelled: true,
        ..*options
    };
    let mut items = Vec::new();
    for c in duos {
        let Some(result) = c.result else { continue };
        if let Ok(Verdict::Odds(o)) = simulate_teams(&team_of(&c.own), &team_of(&c.opponent), &opts)
        {
            let p = Prediction {
                won: o.share(CombatResult::Won),
                tied: o.share(CombatResult::Tie),
                lost: o.share(CombatResult::Lost),
            };
            items.push((p, result));
        }
    }
    score(&items, 5)
}

pub fn render_summary(s: &Summary) -> String {
    let h = s.history;
    let mut out = String::new();
    let _ = writeln!(out, "# Combat simulator accuracy: summary (T-609)\n");
    let _ = writeln!(
        out,
        "Engine {}; {} trials per combat, seed {}. Counts only: no names, players, heroes, \
         games or rounds.\n",
        crate::VERSION,
        s.options.trials,
        s.options.seed
    );
    let _ = writeln!(out, "## Input\n");
    let duos_with_result = h.duos.iter().filter(|c| c.result.is_some()).count();
    let _ = writeln!(
        out,
        "Files read: {}. Games: {} ({} Solo, {} Duos). Lines that could not be read: {}.\n",
        s.sources, h.games, h.solo_games, h.duos_games, h.bad_lines
    );
    let _ = writeln!(
        out,
        "| Mode | Combats with a board on each side and the game's result | Left out (no \
         board on a side, or no result) |"
    );
    let _ = writeln!(out, "|---|---|---|");
    let _ = writeln!(
        out,
        "| Solo | {} | {} |",
        h.combats.len(),
        h.solo_rounds_skipped
    );
    let _ = writeln!(
        out,
        "| Duos | {} | {} |",
        duos_with_result,
        h.duos_rounds.saturating_sub(duos_with_result)
    );
    let _ = writeln!(out, "\n## What the app would show\n");
    let _ = writeln!(
        out,
        "| Mode | With a result | Blocked (a card not modelled) | Refused by the board check \
         | Scored | Mean p(real result) | Brier | Most likely = real |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    let _ = writeln!(out, "{}", app_row("Solo", &s.accuracy.solo));
    let _ = writeln!(out, "{}", app_row("Duos", &s.accuracy.duos));
    let blockers: Vec<String> = s
        .accuracy
        .blockers
        .iter()
        .map(|(id, n)| format!("`{id}` {n}"))
        .collect();
    let _ = writeln!(
        out,
        "\nBlocked, by card (card id: combats): {}.",
        if blockers.is_empty() {
            "none".to_string()
        } else {
            blockers.join(", ")
        }
    );
    let solo_items: Vec<(Prediction, CombatResult)> = s
        .run
        .scored
        .iter()
        .map(|x| (x.prediction, x.combat.result))
        .collect();
    let _ = writeln!(
        out,
        "\n## The engine on every board that fought\n\nCards not modelled played as plain \
         minions, for measuring the engine only.\n"
    );
    let _ = writeln!(
        out,
        "| Mode | n | Mean p(real result) | Brier | Most likely = real |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    let _ = writeln!(out, "| Solo | {} |", score_cells(&score(&solo_items, 5)));
    let _ = writeln!(
        out,
        "| Duos | {} |",
        score_cells(&duos_engine(&h.duos, s.options))
    );
    render_replay_counts(&mut out, s.replays, s.unreadable);
    let _ = writeln!(
        out,
        "\nMost likely result not the real one, on every board that fought: {} combats.",
        s.misses.len()
    );
    out.push_str(&render_why(s.why));
    out
}

fn render_replay_counts(out: &mut String, rows: &[Replayed], unreadable: usize) {
    let to_end = rows
        .iter()
        .filter(|r| r.replay.difference.is_none())
        .count();
    let attacks: usize = rows.iter().map(|r| r.replay.attacks).sum();
    let matched: usize = rows.iter().map(|r| r.replay.matched).sum();
    let _ = writeln!(out, "\n## Attack-by-attack replays\n");
    let _ = writeln!(
        out,
        "Combats replayed: {}; with a replay that could not be read: {unreadable}. Followed to \
         the end: {to_end}. Attacks played as the game did before the first difference: \
         {matched} of {attacks}.\n",
        rows.len()
    );
    let mut by_rule: std::collections::BTreeMap<Rule, usize> = Default::default();
    for d in rows.iter().filter_map(|r| r.replay.difference.as_ref()) {
        *by_rule.entry(d.rule).or_insert(0) += 1;
    }
    let _ = writeln!(out, "| First difference | Combats |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| none | {to_end} |");
    for (rule, n) in by_rule {
        let _ = writeln!(out, "| {} | {n} |", rule.label());
    }
}

#[cfg(test)]
#[path = "harness_summary_tests.rs"]
mod tests;
