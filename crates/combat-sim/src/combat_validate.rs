//! Plays every logged Solo combat of a local history through the engine and
//! prints the accuracy report (T-603), then the accuracy of the overlay's
//! inputs (T-605). With `--logs` it reads the game's own logs instead and
//! also replays each combat's attacks (T-603b). Reads only local files;
//! prints counts, shares, rules and card ids, never names.
//!
//! combat-validate (--history <games.jsonl> | --logs <Logs folder or Power.log>)
//!                 --cards <cards.json> [--trials N] [--seed N]

use std::path::Path;
use std::process::ExitCode;

use combat_sim::harness::{load_cards, load_history, render, run};
use combat_sim::harness_logs::{
    load_logs, power_logs, render_overlay_path, render_replays, replay_all,
};
use combat_sim::Options;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("combat-validate: {e}");
            ExitCode::FAILURE
        }
    }
}

fn real_main() -> Result<(), String> {
    let mut history = None;
    let mut logs = None;
    let mut cards = None;
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--history" => history = Some(value()?),
            "--logs" => logs = Some(value()?),
            "--cards" => cards = Some(value()?),
            "--trials" => {
                options.trials = value()?
                    .parse()
                    .map_err(|_| "--trials needs a whole number")?
            }
            "--seed" => {
                options.seed = value()?
                    .parse()
                    .map_err(|_| "--seed needs a whole number")?
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    let cards = cards.ok_or("--cards <cards.json> is required")?;
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let db = load_cards(&read(&cards)?)?;
    match (history, logs) {
        (Some(history), None) => {
            let history = load_history(&read(&history)?, &db);
            let run = run(&history, &options);
            print!("{}", render(&history, &run, &options));
            print!("{}", render_overlay_path(&history, &options));
        }
        (None, Some(logs)) => {
            let files = power_logs(Path::new(&logs))?;
            if files.is_empty() {
                return Err(format!("{logs}: no Power.log found"));
            }
            let read = load_logs(&files, &db)?;
            let run = run(&read.history, &options);
            print!("{}", render(&read.history, &run, &options));
            let rows = replay_all(&read.combats, options.seed);
            print!("{}", render_replays(&rows, &run.scored, read.unreadable));
            print!("{}", render_overlay_path(&read.history, &options));
        }
        _ => return Err("give one of --history <games.jsonl> or --logs <folder>".into()),
    }
    Ok(())
}
