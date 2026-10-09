//! Supabase Auth calls the desktop app makes (T-104d): ask for a sign-in link
//! with a PKCE challenge, trade the one-time code for a session, refresh it,
//! and sign out. The user signs in in the system browser (the link in their
//! email); the app never sees or asks for a password.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::config::ServerConfig;
use crate::http::{Http, Method, NetError, Request, Response};

const MAX_TOKEN: usize = 8192;

/// A signed-in session. The refresh token is kept in Windows Credential
/// Manager; the access token only in memory.
#[derive(Clone, PartialEq, Eq)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Seconds since 1970 when the access token expires.
    pub expires_at: u64,
    pub user_id: String,
    pub email: Option<String>,
}

/// Never prints the tokens.
impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("expires_at", &self.expires_at)
            .field("user_id", &self.user_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthError {
    Network(NetError),
    /// The server said no; `code` is Supabase's error code when it gave one.
    Rejected {
        status: u16,
        code: Option<String>,
    },
    /// An answer that is not what Supabase sends.
    BadAnswer,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Network(e) => write!(f, "{e}"),
            AuthError::Rejected { status: 429, .. } => {
                write!(f, "too many sign-in attempts; wait a few minutes")
            }
            AuthError::Rejected { status, code } => match code {
                Some(code) => write!(f, "the server refused ({status}, {code})"),
                None => write!(f, "the server refused ({status})"),
            },
            AuthError::BadAnswer => write!(f, "the server's answer was not understood"),
        }
    }
}

