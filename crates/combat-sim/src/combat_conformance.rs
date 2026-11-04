//! Plays every combat of a conformance corpus (T-640) through the engine
//! and compares each one with the baseline (T-642). Exits with an error
//! when any combat moved, listing each with the attack where its replay
//! now leaves the game; `--bless` writes the baseline from this run
//! instead. Prints counts, rules, card ids and corpus ids only.
//!
//! combat-conformance --corpus <corpus.jsonl[.gz]> --cards <build>=<cards.json>...
//!                    --baseline <baseline.json> [--bless] [--trials N]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use combat_sim::conformance::{
    baseline_json, compare, read_baseline, render_diff, render_summary, run, Kind, TRIALS,
};
use combat_sim::corpus::read_corpus_file;
use combat_sim::harness::{load_cards, CardDb};
use combat_sim::Options;

const USAGE: &str = "usage: combat-conformance --corpus <file> --cards <build>=<cards.json>... \
                     --baseline <file> [--bless] [--trials N]";

struct Args {
    corpus: String,
    cards: Vec<(u64, String)>,
    baseline: String,
    bless: bool,
    trials: Option<u32>,
}

fn parse_args() -> Result<Args, String> {
    let (mut corpus, mut baseline, mut cards) = (None, None, Vec::new());
    let (mut bless, mut trials) = (false, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--corpus" => corpus = Some(value()?),
            "--baseline" => baseline = Some(value()?),
            "--bless" => bless = true,
            "--trials" => {
                let v = value()?;
                let t: u32 = v.parse().map_err(|_| format!("bad --trials {v}"))?;
                if t == 0 {
                    return Err("--trials must be at least 1".into());
                }
                trials = Some(t);
            }
            "--cards" => {
                let v = value()?;
                let (build, path) = v.split_once('=').ok_or("--cards takes <build>=<path>")?;
                let build = build.parse().map_err(|_| format!("bad build {build}"))?;
                cards.push((build, path.to_string()));
            }
            _ => return Err(format!("unknown argument {a}\n{USAGE}")),
        }
    }
    if trials.is_some() && !bless {
        return Err("--trials only goes with --bless; a check uses the baseline's".into());
    }
    match (corpus, baseline) {
        (Some(corpus), Some(baseline)) => Ok(Args {
            corpus,
            cards,
            baseline,
            bless,
            trials,
        }),
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    match real_main() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("combat-conformance: {e}");
            ExitCode::from(2)
        }
    }
}

/// `Ok(false)` when combats moved from the baseline.
fn real_main() -> Result<bool, String> {
    let args = parse_args()?;
    let corpus = read_corpus_file(Path::new(&args.corpus))?;
    if corpus.is_empty() {
        return Err(format!("{}: no combat in the corpus", args.corpus));
    }
    let mut cards: BTreeMap<u64, CardDb> = BTreeMap::new();
    for (build, path) in &args.cards {
        let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        cards.insert(
            *build,
            load_cards(&text).map_err(|e| format!("{path}: {e}"))?,
        );
    }
    let baseline = if args.bless {
        None
    } else {
        let text = fs::read_to_string(&args.baseline)
            .map_err(|e| format!("{}: {e} (run with --bless to make it)", args.baseline))?;
        Some(read_baseline(&text)?)
    };
    // The baseline's own options, so a check compares like with like.
    let options = match (&baseline, args.trials) {
        (Some((o, _)), _) => *o,
        (None, trials) => Options {
            trials: trials.unwrap_or(TRIALS),
            ..Options::default()
        },
    };
    let start = Instant::now();
    let outcomes = run(&corpus, &cards, &options)?;
    print!("{}", render_summary(&outcomes));
    println!(
        "{} combats in {:.1} s ({} trials each)",
        outcomes.len(),
        start.elapsed().as_secs_f64(),
        options.trials
    );
    let Some((_, old)) = baseline else {
        fs::write(&args.baseline, baseline_json(&outcomes, &options))
            .map_err(|e| format!("{}: {e}", args.baseline))?;
        println!("baseline written: {}", args.baseline);
        return Ok(true);
    };
    let diff = compare(&old, &outcomes);
    print!("{}", render_diff(&diff));
    if diff.fails() {
        let moved = diff
            .changes
            .iter()
            .filter(|c| c.kind != Kind::CloseFlip)
            .count();
        println!(
            "the engine moved on {} combats; if every change is meant, run with --bless and \
             commit the baseline",
            moved + diff.added.len() + diff.removed.len()
        );
        return Ok(false);
    }
    println!("every combat as in the baseline");
    Ok(true)
}
