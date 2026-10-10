//! Card names and art for the app (T-304, D-038). Few requests: one card
//! data file, refreshed at most once an hour and only when it is a week old
//! or the log showed a card it does not have (with the ETag, so an unchanged
//! file costs a 304); one small image per card, only when the app is about
//! to show it, and never for an id the card data does not know. Anything
//! that fails leaves the app showing the card id, never a guessed name or
//! picture.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;

use crate::art::{is_jpeg, ArtCache, CACHE_CAP_BYTES, MAX_IMAGE_BYTES};
use crate::catalog::{
    gunzip, valid_etag, valid_id, Catalog, PoolMinion, SavedCatalog, MAX_INFLATED,
};
use crate::net::{Fetch, Get, Reply};

pub const DATA_URL: &str = "https://api.hearthstonejson.com/v1/latest/enUS/cards.json";
const ART_BASE: &str = "https://art.hearthstonejson.com/v1/256x/";
/// The gzip download is about 1.6 MB.
pub const MAX_DATA_BYTES: usize = 16 * 1024 * 1024;
/// A card data file this old is checked again.
pub const REFRESH_AFTER_SECS: u64 = 7 * 24 * 3600;
/// Never more than one card data request an hour, failed or not.
pub const RETRY_AFTER_SECS: u64 = 3600;
/// An image that could not be fetched is not asked for again for an hour.
pub const ART_RETRY_AFTER_SECS: u64 = 3600;
/// After the art server could not be reached, no image is asked for this long.
pub const ART_OFFLINE_SECS: u64 = 300;
/// At most this many images are fetched in any hour (a full lobby is ~60).
pub const ART_PER_HOUR: u32 = 300;
const CATALOG_FILE: &str = "catalog.json";
/// When the card data was last asked for, so a restart does not ask again.
const ATTEMPT_FILE: &str = "last-attempt.txt";

pub fn art_url(id: &str) -> String {
    format!("{ART_BASE}{id}.jpg")
}

/// What the window says about the card data, in plain words.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CardStatus {
    /// Number of card names known; 0 when there is no card data.
    pub cards: usize,
    /// Seconds since 1970 when the server last confirmed the names.
    pub fetched_at: Option<u64>,
    /// The last thing that went wrong, if the last attempt failed.
    pub problem: Option<String>,
}

#[derive(Default)]
struct Inner {
    saved: Option<SavedCatalog>,
    last_attempt: Option<u64>,
    problem: Option<String>,
    /// The app asked for a card the data does not have: maybe a new patch.
    want_newer: bool,
    art_misses: HashMap<String, u64>,
    /// No image is fetched before this time (the server was unreachable).
    art_offline_until: u64,
    /// Start of the current hour of image fetches, and how many so far.
    art_hour: (u64, u32),
}

