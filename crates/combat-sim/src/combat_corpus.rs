//! Writes the conformance corpus (T-640) from the player's own saved games:
//! one line per combat with a result, nothing private and no card data.
//! Reads only local files and prints counts only.
//!
//! combat-corpus --history <games.jsonl> --out <corpus.jsonl | corpus.jsonl.gz>

use std::fs;
use std::io::Write as _;
use std::process::ExitCode;

use combat_sim::corpus::{export, render_export};
use flate2::write::GzEncoder;
use flate2::{Compression, GzBuilder};

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("combat-corpus: {e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage: combat-corpus --history <games.jsonl> --out <file>";

fn real_main() -> Result<(), String> {
    let (mut history, mut out) = (None, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--history" => history = Some(value()?),
            "--out" => out = Some(value()?),
            _ => {
                return Err(format!(
                    "unknown argument {a}
{USAGE}"
                ))
            }
        }
    }
    let (Some(history_path), Some(out)) = (history, out) else {
        return Err(USAGE.into());
    };
    if same_file(&history_path, &out) {
        return Err("--out names the history itself; it would be overwritten".into());
    }
    let history = fs::read_to_string(&history_path).map_err(|e| format!("{history_path}: {e}"))?;
    let e = export(&history)?;
    if e.lines.is_empty() {
        return Err("no combat with a result in the history; nothing written".into());
    }
    let mut text = e.lines.join("\n");
    text.push('\n');
    let bytes = if out.ends_with(".gz") {
        // No file name and time 0 in the header, so the same corpus gives
        // the same file.
        let mut gz: GzEncoder<Vec<u8>> = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::best());
        gz.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        gz.finish().map_err(|e| e.to_string())?
    } else {
        text.into_bytes()
    };
    fs::write(&out, bytes).map_err(|e| format!("{}: {e}", out))?;
    print!("{}", render_export(&e));
    Ok(())
}

/// Both paths name one file (an `--out` that does not exist yet is not the
/// history).
fn same_file(a: &str, b: &str) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
