//! Which games still need uploading. `uploads.jsonl`, next to the history,
//! keeps one line per answer from the server: the game, the account, the
//! SHA-256 of the exact JSON sent and whether it was stored or refused. A game
//! is waiting when its current record has no mark for this account and these
//! bytes, so a game re-imported locally (the last record wins, D-015) is sent
//! again, and a game the server refused is not retried until it changes.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracker::dev_reconnect::Reconnects;
use tracker::store::{GameKey, Store};

use crate::pkce::sha256_hex;

pub const FILE_NAME: &str = "uploads.jsonl";
/// Modes the server stores; other Battlegrounds modes (friendly games) stay
/// on this PC.
pub const UPLOADED_MODES: [&str; 2] = ["GT_BATTLEGROUNDS", "GT_BATTLEGROUNDS_DUO"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Uploaded,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub session: String,
    pub index: u64,
    pub user: String,
    pub sha256: String,
    pub outcome: Outcome,
    /// The server's error code for a refused game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub at: u64,
}

pub struct Marks {
    path: PathBuf,
    latest: BTreeMap<GameKey, Mark>,
    needs_newline: bool,
}

impl Marks {
    pub fn open(dir: &Path) -> io::Result<Marks> {
        let path = dir.join(FILE_NAME);
        let mut marks = Marks {
            path,
            latest: BTreeMap::new(),
            needs_newline: false,
        };
        let file = match File::open(&marks.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(marks),
            Err(e) => return Err(e),
        };
        let mut reader = BufReader::new(file);
        let mut raw = Vec::new();
        loop {
            raw.clear();
            if reader.read_until(b'\n', &mut raw)? == 0 {
                break;
            }
            marks.needs_newline = !raw.ends_with(b"\n");
            // A broken line (a crash mid-write) only means that game is sent again.
            if let Ok(mark) = serde_json::from_slice::<Mark>(&raw) {
                let key = GameKey {
                    session: mark.session.clone(),
                    index: mark.index,
                };
                marks.latest.insert(key, mark);
            }
        }
        Ok(marks)
    }

    pub fn get(&self, key: &GameKey) -> Option<&Mark> {
        self.latest.get(key)
    }

    pub fn record(&mut self, mark: Mark) -> io::Result<()> {
        let mut line = if self.needs_newline {
            "\n".to_string()
        } else {
            String::new()
        };
        line.push_str(&serde_json::to_string(&mark).map_err(io::Error::other)?);
        line.push('\n');
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.flush()?;
        self.needs_newline = false;
        let key = GameKey {
            session: mark.session.clone(),
            index: mark.index,
        };
        self.latest.insert(key, mark);
        Ok(())
    }
}

/// One game ready to send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub key: GameKey,
    /// The JSON sent: `{session, index, saved_at, parser, report}`.
    pub body: Vec<u8>,
    pub sha256: String,
}

impl Item {
    pub fn gzip(&self) -> io::Result<Vec<u8>> {
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&self.body)?;
        enc.finish()
    }
}

/// What the app shows about the history.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub waiting: usize,
    pub uploaded: usize,
    pub rejected: usize,
    /// Friendly and other modes the server does not store.
    pub not_uploadable: usize,
    /// Games played with the reconnect dev tool (D-043): never uploaded.
    pub dev_reconnect: usize,
}

/// The server's address pattern: a log folder name and a game number 1-1000.
pub fn is_upload_key(key: &GameKey) -> bool {
    let s = key.session.as_bytes();
    let digits = |r: std::ops::Range<usize>| s[r].iter().all(u8::is_ascii_digit);
    (1..=1000).contains(&key.index)
        && s.len() == 31
        && key.session.starts_with("Hearthstone_")
        && digits(12..16)
        && [16, 19, 22, 25, 28]
            .iter()
            .all(|&i| s[i] == b'_' && digits(i + 1..i + 3))
}

/// The record as uploaded. serde_json sorts object keys, so the same record
/// always gives the same bytes and the same hash.
pub fn body_of(store: &Store, key: &GameKey, report: &Value) -> Vec<u8> {
    let mut record = json!({
        "session": key.session,
        "index": key.index,
        "saved_at": store.saved_at(key).unwrap_or(0),
        "report": report,
    });
    if let Some(stamp) = store.parser(key) {
        record["parser"] = json!(stamp);
    }
    record.to_string().into_bytes()
}

