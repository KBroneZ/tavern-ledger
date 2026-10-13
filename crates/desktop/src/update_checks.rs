//! The updater end to end (T-105b): the real plugin, configured from
//! tauri.conf.json, on a mock app, against a small HTTP server in the test.
//! Signing keys are made here for each test and thrown away; no real key is
//! ever used or stored. Plain `http` is only possible because tests are debug
//! builds (the plugin refuses it in release builds); everything else is
//! what a release does. Windows only, like the app: the plugin looks for
//! the `windows-x86_64` entry of `latest.json`.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use base64::Engine as _;
use minisign::KeyPair;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

use crate::updates::{installer_url, GitHub, Reason, Source};

const CONF: &str = include_str!("../tauri.conf.json");
/// The mock app's own version.
const CURRENT: &str = "0.1.0";
const INSTALLER: &[u8] = b"MZ pretend installer bytes";
const LATEST: &str = "/latest.json";

fn b64(text: String) -> String {
    base64::engine::general_purpose::STANDARD.encode(text)
}

fn new_key() -> KeyPair {
    KeyPair::generate_unencrypted_keypair().unwrap()
}

fn public_key(key: &KeyPair) -> String {
    b64(key.pk.to_box().unwrap().into_string())
}

/// A signature in the form the plugin reads (base64 of the minisign text).
fn sign(key: &KeyPair, data: &[u8], trusted: &str) -> String {
    let sig = minisign::sign(Some(&key.pk), &key.sk, data, Some(trusted), None).unwrap();
    b64(sig.into_string())
}

/// As `tauri signer sign --app-version <version>` writes it.
fn signed_for(key: &KeyPair, data: &[u8], version: &str) -> String {
    let trusted = format!(
        "timestamp:1760000000\tfile:TavernLedger_{version}_x64-setup.exe\tversion:{version}"
    );
    sign(key, data, &trusted)
}

enum Response {
    Body(Vec<u8>),
    Redirect(String),
}

/// Answers each path with its fixed response and records every path asked.
struct Server {
    base: String,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn releases(&self) -> String {
        format!("{}/releases/download/", self.base)
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

/// `files` gets the server's address (`http://127.0.0.1:<port>`) and returns
/// what to serve at each path.
fn serve_with(files: impl FnOnce(&str) -> Vec<(String, Response)>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files = files(&base);
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = asked.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            // A client that sends nothing must not hold up the next request.
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut first = String::new();
            if reader.read_line(&mut first).is_err() {
                continue;
            }
            // The rest of the request head; these requests have no body.
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                line.clear();
            }
            let path = first.split(' ').nth(1).unwrap_or("").to_string();
            log.lock().unwrap().push(path.clone());
            let head = |status: &str, extra: &str, len: usize| {
                format!(
                    "HTTP/1.1 {status}\r\n{extra}Content-Length: {len}\r\nConnection: close\r\n\r\n"
                )
            };
            let _ = match files.iter().find(|(p, _)| *p == path).map(|(_, r)| r) {
                Some(Response::Body(body)) => stream
                    .write_all(head("200 OK", "", body.len()).as_bytes())
                    .and_then(|()| stream.write_all(body)),
                Some(Response::Redirect(to)) => stream
                    .write_all(head("302 Found", &format!("Location: {to}\r\n"), 0).as_bytes()),
                None => stream.write_all(head("404 Not Found", "", 0).as_bytes()),
            };
        }
    });
    Server { base, asked }
}

fn latest_json(version: &str, url: &str, signature: &str) -> Response {
    Response::Body(
        serde_json::to_vec(&serde_json::json!({
            "version": version,
            "notes": "Test release.",
            "pub_date": "2026-10-13T00:00:00Z",
            "platforms": { "windows-x86_64": { "signature": signature, "url": url } },
        }))
        .unwrap(),
    )
}

/// A `latest.json` announcing `version` with `signature`, and `bytes` at the
/// tag's installer address.
fn serve_release(version: &str, bytes: &[u8], signature: &str) -> Server {
    serve_with(|base| {
        let url = installer_url(&format!("{base}/releases/download/"), version);
        let path = url.strip_prefix(base).unwrap().to_string();
        vec![
            (LATEST.into(), latest_json(version, &url, signature)),
            (path, Response::Body(bytes.to_vec())),
        ]
    })
}

/// A mock app with the updater plugin set up from the committed config, with
/// only the trusted key and the endpoint replaced.
fn source(server: &Server, trusted: &KeyPair) -> GitHub<MockRuntime> {
    let conf: serde_json::Value = serde_json::from_str(CONF).unwrap();
    let mut updater = conf["plugins"]["updater"].clone();
    updater["pubkey"] = public_key(trusted).into();
    updater["endpoints"] = serde_json::json!([format!("{}{LATEST}", server.base)]);
    let mut context = mock_context(noop_assets());
    context
        .config_mut()
        .plugins
        .0
        .insert("updater".into(), updater);
    let app = mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .unwrap();
    assert_eq!(app.package_info().version.to_string(), CURRENT);
    GitHub {
        app: app.handle().clone(),
        releases: server.releases(),
        no_proxy: true,
    }
}

