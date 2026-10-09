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

use bg_parser::{PARSER_REVISION, PARSER_VERSION};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const FILE_NAME: &str = "games.jsonl";

/// %APPDATA%\TavernLedger on Windows (D-015); .local/ elsewhere (development).
pub fn default_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(|d| PathBuf::from(d).join("TavernLedger"))
        .unwrap_or_else(|| PathBuf::from(".local"))
}

/// Which parser wrote a record (T-107). Records from before this existed
/// have none and load as "unknown version".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserStamp {
    pub version: String,
    pub revision: u32,
}

impl ParserStamp {
    pub fn current() -> Self {
        ParserStamp {
            version: PARSER_VERSION.to_string(),
            revision: PARSER_REVISION,
        }
    }
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
    parsers: BTreeMap<GameKey, ParserStamp>,
    /// Lines that could not be read (e.g. cut short by a crash). Reported,
    /// never silently dropped, and left in the file untouched.
    pub unreadable_lines: usize,
    /// The file ends without a newline (a write cut short): the next record
    /// must start on a new line or it would be glued to the broken one.
    needs_newline: bool,
}

impl Store {
    pub fn open(dir: &Path) -> io::Result<Store> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let mut store = Store {
            path,
            games: BTreeMap::new(),
            parsers: BTreeMap::new(),
            unreadable_lines: 0,
            needs_newline: false,
        };
        match File::open(&store.path) {
            Ok(file) => store.load(BufReader::new(file))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(store)
    }

    fn load(&mut self, mut input: impl BufRead) -> io::Result<()> {
        let mut raw = Vec::new();
        loop {
            raw.clear();
            if input.read_until(b'\n', &mut raw)? == 0 {
                return Ok(());
            }
            self.needs_newline = !raw.ends_with(b"\n");
            // Invalid UTF-8 makes one unreadable line, not an unreadable history.
            let line = String::from_utf8_lossy(&raw);
            if line.trim().is_empty() {
                continue;
            }
            match parse_record(&line) {
                Some((key, report, parser)) => {
                    match parser {
                        Some(stamp) => self.parsers.insert(key.clone(), stamp),
                        None => self.parsers.remove(&key),
                    };
                    self.games.insert(key, report);
                }
                None => self.unreadable_lines += 1,
            }
        }
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
            "parser": ParserStamp::current(),
            "report": report,
        });
        let mut line = if self.needs_newline {
            "\n".to_string()
        } else {
            String::new()
        };
        line.push_str(&record.to_string());
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        // One write per record, so two processes appending at once cannot
        // interleave pieces of their lines.
        file.write_all(line.as_bytes())?;
        file.flush()?;
        self.needs_newline = false;
        self.parsers.insert(key.clone(), ParserStamp::current());
        self.games.insert(key, report);
        Ok(true)
    }

    /// Latest report of every game, oldest session first.
    pub fn games(&self) -> impl Iterator<Item = (&GameKey, &Value)> {
        self.games.iter()
    }

    /// Latest report of one game.
    pub fn report(&self, key: &GameKey) -> Option<&Value> {
        self.games.get(key)
    }

    /// The parser that wrote the latest record of a game; `None` is
    /// "unknown version" (a record from before the version was saved).
    pub fn parser(&self, key: &GameKey) -> Option<&ParserStamp> {
        self.parsers.get(key)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn parse_record(line: &str) -> Option<(GameKey, Value, Option<ParserStamp>)> {
    let mut value: Value = serde_json::from_str(line).ok()?;
    let session = value.get("session")?.as_str()?.to_string();
    let index = value.get("index")?.as_u64()?;
    // A missing or malformed stamp is "unknown version", never an error.
    let parser = value
        .get("parser")
        .and_then(|p| serde_json::from_value(p.clone()).ok());
    let report = value.get_mut("report")?.take();
    report
        .is_object()
        .then_some((GameKey { session, index }, report, parser))
}
