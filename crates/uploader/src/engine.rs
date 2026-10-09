//! The upload queue (T-104d). After each game and at start-up the app calls
//! [`Engine::run`]: when upload is on and the user is signed in, it sends the
//! games not yet marked, oldest first, one at a time.
//!
//! * Network error, timeout (15 s) or 5xx: back off with jitter (2 s, 4 s,
//!   8 s... up to 15 min); the game stays waiting, also across restarts.
//! * 429: wait for `Retry-After`.
//! * 400, 409, 413: the server refused this game; it is marked refused and
//!   shown, never retried in a loop (a changed record is sent again).
//! * 401: refresh the session once; if that fails, sign out and ask the user
//!   to sign in again.
//! * 403 and any other 4xx: stop and show the reason.
//!
//! Upload is off until the user signs in and turns it on, and nothing here
//! turns it on by itself. Nothing fails silently: [`Status`] says what
//! happened last, what is waiting and why it stopped.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use tracker::store::Store;

use crate::auth::{now_secs, Auth, AuthError, Session};
use crate::config::{ConfigError, ServerConfig};
use crate::http::{Http, Method, NetError, Request, Response};
use crate::queue::{pending, Counts, Item, Mark, Marks, Outcome};
use crate::secrets::TokenStore;
use crate::settings::{self, Settings};

const BACKOFF_FIRST: Duration = Duration::from_secs(2);
const BACKOFF_MAX: Duration = Duration::from_secs(15 * 60);
const RETRY_AFTER_MAX: Duration = Duration::from_secs(24 * 3600);
/// Codes upload-game gives when it refuses one game for what it holds.
const GAME_REFUSALS: [&str; 6] = [
    "invalid_report",
    "player_name",
    "invalid_json",
    "bad_gzip",
    "too_large",
    "older_revision",
];
/// Refresh the access token when it has less than this left.
const REFRESH_MARGIN_SECS: u64 = 60;

/// What the app shows. Plain words; no token, key or server text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Status {
    /// A server is set up (`server.json`); without one, upload cannot be used.
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub enabled: bool,
    pub signed_in: bool,
    pub email: Option<String>,
    pub counts: Counts,
    /// What the last run did, e.g. "3 games uploaded".
    pub last_result: Option<String>,
    /// Seconds since 1970 of `last_result`.
    pub last_at: Option<u64>,
    /// Why uploading stopped or is paused; cleared when a game goes through.
    pub problem: Option<String>,
    /// Seconds until the next try after an error.
    pub retry_in: Option<u64>,
}

/// When to call [`Engine::run`] again.
#[derive(Debug, PartialEq, Eq)]
pub enum Wake {
    /// Only after something changes (a game, the switch, sign-in).
    Idle,
    In(Duration),
    Now,
}

/// Equal jitter: between half and all of 2 s × 2^attempt, capped at 15 min.
pub fn backoff(attempt: u32, fraction: f64) -> Duration {
    let base = BACKOFF_FIRST
        .saturating_mul(2u32.saturating_pow(attempt.min(20)))
        .min(BACKOFF_MAX);
    base / 2 + base.mul_f64(fraction.clamp(0.0, 1.0) / 2.0)
}

/// What one upload attempt came to.
enum Sent {
    Stored,
    Refused(String),
    Unauthorized,
    /// Network trouble or a server error: back off.
    BackOff(String),
    /// 429: wait as long as the server says.
    Wait(Duration),
    Stop(String),
}

pub struct Engine {
    data_dir: PathBuf,
    server: Result<ServerConfig, ConfigError>,
    http: Arc<dyn Http>,
    tokens: Box<dyn TokenStore>,
    settings: Settings,
    session: Option<Session>,
    /// The session's refresh token is not in Credential Manager yet.
    unsaved: bool,
    attempt: u32,
    paused_until: Option<Instant>,
    jitter: Box<dyn FnMut() -> f64 + Send>,
    status: Status,
}

