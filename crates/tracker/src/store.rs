//! Local game history: one JSON object per line in `games.jsonl`.
//!
//! Each record is keyed by session folder and game index. A game can be saved
//! more than once (at STATE=COMPLETE, again when the next game starts, or
//! after a restart that re-reads the session); the last record wins. Records
//! hold the parser's report only: card ids and lobby player ids, never names.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

pub const FILE_NAME: &str = "games.jsonl";

/// %APPDATA%\TavernLedger on Windows (D-015); .local/ elsewhere (development).
pub fn default_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(|d| PathBuf::from(d).join("TavernLedger"))
        .unwrap_or_else(|| PathBuf::from(".local"))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameKey {
    pub session: String,
    pub index: u64,
}

#[derive(Debug)]
pub struct Store {
    path: PathBuf,
    games: BTreeMap<GameKey, Value>,
    /// Lines that could not be read (e.g. cut short by a crash). Reported,
    /// never silently dropped, and left in the file untouched.
    pub unreadable_lines: usize,
}

impl Store {
    pub fn open(dir: &Path) -> io::Result<Store> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let mut store = Store {
            path,
            games: BTreeMap::new(),
            unreadable_lines: 0,
        };
        match File::open(&store.path) {
            Ok(file) => store.load(BufReader::new(file))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(store)
    }

    fn load(&mut self, input: impl BufRead) -> io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match parse_record(&line) {
                Some((key, report)) => {
                    self.games.insert(key, report);
                }
                None => self.unreadable_lines += 1,
            }
        }
        Ok(())
    }

    /// Saves a report unless the same one is already stored. Returns whether
    /// anything was written.
    pub fn save(&mut self, key: GameKey, report: Value) -> io::Result<bool> {
        if self.games.get(&key) == Some(&report) {
            return Ok(false);
        }
        let saved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let record = json!({
            "session": key.session,
            "index": key.index,
            "saved_at": saved_at,
            "report": report,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{record}")?;
        file.flush()?;
        self.games.insert(key, report);
        Ok(true)
    }

    /// Latest report of every game, oldest session first.
    pub fn games(&self) -> impl Iterator<Item = (&GameKey, &Value)> {
        self.games.iter()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn parse_record(line: &str) -> Option<(GameKey, Value)> {
    let mut value: Value = serde_json::from_str(line).ok()?;
    let session = value.get("session")?.as_str()?.to_string();
    let index = value.get("index")?.as_u64()?;
    let report = value.get_mut("report")?.take();
    report
        .is_object()
        .then_some((GameKey { session, index }, report))
}