/// Games waiting for `user`, oldest first, and the counts for the app.
/// Games played with the reconnect dev tool are never waiting.
pub fn pending(
    store: &Store,
    marks: &Marks,
    reconnects: &Reconnects,
    user: Option<&str>,
) -> (Vec<Item>, Counts) {
    let mut items = Vec::new();
    let mut counts = Counts::default();
    for (key, report) in store.games() {
        if reconnects.marks(store.played(key)) {
            counts.dev_reconnect += 1;
            continue;
        }
        let mode = report
            .get("game_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !UPLOADED_MODES.contains(&mode) || !is_upload_key(key) {
            counts.not_uploadable += 1;
            continue;
        }
        let body = body_of(store, key, report);
        let sha256 = sha256_hex(&body);
        let mark = marks
            .get(key)
            .filter(|m| Some(m.user.as_str()) == user && m.sha256 == sha256);
        match mark.map(|m| m.outcome) {
            Some(Outcome::Uploaded) => counts.uploaded += 1,
            Some(Outcome::Rejected) => counts.rejected += 1,
            None => {
                counts.waiting += 1;
                items.push(Item {
                    key: key.clone(),
                    body,
                    sha256,
                });
            }
        }
    }
    (items, counts)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    pub fn temp_dir(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tl-uploader-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    pub fn report(index: u64, mode: &str) -> Value {
        json!({"index": index, "status": "ok", "game_type": mode, "hero": "H", "final_place": 1})
    }

    pub fn store_with(dir: &Path, games: &[(&str, u64, &str)]) -> Store {
        let mut store = Store::open(dir).unwrap();
        for (session, index, mode) in games {
            let key = GameKey {
                session: session.to_string(),
                index: *index,
            };
            store.save(key, report(*index, mode)).unwrap();
        }
        store
    }

    const S1: &str = "Hearthstone_2026_10_01_10_00_00";
    const S2: &str = "Hearthstone_2026_10_02_10_00_00";

    fn mark(item: &Item, user: &str, outcome: Outcome) -> Mark {
        Mark {
            session: item.key.session.clone(),
            index: item.key.index,
            user: user.into(),
            sha256: item.sha256.clone(),
            outcome,
            code: None,
            at: 1,
        }
    }

    #[test]
    fn games_played_with_the_reconnect_dev_tool_are_never_uploaded() {
        use tracker::game_clock::Played;
        let dir = temp_dir("dev-reconnect");
        let mut store = Store::open(&dir).unwrap();
        // 2026-10-01 08:00-08:20 and 09:00-09:20 UTC.
        for (index, from) in [(1, 1_790_841_600), (2, 1_790_845_200)] {
            let key = GameKey {
                session: S1.into(),
                index,
            };
            let played = Some(Played {
                from,
                to: from + 1200,
            });
            let report = report(index, "GT_BATTLEGROUNDS");
            store.save_played(key, report, played).unwrap();
        }
        let reconnects = Reconnects::parse("{\"utc\": \"2026-10-01T08:10:00Z\"}\n");
        let (items, counts) = pending(&store, &Marks::open(&dir).unwrap(), &reconnects, Some("u"));
        assert_eq!(items.iter().map(|i| i.key.index).collect::<Vec<_>>(), [2]);
        assert_eq!(counts.dev_reconnect, 1);
        assert_eq!(counts.waiting, 1);
    }

    #[test]
    fn waiting_games_come_oldest_first_and_other_modes_stay_local() {
        let dir = temp_dir("pending");
        let store = store_with(
            &dir,
            &[
                (S2, 1, "GT_BATTLEGROUNDS"),
                (S1, 2, "GT_BATTLEGROUNDS_DUO"),
                (S1, 1, "GT_BATTLEGROUNDS_FRIENDLY"),
            ],
        );
        let marks = Marks::open(&dir).unwrap();
        let (items, counts) = pending(&store, &marks, &Reconnects::default(), Some("u"));
        let keys: Vec<_> = items
            .iter()
            .map(|i| (i.key.session.as_str(), i.key.index))
            .collect();
        assert_eq!(keys, vec![(S1, 2), (S2, 1)]);
        assert_eq!(
            counts,
            Counts {
                waiting: 2,
                uploaded: 0,
                rejected: 0,
                not_uploadable: 1,
                dev_reconnect: 0
            }
        );
    }

    #[test]
    fn only_keys_the_server_accepts_are_sent() {
        let key = |session: &str, index| GameKey {
            session: session.into(),
            index,
        };
        assert!(is_upload_key(&key(S1, 1)));
        assert!(is_upload_key(&key(S1, 1000)));
        for (session, index) in [
            (S1, 0),
            (S1, 1001),
            ("S", 1),
            ("Hearthstone_2026_10_01_10_00", 1),
            ("Hearthstone_2026_10_01_10_00_0x", 1),
            ("../Hearthstone_2026_10_01_1", 1),
        ] {
            assert!(!is_upload_key(&key(session, index)), "{session} {index}");
        }
    }

    #[test]
    fn the_body_is_the_record_with_its_revision_and_parser() {
        let dir = temp_dir("body");
        let store = store_with(&dir, &[(S1, 1, "GT_BATTLEGROUNDS")]);
        let (items, _) = pending(
            &store,
            &Marks::open(&dir).unwrap(),
            &Reconnects::default(),
            None,
        );
        let body: Value = serde_json::from_slice(&items[0].body).unwrap();
        let key = GameKey {
            session: S1.into(),
            index: 1,
        };
        assert_eq!(body["session"], S1);
        assert_eq!(body["index"], 1);
        assert_eq!(body["saved_at"], store.saved_at(&key).unwrap());
        assert_eq!(body["parser"]["revision"], bg_parser_revision());
        assert_eq!(body["report"], report(1, "GT_BATTLEGROUNDS"));
        assert_eq!(items[0].sha256, sha256_hex(&items[0].body));
        let unzipped = {
            use std::io::Read;
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(&items[0].gzip().unwrap()[..])
                .read_to_end(&mut out)
                .unwrap();
            out
        };
        assert_eq!(unzipped, items[0].body);
    }

    fn bg_parser_revision() -> u32 {
        tracker::store::ParserStamp::current().revision
    }

    #[test]
    fn marks_survive_a_restart_and_only_count_for_the_same_user_and_bytes() {
        let dir = temp_dir("marks");
        let mut store = store_with(
            &dir,
            &[(S1, 1, "GT_BATTLEGROUNDS"), (S1, 2, "GT_BATTLEGROUNDS")],
        );
        let mut marks = Marks::open(&dir).unwrap();
        let (items, _) = pending(&store, &marks, &Reconnects::default(), Some("u"));
        marks
            .record(mark(&items[0], "u", Outcome::Uploaded))
            .unwrap();
        marks
            .record(mark(&items[1], "u", Outcome::Rejected))
            .unwrap();

        let marks = Marks::open(&dir).unwrap();
        let (left, counts) = pending(&store, &marks, &Reconnects::default(), Some("u"));
        assert!(left.is_empty());
        assert_eq!((counts.uploaded, counts.rejected), (1, 1));

        let (other, _) = pending(&store, &marks, &Reconnects::default(), Some("someone-else"));
        assert_eq!(other.len(), 2, "another account has not got these games");

        // A re-imported game (new record, last one wins) is waiting again.
        let key = GameKey {
            session: S1.into(),
            index: 1,
        };
        let mut changed = report(1, "GT_BATTLEGROUNDS");
        changed["final_place"] = json!(2);
        store.save(key, changed).unwrap();
        let (again, _) = pending(&store, &marks, &Reconnects::default(), Some("u"));
        assert_eq!(
            again.iter().map(|i| i.key.index).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn a_broken_marks_line_only_means_that_game_is_sent_again() {
        let dir = temp_dir("broken");
        let store = store_with(
            &dir,
            &[(S1, 1, "GT_BATTLEGROUNDS"), (S1, 2, "GT_BATTLEGROUNDS")],
        );
        let mut marks = Marks::open(&dir).unwrap();
        let (items, _) = pending(&store, &marks, &Reconnects::default(), Some("u"));
        marks
            .record(mark(&items[0], "u", Outcome::Uploaded))
            .unwrap();
        let mut f = OpenOptions::new()
            .append(true)
            .open(dir.join(FILE_NAME))
            .unwrap();
        f.write_all(b"{\"session\":\"Hearth").unwrap();
        let mut marks = Marks::open(&dir).unwrap();
        marks
            .record(mark(&items[1], "u", Outcome::Uploaded))
            .unwrap();
        let (left, counts) = pending(
            &store,
            &Marks::open(&dir).unwrap(),
            &Reconnects::default(),
            Some("u"),
        );
        assert!(
            left.is_empty(),
            "the record after the broken line is readable"
        );
        assert_eq!(counts.uploaded, 2);
    }
}
