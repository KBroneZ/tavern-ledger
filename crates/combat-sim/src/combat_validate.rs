//! Plays every logged Solo combat of a local history through the engine and
//! prints the accuracy report (T-603). Reads only local files; prints counts,
//! shares and card ids, never names.
//!
//! combat-validate --history <games.jsonl> --cards <cards.json>
//!                 [--trials N] [--seed N]

use std::process::ExitCode;

use combat_sim::harness::{load_cards, load_history, render, run};
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
    let mut cards = None;
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--history" => history = Some(value()?),
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
    let history = history.ok_or("--history <games.jsonl> is required")?;
    let cards = cards.ok_or("--cards <cards.json> is required")?;
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let db = load_cards(&read(&cards)?)?;
    let history = load_history(&read(&history)?, &db);
    let run = run(&history, &options);
    print!("{}", render(&history, &run, &options));
    Ok(())
}
