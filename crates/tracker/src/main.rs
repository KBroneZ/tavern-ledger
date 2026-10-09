//! `tavern-watch`: follows Power.log and saves each Battlegrounds game locally.
//!
//!     tavern-watch [--logs-dir PATH] [--data-dir PATH] [--once] [--import]
//!
//! --once     read what is there now, save it and exit
//! --import   also read every older session folder first
//!
//! Prints card ids and places only, never player names. Only reads the
//! game's log files; never touches the game (D-004).

use std::path::PathBuf;
use std::process::ExitCode;
use std::thread::sleep;
use std::time::Duration;

use tracker::discover::{find_logs_dir, is_session_name};
use tracker::lock::{HistoryLock, LockError};
use tracker::store::Store;
use tracker::{import_session, Saved, Watcher};

const POLL_INTERVAL: Duration = Duration::from_secs(1);

struct Args {
    logs_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    once: bool,
    import: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        logs_dir: None,
        data_dir: None,
        once: false,
        import: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--logs-dir" => {
                args.logs_dir = Some(it.next().ok_or("--logs-dir needs a path")?.into())
            }
            "--data-dir" => {
                args.data_dir = Some(it.next().ok_or("--data-dir needs a path")?.into())
            }
            "--once" => args.once = true,
            "--import" => args.import = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(args)
}

fn describe(saved: &Saved) -> String {
    let r = &saved.report;
    let status = serde_json::to_value(r.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from));
    let place = r
        .final_place
        .map(|p| format!("place {p}"))
        .unwrap_or_else(|| "place not available".into());
    format!(
        "{} game {}: {} - {}, {}, {} rounds",
        saved.key.session,
        saved.key.index,
        status.unwrap_or_default(),
        r.hero
            .as_deref()
            .map(|id| r.card_names.get(id).map_or(id, String::as_str))
            .unwrap_or("unknown hero"),
        place,
        r.rounds.len()
    )
}

fn import_all(logs_dir: &std::path::Path, store: &mut Store) -> std::io::Result<()> {
    let mut sessions: Vec<PathBuf> = std::fs::read_dir(logs_dir)?
        .filter_map(Result::ok)
        .filter(|e| is_session_name(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    sessions.sort();
    sessions.pop(); // the newest one is followed live
    for dir in sessions {
        for saved in import_session(&dir, store)? {
            println!("Imported {}", describe(&saved));
        }
    }
    Ok(())
}

fn run(args: Args) -> Result<(), String> {
    let logs_dir = args
        .logs_dir
        .or_else(find_logs_dir)
        .ok_or("Hearthstone's Logs folder not found; pass --logs-dir")?;
    let data_dir = args.data_dir.unwrap_or_else(tracker::store::default_dir);
    // Held until the end of run: one writer for the history at a time.
    let _lock = HistoryLock::acquire(&data_dir).map_err(|e| match e {
        LockError::Busy => "The history is in use by another Tavern Ledger process (the app or another tavern-watch). Close it first."
            .to_string(),
        e => e.to_string(),
    })?;
    let mut store = Store::open(&data_dir).map_err(|e| format!("cannot open the history: {e}"))?;
    if store.unreadable_lines > 0 {
        println!(
            "Warning: {} unreadable lines in {}",
            store.unreadable_lines,
            store.path().display()
        );
    }
    println!("Logs: {}", logs_dir.display());
    println!("History: {}", store.path().display());
    if args.import {
        import_all(&logs_dir, &mut store).map_err(|e| format!("import failed: {e}"))?;
    }
    let mut watcher = Watcher::new(&logs_dir, store);
    loop {
        match watcher.poll() {
            Ok(saved) => saved.iter().for_each(|s| println!("Saved {}", describe(s))),
            // Keep going: the game may be rotating or locking the file.
            Err(e) => eprintln!(
                "Could not read the log right now ({:?}); retrying.",
                e.kind()
            ),
        }
        if args.once {
            break;
        }
        sleep(POLL_INTERVAL);
    }
    let (saved, _) = watcher.finish().map_err(|e| format!("cannot save: {e}"))?;
    saved.iter().for_each(|s| println!("Saved {}", describe(s)));
    Ok(())
}

fn main() -> ExitCode {
    match parse_args().and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}
