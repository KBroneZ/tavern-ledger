use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;

use crate::art::MAX_IMAGE_BYTES;
use crate::catalog::tests::{gzip, synthetic};
use crate::catalog::MIN_CARDS;
use crate::net::fake::{ok, status, FakeFetch};
use crate::net::{Fetch, Get, NetError, Reply, USER_AGENT};
use crate::store::*;

const NOW: u64 = 1_800_000_000;
const HOUR: u64 = 3600;

/// Lets a test keep a handle on the fake after the store owns it.
struct Shared(Arc<FakeFetch>);

impl Fetch for Shared {
    fn get(&self, request: &Get) -> Result<Reply, NetError> {
        self.0.get(request)
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tl-store-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn store(name: &str) -> (CardStore, Arc<FakeFetch>, PathBuf) {
    let dir = temp_dir(name);
    let fake = Arc::new(FakeFetch::default());
    let store = CardStore::open(dir.clone(), Box::new(Shared(fake.clone())));
    (store, fake, dir)
}

fn data_reply(etag: &str) -> Result<Reply, NetError> {
    let extra = vec![json!({"id": "HERO_X", "name": "Made-up Hero"})];
    Ok(Reply {
        status: 200,
        etag: Some(etag.into()),
        content_type: Some("application/json".into()),
        content_encoding: Some("gzip".into()),
        body: gzip(&synthetic(MIN_CARDS, extra)),
        truncated: false,
    })
}

fn jpeg() -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
    bytes.resize(64, 0x22);
    bytes
}

fn ready(name: &str) -> (CardStore, Arc<FakeFetch>, PathBuf) {
    let (store, fake, dir) = store(name);
    fake.on(DATA_URL, data_reply("\"v1\""));
    assert!(store.refresh_if_due(NOW));
    (store, fake, dir)
}

#[test]
fn with_no_card_data_names_are_unknown_and_nothing_is_fetched_for_art() {
    let (store, fake, _) = store("empty");
    assert_eq!(store.name("CARD_1"), None);
    assert_eq!(store.art("CARD_1", NOW), None);
    assert!(
        fake.urls().is_empty(),
        "no image for an id the data does not know"
    );
    assert_eq!(store.status().cards, 0);
}

#[test]
fn the_card_data_is_fetched_with_gzip_and_names_show() {
    let (store, fake, dir) = ready("fetch");
    let request = fake.requests.lock().unwrap()[0].clone();
    assert_eq!(request.url, DATA_URL);
    assert!(request.gzip);
    assert_eq!(request.if_none_match, None);
    assert_eq!(request.max_bytes, MAX_DATA_BYTES);
    assert_eq!(store.name("HERO_X").as_deref(), Some("Made-up Hero"));
    let status = store.status();
    assert_eq!(
        (status.cards, status.fetched_at, status.problem),
        (MIN_CARDS + 1, Some(NOW), None)
    );
    assert!(dir.join("catalog.json").exists(), "kept on the PC");
}

#[test]
fn the_user_agent_says_who_we_are() {
    assert!(USER_AGENT.starts_with("TavernLedger/"));
    assert!(USER_AGENT.contains("tavernledger.net"));
}

#[test]
fn saved_card_data_is_used_at_start_without_a_request() {
    let (_, _, dir) = ready("saved");
    let fake = Arc::new(FakeFetch::default());
    let again = CardStore::open(dir, Box::new(Shared(fake.clone())));
    assert_eq!(again.name("CARD_3").as_deref(), Some("Card 3"));
    assert!(
        !again.refresh_if_due(NOW + HOUR),
        "a fresh copy is not fetched again"
    );
    assert!(fake.urls().is_empty());
}

#[test]
fn a_week_old_copy_is_checked_with_its_etag_and_a_304_keeps_it() {
    let (store, fake, _) = ready("etag");
    assert!(!store.refresh_if_due(NOW + HOUR), "not stale yet");
    let later = NOW + REFRESH_AFTER_SECS;
    fake.on(DATA_URL, status(304));
    assert!(!store.refresh_if_due(later), "unchanged");
    let last = fake.requests.lock().unwrap().last().cloned().unwrap();
    assert_eq!(last.if_none_match.as_deref(), Some("\"v1\""));
    assert_eq!(store.status().fetched_at, Some(later));
    assert_eq!(store.name("CARD_1").as_deref(), Some("Card 1"));
}

#[test]
fn a_card_the_data_lacks_asks_for_a_newer_file_but_at_most_once_an_hour() {
    let (store, fake, _) = ready("newer");
    assert_eq!(store.name("NEW_CARD"), None);
    assert!(
        !store.refresh_if_due(NOW + 60),
        "within the hour: no request"
    );
    fake.on(DATA_URL, status(304));
    assert!(!store.refresh_if_due(NOW + HOUR));
    assert_eq!(fake.urls().len(), 2);
    assert!(
        !store.refresh_if_due(NOW + HOUR + 60),
        "asked once, not again"
    );
    assert_eq!(fake.urls().len(), 2);
}

#[test]
fn every_failure_keeps_the_old_names_and_says_what_went_wrong() {
    let not_json = Ok(Reply {
        content_type: Some("text/html".into()),
        ..data_reply("x").unwrap()
    });
    let truncated = Ok(Reply {
        truncated: true,
        ..data_reply("x").unwrap()
    });
    let brotli = Ok(Reply {
        content_encoding: Some("br".into()),
        ..data_reply("x").unwrap()
    });
    let bad_gzip = Ok(Reply {
        body: b"not gzip".to_vec(),
        ..data_reply("x").unwrap()
    });
    let wrong_shape = Ok(Reply {
        body: gzip(b"{\"error\": true}"),
        ..data_reply("x").unwrap()
    });
    let too_few = Ok(Reply {
        body: gzip(&synthetic(5, vec![])),
        ..data_reply("x").unwrap()
    });
    let cases: Vec<(Result<Reply, NetError>, &str)> = vec![
        (Err(NetError::Timeout), "did not answer in time"),
        (Err(NetError::Connect("connect".into())), "could not reach"),
        (status(500), "answered 500"),
        (status(404), "answered 404"),
        (status(302), "answered 302"),
        (not_json, "not JSON"),
        (truncated, "larger than expected"),
        (brotli, "unexpected compression"),
        (bad_gzip, "could not be unpacked"),
        (wrong_shape, "not in the expected format"),
        (too_few, "only 5 usable cards"),
    ];
    for (i, (reply, words)) in cases.into_iter().enumerate() {
        let (store, fake, _) = ready(&format!("fail-{i}"));
        let later = NOW + REFRESH_AFTER_SECS;
        fake.on(DATA_URL, reply);
        assert!(!store.refresh_if_due(later));
        let problem = store.status().problem.unwrap_or_default();
        assert!(problem.contains(words), "{words}: {problem}");
        assert_eq!(
            store.name("CARD_1").as_deref(),
            Some("Card 1"),
            "old names kept"
        );
        assert!(
            !store.refresh_if_due(later + 60),
            "no retry within the hour"
        );
    }
}

#[test]
fn a_failure_with_no_saved_copy_leaves_ids_and_retries_after_an_hour() {
    let (store, fake, _) = store("first-fail");
    fake.on(DATA_URL, Err(NetError::Timeout));
    assert!(!store.refresh_if_due(NOW));
    assert_eq!(store.name("CARD_1"), None);
    assert!(store.status().problem.is_some());
    assert!(!store.refresh_if_due(NOW + 60));
    fake.on(DATA_URL, data_reply("\"v1\""));
    assert!(store.refresh_if_due(NOW + HOUR));
    assert_eq!(store.status().problem, None);
}

#[test]
fn a_304_with_no_saved_copy_is_a_problem_not_a_success() {
    let (store, fake, _) = store("odd-304");
    fake.on(DATA_URL, status(304));
    assert!(!store.refresh_if_due(NOW));
    assert!(store.status().problem.is_some());
}

#[test]
fn a_broken_saved_file_is_reported_and_fetched_again() {
    let dir = temp_dir("broken-saved");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("catalog.json"), b"{not json").unwrap();
    let fake = Arc::new(FakeFetch::default());
    let store = CardStore::open(dir, Box::new(Shared(fake.clone())));
    assert!(store
        .status()
        .problem
        .unwrap()
        .contains("could not be read"));
    fake.on(DATA_URL, data_reply("\"v1\""));
    assert!(store.refresh_if_due(NOW));
}

