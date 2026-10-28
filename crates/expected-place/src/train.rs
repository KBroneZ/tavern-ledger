//! `place-train`: trains the expected-place models (T-653) on a history of
//! saved games, one per mode with enough games, cross-validates them by
//! game and writes each model with its card: what it was trained on and
//! how well it did against two baselines. Reads only local files; the
//! card holds counts, builds and scores, never a name, account or time.
//! It also writes `calibration.json`, the website's calibration file
//! (T-655): totals only, copied to `web/src/data/` by hand when a model is
//! published.
//!
//!     place-train --history <games.jsonl> --out <folder> [--folds N]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use expected_place::calibration::{calibration_file, day_text};
use expected_place::eval::{cross_validate, Report};
use expected_place::model::{Model, Train};
use expected_place::{encode, latest_reports, Game, VERSION};

const USAGE: &str = "usage: place-train --history <games.jsonl> --out <folder> [--folds N]";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("place-train: {e}");
            ExitCode::FAILURE
        }
    }
}

fn card(model: &Model, report: Option<&Report>, games: &[Game], opts: &Train) -> String {
    let mode = if model.solo { "Solo" } else { "Duos" };
    let builds: BTreeSet<i64> = games
        .iter()
        .filter(|g| g.solo == model.solo)
        .filter_map(|g| g.build)
        .collect();
    let mut out = String::new();
    let _ = writeln!(out, "# Expected-place model, {mode}\n");
    let _ = writeln!(
        out,
        "Multinomial logistic regression over the {} places, on the {} features of the rows' format {VERSION} (each standardised, with a mark when unknown). Trained by full-batch gradient descent: {} steps, rate {}, L2 {}; each game weighs the same.\n",
        expected_place::model::places(model.solo),
        expected_place::WIDTH,
        opts.steps,
        opts.rate,
        opts.l2
    );
    let shown = if model.ready() {
        "Players may see its numbers: it was trained on at least the minimum number of games"
    } else {
        "Not shown to players: it was trained on fewer games than the minimum"
    };
    let _ = writeln!(
        out,
        "{shown} ({} for a mode). The L2 penalty was chosen among {:?} by the lowest cross-validated log-loss on the folds below, so those scores are a little optimistic.\n",
        expected_place::model::MIN_GAMES,
        L2
    );
    let _ = writeln!(out, "## Data\n");
    let _ = writeln!(
        out,
        "- Games: {}; rows (combats): {}",
        model.games, model.rows
    );
    let list: Vec<String> = builds.iter().map(i64::to_string).collect();
    let _ = writeln!(out, "- Game builds: {}", list.join(", "));
    let _ = writeln!(
        out,
        "- Only games whose final place and at least one combat are known.\n"
    );
    let _ = writeln!(out, "## Scores\n");
    match report {
        None => {
            let _ = writeln!(
                out,
                "Too few games to cross-validate: no score. Do not show this model to players."
            );
        }
        Some(r) => {
            let _ = writeln!(
                out,
                "Cross-validated by game in {} folds (a game's combats are never on both sides). Lower is better.\n",
                r.folds
            );
            let _ = writeln!(
                out,
                "| | Log-loss | Ranked probability score | Mean distance to the real place |"
            );
            let _ = writeln!(out, "|---|---|---|---|");
            for (name, s) in [
                ("This model", r.model),
                ("Baseline: place shares over all games", r.overall),
                ("Baseline: place shares at the same turn", r.per_turn),
            ] {
                let _ = writeln!(
                    out,
                    "| {name} | {:.3} | {:.3} | {:.2} |",
                    s.log_loss, s.rps, s.mae
                );
            }
            let _ = writeln!(out, "\n### Calibration\n");
            let _ = writeln!(
                out,
                "| Expected place | Rows | Games | Mean said | Mean real |"
            );
            let _ = writeln!(out, "|---|---|---|---|---|");
            for b in &r.calibration {
                let _ = writeln!(
                    out,
                    "| {}-{} | {} | {} | {:.2} | {:.2} |",
                    b.from,
                    b.from + 1,
                    b.rows,
                    b.games,
                    b.said,
                    b.real
                );
            }
        }
    }
    let _ = writeln!(
        out,
        "\n## Limits\n\nTrained only on the games listed above; a small set gives a model that is close to the baselines and can be overconfident. The rows hold no name, account or time, and the model holds only weights."
    );
    out
}

