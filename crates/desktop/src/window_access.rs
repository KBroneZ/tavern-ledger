//! What each window's page may do (T-573): the commands its capability file
//! grants are exactly the ones its scripts call, every registered command is
//! in the manifest that makes the grants count (build.rs), the page's
//! prototypes are frozen and no script writes HTML from strings.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

mod manifest {
    include!("../app_commands.rs");
}
use manifest::APP_COMMANDS;

const MAIN_RS: &str = include_str!("main.rs");
const CONF: &str = include_str!("../tauri.conf.json");
/// Each window, its page and its capability file.
const WINDOWS: [(&str, &str, &str); 2] = [
    ("main", "index.html", "main.json"),
    ("overlay", "overlay.html", "overlay.json"),
];
/// The one call with a command name held in a variable: overlay-layout.js's
/// `send(command, args)`, whose callers all pass a literal name.
const DYNAMIC_CALLS: [(&str, &str); 1] = [("overlay-layout.js", "call(command, args)")];

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// The scripts a page loads, in order: plain ones and `type="module"` ones.
fn scripts(page: &str) -> Vec<String> {
    let html = read(&dir().join("ui").join(page));
    html.split("<script ")
        .skip(1)
        .filter_map(|tag| {
            let tag = &tag[..tag.find('>')?];
            let src = &tag[tag.find("src=\"")? + "src=\"".len()..];
            Some(src[..src.find('"')?].to_string())
        })
        .collect()
}

/// Calls of `invoke`, `call` or `send` (not a method like `x.call`) in a
/// script, with what follows the opening parenthesis.
fn calls(js: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for name in ["invoke(", "call(", "send("] {
        for (at, _) in js.match_indices(name) {
            let before = js[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                continue;
            }
            if js[..at].ends_with("function ") {
                continue;
            }
            out.push(&js[at..]);
        }
    }
    out
}

/// The command names a script passes as a literal string.
fn commands_called(script: &str, js: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for call in calls(js) {
        let args = call[call.find('(').unwrap() + 1..].trim_start();
        if let Some(rest) = args.strip_prefix('"') {
            out.insert(rest[..rest.find('"').unwrap()].to_string());
        } else {
            assert!(
                DYNAMIC_CALLS
                    .iter()
                    .any(|(file, text)| *file == script && call.starts_with(text)),
                "{script}: a command name that is not a literal: {}",
                &call[..call.len().min(40)]
            );
        }
    }
    out
}

/// What a page's scripts call.
fn used_by(page: &str) -> BTreeSet<String> {
    scripts(page)
        .iter()
        .flat_map(|s| commands_called(s, &read(&dir().join("ui").join(s))))
        .collect()
}

fn capability(file: &str) -> Value {
    serde_json::from_str(&read(&dir().join("capabilities").join(file))).unwrap()
}

/// The app commands a capability grants (`allow-list-games` is `list_games`).
fn granted(cap: &Value) -> BTreeSet<String> {
    cap["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p.as_str()?.strip_prefix("allow-"))
        .map(|c| c.replace('-', "_"))
        .collect()
}

#[test]
fn every_registered_command_is_in_the_manifest_once() {
    let start = MAIN_RS.find("generate_handler![").unwrap() + "generate_handler![".len();
    let end = start + MAIN_RS[start..].find(']').unwrap();
    let registered: Vec<&str> = MAIN_RS[start..end]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|path| path.rsplit("::").next().unwrap())
        .collect();
    assert_eq!(names(&registered), names(APP_COMMANDS));
    assert_eq!(
        names(APP_COMMANDS).len(),
        APP_COMMANDS.len(),
        "listed twice"
    );
}

#[test]
fn each_window_is_granted_exactly_the_commands_its_scripts_call() {
    for (window, page, file) in WINDOWS {
        let cap = capability(file);
        assert_eq!(cap["windows"], serde_json::json!([window]), "{file}");
        let used = used_by(page);
        assert!(
            used.is_subset(&names(APP_COMMANDS)),
            "{page} calls a command that is not registered: {:?}",
            used.difference(&names(APP_COMMANDS)).collect::<Vec<_>>()
        );
        let granted = granted(&cap);
        assert_eq!(
            granted.difference(&used).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "{window} may call commands it does not need"
        );
        assert_eq!(
            used.difference(&granted).collect::<Vec<_>>(),
            Vec::<&String>::new(),
            "{window} calls commands it is not granted"
        );
    }
}

#[test]
fn the_overlay_cannot_reach_the_main_window_s_commands() {
    let overlay = granted(&capability("overlay.json"));
    for command in [
        "upload_sign_in",
        "upload_sign_out",
        "upload_set_enabled",
        "save_problem_report",
        "edit_lobby_tribes",
        "game_memory_set_enabled",
        "app_set_autostart",
        "overlay_set_enabled",
        "game_folder_choose",
        "game_folder_reset",
        "game_folder_use_running",
        "game_folder_decline_running",
        "setup_fix",
        "setup_recheck",
        "open_log_folder",
    ] {
        assert!(!overlay.contains(command), "{command}");
    }
}

#[test]
fn only_the_two_windows_have_capabilities_and_may_only_listen_to_events_besides() {
    let files: BTreeSet<String> = fs::read_dir(dir().join("capabilities"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, names(&["main.json", "overlay.json"]));
    for (_, _, file) in WINDOWS {
        let cap = capability(file);
        assert!(cap.get("remote").is_none(), "{file}: no remote origin");
        // Pages listen to the app's events; none sends one.
        const EVENTS: [&str; 2] = ["core:event:allow-listen", "core:event:allow-unlisten"];
        for p in cap["permissions"].as_array().unwrap() {
            let p = p.as_str().expect("a plain permission, no scope");
            assert!(
                EVENTS.contains(&p) || (p.starts_with("allow-") && !p.contains(':')),
                "{file}: {p}"
            );
        }
    }
}

#[test]
fn the_pages_prototypes_are_frozen() {
    let conf: Value = serde_json::from_str(CONF).unwrap();
    assert_eq!(conf["app"]["security"]["freezePrototype"], true);
}

/// Every script under ui/ with its text.
fn all_scripts() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir().join("ui")];
    while let Some(folder) = stack.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "js") {
                let js = read(&path);
                out.push((path, js));
            }
        }
    }
    assert!(out.len() > 20, "the scripts were found");
    out
}

#[test]
fn no_script_writes_html_from_a_string() {
    const SINKS: [&str; 9] = [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
        "eval(",
        "new Function",
        "setTimeout(\"",
        "setInterval(\"",
        "createContextualFragment",
    ];
    for (path, js) in all_scripts() {
        for sink in SINKS {
            assert!(!js.contains(sink), "{}: {sink}", path.display());
        }
    }
}

#[test]
fn no_page_sends_events() {
    // Only listening is granted; a page that sends one would fail at run time.
    for (path, js) in all_scripts() {
        for call in ["emit(", "emitTo("] {
            assert!(!js.contains(call), "{}: {call}", path.display());
        }
    }
}
