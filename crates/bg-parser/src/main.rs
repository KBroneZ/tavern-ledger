//! `bg-parse PATH [--json]`: same output and exit codes as `tools/parse_bg.py`.
//!
//! Exit codes: 0 = parsed, 1 = some game unsupported or the log could not be
//! read, 2 = file not found. Never prints log lines: they can hold player names.

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;

use bg_parser::report::{format_text, Status};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let [path] = paths.as_slice() else {
        eprintln!("usage: bg-parse PATH_TO_POWER_LOG [--json]");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(path);
    if !path.is_file() {
        println!("File not found: {}", path.display());
        return ExitCode::from(2);
    }
    let reports = match File::open(&path).and_then(|f| bg_parser::parse_reader(BufReader::new(f))) {
        Ok(reports) => reports,
        Err(err) => {
            println!("Could not read the log ({:?}).", err.kind());
            return ExitCode::from(1);
        }
    };
    if json {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let out = serde_json::json!({ "file": name, "games": reports });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).expect("reports always serialize")
        );
    } else if reports.is_empty() {
        println!("No games found.");
    } else {
        let text: Vec<String> = reports.iter().map(format_text).collect();
        println!("{}", text.join("\n\n"));
    }
    if reports.iter().any(|r| r.status == Status::Unsupported) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
