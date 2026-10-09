//! End to end against the local Supabase stack (T-104d): sign in through a
//! real magic link and the loopback redirect, upload, upload again, re-import,
//! hit the quota, sign out. Skipped unless TAVERN_SUPABASE_LOCAL=1 and
//! `npx supabase start` is running; needs the `http` feature:
//!
//!     $env:TAVERN_SUPABASE_LOCAL = "1"
//!     cargo test -p uploader --features http --test local_stack -- --nocapture
//!
//! The user is made up and removed at the end. The stack's keys are read from
//! `npx supabase status` into memory and never printed. Sends one sign-in
//! email (the local stack allows two an hour).
#![cfg(feature = "http")]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tracker::store::{GameKey, Store};
use uploader::auth::Auth;
use uploader::config::{self, ServerConfig};
use uploader::http::{Http, Method, Request, ReqwestHttp};
use uploader::secrets::{credential_name, CredentialManager, TokenStore};
use uploader::signin::SignIn;
use uploader::{Engine, Wake};

const MAIL: &str = "http://127.0.0.1:54324";
const S1: &str = "Hearthstone_2026_10_09_18_30_00";
const S2: &str = "Hearthstone_2026_10_09_20_00_00";

struct Stack {
    url: String,
    anon: String,
    service: String,
}

fn stack() -> Stack {
    let out = Command::new("cmd")
        .args(["/c", "npx", "supabase", "status", "-o", "json"])
        .output()
        .expect("npx supabase status");
    let text = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&text[text.find('{').expect("status json")..]).unwrap();
    let get = |k: &str| v[k].as_str().unwrap().to_string();
    Stack {
        url: get("API_URL"),
        anon: get("ANON_KEY"),
        service: get("SERVICE_ROLE_KEY"),
    }
}

fn call(
    http: &dyn Http,
    method: Method,
    url: &str,
    key: &str,
    bearer: &str,
    body: Value,
) -> (u16, Value) {
    let res = http
        .send(Request {
            method,
            url: url.to_string(),
            headers: vec![
                ("apikey".into(), key.into()),
                ("Authorization".into(), format!("Bearer {bearer}")),
                ("Content-Type".into(), "application/json".into()),
            ],
            body: if body.is_null() {
                Vec::new()
            } else {
                body.to_string().into_bytes()
            },
        })
        .unwrap();
    (res.status, res.json().unwrap_or(Value::Null))
}

fn psql(sql: &str) -> String {
    let out = Command::new("docker")
        .args([
            "exec",
            "supabase_db_tavern-ledger",
            "psql",
            "-U",
            "postgres",
            "-tAc",
            sql,
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "psql failed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn fixture(name: &str) -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../bg-parser/tests/data/{name}.json"));
    serde_json::from_str::<Value>(&fs::read_to_string(path).unwrap()).unwrap()[0].clone()
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tl-local-stack-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The newest sign-in link mailed to `email`, as the local mail catcher holds it.
fn sign_in_link(http: &dyn Http, email: &str) -> String {
    for _ in 0..40 {
        let (_, list) = call(
            http,
            Method::Get,
            &format!("{MAIL}/api/v1/messages"),
            "",
            "",
            Value::Null,
        );
        let found = list["messages"].as_array().and_then(|msgs| {
            msgs.iter().find(|m| {
                m["To"]
                    .as_array()
                    .is_some_and(|to| to.iter().any(|t| t["Address"] == email))
            })
        });
        if let Some(msg) = found {
            let id = msg["ID"].as_str().unwrap();
            let (_, full) = call(
                http,
                Method::Get,
                &format!("{MAIL}/api/v1/message/{id}"),
                "",
                "",
                Value::Null,
            );
            let text = full["Text"].as_str().unwrap();
            let start = text.find("http").unwrap();
            let end = text[start..]
                .find(|c: char| c.is_whitespace() || c == ')')
                .map_or(text.len(), |e| start + e);
            return text[start..end].to_string();
        }
        thread::sleep(Duration::from_millis(250));
    }
    panic!("no sign-in email arrived");
}

/// What the browser does with the link: one GET (the link works once), then
/// follow the auth server's redirect to the app's loopback address. Returns
/// the loopback page's status line.
fn open_in_browser(link: &str) -> String {
    let location = raw_location(link);
    assert!(
        location.starts_with("http://127.0.0.1:"),
        "redirected to the loopback listener"
    );
    let rest = &location["http://127.0.0.1:".len()..];
    let slash = rest.find('/').unwrap();
    let port: u16 = rest[..slash].parse().unwrap();
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        &rest[slash..]
    )
    .unwrap();
    let mut page = String::new();
    s.read_to_string(&mut page).unwrap();
    page.lines().next().unwrap_or("").to_string()
}