#[test]
fn art_is_fetched_once_for_a_known_card_and_then_read_from_the_pc() {
    let (store, fake, _) = ready("art");
    fake.on(&art_url("CARD_1"), ok("image/jpeg", jpeg()));
    assert_eq!(store.art("CARD_1", NOW), Some(jpeg()));
    assert_eq!(store.art("CARD_1", NOW + 1), Some(jpeg()));
    let urls = fake.urls();
    assert_eq!(
        urls.last().unwrap(),
        "https://art.hearthstonejson.com/v1/256x/CARD_1.jpg"
    );
    assert_eq!(urls.len(), 2, "data + one image");
    let last = fake.requests.lock().unwrap().last().cloned().unwrap();
    assert_eq!(last.max_bytes, MAX_IMAGE_BYTES);
}

#[test]
fn art_is_never_fetched_for_an_unknown_or_unsafe_id() {
    let (store, fake, _) = ready("art-unknown");
    for id in ["NOT_A_CARD", "../catalog", "CARD_1.jpg", "", "a b"] {
        assert_eq!(store.art(id, NOW), None, "{id:?}");
    }
    assert_eq!(fake.urls().len(), 1, "only the data file");
}

#[test]
fn a_bad_image_answer_shows_no_picture_and_is_not_asked_again_for_an_hour() {
    let html = ok("text/html", b"<html>".to_vec());
    let lying = ok("image/jpeg", b"<html>not an image</html>".to_vec());
    let png = ok("image/png", jpeg());
    let mut truncated = ok("image/jpeg", jpeg()).unwrap();
    truncated.truncated = true;
    let cases = vec![
        status(404),
        status(500),
        Err(NetError::Timeout),
        html,
        lying,
        png,
        Ok(truncated),
    ];
    for (i, reply) in cases.into_iter().enumerate() {
        let (store, fake, _) = ready(&format!("art-bad-{i}"));
        fake.on(&art_url("CARD_2"), reply);
        assert_eq!(store.art("CARD_2", NOW), None, "case {i}");
        assert_eq!(store.art("CARD_2", NOW + 60), None);
        assert_eq!(fake.urls().len(), 2, "case {i}: one try within the hour");
        fake.on(&art_url("CARD_2"), ok("image/jpeg", jpeg()));
        assert_eq!(
            store.art("CARD_2", NOW + HOUR),
            Some(jpeg()),
            "case {i}: retried later"
        );
    }
}