/// The version found and downloaded, and the versions announced on the way.
fn fetch(source: &GitHub<MockRuntime>) -> (Result<Option<String>, Reason>, Vec<String>) {
    let mut found = Vec::new();
    let result = source
        .fetch(&mut |v: &str| found.push(v.to_string()))
        .map(|d| d.map(|d| d.version().to_string()));
    (result, found)
}

#[test]
fn a_release_signed_by_the_project_key_is_downloaded_and_verified() {
    let key = new_key();
    let server = serve_release("0.2.0", INSTALLER, &signed_for(&key, INSTALLER, "0.2.0"));
    let (result, found) = fetch(&source(&server, &key));
    assert_eq!(result, Ok(Some("0.2.0".to_string())));
    assert_eq!(found, ["0.2.0"]);
    assert_eq!(server.asked().len(), 2, "latest.json, then the installer");
}

#[test]
fn a_release_signed_by_another_key_is_refused() {
    let project = new_key();
    let other = new_key();
    let server = serve_release("0.2.0", INSTALLER, &signed_for(&other, INSTALLER, "0.2.0"));
    let (result, _) = fetch(&source(&server, &project));
    assert_eq!(result, Err(Reason::Signature));
}

#[test]
fn changed_installer_bytes_are_refused() {
    let key = new_key();
    let signature = signed_for(&key, INSTALLER, "0.2.0");
    let server = serve_release("0.2.0", b"MZ something else", &signature);
    let (result, _) = fetch(&source(&server, &key));
    assert_eq!(result, Err(Reason::Signature));
}

#[test]
fn an_older_installer_announced_as_a_newer_version_is_refused() {
    // A genuine 0.2.0 file and signature, in a latest.json that says 0.3.0.
    let key = new_key();
    let server = serve_release("0.3.0", INSTALLER, &signed_for(&key, INSTALLER, "0.2.0"));
    let (result, _) = fetch(&source(&server, &key));
    assert_eq!(result, Err(Reason::Signature));
}

#[test]
fn a_signature_that_does_not_name_its_version_is_refused() {
    let key = new_key();
    let unbound = sign(
        &key,
        INSTALLER,
        "timestamp:1760000000\tfile:TavernLedger_0.2.0_x64-setup.exe",
    );
    let server = serve_release("0.2.0", INSTALLER, &unbound);
    let (result, _) = fetch(&source(&server, &key));
    assert_eq!(result, Err(Reason::Signature));
}

#[test]
fn an_older_or_the_same_version_is_not_an_update_and_is_not_downloaded() {
    let key = new_key();
    for version in ["0.0.9", CURRENT] {
        let server = serve_release(version, INSTALLER, &signed_for(&key, INSTALLER, version));
        let (result, found) = fetch(&source(&server, &key));
        assert_eq!(result, Ok(None), "{version}");
        assert!(found.is_empty());
        assert_eq!(server.asked(), [LATEST], "{version}: no download");
    }
}

#[test]
fn an_installer_anywhere_but_the_tag_s_own_file_is_not_downloaded() {
    let key = new_key();
    let signature = signed_for(&key, INSTALLER, "0.2.0");
    let elsewhere: [fn(&str) -> String; 3] = [
        |base| format!("{base}/other/TavernLedger_0.2.0_x64-setup.exe"),
        // Another tag's file, or the right file name under another tag.
        |base| installer_url(&format!("{base}/releases/download/"), "0.1.9"),
        |base| format!("{base}/releases/download/v0.1.9/TavernLedger_0.2.0_x64-setup.exe"),
    ];
    for wrong in elsewhere {
        let server = serve_with(|base| {
            let url = wrong(base);
            let path = url.strip_prefix(base).unwrap().to_string();
            vec![
                (LATEST.into(), latest_json("0.2.0", &url, &signature)),
                (path, Response::Body(INSTALLER.to_vec())),
            ]
        });
        let (result, found) = fetch(&source(&server, &key));
        assert_eq!(result, Err(Reason::Invalid));
        assert!(found.is_empty());
        assert_eq!(server.asked(), [LATEST], "nothing else is fetched");
    }
}

#[test]
fn a_redirect_away_from_https_is_not_followed() {
    let key = new_key();
    let signature = signed_for(&key, INSTALLER, "0.2.0");
    let server = serve_with(|base| {
        let url = installer_url(&format!("{base}/releases/download/"), "0.2.0");
        let path = url.strip_prefix(base).unwrap().to_string();
        vec![
            (LATEST.into(), latest_json("0.2.0", &url, &signature)),
            (
                path,
                Response::Redirect(format!("{base}/plain/installer.exe")),
            ),
            (
                "/plain/installer.exe".into(),
                Response::Body(INSTALLER.to_vec()),
            ),
        ]
    });
    let (result, _) = fetch(&source(&server, &key));
    assert_eq!(result, Err(Reason::Network));
    assert!(!server.asked().iter().any(|p| p == "/plain/installer.exe"));
}

#[test]
fn no_update_information_is_said_as_such() {
    let key = new_key();
    let server = serve_with(|_| Vec::new());
    let (result, _) = fetch(&source(&server, &key));
    assert_eq!(result, Err(Reason::NotFound));
}
