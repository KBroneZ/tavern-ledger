//! One sign-in attempt: authorization code + PKCE with a loopback redirect
//! (RFC 7636, RFC 8252). `start` binds the listener and asks Supabase to
//! email the user a one-time link; the user opens it in their browser, which
//! lands on the listener with the code; `finish` trades code and verifier for
//! a session. The verifier never leaves this process except in that trade.

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::auth::{is_plausible_email, Auth, Session};
use crate::config::ServerConfig;
use crate::http::Http;
use crate::loopback::Loopback;
use crate::pkce::Pkce;

pub struct SignIn {
    loopback: Loopback,
    pkce: Pkce,
}

impl SignIn {
    pub fn start(server: &ServerConfig, http: &dyn Http, email: &str) -> Result<SignIn, String> {
        let email = email.trim();
        if !is_plausible_email(email) {
            return Err("Type the email of your Tavern Ledger account.".into());
        }
        let loopback =
            Loopback::bind().map_err(|e| format!("Cannot start the sign-in ({:?}).", e.kind()))?;
        let pkce =
            Pkce::new().map_err(|e| format!("Cannot start the sign-in ({:?}).", e.kind()))?;
        Auth { server, http }
            .request_link(email, &pkce.challenge, &loopback.redirect_uri())
            .map_err(|e| format!("Could not ask for a sign-in link: {e}."))?;
        Ok(SignIn { loopback, pkce })
    }

    pub fn redirect_uri(&self) -> String {
        self.loopback.redirect_uri()
    }

    /// Waits for the link to be opened (until `deadline` or `cancel`), then
    /// trades the code for a session.
    pub fn finish(
        self,
        server: &ServerConfig,
        http: &dyn Http,
        deadline: Instant,
        cancel: &AtomicBool,
    ) -> Result<Session, String> {
        let code = self
            .loopback
            .wait(deadline, cancel)
            .map_err(|e| format!("Not signed in: {e}."))?;
        Auth { server, http }
            .exchange(&code, &self.pkce.verifier)
            .map_err(|e| format!("Not signed in: {e}."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::tests::{server, session_json};
    use crate::http::fake::{reply, FakeHttp};
    use crate::pkce::challenge_for;
    use serde_json::{json, Value};
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    fn open_link(redirect_uri: &str, query: &str) {
        let rest = redirect_uri.strip_prefix("http://127.0.0.1:").unwrap();
        let (port, path) = rest.split_at(rest.find('/').unwrap());
        let mut s = TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
        write!(s, "GET {path}?{query} HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
    }

    #[test]
    fn the_full_flow_against_a_fake_server() {
        let http = FakeHttp::default();
        http.on("Post /auth/v1/otp", reply(200, json!({})));
        http.on("Post /auth/v1/token", reply(200, session_json("AT", "RT")));
        let server = server();

        let flow = SignIn::start(&server, &http, " player@example.test ").unwrap();
        let otp = http.last();
        let sent: Value = serde_json::from_slice(&otp.body).unwrap();
        assert_eq!(sent["email"], "player@example.test");
        let redirect = flow.redirect_uri();
        assert!(otp.url.contains(&crate::auth::encode_component(&redirect)));

        let browser = {
            let redirect = redirect.clone();
            std::thread::spawn(move || {
                open_link(&redirect, "code=0b5c4a1e-1111-4222-8333-944455556666")
            })
        };
        let cancel = AtomicBool::new(false);
        let session = flow
            .finish(
                &server,
                &http,
                Instant::now() + Duration::from_secs(10),
                &cancel,
            )
            .unwrap();
        browser.join().unwrap();
        assert_eq!(session.refresh_token, "RT");

        let exchange: Value = serde_json::from_slice(&http.last().body).unwrap();
        assert_eq!(
            exchange["auth_code"],
            "0b5c4a1e-1111-4222-8333-944455556666"
        );
        let verifier = exchange["code_verifier"].as_str().unwrap();
        assert_eq!(
            challenge_for(verifier),
            sent["code_challenge"],
            "the verifier matches the challenge"
        );
        assert!(
            !otp.url.contains(verifier) && !String::from_utf8_lossy(&otp.body).contains(verifier),
            "the verifier is not sent before the exchange"
        );
    }

    #[test]
    fn a_bad_email_sends_nothing() {
        let http = FakeHttp::default();
        assert!(SignIn::start(&server(), &http, "not an email").is_err());
        assert!(http.routes().is_empty());
    }

    #[test]
    fn a_failed_link_request_says_why() {
        let http = FakeHttp::default();
        http.on(
            "Post /auth/v1/otp",
            reply(429, json!({"error_code": "over_email_send_rate_limit"})),
        );
        let err = SignIn::start(&server(), &http, "player@example.test")
            .err()
            .unwrap();
        assert!(err.contains("too many sign-in attempts"), "{err}");
    }

    #[test]
    fn a_cancelled_sign_in_makes_no_exchange() {
        let http = FakeHttp::default();
        http.on("Post /auth/v1/otp", reply(200, json!({})));
        let flow = SignIn::start(&server(), &http, "player@example.test").unwrap();
        let cancel = AtomicBool::new(true);
        let err = flow
            .finish(
                &server(),
                &http,
                Instant::now() + Duration::from_secs(5),
                &cancel,
            )
            .unwrap_err();
        assert!(err.contains("cancelled"));
        assert_eq!(http.routes(), vec!["Post /auth/v1/otp"]);
    }
}