impl Engine {
    pub fn new(
        data_dir: PathBuf,
        server: Result<ServerConfig, ConfigError>,
        http: Arc<dyn Http>,
        tokens: Box<dyn TokenStore>,
    ) -> Engine {
        let settings = settings::load(&data_dir);
        let mut engine = Engine {
            data_dir,
            server,
            http,
            tokens,
            settings,
            session: None,
            unsaved: false,
            attempt: 0,
            paused_until: None,
            jitter: Box::new(crate::pkce::random_fraction),
            status: Status::default(),
        };
        engine.refresh_status();
        engine
    }

    #[cfg(test)]
    fn with_jitter(mut self, jitter: impl FnMut() -> f64 + Send + 'static) -> Engine {
        self.jitter = Box::new(jitter);
        self
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn server(&self) -> Option<&ServerConfig> {
        self.server.as_ref().ok()
    }

    pub fn http(&self) -> Arc<dyn Http> {
        self.http.clone()
    }

    fn signed_in(&self) -> bool {
        self.settings.user_id.is_some()
    }

    fn save_settings(&mut self) -> Result<(), String> {
        settings::save(&self.data_dir, &self.settings)
            .map_err(|e| format!("Could not save the upload settings ({:?}).", e.kind()))
    }

    /// The user's switch. Turning it on needs a signed-in account.
    pub fn set_enabled(&mut self, on: bool) -> Result<(), String> {
        if on && !self.signed_in() {
            return Err("Sign in first.".into());
        }
        if on && self.server.is_err() {
            return Err("No upload server is set up yet.".into());
        }
        self.settings.enabled = on;
        self.attempt = 0;
        self.paused_until = None;
        let saved = self.save_settings();
        self.refresh_status();
        saved
    }

    /// A finished sign-in. The refresh token goes to Credential Manager
    /// before anything else; upload stays as the user left it (off after a
    /// sign-out), so signing in never turns it on.
    pub fn signed_in_with(&mut self, session: Session) -> Result<(), String> {
        self.tokens.save(&session.refresh_token).map_err(|e| {
            format!(
                "Could not save the sign-in in Windows Credential Manager ({:?}).",
                e.kind()
            )
        })?;
        if self.settings.user_id.as_deref() != Some(session.user_id.as_str()) {
            self.settings.enabled = false;
        }
        self.settings.user_id = Some(session.user_id.clone());
        self.settings.email = session.email.clone();
        self.session = Some(session);
        self.status.problem = None;
        let saved = self.save_settings();
        self.refresh_status();
        saved
    }

    /// Ends the session on the server when it can and always removes the
    /// tokens from this PC; upload turns off. Returns a message for the app.
    pub fn sign_out(&mut self) -> String {
        let mut message = if self.end_server_session() {
            "Signed out.".to_string()
        } else {
            "Signed out on this PC, but the server session could not be ended (the server could not be reached or refused).".to_string()
        };
        if let Err(e) = self.forget_session() {
            message.push(' ');
            message.push_str(&e);
        }
        self.status.problem = None;
        self.status.last_result = Some(message.clone());
        self.status.last_at = Some(now_secs());
        self.refresh_status();
        message
    }

    /// Revokes this session on the server, refreshing first when the access
    /// token is missing (after a restart) or expired. True when nothing is
    /// left to end.
    fn end_server_session(&mut self) -> bool {
        if !self.signed_in() {
            return true;
        }
        let Ok(server) = self.server.clone() else {
            return false;
        };
        let auth = Auth {
            server: &server,
            http: self.http.as_ref(),
        };
        let access = match &self.session {
            Some(s) if s.expires_at > now_secs() + REFRESH_MARGIN_SECS => s.access_token.clone(),
            _ => {
                let refresh = match &self.session {
                    Some(s) => Some(s.refresh_token.clone()),
                    None => self.tokens.load().ok().flatten(),
                };
                match refresh.map(|r| auth.refresh(&r)) {
                    Some(Ok(session)) => session.access_token,
                    _ => return false,
                }
            }
        };
        auth.sign_out(&access).is_ok()
    }

    /// Removes the tokens and the account from this PC; says what could not
    /// be removed, never silently.
    fn forget_session(&mut self) -> Result<(), String> {
        self.session = None;
        self.unsaved = false;
        self.attempt = 0;
        self.paused_until = None;
        self.settings = Settings::default();
        let mut problems = Vec::new();
        if let Err(e) = self.tokens.delete() {
            problems.push(format!(
                "The sign-in could not be removed from Windows Credential Manager ({:?}).",
                e.kind()
            ));
        }
        if let Err(e) = self.save_settings() {
            problems.push(e);
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join(" "))
        }
    }