#[test]
fn the_image_folder_stays_under_its_cap() {
    let dir = temp_dir("art-cap");
    let fake = Arc::new(FakeFetch::default());
    let store = CardStore::with_cap(dir.clone(), Box::new(Shared(fake.clone())), 200);
    fake.on(DATA_URL, data_reply("\"v1\""));
    assert!(store.refresh_if_due(NOW));
    for i in 0..10 {
        let id = format!("CARD_{i}");
        fake.on(&art_url(&id), ok("image/jpeg", jpeg()));
        assert!(store.art(&id, NOW).is_some());
    }
    let size: u64 = fs::read_dir(dir.join("art"))
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().len())
        .sum();
    assert!(size <= 200, "{size}");
}

#[test]
fn a_restart_does_not_ask_for_the_card_data_again_within_the_hour() {
    let (store, fake, dir) = store("restart");
    fake.on(DATA_URL, Err(NetError::Timeout));
    assert!(!store.refresh_if_due(NOW));
    let fake2 = Arc::new(FakeFetch::default());
    let again = CardStore::open(dir, Box::new(Shared(fake2.clone())));
    assert!(!again.refresh_if_due(NOW + 60));
    assert!(fake2.urls().is_empty(), "the last attempt was kept on disk");
}

#[test]
fn a_fetch_time_in_the_future_counts_as_stale() {
    let (store, fake, _) = ready("future");
    // The clock was a year ahead when the data was fetched, and the last
    // attempt also looks like it is in the future.
    let earlier = NOW - 365 * 24 * HOUR;
    fake.on(DATA_URL, status(304));
    store.refresh_if_due(earlier);
    assert_eq!(fake.urls().len(), 2, "refreshed instead of waiting a year");
}

#[test]
fn an_unreachable_art_server_pauses_every_image_for_a_while() {
    let (store, fake, _) = ready("offline");
    fake.on(&art_url("CARD_1"), Err(NetError::Timeout));
    assert_eq!(store.art("CARD_1", NOW), None);
    assert_eq!(
        store.art("CARD_2", NOW + 10),
        None,
        "no request while paused"
    );
    assert_eq!(fake.urls().len(), 2);
    fake.on(&art_url("CARD_2"), ok("image/jpeg", jpeg()));
    assert_eq!(store.art("CARD_2", NOW + ART_OFFLINE_SECS), Some(jpeg()));
}

#[test]
fn at_most_a_few_hundred_images_are_fetched_an_hour() {
    let (store, fake, _) = ready("budget");
    for i in 0..ART_PER_HOUR {
        let id = format!("CARD_{i}");
        fake.on(&art_url(&id), ok("image/jpeg", jpeg()));
        assert!(store.art(&id, NOW).is_some(), "{id}");
    }
    let over = format!("CARD_{ART_PER_HOUR}");
    assert_eq!(store.art(&over, NOW + 60), None, "budget spent");
    fake.on(&art_url(&over), ok("image/jpeg", jpeg()));
    assert!(store.art(&over, NOW + HOUR).is_some(), "next hour");
}

#[test]
fn has_data_only_once_names_are_there() {
    let (store, fake, _) = store("has-data");
    assert!(!store.has_data());
    fake.on(DATA_URL, data_reply("\"v1\""));
    store.refresh_if_due(NOW);
    assert!(store.has_data());
}