/// Removes the test user's rows, files and account, even when the test fails.
struct Cleanup {
    st: Stack,
    uid: String,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let uid = &self.uid;
        psql(&format!(
            "delete from private.upload_events where user_id = '{uid}'"
        ));
        let names = psql(&format!(
            "select coalesce(string_agg(name, ','), '') from storage.objects where name like '{uid}/%'"
        ));
        // Files go through the Storage API (deleting rows would leave the bytes).
        if !names.is_empty() {
            let body = json!({ "prefixes": names.split(',').collect::<Vec<_>>() }).to_string();
            curl_delete(&self.st, "/storage/v1/object/games", Some(&body));
        }
        curl_delete(&self.st, &format!("/auth/v1/admin/users/{uid}"), None);
    }
}

/// DELETE with the service role (the Http trait has no DELETE).
fn curl_delete(st: &Stack, path: &str, body: Option<&str>) {
    let url = format!("{}{path}", st.url);
    let apikey = format!("apikey: {}", st.service);
    let bearer = format!("Authorization: Bearer {}", st.service);
    let mut args = vec!["-s", "-X", "DELETE", &url, "-H", &apikey, "-H", &bearer];
    if let Some(b) = body {
        args.extend(["-H", "Content-Type: application/json", "-d", b]);
    }
    let _ = Command::new("curl").args(args).output();
}

fn raw_location(link: &str) -> String {
    let rest = link.strip_prefix("http://127.0.0.1:").unwrap();
    let slash = rest.find('/').unwrap();
    let port: u16 = rest[..slash].parse().unwrap();
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        s,
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n",
        &rest[slash..]
    )
    .unwrap();
    let mut answer = String::new();
    s.read_to_string(&mut answer).unwrap();
    answer
        .lines()
        .find_map(|l| {
            l.strip_prefix("Location: ")
                .or_else(|| l.strip_prefix("location: "))
        })
        .expect("a Location header")
        .trim()
        .to_string()
}

fn save_game(dir: &Path, session: &str, report: &Value) {
    let mut store = Store::open(dir).unwrap();
    let key = GameKey {
        session: session.into(),
        index: report["index"].as_u64().unwrap(),
    };
    store.save(key, report.clone()).unwrap();
}

fn no_token_in_files(dir: &Path, secrets: &[&str]) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap_or_default();
        for s in secrets {
            assert!(!text.contains(s), "{} holds a token", path.display());
        }
    }
}

