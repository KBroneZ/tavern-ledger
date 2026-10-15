use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;

use crate::net::fake::{ok, status, FakeFetch};
use crate::net::{Fetch, Get, NetError, Reply};
use crate::textures::*;

const NOW: u64 = 1_800_000_000;

/// Lets a test keep a handle on the fake after the store owns it.
struct Shared(Arc<FakeFetch>);

impl Fetch for Shared {
    fn get(&self, request: &Get) -> Result<Reply, NetError> {
        self.0.get(request)
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tl-textures-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// A made-up set: three images and a layout, with the manifest that lists them.
struct Set {
    files: Vec<(String, Vec<u8>)>,
}

impl Set {
    fn new() -> Set {
        let layout = json!({"canvas": {"width": 400, "height": 524}}).to_string();
        Set {
            files: vec![
                ("frame_normal.webp".into(), b"RIFF-frame-normal".to_vec()),
                ("gem_attack.webp".into(), b"RIFF-gem-attack".to_vec()),
                ("skull.webp".into(), b"RIFF-skull".to_vec()),
                ("layout.json".into(), layout.into_bytes()),
            ],
        }
    }

    fn entry(&self, file: &str, bytes: &[u8]) -> serde_json::Value {
        let name = file.rsplit_once('.').map_or(file, |(n, _)| n);
        json!({"name": name, "file": file, "sha256": sha256_hex(bytes), "bytes": bytes.len()})
    }

    fn manifest(&self) -> Vec<u8> {
        let files: Vec<_> = self.files.iter().map(|(f, b)| self.entry(f, b)).collect();
        json!({"version": "v1", "game_build": "1", "files": files})
            .to_string()
            .into_bytes()
    }

    /// The server answering for every file; `tweak` can change one reply.
    fn serve(&self, fake: &FakeFetch) {
        fake.on(&manifest_url(), ok("application/json", self.manifest()));
        for (file, bytes) in &self.files {
            fake.on(&file_url(file), ok("image/webp", bytes.clone()));
        }
    }
}

fn manifest_url() -> String {
    format!("{BASE_URL}manifest.json")
}

fn file_url(file: &str) -> String {
    format!("{BASE_URL}{file}")
}

fn fresh(name: &str) -> (TextureStore, Arc<FakeFetch>, PathBuf) {
    let dir = temp_dir(name);
    let fake = Arc::new(FakeFetch::default());
    let store = TextureStore::open(dir.clone(), Box::new(Shared(fake.clone())));
    (store, fake, dir)
}

fn downloaded(name: &str) -> (TextureStore, Arc<FakeFetch>, PathBuf, Set) {
    let (store, fake, dir) = fresh(name);
    let set = Set::new();
    set.serve(&fake);
    assert!(store.refresh_if_due(NOW), "the first download changes the set");
    (store, fake, dir, set)
}

#[test]
fn the_whole_set_is_downloaded_checked_and_served() {
    let (store, fake, dir, set) = downloaded("whole");
    let view = store.view().expect("a set is ready");
    assert_eq!(view.version, "v1");
    assert_eq!(view.layout["canvas"]["width"], 400);
    assert_eq!(view.files["frame_normal"], "frame_normal.webp");
    assert_eq!(view.files["skull"], "skull.webp");
    for (file, bytes) in &set.files {
        let (body, kind) = store.file(file).unwrap_or_else(|| panic!("{file} served"));
        assert_eq!(&body, bytes);
        assert_eq!(kind, if file.ends_with(".json") { "application/json" } else { "image/webp" });
    }
    assert!(dir.join("v1").join("frame_normal.webp").is_file());
    assert!(!dir.join("v1.tmp").exists(), "the side folder is gone");
    // Every request went to the textures host with the manifest first.
    let urls = fake.urls();
    assert_eq!(urls[0], manifest_url());
    assert!(urls.iter().all(|u| u.starts_with(BASE_URL)), "{urls:?}");
    assert_eq!(store.problem(), None);
}

#[test]
fn a_bad_hash_leaves_no_set_at_all() {
    let (store, fake, dir) = fresh("hash");
    let mut set = Set::new();
    let manifest = set.manifest();
    // The server sends different bytes (same length) for the last image.
    set.files[2].1 = b"RIFF-SKULL".to_vec();
    fake.on(&manifest_url(), ok("application/json", manifest));
    for (file, bytes) in &set.files {
        fake.on(&file_url(file), ok("image/webp", bytes.clone()));
    }
    assert!(!store.refresh_if_due(NOW));
    assert!(store.view().is_none());
    assert!(store.file("frame_normal.webp").is_none(), "no half set");
    assert!(!dir.join("v1").exists());
    assert!(!dir.join("v1.tmp").exists(), "the side folder is removed");
    assert!(store.problem().unwrap().contains("skull.webp"), "{:?}", store.problem());
}

#[test]
fn a_manifest_with_an_unsafe_or_odd_name_is_refused_before_any_file_is_asked_for() {
    let bad = [
        "../evil.webp",
        "sub/evil.webp",
        "sub\\evil.webp",
        "C:evil.webp",
        ".hidden.webp",
        "evil.exe",
        "evil",
        "a b.webp",
        "",
        "https:evil.webp",
    ];
    for (i, file) in bad.iter().enumerate() {
        let (store, fake, dir) = fresh(&format!("badname{i}"));
        let set = Set::new();
        let mut manifest: serde_json::Value = serde_json::from_slice(&set.manifest()).unwrap();
        manifest["files"][0]["file"] = json!(file);
        fake.on(&manifest_url(), ok("application/json", manifest.to_string().into_bytes()));
        assert!(!store.refresh_if_due(NOW), "{file}");
        assert_eq!(fake.urls(), [manifest_url()], "{file}: only the manifest was asked for");
        assert!(store.view().is_none());
        assert!(!dir.join("v1").exists());
        assert!(store.problem().is_some(), "{file}");
    }
}

#[test]
fn a_manifest_must_be_the_expected_version_with_unique_files_and_a_layout() {
    let set = Set::new();
    let good: serde_json::Value = serde_json::from_slice(&set.manifest()).unwrap();
    let mut cases: Vec<(&str, serde_json::Value)> = Vec::new();
    let mut other_version = good.clone();
    other_version["version"] = json!("v2");
    cases.push(("version", other_version));
    let mut twice = good.clone();
    let first = twice["files"][0].clone();
    twice["files"].as_array_mut().unwrap().push(first);
    cases.push(("duplicate", twice));
    let mut no_layout = good.clone();
    no_layout["files"].as_array_mut().unwrap().pop();
    cases.push(("layout", no_layout));
    let mut short_hash = good.clone();
    short_hash["files"][1]["sha256"] = json!("abc");
    cases.push(("hash", short_hash));
    let mut upper_hash = good.clone();
    upper_hash["files"][1]["sha256"] = json!("G".repeat(64));
    cases.push(("hash chars", upper_hash));
    for (i, (what, manifest)) in cases.into_iter().enumerate() {
        let (store, fake, _) = fresh(&format!("manifest{i}"));
        fake.on(&manifest_url(), ok("application/json", manifest.to_string().into_bytes()));
        assert!(!store.refresh_if_due(NOW), "{what}");
        assert_eq!(fake.urls().len(), 1, "{what}");
        assert!(store.problem().is_some(), "{what}");
    }
}

#[test]
fn sizes_are_capped_per_file_in_total_and_by_what_the_server_sends() {
    let set = Set::new();
    let good: serde_json::Value = serde_json::from_slice(&set.manifest()).unwrap();
    // A file the manifest says is too big, and a set that is too big in total.
    let mut big_file = good.clone();
    big_file["files"][0]["bytes"] = json!(MAX_FILE_BYTES + 1);
    let mut big_total = good.clone();
    for i in 0..=(MAX_TOTAL_BYTES / MAX_FILE_BYTES) {
        big_total["files"].as_array_mut().unwrap().push(json!({
            "name": format!("big{i}"), "file": format!("big{i}.webp"),
            "sha256": "0".repeat(64), "bytes": MAX_FILE_BYTES
        }));
    }
    let mut too_many = good.clone();
    for i in 0..MAX_FILES {
        too_many["files"].as_array_mut().unwrap().push(json!({
            "name": format!("n{i}"), "file": format!("n{i}.webp"),
            "sha256": "0".repeat(64), "bytes": 10
        }));
    }
    for (i, (what, manifest)) in [("file", big_file), ("total", big_total), ("count", too_many)]
        .into_iter()
        .enumerate()
    {
        let (store, fake, _) = fresh(&format!("caps{i}"));
        fake.on(&manifest_url(), ok("application/json", manifest.to_string().into_bytes()));
        assert!(!store.refresh_if_due(NOW), "{what}");
        assert_eq!(fake.urls().len(), 1, "{what}: nothing else asked for");
    }
    // A manifest longer than its cap is cut and refused.
    let (store, fake, _) = fresh("manifest-cap");
    fake.on(
        &manifest_url(),
        ok("application/json", vec![b' '; MAX_MANIFEST_BYTES + 10]),
    );
    assert!(!store.refresh_if_due(NOW));
    // A file longer than the manifest says is cut at that size and refused.
    let (store, fake, dir) = fresh("longer");
    let mut set = Set::new();
    let manifest = set.manifest();
    set.files[1].1.extend_from_slice(b"-and-more-bytes");
    fake.on(&manifest_url(), ok("application/json", manifest));
    for (file, bytes) in &set.files {
        fake.on(&file_url(file), ok("image/webp", bytes.clone()));
    }
    assert!(!store.refresh_if_due(NOW));
    assert!(!dir.join("v1").exists());
}

#[test]
fn a_403_or_a_redirect_is_a_plain_miss_and_is_not_retried_at_once() {
    for (i, reply) in [status(403), status(301), status(302), status(500)].into_iter().enumerate() {
        let (store, fake, dir) = fresh(&format!("status{i}"));
        let code = reply.as_ref().unwrap().status;
        fake.on(&manifest_url(), reply);
        assert!(!store.refresh_if_due(NOW));
        assert!(store.view().is_none());
        assert!(!dir.join("v1").exists());
        assert!(store.problem().unwrap().contains(&code.to_string()), "{:?}", store.problem());
        // Not again before the retry time, even though the set is missing.
        assert!(!store.refresh_if_due(NOW + RETRY_AFTER_SECS - 1));
        assert_eq!(fake.urls().len(), 1, "no second request yet");
    }
}

#[test]
fn after_the_retry_time_it_asks_again_and_recovers() {
    let (store, fake, _) = fresh("retry");
    fake.on(&manifest_url(), Err(NetError::Timeout));
    assert!(!store.refresh_if_due(NOW));
    assert!(store.problem().unwrap().contains("answer in time"));
    let set = Set::new();
    set.serve(&fake);
    assert!(store.refresh_if_due(NOW + RETRY_AFTER_SECS));
    assert!(store.view().is_some());
    assert_eq!(store.problem(), None, "the problem is gone once it works");
}

#[test]
fn a_complete_set_on_disk_is_used_without_the_network() {
    let (_, _, dir, set) = downloaded("ondisk");
    let fake = Arc::new(FakeFetch::default());
    let again = TextureStore::open(dir, Box::new(Shared(fake.clone())));
    assert!(again.view().is_some());
    assert!(!again.refresh_if_due(NOW));
    assert!(fake.urls().is_empty(), "nothing to fetch");
    assert_eq!(again.file("skull.webp").unwrap().0, set.files[2].1);
}

#[test]
fn leftovers_of_an_interrupted_download_are_cleaned_on_start() {
    let dir = temp_dir("leftovers");
    fs::create_dir_all(dir.join("v1.tmp")).unwrap();
    fs::write(dir.join("v1.tmp").join("frame_normal.webp"), b"half").unwrap();
    fs::create_dir_all(dir.join("v1.old")).unwrap();
    fs::write(dir.join("stray.part"), b"half").unwrap();
    let fake = Arc::new(FakeFetch::default());
    let store = TextureStore::open(dir.clone(), Box::new(Shared(fake)));
    assert!(!dir.join("v1.tmp").exists());
    assert!(!dir.join("v1.old").exists());
    assert!(!dir.join("stray.part").exists());
    assert!(store.view().is_none());
}

#[test]
fn a_file_damaged_on_disk_is_left_out_and_the_set_is_fetched_again() {
    let (_, _, dir, set) = downloaded("damaged");
    fs::write(dir.join("v1").join("gem_attack.webp"), b"RIFF-tampered").unwrap();
    let fake = Arc::new(FakeFetch::default());
    let store = TextureStore::open(dir.clone(), Box::new(Shared(fake.clone())));
    // The rest is drawn; the damaged layer is missing, never a wrong image.
    let view = store.view().expect("the other files still serve");
    assert!(!view.files.contains_key("gem_attack"));
    assert!(view.files.contains_key("skull"));
    assert!(store.file("gem_attack.webp").is_none());
    assert!(store.file("skull.webp").is_some());
    // Then the whole set is fetched again and replaces it.
    set.serve(&fake);
    assert!(store.refresh_if_due(NOW));
    assert_eq!(store.file("gem_attack.webp").unwrap().0, set.files[1].1);
    assert!(store.view().unwrap().files.contains_key("gem_attack"));
}

#[test]
fn a_set_without_a_readable_layout_is_no_set() {
    let (_, _, dir, _) = downloaded("nolayout");
    fs::write(dir.join("v1").join("layout.json"), b"{not json").unwrap();
    let store = TextureStore::open(dir, Box::new(Shared(Arc::new(FakeFetch::default()))));
    assert!(store.view().is_none(), "the layout drives every position");
}

#[test]
fn only_one_download_runs_at_a_time() {
    let (store, fake, _) = fresh("one");
    let set = Set::new();
    set.serve(&fake);
    let busy = store.hold_download_for_test();
    assert!(!store.refresh_if_due(NOW));
    assert!(fake.urls().is_empty(), "the busy store asked for nothing");
    drop(busy);
    assert!(store.refresh_if_due(NOW));
}

#[test]
fn only_files_of_the_set_are_served() {
    let (store, _, _, _) = downloaded("serve");
    for file in [
        "../v1/skull.webp",
        "..\\skull.webp",
        "skull",
        "SKULL.WEBP",
        "manifest.json.bak",
        "",
        "nope.webp",
    ] {
        assert!(store.file(file).is_none(), "{file}");
    }
    assert!(store.file("layout.json").is_some());
}

#[test]
fn the_user_agent_and_host_are_the_apps() {
    assert!(BASE_URL.starts_with("https://assets.tavernledger.net/v"));
    assert!(BASE_URL.ends_with(&format!("{VERSION}/")));
    assert!(crate::net::USER_AGENT.starts_with("TavernLedger/"));
}

#[test]
fn sha256_is_lower_case_hex() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn a_content_type_comes_from_the_extension_only() {
    assert_eq!(content_type("a.webp"), Some("image/webp"));
    assert_eq!(content_type("a.png"), Some("image/png"));
    assert_eq!(content_type("a.json"), Some("application/json"));
    assert_eq!(content_type("a.svg"), None);
    assert_eq!(content_type("a"), None);
}
