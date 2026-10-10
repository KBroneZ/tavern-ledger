//! The upload queue against a fake server.

use super::*;
use crate::auth::tests::{server, session_json, USER};
use crate::http::fake::{reply, FakeHttp, Reply};
use crate::queue::tests::{store_with, temp_dir};
use crate::secrets::memory::MemoryStore;
use serde_json::json;
use std::path::Path;
use tracker::store::GameKey;

const S1: &str = "Hearthstone_2026_10_01_10_00_00";
const S2: &str = "Hearthstone_2026_10_02_10_00_00";
const PUT: &str = "Put /functions/v1/upload-game/v1/games";
const SOLO: &str = "GT_BATTLEGROUNDS";

struct Rig {
    dir: PathBuf,
    http: Arc<FakeHttp>,
    tokens: MemoryStore,
    engine: Engine,
}

fn session(access: &str, refresh: &str, expires_at: u64) -> Session {
    Session {
        access_token: access.into(),
        refresh_token: refresh.into(),
        expires_at,
        user_id: USER.into(),
        email: Some("player@example.test".into()),
    }
}

fn rig(games: &[(&str, u64, &str)]) -> Rig {
    let dir = temp_dir("engine");
    store_with(&dir, games);
    let http = Arc::new(FakeHttp::default());
    let tokens = MemoryStore::default();
    let engine = Engine::new(
        dir.clone(),
        Ok(server()),
        http.clone(),
        Box::new(tokens.clone()),
    )
    .with_jitter(|| 0.0);
    Rig {
        dir,
        http,
        tokens,
        engine,
    }
}

/// Signed in with a fresh access token and upload turned on.
fn ready(games: &[(&str, u64, &str)]) -> Rig {
    let mut r = rig(games);
    r.engine
        .signed_in_with(session("AT", "RT", now_secs() + 3600))
        .unwrap();
    r.engine.set_enabled(true).unwrap();
    r
}

/// What the server answers for a stored game: the hash of what was sent.
fn stored(r: &Rig, nth: usize) -> Reply {
    let (store, marks) = (Store::open(&r.dir).unwrap(), Marks::open(&r.dir).unwrap());
    let (items, _) = pending(&store, &marks, &Reconnects::default(), Some(USER));
    reply(
        201,
        json!({"result": "created", "sha256": items[nth].sha256}),
    )
}

fn puts(http: &FakeHttp) -> Vec<String> {
    http.requests
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.method == Method::Put)
        .map(|r| r.url.rsplit("/games/").next().unwrap().to_string())
        .collect()
}

fn add_game(dir: &Path, session: &str, index: u64, place: i64) {
    let mut store = Store::open(dir).unwrap();
    let mut report = crate::queue::tests::report(index, SOLO);
    report["final_place"] = json!(place);
    store
        .save(
            GameKey {
                session: session.into(),
                index,
            },
            report,
        )
        .unwrap();
}

#[test]
fn off_by_default_and_nothing_is_sent() {
    let mut r = rig(&[(S1, 1, SOLO)]);
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    let status = r.engine.status();
    assert!(!status.enabled && !status.signed_in && status.available);
    assert_eq!(
        status.counts.waiting, 1,
        "the app still shows what is waiting"
    );
    assert!(r.http.routes().is_empty());
}

#[test]
fn the_switch_needs_a_sign_in_and_signing_in_does_not_turn_it_on() {
    let mut r = rig(&[(S1, 1, SOLO)]);
    assert!(r.engine.set_enabled(true).is_err());
    r.engine
        .signed_in_with(session("AT", "RT", now_secs() + 3600))
        .unwrap();
    assert!(!r.engine.status().enabled);
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert!(r.http.routes().is_empty());
    assert_eq!(r.tokens.token.lock().unwrap().as_deref(), Some("RT"));
    let on_disk = std::fs::read_to_string(settings::path(&r.dir)).unwrap();
    assert!(
        !on_disk.contains("RT") && !on_disk.contains("AT"),
        "no token in upload.json"
    );
}

#[test]
fn without_a_server_upload_says_it_is_not_available() {
    let dir = temp_dir("noserver");
    let engine = Engine::new(
        dir,
        Err(ConfigError::Missing),
        Arc::new(FakeHttp::default()),
        Box::new(MemoryStore::default()),
    );
    let status = engine.status();
    assert!(!status.available);
    assert_eq!(
        status.unavailable_reason.as_deref(),
        Some("no upload server is set up yet")
    );
}