pub struct CardStore {
    dir: PathBuf,
    fetch: Box<dyn Fetch>,
    inner: Mutex<Inner>,
    /// One image request at a time.
    art_lock: Mutex<()>,
    data_lock: Mutex<()>,
    art: ArtCache,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl CardStore {
    /// `dir` is the cache folder on the PC (`<data folder>/card-cache`).
    /// Reads the saved card data if it passes the checks; nothing is fetched here.
    pub fn open(dir: PathBuf, fetch: Box<dyn Fetch>) -> CardStore {
        CardStore::with_cap(dir, fetch, CACHE_CAP_BYTES)
    }

    pub fn with_cap(dir: PathBuf, fetch: Box<dyn Fetch>, cap: u64) -> CardStore {
        let mut inner = Inner::default();
        // A missing file is a first start: the data is fetched. A file far
        // larger than the real one is not read at all.
        let path = dir.join(CATALOG_FILE);
        let small = fs::metadata(&path).is_ok_and(|m| m.len() <= MAX_INFLATED as u64);
        if let Some(bytes) = small.then(|| fs::read(&path).ok()).flatten() {
            match SavedCatalog::from_json(&bytes) {
                Ok(saved) => inner.saved = Some(saved),
                Err(e) => {
                    inner.problem = Some(format!(
                        "The saved card data could not be read ({e}); it will be fetched again."
                    ))
                }
            }
        }
        inner.last_attempt = fs::read_to_string(dir.join(ATTEMPT_FILE))
            .ok()
            .and_then(|text| text.trim().parse().ok());
        let art = ArtCache::new(dir.join("art"), cap);
        CardStore {
            dir,
            fetch,
            inner: Mutex::new(inner),
            art_lock: Mutex::new(()),
            data_lock: Mutex::new(()),
            art,
        }
    }

    pub fn status(&self) -> CardStatus {
        let inner = lock(&self.inner);
        CardStatus {
            cards: inner.saved.as_ref().map_or(0, |s| s.catalog.len()),
            fetched_at: inner.saved.as_ref().map(|s| s.fetched_at),
            problem: inner.problem.clone(),
        }
    }

    /// True once there are card names to look up.
    pub fn has_data(&self) -> bool {
        lock(&self.inner).saved.is_some()
    }

    /// The card's name, or None (the app then shows the id, "unknown").
    pub fn name(&self, id: &str) -> Option<String> {
        if !valid_id(id) {
            return None;
        }
        let mut inner = lock(&self.inner);
        let found = inner
            .saved
            .as_ref()
            .and_then(|s| s.catalog.name(id))
            .map(String::from);
        if found.is_none() && inner.saved.is_some() {
            inner.want_newer = true;
        }
        found
    }

    /// The Battlegrounds pool minions (T-307), empty while there is no card
    /// data or the data has no pool.
    pub fn pool(&self) -> Vec<(String, PoolMinion)> {
        lock(&self.inner)
            .saved
            .as_ref()
            .map(|s| {
                s.catalog
                    .pool()
                    .iter()
                    .map(|(id, m)| (id.clone(), m.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn knows(&self, id: &str) -> bool {
        lock(&self.inner)
            .saved
            .as_ref()
            .is_some_and(|s| s.catalog.contains(id))
    }

    fn due(&self, now: u64) -> bool {
        let inner = lock(&self.inner);
        // A time in the future means the clock was wrong then: not trusted.
        let waited = inner
            .last_attempt
            .is_none_or(|t| t > now || now - t >= RETRY_AFTER_SECS);
        let stale = match &inner.saved {
            None => true,
            Some(s) => {
                s.fetched_at > now || now - s.fetched_at >= REFRESH_AFTER_SECS || inner.want_newer
            }
        };
        waited && stale
    }

    /// Fetches the card data when it is missing, a week old or missing a
    /// card the log showed, at most once an hour. True when the names changed.
    pub fn refresh_if_due(&self, now: u64) -> bool {
        let _one = lock(&self.data_lock);
        if !self.due(now) {
            return false;
        }
        let etag = {
            let mut inner = lock(&self.inner);
            inner.last_attempt = Some(now);
            inner.want_newer = false;
            let _ = fs::create_dir_all(&self.dir)
                .and_then(|()| fs::write(self.dir.join(ATTEMPT_FILE), now.to_string()));
            inner.saved.as_ref().and_then(|s| s.etag.clone())
        };
        let request = Get {
            url: DATA_URL.into(),
            gzip: true,
            if_none_match: etag,
            max_bytes: MAX_DATA_BYTES,
        };
        let result = self
            .fetch
            .get(&request)
            .map_err(|e| format!("Could not get the card data from HearthstoneJSON: {e}."))
            .and_then(|reply| self.take_data(reply, now));
        let mut inner = lock(&self.inner);
        match result {
            Ok(Some(saved)) => {
                inner.saved = Some(saved);
                inner.problem = None;
                true
            }
            Ok(None) => {
                inner.problem = None;
                false
            }
            Err(problem) => {
                inner.problem = Some(problem);
                false
            }
        }
    }

    /// Ok(None): the server says our copy is current (304).
    fn take_data(&self, reply: Reply, now: u64) -> Result<Option<SavedCatalog>, String> {
        let bad = |why: &str| Err(format!("HearthstoneJSON's card data was not used: {why}."));
        if reply.status == 304 {
            let mut inner = lock(&self.inner);
            let Some(saved) = inner.saved.as_mut() else {
                return bad("the server said unchanged, but there is no saved copy");
            };
            saved.fetched_at = now;
            let _ = self.save(saved);
            return Ok(None);
        }
        if reply.status != 200 {
            return bad(&format!("the server answered {}", reply.status));
        }
        if reply.truncated {
            return bad("the file is larger than expected");
        }
        let json_type = reply.content_type.as_deref().is_some_and(|t| {
            t.trim()
                .to_ascii_lowercase()
                .starts_with("application/json")
        });
        if !json_type {
            return bad("it is not JSON");
        }
        let json = match reply
            .content_encoding
            .as_deref()
            .map(str::to_ascii_lowercase)
        {
            None => reply.body,
            Some(e) if e == "identity" => reply.body,
            Some(e) if e == "gzip" => match gunzip(&reply.body) {
                Ok(json) => json,
                Err(e) => return bad(&e.to_string()),
            },
            Some(_) => return bad("unexpected compression"),
        };
        let catalog = match Catalog::from_hearthstonejson(&json) {
            Ok(catalog) => catalog,
            Err(e) => return bad(&e.to_string()),
        };
        let saved = SavedCatalog {
            catalog,
            fetched_at: now,
            etag: reply.etag.filter(|e| valid_etag(e)),
        };
        // Not saving only costs a download next time; the names still show.
        let _ = self.save(&saved);
        Ok(Some(saved))
    }

    fn save(&self, saved: &SavedCatalog) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(CATALOG_FILE);
        let side = path.with_extension("json.part");
        fs::write(&side, saved.to_json())?;
        fs::rename(&side, &path)
    }

    /// The card's 256x art (JPEG bytes), from the PC or fetched now. None
    /// for an id the card data does not know, an image the server does not
    /// have or an answer that is not a JPEG: the app then shows no picture.
    pub fn art(&self, id: &str, now: u64) -> Option<Vec<u8>> {
        if !valid_id(id) {
            return None;
        }
        if let Some(bytes) = self.art.read(id) {
            return Some(bytes);
        }
        if !self.knows(id) || !self.may_fetch_art(id, now) {
            return None;
        }
        let _one = lock(&self.art_lock);
        // Another request may have fetched it while this one waited.
        if let Some(bytes) = self.art.read(id) {
            return Some(bytes);
        }
        if !self.take_art_slot(id, now) {
            return None;
        }
        let request = Get {
            url: art_url(id),
            gzip: false,
            if_none_match: None,
            max_bytes: MAX_IMAGE_BYTES,
        };
        let answer = self.fetch.get(&request);
        if answer.is_err() {
            lock(&self.inner).art_offline_until = now + ART_OFFLINE_SECS;
        }
        let image = answer.ok().filter(|r| {
            r.status == 200
                && !r.truncated
                && r.content_type
                    .as_deref()
                    .is_some_and(|t| t.trim().eq_ignore_ascii_case("image/jpeg"))
                && is_jpeg(&r.body)
        });
        match image {
            Some(reply) => {
                // Not caching only costs a download next time.
                let _ = self.art.write(id, &reply.body);
                Some(reply.body)
            }
            None => {
                let mut inner = lock(&self.inner);
                if inner.art_misses.len() > 10_000 {
                    inner.art_misses.clear();
                }
                inner.art_misses.insert(id.to_string(), now);
                None
            }
        }
    }

    /// Not a recent miss, the server not just found unreachable, and the
    /// hourly budget not spent.
    fn may_fetch_art(&self, id: &str, now: u64) -> bool {
        let inner = lock(&self.inner);
        let missed = inner
            .art_misses
            .get(id)
            .is_some_and(|&t| t <= now && now - t < ART_RETRY_AFTER_SECS);
        let (start, used) = inner.art_hour;
        let spent = start <= now && now - start < 3600 && used >= ART_PER_HOUR;
        !missed && now >= inner.art_offline_until && !spent
    }

    /// Checks again under the art lock and counts the request.
    fn take_art_slot(&self, id: &str, now: u64) -> bool {
        if !self.may_fetch_art(id, now) {
            return false;
        }
        let mut inner = lock(&self.inner);
        let (start, used) = inner.art_hour;
        inner.art_hour = if start <= now && now - start < 3600 {
            (start, used + 1)
        } else {
            (now, 1)
        };
        true
    }
}