    fn forget_and_stop(&mut self, why: &str) -> Wake {
        let message = match self.forget_session() {
            Ok(()) => why.to_string(),
            Err(e) => format!("{why} {e}"),
        };
        self.stop(message)
    }

    fn credential_problem(e: &io::Error) -> String {
        format!(
            "Upload stopped: could not save the sign-in in Windows Credential Manager ({:?}).",
            e.kind()
        )
    }

    fn stop(&mut self, problem: String) -> Wake {
        self.status.problem = Some(problem);
        self.status.retry_in = None;
        self.refresh_status();
        Wake::Idle
    }

    fn pause(&mut self, now: Instant, wait: Duration, problem: String) -> Wake {
        self.paused_until = Some(now + wait);
        self.status.problem = Some(problem);
        self.status.retry_in = Some(wait.as_secs().max(1));
        self.refresh_status();
        Wake::In(wait)
    }

    fn back_off(&mut self, now: Instant, why: String) -> Wake {
        let wait = backoff(self.attempt, (self.jitter)());
        self.attempt = self.attempt.saturating_add(1);
        self.pause(
            now,
            wait,
            format!(
                "Upload paused: {why}. Trying again in {} s.",
                wait.as_secs().max(1)
            ),
        )
    }

    /// Recounts the history and copies the settings into the status.
    fn refresh_status(&mut self) {
        let user = self.settings.user_id.clone();
        match self.load() {
            Ok((store, marks)) => self.status.counts = pending(&store, &marks, user.as_deref()).1,
            Err(e) => {
                self.status.problem =
                    Some(format!("Cannot read the game history ({:?}).", e.kind()))
            }
        }
        self.status.available = self.server.is_ok();
        self.status.unavailable_reason = self.server.as_ref().err().map(ToString::to_string);
        self.status.enabled = self.settings.enabled;
        self.status.signed_in = self.signed_in();
        self.status.email = self.settings.email.clone();
    }

    fn load(&self) -> io::Result<(Store, Marks)> {
        Ok((Store::open(&self.data_dir)?, Marks::open(&self.data_dir)?))
    }