/// The L2 penalties tried; the one with the lowest cross-validated
/// log-loss is kept (the card says which, and that it was chosen on the
/// same folds it reports, so its scores are a little optimistic).
const L2: [f64; 4] = [0.01, 0.05, 0.2, 1.0];

fn choose(games: &[Game], solo: bool, folds: usize) -> (Train, Option<Report>) {
    let mut best: Option<(Train, Report)> = None;
    for l2 in L2 {
        let opts = Train {
            l2,
            ..Train::default()
        };
        if let Some(r) = cross_validate(games, solo, folds, &opts) {
            if best
                .as_ref()
                .is_none_or(|(_, b)| r.model.log_loss < b.model.log_loss)
            {
                best = Some((opts, r));
            }
        }
    }
    match best {
        Some((opts, r)) => (opts, Some(r)),
        None => (Train::default(), None),
    }
}

/// Days since 1970-01-01, UTC, from the system clock.
fn today() -> Result<i64, String> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "the system clock is before 1970")?
        .as_secs();
    i64::try_from(secs / 86_400).map_err(|_| "the system clock is out of range".into())
}

fn run() -> Result<(), String> {
    let (mut history, mut out, mut folds) = (None, None, 5usize);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--history" => history = Some(value()?),
            "--out" => out = Some(value()?),
            "--folds" => {
                let v = value()?;
                folds = v
                    .parse()
                    .ok()
                    .filter(|f| *f >= 2)
                    .ok_or(format!("bad --folds {v}"))?;
            }
            _ => return Err(format!("unknown argument {a}\n{USAGE}")),
        }
    }
    let (Some(history), Some(out)) = (history, out) else {
        return Err(USAGE.into());
    };
    let text = fs::read_to_string(&history).map_err(|e| format!("{history}: {e}"))?;
    let games: Vec<Game> = latest_reports(&text)
        .0
        .iter()
        .filter_map(|r| encode(r).ok())
        .collect();
    let dir = Path::new(&out);
    fs::create_dir_all(dir).map_err(|e| format!("{out}: {e}"))?;
    let mut written = 0;
    let mut reports: [Option<Report>; 2] = [None, None];
    for solo in [true, false] {
        let (opts, report) = choose(&games, solo, folds);
        let Some(model) = Model::train(&games, solo, &opts) else {
            continue;
        };
        let name = if solo { "solo" } else { "duos" };
        let json = serde_json::to_string_pretty(&model.to_json()).map_err(|e| e.to_string())?;
        fs::write(dir.join(format!("model-{name}.json")), json + "\n")
            .map_err(|e| e.to_string())?;
        fs::write(
            dir.join(format!("model-{name}.md")),
            card(&model, report.as_ref(), &games, &opts),
        )
        .map_err(|e| e.to_string())?;
        match &report {
            Some(r) => println!(
                "{name}: {} games, L2 {}; log-loss {:.3} (baselines {:.3}, {:.3}); RPS {:.3} (baselines {:.3}, {:.3}); mean distance {:.2} (baselines {:.2}, {:.2})",
                r.games, opts.l2, r.model.log_loss, r.overall.log_loss, r.per_turn.log_loss,
                r.model.rps, r.overall.rps, r.per_turn.rps, r.model.mae, r.overall.mae, r.per_turn.mae
            ),
            None => println!("{name}: {} games, too few to cross-validate", model.games),
        }
        written += 1;
        reports[usize::from(!solo)] = report;
    }
    if written == 0 {
        return Err("no game with a final place and a combat; nothing written".into());
    }
    for r in reports.iter().flatten() {
        let bad = r.predictions.iter().filter(|p| !p.is_finite()).count();
        if bad > 0 {
            let name = if r.solo { "solo" } else { "duos" };
            return Err(format!(
                "{name}: {bad} held-out predictions are not a number; calibration.json not written"
            ));
        }
    }
    let file = calibration_file(
        &day_text(today()?),
        reports[0].as_ref(),
        reports[1].as_ref(),
    );
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    fs::write(dir.join("calibration.json"), json + "\n").map_err(|e| e.to_string())?;
    Ok(())
}