#[test]
fn sign_in_upload_quota_and_sign_out_against_the_local_stack() {
    if std::env::var("TAVERN_SUPABASE_LOCAL").as_deref() != Ok("1") {
        eprintln!("skipped: set TAVERN_SUPABASE_LOCAL=1 with the local stack running");
        return;
    }
    let st = stack();
    let http: Arc<dyn Http> = Arc::new(ReqwestHttp::new().unwrap());
    let email = format!("desktop-{}@example.test", std::process::id());
    let (status, user) = call(
        http.as_ref(),
        Method::Post,
        &format!("{}/auth/v1/admin/users", st.url),
        &st.service,
        &st.service,
        json!({"email": email, "password": format!("pw-{}-not-used", std::process::id()), "email_confirm": true}),
    );
    assert_eq!(status, 200, "create user");
    let uid = user["id"].as_str().unwrap().to_string();
    let _cleanup = Cleanup {
        st: Stack {
            url: st.url.clone(),
            anon: String::new(),
            service: st.service.clone(),
        },
        uid: uid.clone(),
    };

    let dir = temp_dir();
    fs::write(
        dir.join(config::FILE_NAME),
        json!({"url": st.url, "anon_key": st.anon}).to_string(),
    )
    .unwrap();
    let server: ServerConfig = config::load(&dir).unwrap();
    let mut duo = fixture("duo_game");
    duo["index"] = json!(1);
    let mut solo = fixture("solo_game");
    solo["index"] = json!(1);
    save_game(&dir, S1, &duo);
    save_game(&dir, S2, &solo);

    // ---- sign in: magic link -> browser -> loopback -> code exchange
    let flow = SignIn::start(&server, http.as_ref(), &email).unwrap();
    let finisher = {
        let (server, http) = (server.clone(), http.clone());
        thread::spawn(move || {
            flow.finish(
                &server,
                http.as_ref(),
                Instant::now() + Duration::from_secs(60),
                &AtomicBool::new(false),
            )
        })
    };
    let link = sign_in_link(http.as_ref(), &email);
    assert!(
        link.contains("redirect_to=http://127.0.0.1:")
            || link.contains("redirect_to=http%3A%2F%2F127.0.0.1"),
        "the link ends at the loopback listener"
    );
    let page = open_in_browser(&link);
    if !page.contains("200") {
        let result = finisher.join().unwrap();
        panic!(
            "loopback page {page:?}; sign-in result: {:?}",
            result.map(|_| "session")
        );
    }
    assert!(
        page.contains("200"),
        "the loopback page says signed in: {page}"
    );
    let session = finisher.join().unwrap().expect("signed in");
    assert_eq!(session.user_id, uid);
    eprintln!("signed in through the browser flow");

    // ---- upload
    let creds = CredentialManager::new(&credential_name(&dir));
    let mut engine = Engine::new(
        dir.clone(),
        Ok(server.clone()),
        http.clone(),
        Box::new(CredentialManager::new(&credential_name(&dir))),
    );
    assert!(
        !engine.status().enabled,
        "off after sign-in until the user turns it on"
    );
    engine.signed_in_with(session.clone()).unwrap();
    assert_eq!(
        creds.load().unwrap().as_deref(),
        Some(session.refresh_token.as_str())
    );
    engine.set_enabled(true).unwrap();
    assert_eq!(engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(engine.status().counts.uploaded, 2, "{:?}", engine.status());
    let rows = psql(&format!(
        "select string_agg(session || '/' || game_index || ':' || coalesce(parser_version, '-'), ',' order by session) from public.games where user_id = '{uid}'"
    ));
    let stamp = tracker::store::ParserStamp::current();
    let parser = format!("{}+r{}", stamp.version, stamp.revision);
    assert_eq!(rows, format!("{S1}/1:{parser},{S2}/1:{parser}"));
    eprintln!("uploaded 2 games");

    // ---- upload again (marks lost): the server answers "unchanged"
    fs::remove_file(dir.join(uploader::queue::FILE_NAME)).unwrap();
    let before = psql(&format!(
        "select string_agg(updated_at::text, ',') from public.games where user_id = '{uid}'"
    ));
    let mut engine = Engine::new(
        dir.clone(),
        Ok(server.clone()),
        http.clone(),
        Box::new(CredentialManager::new(&credential_name(&dir))),
    );
    assert_eq!(
        engine.run(Instant::now(), 10),
        Wake::Idle,
        "{:?}",
        engine.status()
    );
    assert_eq!(engine.status().counts.uploaded, 2);
    let after = psql(&format!(
        "select string_agg(updated_at::text, ',') from public.games where user_id = '{uid}'"
    ));
    assert_eq!(before, after, "nothing rewritten");
    eprintln!("re-upload: unchanged, nothing rewritten");

    // ---- a re-imported game (last record wins) replaces the stored one
    thread::sleep(Duration::from_millis(1100)); // a new saved_at second
    let mut changed = solo.clone();
    changed["final_place"] = json!(5);
    save_game(&dir, S2, &changed);
    assert_eq!(engine.run(Instant::now(), 10), Wake::Idle);
    let place = psql(&format!(
        "select final_place from public.games where user_id = '{uid}' and session = '{S2}'"
    ));
    assert_eq!(place, "5");
    eprintln!("re-import replaced the stored game");

    // ---- quota: 600 uploads in the last hour
    psql(&format!("insert into private.upload_events (user_id, at) select '{uid}', now() from generate_series(1, 600)"));
    let mut third = solo.clone();
    third["index"] = json!(2);
    save_game(&dir, S2, &third);
    match engine.run(Instant::now(), 10) {
        Wake::In(wait) => assert!(wait > Duration::from_secs(3000), "{wait:?}"),
        other => panic!("expected a wait, got {other:?}"),
    }
    assert!(engine
        .status()
        .problem
        .as_deref()
        .unwrap()
        .contains("too many uploads"));
    assert_eq!(engine.status().counts.waiting, 1);
    eprintln!("hourly quota: paused with Retry-After");

    // ---- no token in plain text on disk
    no_token_in_files(&dir, &[&session.refresh_token, &session.access_token]);

    // ---- sign out: the server session ends and the tokens leave this PC
    let current_refresh = creds.load().unwrap().unwrap();
    assert_eq!(engine.sign_out(), "Signed out.");
    assert_eq!(creds.load().unwrap(), None, "the credential is gone");
    let auth = Auth {
        server: &server,
        http: http.as_ref(),
    };
    assert!(
        auth.refresh(&current_refresh).is_err(),
        "the old refresh token no longer works"
    );
    assert!(!engine.status().signed_in && !engine.status().enabled);
    eprintln!("signed out: credential removed, session revoked");

    fs::remove_dir_all(&dir).unwrap();
}