    /// A valid access token, refreshing (and saving the rotated refresh
    /// token first) when needed.
    fn access_token(&mut self, now: Instant, force: bool) -> Result<String, Wake> {
        // A rotated token that could not be saved: save it before going on.
        if self.unsaved {
            if let Some(s) = &self.session {
                if let Err(e) = self.tokens.save(&s.refresh_token) {
                    return Err(self.stop(Self::credential_problem(&e)));
                }
            }
            self.unsaved = false;
        }
        if let Some(s) = &self.session {
            if !force && s.expires_at > now_secs() + REFRESH_MARGIN_SECS {
                return Ok(s.access_token.clone());
            }
        }
        let refresh = match self.session.as_ref().map(|s| s.refresh_token.clone()) {
            Some(r) => r,
            None => match self.tokens.load() {
                Ok(Some(r)) => r,
                Ok(None) => {
                    return Err(self.forget_and_stop(
                        "Signed out: the sign-in on this PC is gone. Sign in again.",
                    ));
                }
                Err(e) => {
                    return Err(self.stop(format!(
                        "Cannot read the sign-in from Windows Credential Manager ({:?}).",
                        e.kind()
                    )))
                }
            },
        };
        let server = self.server.clone().map_err(|e| self.stop(e.to_string()))?;
        let auth = Auth {
            server: &server,
            http: self.http.as_ref(),
        };
        match auth.refresh(&refresh) {
            Ok(session) => {
                // The old refresh token stops working soon: keep the new one
                // before using it, so a crash cannot lock the user out.
                if let Err(e) = self.tokens.save(&session.refresh_token) {
                    // The old token may already be spent: keep the new one in
                    // memory, upload nothing, and save it on the next run.
                    self.session = Some(session);
                    self.unsaved = true;
                    return Err(self.stop(Self::credential_problem(&e)));
                }
                let token = session.access_token.clone();
                self.session = Some(session);
                Ok(token)
            }
            Err(e) if e.is_session_dead() => Err(self
                .forget_and_stop("Signed out: your sign-in expired or was ended. Sign in again.")),
            Err(AuthError::Rejected { status: 429, .. }) => Err(self.pause(
                now,
                Duration::from_secs(60),
                "Upload paused: too many sign-in refreshes.".into(),
            )),
            Err(e @ AuthError::Rejected { .. }) => Err(self.stop(format!(
                "Upload stopped: the sign-in could not be refreshed ({e})."
            ))),
            Err(e) => Err(self.back_off(now, format!("the sign-in could not be refreshed ({e})"))),
        }
    }

    fn put(&self, server: &ServerConfig, token: &str, item: &Item) -> Result<Response, NetError> {
        let body = item.gzip().map_err(|_| NetError::Connect("gzip".into()))?;
        self.http.send(Request {
            method: Method::Put,
            url: format!(
                "{}/functions/v1/upload-game/v1/games/{}/{}",
                server.url, item.key.session, item.key.index
            ),
            headers: vec![
                ("apikey".into(), server.anon_key.clone()),
                ("Authorization".into(), format!("Bearer {token}")),
                ("Content-Type".into(), "application/gzip".into()),
            ],
            body,
        })
    }