#[test]
fn games_go_oldest_first_one_at_a_time_and_are_marked() {
    let mut r = ready(&[
        (S2, 1, SOLO),
        (S1, 2, SOLO),
        (S1, 3, "GT_BATTLEGROUNDS_FRIENDLY"),
    ]);
    r.http.on(PUT, stored(&r, 0));
    r.http.on(PUT, stored(&r, 1));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(puts(&r.http), vec![format!("{S1}/2"), format!("{S2}/1")]);
    let req = r.http.last();
    assert_eq!(req.header("authorization"), Some("Bearer AT"));
    assert_eq!(req.header("content-type"), Some("application/gzip"));
    let status = r.engine.status();
    assert_eq!(
        (
            status.counts.waiting,
            status.counts.uploaded,
            status.counts.not_uploadable
        ),
        (0, 2, 1)
    );
    assert_eq!(status.last_result.as_deref(), Some("2 games uploaded"));

    // Restart: nothing left to send.
    let mut again = Engine::new(
        r.dir.clone(),
        Ok(server()),
        r.http.clone(),
        Box::new(r.tokens.clone()),
    );
    assert_eq!(again.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(puts(&r.http).len(), 2);
}

#[test]
fn a_re_imported_game_is_uploaded_again() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on(PUT, stored(&r, 0));
    r.engine.run(Instant::now(), 10);
    add_game(&r.dir, S1, 1, 5);
    r.http.on(PUT, stored(&r, 0));
    r.engine.run(Instant::now(), 10);
    assert_eq!(puts(&r.http), vec![format!("{S1}/1"), format!("{S1}/1")]);
}

#[test]
fn a_budget_leaves_the_rest_for_the_next_call() {
    let mut r = ready(&[(S1, 1, SOLO), (S1, 2, SOLO), (S1, 3, SOLO)]);
    r.http.on(PUT, stored(&r, 0));
    r.http.on(PUT, stored(&r, 1));
    assert_eq!(r.engine.run(Instant::now(), 2), Wake::Now);
    r.http.on(PUT, stored(&r, 0));
    assert_eq!(r.engine.run(Instant::now(), 2), Wake::Idle);
    assert_eq!(puts(&r.http).len(), 3);
}

#[test]
fn network_errors_and_5xx_back_off_with_growing_waits() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    let t0 = Instant::now();
    r.http.on(PUT, Err(NetError::Timeout));
    assert_eq!(r.engine.run(t0, 10), Wake::In(Duration::from_secs(1)));
    assert!(r
        .engine
        .status()
        .problem
        .as_deref()
        .unwrap()
        .contains("did not answer in time"));
    assert_eq!(
        r.engine.status().counts.waiting,
        1,
        "the game stays waiting"
    );
    // Before the wait is over, nothing is sent.
    assert_eq!(
        r.engine.run(t0 + Duration::from_millis(500), 10),
        Wake::In(Duration::from_millis(500))
    );
    assert_eq!(puts(&r.http).len(), 1);
    r.http.on(PUT, reply(503, json!({})));
    assert_eq!(
        r.engine.run(t0 + Duration::from_secs(1), 10),
        Wake::In(Duration::from_secs(2))
    );
    r.http.on(PUT, stored(&r, 0));
    assert_eq!(r.engine.run(t0 + Duration::from_secs(3), 10), Wake::Idle);
    assert_eq!(
        r.engine.status().problem,
        None,
        "cleared once a game goes through"
    );
}

#[test]
fn backoff_doubles_with_jitter_up_to_fifteen_minutes() {
    assert_eq!(backoff(0, 0.0), Duration::from_secs(1));
    assert_eq!(backoff(0, 1.0), Duration::from_secs(2));
    assert_eq!(backoff(2, 1.0), Duration::from_secs(8));
    assert_eq!(backoff(30, 1.0), Duration::from_secs(900));
    assert_eq!(backoff(30, 0.0), Duration::from_secs(450));
    let d = backoff(3, 0.37);
    assert!(d >= Duration::from_secs(8) && d <= Duration::from_secs(16));
}

#[test]
fn too_many_requests_waits_for_retry_after() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on(
        PUT,
        Ok(Response {
            status: 429,
            retry_after: Some("120".into()),
            body: br#"{"code":"rate_limited"}"#.to_vec(),
        }),
    );
    assert_eq!(
        r.engine.run(Instant::now(), 10),
        Wake::In(Duration::from_secs(120))
    );
    assert_eq!(r.engine.status().retry_in, Some(120));
}

#[test]
fn a_refused_game_is_marked_shown_and_the_queue_goes_on() {
    let mut r = ready(&[(S1, 1, SOLO), (S1, 2, SOLO), (S1, 3, SOLO)]);
    r.http.on(
        PUT,
        reply(
            400,
            json!({"code": "invalid_report", "field": "report.hero"}),
        ),
    );
    r.http
        .on(PUT, reply(409, json!({"code": "older_revision"})));
    r.http.on(PUT, stored(&r, 2));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    let status = r.engine.status();
    assert_eq!(
        (
            status.counts.rejected,
            status.counts.uploaded,
            status.counts.waiting
        ),
        (2, 1, 0)
    );
    assert_eq!(
        status.last_result.as_deref(),
        Some("1 game uploaded, 2 refused by the server (older_revision)")
    );
    // Never retried in a loop.
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(puts(&r.http).len(), 3);
}

