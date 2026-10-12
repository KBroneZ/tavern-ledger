//! Plays every logged combat of a set of games through the engine and
//! prints the accuracy report (T-603), the accuracy of the overlay's inputs
//! (T-605), the attack-by-attack replays (T-603b, T-609) and the misses.
//! With `--logs` it reads the game's own logs; with `--history` the app's
//! history; with `--games` any number of history files and game records
//! (files or folders, T-609). `--summary` prints only the aggregate summary
//! (counts, shares, rules and card ids: no game or round), for any set of
//! games. Reads only local files; never prints a name. `--export` also
//! writes the accuracy numbers as a small JSON file for the website (T-607).
//!
//! combat-validate (--history <games.jsonl> | --games <file or folder>... |
//!                  --logs <Logs folder or Power.log>)
//!                 --cards <cards.json> [--summary] [--trials N] [--seed N]
//!                 [--export <file.json>]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use combat_sim::harness::{load_cards, load_history, render, run, CardDb, History};
use combat_sim::harness_duos::{render_duos, Source};
use combat_sim::harness_export::{accuracy, date_of_day, export_json, render_accuracy};
use combat_sim::harness_games::read_games;
use combat_sim::harness_logs::{
    load_logs, power_logs, render_overlay_path, render_replays, replay_all,
};
use combat_sim::harness_misses::{duos_misses, render_misses, solo_misses};
use combat_sim::harness_summary::{render_summary, Summary};
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

/// What to read.
enum Input {
    History(String),
    Games(Vec<PathBuf>),
    Logs(String),
}

struct Args {
    input: Input,
    cards: String,
    export: Option<String>,
    summary: bool,
    options: Options,
}

fn parse_args() -> Result<Args, String> {
    let (mut history, mut logs, mut games) = (None, None, Vec::new());
    let (mut cards, mut export, mut summary) = (None, None, false);
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--history" => history = Some(value()?),
            "--games" => games.push(PathBuf::from(value()?)),
            "--logs" => logs = Some(value()?),
            "--cards" => cards = Some(value()?),
            "--export" => export = Some(value()?),
            "--summary" => summary = true,
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
    let input = match (history, logs, games.is_empty()) {
        (Some(h), None, true) => Input::History(h),
        (None, Some(l), true) => Input::Logs(l),
        (None, None, false) => Input::Games(games),
        _ => {
            return Err(
                "give one of --history <games.jsonl>, --games <file or folder>... or --logs \
                 <folder>"
                    .into(),
            )
        }
    };
    Ok(Args {
        input,
        cards: cards.ok_or("--cards <cards.json> is required")?,
        export,
        summary,
        options,
    })
}

fn read(p: &str) -> Result<String, String> {
    std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))
}

fn real_main() -> Result<(), String> {
    let args = parse_args()?;
    let options = args.options;
    let db = load_cards(&read(&args.cards)?)?;
    let (history, sources, from_logs) = match &args.input {
        Input::History(path) => (load_history(&read(path)?, &db), 1, None),
        Input::Games(paths) => {
            let (jsonl, files) = read_games(paths)?;
            (load_history(&jsonl, &db), files, None)
        }
        Input::Logs(path) => {
            let (history, files, rows, unreadable) = from_logs(path, &db, &options)?;
            (history, files, Some((rows, unreadable)))
        }
    };
    let run = run(&history, &options);
    let (rows, unreadable) = match from_logs {
        Some(r) => r,
        // The replays the app saved with the games (T-609).
        None => (
            replay_all(&history.saved, options.seed),
            history.saved_unreadable,
        ),
    };
    let mut misses = solo_misses(&run.scored);
    misses.extend(duos_misses(&history.duos, &options));
    let acc = accuracy(&history.combats, &history.duos, &options);
    if args.summary {
        print!(
            "{}",
            render_summary(&Summary {
                sources,
                history: &history,
                run: &run,
                accuracy: &acc,
                replays: &rows,
                unreadable,
                misses: &misses,
                options: &options,
            })
        );
    } else {
        let source = match args.input {
            Input::Logs(_) => Source::Logs,
            _ => Source::History,
        };
        print!("{}", render(&history, &run, &options));
        print!("{}", render_replays(&rows, &run.scored, unreadable));
        print!("{}", render_overlay_path(&history, &options));
        print!("{}", render_duos(&history.duos, &options, source));
        print!("{}", render_misses(&misses, Some(&rows)));
        print!("{}", render_accuracy(&acc));
    }
    if let Some(path) = args.export {
        let json = export_json(&acc, &options, &date_of_day(today()?));
        std::fs::write(&path, json).map_err(|e| format!("{path}: {e}"))?;
        println!("\nAccuracy written to {path}.");
    }
    Ok(())
}

/// The game's own logs: the Solo combats and the Duos combats as a history,
/// the number of `Power.log` files, and the attack-by-attack replays.
fn from_logs(
    path: &str,
    db: &CardDb,
    options: &Options,
) -> Result<
    (
        History,
        usize,
        Vec<combat_sim::harness_logs::Replayed>,
        usize,
    ),
    String,
> {
    let files = power_logs(Path::new(path))?;
    if files.is_empty() {
        return Err(format!("{path}: no Power.log found"));
    }
    let read = load_logs(&files, db)?;
    let rows = replay_all(&read.combats, options.seed);
    let history = History {
        duos: read.duos,
        ..read.history
    };
    Ok((history, files.len(), rows, read.unreadable))
}

/// Days since 1970-01-01, UTC, from the system clock.
fn today() -> Result<i64, String> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "the system clock is before 1970")?
        .as_secs();
    i64::try_from(secs / 86_400).map_err(|_| "the system clock is out of range".into())
}
