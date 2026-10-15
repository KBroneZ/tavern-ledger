//! The game's minion textures (D-150, D-151, T-372): a small set of images
//! hosted on our own Cloudflare Pages project, one folder per extraction
//! (`v2` now), listed by a manifest with a SHA-256 for every file. The app
//! downloads the set once, keeps it under `card-cache/textures/<version>/` and
//! serves it to its pages. Nothing is bundled in the installer or the repo.
//!
//! What this file guarantees:
//! - Only the one host and folder in [`BASE_URL`] is asked, with the app's
//!   identifiable User-Agent, a timeout, and no redirect followed (a 3xx or a
//!   403 is a plain miss).
//! - The manifest lists bare file names only; every file has a size cap and
//!   its hash is checked; the whole set has a size cap.
//! - Files are written to a side folder and the set switches only when every
//!   file passed, so a page never sees half a set. One download at a time.
//! - Leftovers of an interrupted download are removed at start. A file that
//!   no longer matches its hash on disk is left out (that layer is missing,
//!   never a wrong image) and the set is fetched again.
//!
//! The textures are the game's own art: they are kept on the player's PC
//! under Blizzard's Fan Content Policy and used only for display.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::net::{Fetch, Get, NetError};

/// The texture set this build of the app knows (`v2` is the one folder the
/// app asks for, on the host and in the cache). A later set is this one
/// line.
macro_rules! set_version {
    () => {
        "v2"
    };
}
pub const VERSION: &str = set_version!();
pub const BASE_URL: &str = concat!("https://assets.tavernledger.net/", set_version!(), "/");
const MANIFEST_FILE: &str = "manifest.json";
const LAYOUT_NAME: &str = "layout";
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
/// The biggest image of v1 is 78 KB.
pub const MAX_FILE_BYTES: u64 = 512 * 1024;
/// The whole of v1 is 640 KB.
pub const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_FILES: usize = 64;
/// After a failed download, no new request for this long.
pub const RETRY_AFTER_SECS: u64 = 900;
const MAX_NAME_LEN: usize = 64;

/// One row of the manifest, after the checks.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    name: String,
    file: String,
    sha256: String,
    bytes: u64,
}

#[derive(Deserialize)]
struct RawManifest {
    version: String,
    files: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    name: String,
    file: String,
    sha256: String,
    bytes: u64,
}

/// What a page gets: the layout (positions of every layer) and which
/// textures exist, by role name to file name.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TextureView {
    pub version: &'static str,
    pub layout: serde_json::Value,
    pub files: BTreeMap<String, String>,
}

/// Lower-case hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The media type of a file of the set, from its extension only.
pub fn content_type(file: &str) -> Option<&'static str> {
    match file.rsplit_once('.')?.1 {
        "webp" => Some("image/webp"),
        "png" => Some("image/png"),
        "json" => Some("application/json"),
        _ => None,
    }
}

/// Names Windows treats as devices whatever the extension (`CON.webp` opens
/// the console, not a file).
fn is_device_name(file: &str) -> bool {
    let stem = file.split('.').next().unwrap_or("").to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && n.as_bytes()[0].is_ascii_digit())
    })
}

