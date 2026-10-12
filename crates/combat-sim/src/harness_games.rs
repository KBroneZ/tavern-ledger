//! Game records from several files for `combat-validate --games` (T-609):
//! the app's history (`games.jsonl`, one record a line) and single game
//! records (`.json`: one record, or a list of them), as the app keeps them
//! and uploads them (D-021: `session`, `index`, `saved_at`, `parser`,
//! `report`). A gzipped upload must be unpacked first.
//!
//! Records from different files never merge: two players can have a game
//! with the same session and index, so each file's sessions get the file's
//! number in front. Within one file the last record of a game wins, as in
//! the app.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The `.json` and `.jsonl` files a path names: the file itself, or those
/// in a folder (not its subfolders), sorted by name.
pub fn game_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let entries = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().is_some_and(|x| x == "json" || x == "jsonl"))
        .collect();
    files.sort();
    Ok(files)
}

/// The records of one file's text: a line each (`.jsonl`), or one record
/// or a list of records (`.json`). A line or value that is not JSON is kept
/// as it is, so the history reader counts it as unreadable.
fn records(text: &str, jsonl: bool) -> Vec<Result<Value, String>> {
    if jsonl {
        return text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|_| l.to_string()))
            .collect();
    }
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(items)) => items.into_iter().map(Ok).collect(),
        Ok(v) => vec![Ok(v)],
        Err(_) => vec![Err(text.replace('\n', " "))],
    }
}

/// Every record of `texts` (file text, whether it is `.jsonl`) as one
/// history, each file's sessions marked with its number.
pub fn merge(texts: &[(String, bool)]) -> String {
    let mut out = String::new();
    for (k, (text, jsonl)) in texts.iter().enumerate() {
        for record in records(text, *jsonl) {
            let line = match record {
                Ok(mut v) => {
                    if let Some(s) = v.get("session").and_then(Value::as_str) {
                        let marked = format!("{}:{s}", k + 1);
                        v["session"] = Value::String(marked);
                    }
                    v.to_string()
                }
                Err(raw) => raw,
            };
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// Reads every game file under `paths` into one history; returns it and the
/// number of files read.
pub fn read_games(paths: &[PathBuf]) -> Result<(String, usize), String> {
    let mut texts = Vec::new();
    for path in paths {
        for file in game_files(path)? {
            let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let jsonl = file.extension().is_some_and(|x| x == "jsonl");
            texts.push((text, jsonl));
        }
    }
    if texts.is_empty() {
        return Err("no .json or .jsonl game file found".into());
    }
    Ok((merge(&texts), texts.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_from_different_files_never_merge() {
        let record = r#"{"session":"Hearthstone_2026_01_01_00_00_00","index":1,"report":{}}"#;
        let merged = merge(&[
            (format!("{record}\n{record}\n"), true),
            (record.to_string(), false),
            (format!("[{record}]"), false),
        ]);
        let sessions: Vec<String> = merged
            .lines()
            .map(|l| {
                serde_json::from_str::<Value>(l).unwrap()["session"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            sessions,
            [
                "1:Hearthstone_2026_01_01_00_00_00",
                "1:Hearthstone_2026_01_01_00_00_00",
                "2:Hearthstone_2026_01_01_00_00_00",
                "3:Hearthstone_2026_01_01_00_00_00",
            ]
        );
    }

    #[test]
    fn a_line_that_is_not_json_stays_for_the_reader_to_count() {
        let merged = merge(&[("not json\n".to_string(), true)]);
        assert_eq!(merged, "not json\n");
    }

    #[test]
    fn a_folder_gives_its_game_files_sorted() {
        let dir = std::env::temp_dir().join(format!("combat-sim-games-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for name in ["b.json", "a.jsonl", "c.txt"] {
            fs::write(dir.join(name), "{}").unwrap();
        }
        let files = game_files(&dir).unwrap();
        fs::remove_dir_all(&dir).ok();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.jsonl", "b.json"]);
    }
}
