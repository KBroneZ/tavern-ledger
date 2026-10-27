//! `place-features`: the expected-place rows (T-652) of a history of saved
//! games, as CSV for training (T-653). Reads only local files; writes card
//! ids, numbers and a running game number, never a name, account or time.
//!
//!     place-features --history <games.jsonl> --out <features.csv>

use std::fs;
use std::process::ExitCode;

use expected_place::{csv_header, csv_rows, encode, Skip};
use serde_json::Value;

const USAGE: &str = "usage: place-features --history <games.jsonl> --out <features.csv>";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("place-features: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The report of each game, the last record of a game winning as in the
/// app (one line per save, keyed by session and index).
fn latest_reports(text: &str) -> (Vec<Value>, usize) {
    let mut latest: Vec<((String, u64), Value)> = Vec::new();
    let mut bad = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            bad += 1;
            continue;
        };
        let key = v
            .get("session")
            .and_then(Value::as_str)
            .zip(v.get("index").and_then(Value::as_u64));
        let (Some((session, index)), Some(report)) = (key, v.get("report")) else {
            bad += 1;
            continue;
        };
        let key = (session.to_string(), index);
        match latest.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = report.clone(),
            None => latest.push((key, report.clone())),
        }
    }
    (latest.into_iter().map(|(_, r)| r).collect(), bad)
}

fn run() -> Result<(), String> {
    let (mut history, mut out) = (None, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--history" => history = Some(value()?),
            "--out" => out = Some(value()?),
            _ => return Err(format!("unknown argument {a}\n{USAGE}")),
        }
    }
    let (Some(history), Some(out)) = (history, out) else {
        return Err(USAGE.into());
    };
    let text = fs::read_to_string(&history).map_err(|e| format!("{history}: {e}"))?;
    let (reports, bad) = latest_reports(&text);
    let mut lines = vec![csv_header()];
    let (mut games, mut skipped) = (0, [0usize; 3]);
    for report in &reports {
        match encode(report) {
            Ok(g) => {
                games += 1;
                lines.extend(csv_rows(games, &g));
            }
            Err(Skip::NotBattlegrounds) => skipped[0] += 1,
            Err(Skip::NoFinalPlace) => skipped[1] += 1,
            Err(Skip::NoCombats) => skipped[2] += 1,
        }
    }
    if games == 0 {
        return Err("no game with a final place and a combat; nothing written".into());
    }
    let rows = lines.len() - 1;
    lines.push(String::new());
    fs::write(&out, lines.join("\n")).map_err(|e| format!("{out}: {e}"))?;
    println!(
        "{games} games, {rows} rows (format {}); left out: {} not Battlegrounds, {} without a final place, {} without a combat; {bad} lines not a game",
        expected_place::VERSION,
        skipped[0],
        skipped[1],
        skipped[2]
    );
    Ok(())
}