/// A bare file name: letters, digits, `_`, `-` and one extension we serve.
/// No separator, drive letter, dot-prefix, space, `..` or device name.
fn valid_file(file: &str) -> bool {
    !is_device_name(file)
        && file.len() <= MAX_NAME_LEN
        && !file.starts_with('.')
        && file.matches('.').count() == 1
        && file
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
        && content_type(file).is_some()
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The manifest's rows, or why it is refused.
fn parse_manifest(body: &[u8]) -> Result<Vec<Entry>, String> {
    let raw: RawManifest =
        serde_json::from_slice(body).map_err(|_| "the manifest is not readable".to_string())?;
    if raw.version != VERSION {
        return Err(format!(
            "the manifest is for {}, not {VERSION}",
            raw.version
        ));
    }
    if raw.files.is_empty() || raw.files.len() > MAX_FILES {
        return Err("the manifest lists too many or no files".into());
    }
    let mut entries: Vec<Entry> = Vec::with_capacity(raw.files.len());
    let mut total = 0u64;
    for row in raw.files {
        if !valid_name(&row.name) || !valid_file(&row.file) || !valid_hash(&row.sha256) {
            return Err("the manifest has a file name or hash that is not allowed".into());
        }
        if row.bytes == 0 || row.bytes > MAX_FILE_BYTES {
            return Err(format!("{} is not an allowed size", row.file));
        }
        let seen = entries
            .iter()
            .any(|e| e.name == row.name || e.file.eq_ignore_ascii_case(&row.file));
        if seen {
            return Err(format!("{} is listed twice", row.file));
        }
        total += row.bytes;
        entries.push(Entry {
            name: row.name,
            file: row.file,
            sha256: row.sha256,
            bytes: row.bytes,
        });
    }
    if total > MAX_TOTAL_BYTES {
        return Err("the set is bigger than allowed".into());
    }
    if !entries
        .iter()
        .any(|e| e.name == LAYOUT_NAME && e.file.ends_with(".json"))
    {
        return Err("the manifest lists no layout".into());
    }
    Ok(entries)
}

/// The verified files of the set, by file name.
struct Loaded {
    files: BTreeMap<String, Vec<u8>>,
    names: BTreeMap<String, String>,
    layout: serde_json::Value,
    /// Every file listed was present and matched its hash.
    complete: bool,
}

#[derive(Default)]
struct Inner {
    loaded: Option<Arc<Loaded>>,
    last_attempt: Option<u64>,
    problem: Option<String>,
}

pub struct TextureStore {
    dir: PathBuf,
    fetch: Box<dyn Fetch>,
    inner: Mutex<Inner>,
    download_lock: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn set_dir(dir: &Path) -> PathBuf {
    dir.join(VERSION)
}

fn side_dir(dir: &Path) -> PathBuf {
    dir.join(format!("{VERSION}.tmp"))
}

impl TextureStore {
    /// `dir` is `<card cache>/textures`. Removes what an interrupted
    /// download left and reads the saved set if it passes the checks;
    /// nothing is fetched here.
    pub fn open(dir: PathBuf, fetch: Box<dyn Fetch>) -> TextureStore {
        clean_leftovers(&dir);
        let loaded = load(&set_dir(&dir)).map(Arc::new);
        TextureStore {
            dir,
            fetch,
            inner: Mutex::new(Inner {
                loaded,
                ..Inner::default()
            }),
            download_lock: Mutex::new(()),
        }
    }

    /// What the pages need, or None while there is no usable set.
    pub fn view(&self) -> Option<TextureView> {
        let loaded = lock(&self.inner).loaded.clone()?;
        Some(TextureView {
            version: VERSION,
            layout: loaded.layout.clone(),
            files: loaded.names.clone(),
        })
    }

    /// A file of the set and its media type; None for anything else.
    pub fn file(&self, file: &str) -> Option<(Vec<u8>, &'static str)> {
        let loaded = lock(&self.inner).loaded.clone()?;
        let bytes = loaded.files.get(file)?.clone();
        Some((bytes, content_type(file)?))
    }

    /// The last thing that went wrong, if the last attempt failed.
    pub fn problem(&self) -> Option<String> {
        lock(&self.inner).problem.clone()
    }

    /// Downloads the set when there is no complete one, at most once per
    /// [`RETRY_AFTER_SECS`] and one download at a time. True when the set
    /// the pages see changed.
    pub fn refresh_if_due(&self, now: u64) -> bool {
        {
            let inner = lock(&self.inner);
            if inner.loaded.as_ref().is_some_and(|l| l.complete) {
                return false;
            }
            let waiting = inner
                .last_attempt
                .is_some_and(|t| now < t.saturating_add(RETRY_AFTER_SECS));
            if waiting {
                return false;
            }
        }
        let guard = match self.download_lock.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => return false,
        };
        lock(&self.inner).last_attempt = Some(now);
        let result = self.download();
        drop(guard);
        let mut inner = lock(&self.inner);
        match result {
            Ok(loaded) => {
                inner.loaded = Some(Arc::new(loaded));
                inner.problem = None;
                true
            }
            Err(problem) => {
                inner.problem = Some(problem);
                false
            }
        }
    }

    fn get(&self, file: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        let request = Get {
            url: format!("{BASE_URL}{file}"),
            gzip: false,
            if_none_match: None,
            max_bytes,
        };
        let reply = self.fetch.get(&request).map_err(|e| match e {
            NetError::Timeout | NetError::Connect(_) => format!("{file}: {e}"),
        })?;
        if reply.status != 200 {
            return Err(format!("{file}: the host answered {}", reply.status));
        }
        if reply.truncated {
            return Err(format!("{file}: longer than allowed"));
        }
        Ok(reply.body)
    }

    /// Fetches the manifest and every file into the side folder, then swaps
    /// the folder in. Any failure removes the side folder and changes nothing.
    fn download(&self) -> Result<Loaded, String> {
        let manifest_body = self.get(MANIFEST_FILE, MAX_MANIFEST_BYTES)?;
        let entries = parse_manifest(&manifest_body)?;
        let side = side_dir(&self.dir);
        let _ = fs::remove_dir_all(&side);
        let written = self.fetch_all(&entries, &manifest_body, &side);
        if let Err(problem) = written {
            let _ = fs::remove_dir_all(&side);
            return Err(problem);
        }
        // The new set must be usable before it replaces anything.
        let Some(loaded) = load(&side).filter(|l| l.complete) else {
            let _ = fs::remove_dir_all(&side);
            return Err("the downloaded textures could not be read".into());
        };
        self.swap_in(&side)?;
        Ok(loaded)
    }

    /// Puts the side folder in place of the live one, keeping the old one
    /// until the new one is in (and putting it back if that fails).
    fn swap_in(&self, side: &Path) -> Result<(), String> {
        let live = set_dir(&self.dir);
        let old = self.dir.join(format!("{VERSION}.old"));
        let _ = fs::remove_dir_all(&old);
        let had_live = live.exists();
        let result = (if had_live {
            fs::rename(&live, &old)
        } else {
            Ok(())
        })
        .and_then(|()| {
            fs::rename(side, &live).inspect_err(|_| {
                if had_live {
                    let _ = fs::rename(&old, &live);
                }
            })
        });
        match result {
            Ok(()) => {
                let _ = fs::remove_dir_all(&old);
                Ok(())
            }
            Err(e) => {
                let _ = fs::remove_dir_all(side);
                Err(format!("the textures could not be saved ({})", e.kind()))
            }
        }
    }

    fn fetch_all(&self, entries: &[Entry], manifest: &[u8], side: &Path) -> Result<(), String> {
        fs::create_dir_all(side).map_err(|e| format!("no room for the textures ({})", e.kind()))?;
        for entry in entries {
            let body = self.get(&entry.file, entry.bytes as usize)?;
            if body.len() as u64 != entry.bytes || sha256_hex(&body) != entry.sha256 {
                return Err(format!("{} does not match the manifest", entry.file));
            }
            write_file(&side.join(&entry.file), &body)?;
        }
        write_file(&side.join(MANIFEST_FILE), manifest)
    }

    #[cfg(test)]
    pub(crate) fn hold_download_for_test(&self) -> MutexGuard<'_, ()> {
        lock(&self.download_lock)
    }
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| format!("the textures could not be saved ({})", e.kind()))
}