impl AuthError {
    /// The session is gone for good (revoked, used, user deleted): sign in
    /// again. Only Supabase's own answers for that; network trouble, server
    /// errors and anything a proxy might say are not.
    pub fn is_session_dead(&self) -> bool {
        const DEAD: [&str; 7] = [
            "refresh_token_not_found",
            "refresh_token_already_used",
            "session_not_found",
            "session_expired",
            "user_not_found",
            "user_banned",
            "invalid_grant",
        ];
        matches!(self, AuthError::Rejected { status: 400 | 401 | 403, code: Some(code) }
            if DEAD.contains(&code.as_str()))
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A plausible email address: what the app sends before asking for a link.
pub fn is_plausible_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    (3..=254).contains(&email.len())
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
        && email
            .chars()
            .all(|c| !c.is_whitespace() && !c.is_control() && c != '<' && c != '>')
}

/// Percent-encodes everything but RFC 3986 unreserved characters.
pub fn encode_component(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub struct Auth<'a> {
    pub server: &'a ServerConfig,
    pub http: &'a dyn Http,
}

impl Auth<'_> {
    fn request(&self, method: Method, path: &str, bearer: Option<&str>, body: Value) -> Request {
        let mut headers = vec![
            ("apikey".to_string(), self.server.anon_key.clone()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        if let Some(token) = bearer {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        Request {
            method,
            url: format!("{}{path}", self.server.url),
            headers,
            body: body.to_string().into_bytes(),
        }
    }

    fn send(&self, request: Request) -> Result<Response, AuthError> {
        let res = self.http.send(request).map_err(AuthError::Network)?;
        if (200..300).contains(&res.status) {
            return Ok(res);
        }
        let code = res
            .json()
            .and_then(|v| {
                v.get("error_code")
                    .or_else(|| v.get("code"))
                    .or_else(|| v.get("error"))
                    .and_then(|c| c.as_str().map(String::from))
            })
            .filter(|c| c.len() <= 64 && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
        Err(AuthError::Rejected {
            status: res.status,
            code,
        })
    }

    /// Asks Supabase to email a one-time sign-in link that ends at
    /// `redirect_uri` with `?code=`. Only for existing accounts: accounts are
    /// made on the website, where the terms are. The answer is the same
    /// whether the email has an account or not.
    pub fn request_link(
        &self,
        email: &str,
        challenge: &str,
        redirect_uri: &str,
    ) -> Result<(), AuthError> {
        let path = format!(
            "/auth/v1/otp?redirect_to={}",
            encode_component(redirect_uri)
        );
        let body = json!({
            "email": email,
            "create_user": false,
            "code_challenge": challenge,
            "code_challenge_method": "s256",
        });
        match self.send(self.request(Method::Post, &path, None, body)) {
            Ok(_) => Ok(()),
            // No account for this email (sign-ups are off for this call):
            // answer as if a link went out, so the app does not tell anyone
            // which emails have accounts.
            Err(AuthError::Rejected {
                status: 400 | 422,
                code,
            }) if matches!(
                code.as_deref(),
                Some("otp_disabled" | "signup_disabled" | "user_not_found")
            ) =>
            {
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Trades the one-time code from the redirect for a session.
    pub fn exchange(&self, code: &str, verifier: &str) -> Result<Session, AuthError> {
        let body = json!({ "auth_code": code, "code_verifier": verifier });
        let res =
            self.send(self.request(Method::Post, "/auth/v1/token?grant_type=pkce", None, body))?;
        parse_session(&res)
    }

    /// A new session from the refresh token. Supabase rotates the refresh
    /// token: the old one stops working soon after.
    pub fn refresh(&self, refresh_token: &str) -> Result<Session, AuthError> {
        let body = json!({ "refresh_token": refresh_token });
        let res = self.send(self.request(
            Method::Post,
            "/auth/v1/token?grant_type=refresh_token",
            None,
            body,
        ))?;
        parse_session(&res)
    }

    /// Ends this session on the server (its refresh tokens stop working).
    pub fn sign_out(&self, access_token: &str) -> Result<(), AuthError> {
        self.send(self.request(
            Method::Post,
            "/auth/v1/logout?scope=local",
            Some(access_token),
            json!({}),
        ))
        .map(|_| ())
    }
}

fn token(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|t| t.as_str())
        .filter(|t| {
            !t.is_empty() && t.len() <= MAX_TOKEN && t.bytes().all(|b| b.is_ascii_graphic())
        })
        .map(String::from)
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

fn parse_session(res: &Response) -> Result<Session, AuthError> {
    let v = res.json().ok_or(AuthError::BadAnswer)?;
    let access_token = token(&v, "access_token").ok_or(AuthError::BadAnswer)?;
    let refresh_token = token(&v, "refresh_token").ok_or(AuthError::BadAnswer)?;
    let expires_at = v
        .get("expires_at")
        .and_then(Value::as_u64)
        .or_else(|| {
            v.get("expires_in")
                .and_then(Value::as_u64)
                .map(|s| now_secs() + s)
        })
        .ok_or(AuthError::BadAnswer)?;
    let user = v.get("user").ok_or(AuthError::BadAnswer)?;
    let user_id = user
        .get("id")
        .and_then(|id| id.as_str())
        .filter(|id| is_uuid(id))
        .ok_or(AuthError::BadAnswer)?
        .to_string();
    let email = user
        .get("email")
        .and_then(|e| e.as_str())
        .filter(|e| is_plausible_email(e))
        .map(String::from);
    Ok(Session {
        access_token,
        refresh_token,
        expires_at,
        user_id,
        email,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::http::fake::{reply, FakeHttp};

    pub const USER: &str = "00000000-0000-4000-8000-00000000000a";

    pub fn server() -> ServerConfig {
        ServerConfig {
            url: "https://api.test".into(),
            anon_key: "sb_publishable_test".into(),
        }
    }

    pub fn session_json(access: &str, refresh: &str) -> Value {
        json!({
            "access_token": access, "refresh_token": refresh, "token_type": "bearer",
            "expires_in": 3600, "expires_at": 4_000_000_000u64,
            "user": {"id": USER, "email": "player@example.test"},
        })
    }

    #[test]
    fn a_link_request_carries_the_challenge_and_the_loopback_redirect() {
        let http = FakeHttp::default();
        http.on("Post /auth/v1/otp", reply(200, json!({})));
        let server = server();
        let auth = Auth {
            server: &server,
            http: &http,
        };
        auth.request_link(
            "player@example.test",
            "CHALLENGE",
            "http://127.0.0.1:50000/desktop-callback/S",
        )
        .unwrap();
        let req = http.last();
        assert_eq!(
            req.url,
            "https://api.test/auth/v1/otp?redirect_to=http%3A%2F%2F127.0.0.1%3A50000%2Fdesktop-callback%2FS"
        );
        assert_eq!(req.header("apikey"), Some("sb_publishable_test"));
        let body: Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(
            body,
            json!({"email": "player@example.test", "create_user": false,
                   "code_challenge": "CHALLENGE", "code_challenge_method": "s256"})
        );
    }

    #[test]
    fn an_email_with_no_account_looks_the_same_as_one_with_an_account() {
        let http = FakeHttp::default();
        http.on(
            "Post /auth/v1/otp",
            reply(422, json!({"error_code": "otp_disabled"})),
        );
        let server = server();
        let auth = Auth {
            server: &server,
            http: &http,
        };
        assert_eq!(
            auth.request_link("x@example.test", "C", "http://127.0.0.1:50000/d/S"),
            Ok(())
        );
        http.on(
            "Post /auth/v1/otp",
            reply(429, json!({"error_code": "over_email_send_rate_limit"})),
        );
        assert!(matches!(
            auth.request_link("x@example.test", "C", "http://127.0.0.1:50000/d/S"),
            Err(AuthError::Rejected { status: 429, .. })
        ));
    }

    #[test]
    fn the_code_and_verifier_are_exchanged_for_a_session() {
        let http = FakeHttp::default();
        http.on("Post /auth/v1/token", reply(200, session_json("AT", "RT")));
        let server = server();
        let auth = Auth {
            server: &server,
            http: &http,
        };
        let session = auth.exchange("CODE", "VERIFIER").unwrap();
        assert_eq!(
            (
                session.access_token.as_str(),
                session.refresh_token.as_str(),
                session.user_id.as_str()
            ),
            ("AT", "RT", USER)
        );
        assert_eq!(session.email.as_deref(), Some("player@example.test"));
        let req = http.last();
        assert!(req.url.ends_with("/auth/v1/token?grant_type=pkce"));
        let body: Value = serde_json::from_slice(&req.body).unwrap();
        assert_eq!(
            body,
            json!({"auth_code": "CODE", "code_verifier": "VERIFIER"})
        );
    }

    #[test]
    fn a_malformed_session_is_refused() {
        for bad in [
            json!({"access_token": "AT"}),
            json!({"access_token": "AT", "refresh_token": "RT", "expires_in": 1, "user": {"id": "x"}}),
            json!({"access_token": "A T", "refresh_token": "RT", "expires_in": 1, "user": {"id": USER}}),
        ] {
            let http = FakeHttp::default();
            http.on("Post /auth/v1/token", reply(200, bad));
            let server = server();
            let auth = Auth {
                server: &server,
                http: &http,
            };
            assert_eq!(auth.refresh("RT"), Err(AuthError::BadAnswer));
        }
    }

    #[test]
    fn refresh_and_sign_out_use_the_right_grant_and_token() {
        let http = FakeHttp::default();
        http.on(
            "Post /auth/v1/token",
            reply(200, session_json("AT2", "RT2")),
        );
        http.on("Post /auth/v1/logout", reply(204, json!(null)));
        let server = server();
        let auth = Auth {
            server: &server,
            http: &http,
        };
        assert_eq!(auth.refresh("RT").unwrap().refresh_token, "RT2");
        assert!(http.last().url.ends_with("grant_type=refresh_token"));
        auth.sign_out("AT2").unwrap();
        assert_eq!(http.last().header("authorization"), Some("Bearer AT2"));
    }

    #[test]
    fn a_dead_session_is_told_apart_from_network_trouble() {
        assert!(AuthError::Rejected {
            status: 400,
            code: Some("refresh_token_not_found".into())
        }
        .is_session_dead());
        assert!(!AuthError::Rejected {
            status: 404,
            code: None
        }
        .is_session_dead());
        assert!(!AuthError::Rejected {
            status: 400,
            code: Some("something_else".into())
        }
        .is_session_dead());
        assert!(!AuthError::Rejected {
            status: 503,
            code: None
        }
        .is_session_dead());
        assert!(!AuthError::Rejected {
            status: 429,
            code: None
        }
        .is_session_dead());
        assert!(!AuthError::Network(NetError::Timeout).is_session_dead());
    }

    #[test]
    fn emails_are_checked_before_anything_is_sent() {
        assert!(is_plausible_email("player@example.test"));
        for bad in [
            "",
            "player",
            "@example.test",
            "a@b",
            "a b@example.test",
            "a@example.test.",
            "<a@b.c>",
        ] {
            assert!(!is_plausible_email(bad), "{bad}");
        }
    }

    #[test]
    fn sessions_never_print_their_tokens() {
        let s = Session {
            access_token: "SECRET_A".into(),
            refresh_token: "SECRET_R".into(),
            expires_at: 1,
            user_id: USER.into(),
            email: None,
        };
        assert!(!format!("{s:?}").contains("SECRET"));
    }
}