    fn send(&self, server: &ServerConfig, token: &str, item: &Item) -> Sent {
        let res = match self.put(server, token, item) {
            Ok(res) => res,
            Err(e) => return Sent::BackOff(e.to_string()),
        };
        let json = res.json().unwrap_or(Value::Null);
        let code = json
            .get("code")
            .and_then(Value::as_str)
            .filter(|c| c.len() <= 40 && c.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
            .map(String::from);
        match res.status {
            200 | 201 => {
                // The server hashes what it stored; it must be what we sent.
                if json.get("sha256").and_then(Value::as_str) == Some(item.sha256.as_str()) {
                    Sent::Stored
                } else {
                    Sent::BackOff("the server's answer did not match the game sent".into())
                }
            }
            // Only the function's own answers about this game mark it
            // refused; anything else (a proxy, a wrong deploy) stops instead,
            // so the history is never marked refused by mistake.
            400 | 409 | 413 if code.as_deref().is_some_and(|c| GAME_REFUSALS.contains(&c)) => {
                Sent::Refused(code.unwrap_or_default())
            }
            401 => Sent::Unauthorized,
            429 => {
                let wait = res
                    .retry_after
                    .as_deref()
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or(Duration::from_secs(60))
                    .clamp(Duration::from_secs(1), RETRY_AFTER_MAX);
                Sent::Wait(wait)
            }
            500..=599 => Sent::BackOff(format!("the server had an error ({})", res.status)),
            403 => Sent::Stop(match code.as_deref() {
                Some("quota_games") => {
                    "Upload stopped: your account holds the most games it can.".into()
                }
                Some("quota_bytes") => {
                    "Upload stopped: your account holds the most data it can.".into()
                }
                _ => "Upload stopped: the server did not allow it (403).".into(),
            }),
            status => Sent::Stop(format!("Upload stopped: the server refused it ({status}).")),
        }
    }

    /// Sends up to `budget` waiting games. Call again when it says so.
    pub fn run(&mut self, now: Instant, budget: usize) -> Wake {
        self.status.retry_in = None;
        self.refresh_status();
        if !self.settings.enabled || !self.signed_in() {
            return Wake::Idle;
        }
        let Ok(server) = self.server.clone() else {
            return Wake::Idle;
        };
        if let Some(until) = self.paused_until {
            if now < until {
                self.status.retry_in = Some((until - now).as_secs().max(1));
                return Wake::In(until - now);
            }
        }
        let user = self.settings.user_id.clone().unwrap_or_default();
        let (store, mut marks) = match self.load() {
            Ok(loaded) => loaded,
            Err(e) => return self.stop(format!("Cannot read the game history ({:?}).", e.kind())),
        };
        let (items, _) = pending(&store, &marks, Some(&user));
        if items.is_empty() {
            return Wake::Idle;
        }
        let (mut uploaded, mut refused) = (0usize, Vec::new());
        for item in items.iter().take(budget) {
            let mut sent = Sent::Unauthorized;
            for force_refresh in [false, true] {
                let token = match self.access_token(now, force_refresh) {
                    Ok(t) => t,
                    Err(w) => return self.finish(uploaded, refused, w),
                };
                sent = self.send(&server, &token, item);
                if !matches!(sent, Sent::Unauthorized) {
                    break;
                }
            }
            let outcome = match sent {
                Sent::Stored => Outcome::Uploaded,
                Sent::Refused(ref code) => {
                    refused.push(code.clone());
                    Outcome::Rejected
                }
                Sent::Unauthorized => {
                    let w = self.forget_and_stop(
                        "Signed out: the server did not accept your sign-in. Sign in again.",
                    );
                    return self.finish(uploaded, refused, w);
                }
                Sent::BackOff(why) => {
                    let w = self.back_off(now, why);
                    return self.finish(uploaded, refused, w);
                }
                Sent::Wait(wait) => {
                    let why = format!(
                        "Upload paused: too many uploads. Trying again in {} s.",
                        wait.as_secs()
                    );
                    let w = self.pause(now, wait, why);
                    return self.finish(uploaded, refused, w);
                }
                Sent::Stop(why) => {
                    let w = self.stop(why);
                    return self.finish(uploaded, refused, w);
                }
            };
            let mark = Mark {
                session: item.key.session.clone(),
                index: item.key.index,
                user: user.clone(),
                sha256: item.sha256.clone(),
                code: refused
                    .last()
                    .filter(|_| outcome == Outcome::Rejected)
                    .cloned(),
                outcome,
                at: now_secs(),
            };
            if let Err(e) = marks.record(mark) {
                let w = self.stop(format!(
                    "Cannot save which games were uploaded ({:?}).",
                    e.kind()
                ));
                return self.finish(uploaded, refused, w);
            }
            if outcome == Outcome::Uploaded {
                uploaded += 1;
                self.attempt = 0;
                self.paused_until = None;
                self.status.problem = None;
            }
        }
        let wake = if items.len() > budget {
            Wake::Now
        } else {
            Wake::Idle
        };
        self.finish(uploaded, refused, wake)
    }

    fn finish(&mut self, uploaded: usize, refused: Vec<String>, wake: Wake) -> Wake {
        if uploaded > 0 || !refused.is_empty() {
            let mut parts = Vec::new();
            if uploaded > 0 {
                parts.push(format!(
                    "{uploaded} game{} uploaded",
                    if uploaded == 1 { "" } else { "s" }
                ));
            }
            if let Some(code) = refused.last() {
                parts.push(format!("{} refused by the server ({code})", refused.len()));
            }
            self.status.last_result = Some(parts.join(", "));
            self.status.last_at = Some(now_secs());
        }
        let (retry_in, problem) = (self.status.retry_in, self.status.problem.clone());
        self.refresh_status();
        self.status.retry_in = retry_in;
        self.status.problem = problem;
        wake
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