#[test]
fn a_full_quota_or_another_4xx_stops_and_says_why() {
    let mut r = ready(&[(S1, 1, SOLO), (S1, 2, SOLO)]);
    r.http.on(PUT, reply(403, json!({"code": "quota_games"})));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(puts(&r.http).len(), 1, "stopped at the first game");
    assert!(r
        .engine
        .status()
        .problem
        .as_deref()
        .unwrap()
        .contains("most games"));
    assert_eq!(r.engine.status().counts.waiting, 2);
    r.http.on(PUT, reply(404, json!({"code": "not_found"})));
    r.engine.run(Instant::now(), 10);
    assert!(r
        .engine
        .status()
        .problem
        .as_deref()
        .unwrap()
        .contains("(404)"));
}

#[test]
fn an_expired_access_token_is_refreshed_and_the_new_refresh_token_saved_first() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.engine.session = Some(session("OLD", "RT", now_secs()));
    r.http.on(
        "Post /auth/v1/token",
        reply(200, session_json("AT2", "RT2")),
    );
    r.http.on(PUT, stored(&r, 0));
    r.engine.run(Instant::now(), 10);
    assert_eq!(r.http.routes()[0], "Post /auth/v1/token");
    assert_eq!(r.http.last().header("authorization"), Some("Bearer AT2"));
    assert_eq!(r.tokens.token.lock().unwrap().as_deref(), Some("RT2"));
}

#[test]
fn at_start_up_the_stored_refresh_token_is_used() {
    let r = ready(&[(S1, 1, SOLO)]);
    let mut fresh = Engine::new(
        r.dir.clone(),
        Ok(server()),
        r.http.clone(),
        Box::new(r.tokens.clone()),
    )
    .with_jitter(|| 0.0);
    r.http.on(
        "Post /auth/v1/token",
        reply(200, session_json("AT2", "RT2")),
    );
    r.http.on(PUT, stored(&r, 0));
    assert_eq!(fresh.run(Instant::now(), 10), Wake::Idle);
    let body: Value = serde_json::from_slice(&r.http.requests.lock().unwrap()[0].body).unwrap();
    assert_eq!(body["refresh_token"], "RT");
    r.engine.status(); // the first engine is untouched
}

#[test]
fn if_the_new_refresh_token_cannot_be_saved_nothing_is_uploaded_until_it_is() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.engine.session = Some(session("OLD", "RT", 0));
    *r.tokens.fail_save.lock().unwrap() = true;
    r.http.on(
        "Post /auth/v1/token",
        reply(200, session_json("AT2", "RT2")),
    );
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert!(puts(&r.http).is_empty());
    let problem = r.engine.status().problem.clone().unwrap();
    assert!(problem.contains("Credential Manager"), "{problem}");
    // The rotated token is kept in memory and saved on the next run, with
    // no second refresh (the old one may already be spent).
    *r.tokens.fail_save.lock().unwrap() = false;
    r.http.on(PUT, stored(&r, 0));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(r.tokens.token.lock().unwrap().as_deref(), Some("RT2"));
    let refreshes = r
        .http
        .routes()
        .iter()
        .filter(|x| x.contains("token"))
        .count();
    assert_eq!(refreshes, 1);
    assert_eq!(r.engine.status().counts.uploaded, 1);
}

#[test]
fn a_4xx_without_one_of_the_functions_codes_stops_instead_of_marking_the_game() {
    // A proxy or a misdeployed function must not mark the whole history refused.
    let mut r = ready(&[(S1, 1, SOLO), (S1, 2, SOLO)]);
    r.http
        .on(PUT, reply(400, json!({"message": "bad request"})));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(puts(&r.http).len(), 1);
    let status = r.engine.status();
    assert_eq!((status.counts.rejected, status.counts.waiting), (0, 2));
    assert!(status.problem.as_deref().unwrap().contains("(400)"));
}

#[test]
fn sign_out_after_a_restart_refreshes_then_ends_the_server_session() {
    let r = ready(&[(S1, 1, SOLO)]);
    let mut fresh = Engine::new(
        r.dir.clone(),
        Ok(server()),
        r.http.clone(),
        Box::new(r.tokens.clone()),
    );
    r.http.on(
        "Post /auth/v1/token",
        reply(200, session_json("AT2", "RT2")),
    );
    r.http.on("Post /auth/v1/logout", reply(204, json!(null)));
    assert_eq!(fresh.sign_out(), "Signed out.");
    assert_eq!(
        r.http.routes(),
        vec!["Post /auth/v1/token", "Post /auth/v1/logout"]
    );
    assert_eq!(r.http.last().header("authorization"), Some("Bearer AT2"));
    assert_eq!(*r.tokens.token.lock().unwrap(), None);
}

