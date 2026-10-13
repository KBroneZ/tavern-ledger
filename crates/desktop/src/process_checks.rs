//! Every child process the app or the tracker starts goes through
//! `tracker::quiet::command`, so none opens a console window (T-582).

use std::fs;
use std::path::{Path, PathBuf};

/// The one file that may call `Command::new` itself.
const HELPER: &str = "quiet.rs";

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source folder") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Files that are test code as a whole.
fn is_test_file(path: &Path) -> bool {
    let name = path.file_stem().unwrap().to_string_lossy();
    name.ends_with("_tests") || name == "tests" || name == "ui_checks" || name == "process_checks"
}

/// Lines with `Command::new` before the file's test module (an unindented `#[cfg(test)]`; an
/// indented one only marks a single item).
fn direct_spawns(text: &str) -> Vec<usize> {
    text.lines()
        .take_while(|line| *line != "#[cfg(test)]")
        .enumerate()
        .filter(|(_, line)| line.contains("Command::new"))
        .map(|(at, _)| at + 1)
        .collect()
}

#[test]
fn no_child_process_is_started_without_the_quiet_helper() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in ["desktop", "tracker"] {
        sources(&root.join(krate).join("src"), &mut files);
    }
    assert!(files.len() > 10, "the scan found no sources");
    let found: Vec<String> = files
        .iter()
        .filter(|f| f.file_name().is_some_and(|n| n != HELPER) && !is_test_file(f))
        .flat_map(|f| {
            let text = fs::read_to_string(f).expect("source file");
            direct_spawns(&text)
                .into_iter()
                .map(|line| format!("{}:{line}", f.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        found.is_empty(),
        "use tracker::quiet::command, not Command::new (a console window flashes): {found:?}"
    );
}

#[test]
fn the_scan_sees_a_direct_spawn_before_the_test_module_only() {
    let text =
        "fn a() {\n    Command::new(x);\n}\n#[cfg(test)]\nmod t { fn b() { Command::new(y); } }\n";
    assert_eq!(direct_spawns(text), vec![2]);
}