/// Removes the side folders and half-written files of an earlier run.
fn clean_leftovers(dir: &Path) {
    let _ = fs::remove_dir_all(side_dir(dir));
    let _ = fs::remove_dir_all(dir.join(format!("{VERSION}.old")));
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "part") {
            let _ = fs::remove_file(path);
        }
    }
}

/// Reads the saved set: the manifest again checked, every file checked
/// against its hash. A file that fails is left out (and the set is marked
/// incomplete); a missing manifest or layout is no set.
fn load(dir: &Path) -> Option<Loaded> {
    let manifest = fs::read(dir.join(MANIFEST_FILE)).ok()?;
    let entries = parse_manifest(&manifest).ok()?;
    let mut files = BTreeMap::new();
    let mut names = BTreeMap::new();
    let mut complete = true;
    for entry in &entries {
        let ok = fs::read(dir.join(&entry.file))
            .ok()
            .filter(|b| b.len() as u64 == entry.bytes && sha256_hex(b) == entry.sha256);
        match ok {
            Some(bytes) => {
                files.insert(entry.file.clone(), bytes);
                names.insert(entry.name.clone(), entry.file.clone());
            }
            None => complete = false,
        }
    }
    let layout_file = names.get(LAYOUT_NAME)?;
    let layout: serde_json::Value = serde_json::from_slice(files.get(layout_file)?).ok()?;
    if !layout.is_object() {
        return None;
    }
    names.remove(LAYOUT_NAME);
    Some(Loaded {
        files,
        names,
        layout,
        complete,
    })
}