#[test]
fn sign_out_says_so_when_the_server_session_could_not_be_ended() {
    let r = ready(&[(S1, 1, SOLO)]);
    let mut fresh = Engine::new(
        r.dir.clone(),
        Ok(server()),
        r.http.clone(),
        Box::new(r.tokens.clone()),
    );
    r.http.on("Post /auth/v1/token", Err(NetError::Timeout));
    let message = fresh.sign_out();
    assert!(message.contains("could not be ended"), "{message}");
    assert_eq!(*r.tokens.token.lock().unwrap(), None);
}

#[test]
fn a_refresh_refused_for_another_reason_stops_but_keeps_the_sign_in() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.engine.session = None;
    r.http.on("Post /auth/v1/token", reply(404, json!({})));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert!(r.engine.status().signed_in);
    assert_eq!(r.tokens.token.lock().unwrap().as_deref(), Some("RT"));
    assert!(r.engine.status().problem.is_some());
}

#[test]
fn a_401_refreshes_once_then_retries_the_same_game() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on(PUT, reply(401, json!({"code": "unauthorized"})));
    r.http.on(
        "Post /auth/v1/token",
        reply(200, session_json("AT2", "RT2")),
    );
    r.http.on(PUT, stored(&r, 0));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert_eq!(r.engine.status().counts.uploaded, 1);
}

#[test]
fn a_second_401_signs_out_and_asks_to_sign_in_again() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on(PUT, reply(401, json!({})));
    r.http.on(
        "Post /auth/v1/token",
        reply(200, session_json("AT2", "RT2")),
    );
    r.http.on(PUT, reply(401, json!({})));
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    let status = r.engine.status();
    assert!(!status.signed_in && !status.enabled);
    assert!(status.problem.as_deref().unwrap().contains("Sign in again"));
    assert_eq!(*r.tokens.token.lock().unwrap(), None);
}

#[test]
fn a_dead_refresh_token_signs_out() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.engine.session = None;
    r.http.on(
        "Post /auth/v1/token",
        reply(400, json!({"error_code": "refresh_token_not_found"})),
    );
    assert_eq!(r.engine.run(Instant::now(), 10), Wake::Idle);
    assert!(!r.engine.status().signed_in);
    assert_eq!(*r.tokens.token.lock().unwrap(), None);
}

#[test]
fn a_refresh_that_cannot_reach_the_server_backs_off_and_keeps_the_sign_in() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.engine.session = None;
    r.http.on(
        "Post /auth/v1/token",
        Err(NetError::Connect("connect".into())),
    );
    assert_eq!(
        r.engine.run(Instant::now(), 10),
        Wake::In(Duration::from_secs(1))
    );
    assert!(r.engine.status().signed_in);
    assert_eq!(r.tokens.token.lock().unwrap().as_deref(), Some("RT"));
}

#[test]
fn sign_out_ends_the_session_removes_the_tokens_and_turns_upload_off() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on("Post /auth/v1/logout", reply(204, json!(null)));
    assert_eq!(r.engine.sign_out(), "Signed out.");
    assert_eq!(r.http.last().header("authorization"), Some("Bearer AT"));
    assert_eq!(*r.tokens.token.lock().unwrap(), None);
    let status = r.engine.status();
    assert!(!status.signed_in && !status.enabled && status.email.is_none());
    assert_eq!(settings::load(&r.dir), Settings::default());
}

#[test]
fn sign_out_without_network_still_removes_the_tokens_here() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on("Post /auth/v1/logout", Err(NetError::Timeout));
    assert!(r.engine.sign_out().contains("could not be ended"));
    assert_eq!(*r.tokens.token.lock().unwrap(), None);
    assert!(!r.engine.status().signed_in);
}

#[test]
fn an_answer_that_does_not_match_the_game_sent_is_not_marked() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on(
        PUT,
        reply(201, json!({"result": "created", "sha256": "0".repeat(64)})),
    );
    assert_eq!(
        r.engine.run(Instant::now(), 10),
        Wake::In(Duration::from_secs(1))
    );
    assert_eq!(r.engine.status().counts.waiting, 1);
}

#[test]
fn another_account_uploads_its_own_copy() {
    let mut r = ready(&[(S1, 1, SOLO)]);
    r.http.on(PUT, stored(&r, 0));
    r.engine.run(Instant::now(), 10);
    let mut other = session("AT", "RT", now_secs() + 3600);
    other.user_id = "00000000-0000-4000-8000-00000000000b".into();
    r.engine.signed_in_with(other).unwrap();
    assert!(
        !r.engine.status().enabled,
        "a new account starts with upload off"
    );
    assert_eq!(r.engine.status().counts.waiting, 1);
}
