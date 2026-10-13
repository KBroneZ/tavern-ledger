//! One sign-in attempt: authorization code + PKCE with a loopback redirect
//! (RFC 7636, RFC 8252). `start` binds the listener and makes the PKCE pair;
//! the user then asks for a one-time link on the website's page
//! ([`SignIn::page_url`], opened in their browser), which passes the
//! anti-robot check the app cannot pass itself and hands Supabase the app's
//! challenge and loopback address. The emailed link lands on the listener
//! with the code; `finish` trades code and verifier for a session. The
//! verifier never leaves this process except in that trade.

use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::auth::{encode_component, is_plausible_email, Auth, Session};
use crate::config::ServerConfig;
use crate::http::Http;
use crate::loopback::Loopback;
use crate::pkce::Pkce;

pub struct SignIn {
    loopback: Loopback,
    pkce: Pkce,
    page_url: String,
}

impl SignIn {
    pub fn start(server: &ServerConfig, email: &str) -> Result<SignIn, String> {
        let email = email.trim();
        if !is_plausible_email(email) {
            return Err("Type the email of your Tavern Ledger account.".into());
        }
        let page = server.sign_in_page().ok_or_else(|| {
            "This upload server has no sign-in page: the app signs in only with Tavern Ledger's own server or this PC's.".to_string()
        })?;
        let loopback =
            Loopback::bind().map_err(|e| format!("Cannot start the sign-in ({:?}).", e.kind()))?;
        let pkce =
            Pkce::new().map_err(|e| format!("Cannot start the sign-in ({:?}).", e.kind()))?;
        // In the fragment: the browser never sends it to the website's server.
        let page_url = format!(
            "{page}#challenge={}&redirect={}&email={}",
            encode_component(&pkce.challenge),
            encode_component(&loopback.redirect_uri()),
            encode_component(email),
        );
        Ok(SignIn {
            loopback,
            pkce,
            page_url,
        })
    }

    /// The website page to open in the browser, where the link is asked for.
    pub fn page_url(&self) -> &str {
        &self.page_url
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
    use crate::auth::tests::session_json;
    use crate::config::{hosted, ServerConfig};
    use crate::http::fake::{reply, FakeHttp};
    use crate::pkce::challenge_for;
    use serde_json::Value;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    /// The local stack: its sign-in page is the local website.
    fn local() -> ServerConfig {
        ServerConfig {
            url: "http://127.0.0.1:54321".into(),
            anon_key: "sb_publishable_test".into(),
        }
    }

    fn open_link(redirect_uri: &str, query: &str) {
        let rest = redirect_uri.strip_prefix("http://127.0.0.1:").unwrap();
        let (port, path) = rest.split_at(rest.find('/').unwrap());
        let mut s = TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap();
        write!(s, "GET {path}?{query} HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
    }

    /// The fragment's values, decoded as the page reads them.
    pub(crate) fn fragment(url: &str) -> Vec<(String, String)> {
        let (_, frag) = url.split_once('#').unwrap();
        frag.split('&')
            .map(|pair| {
                let (k, v) = pair.split_once('=').unwrap();
                (k.to_string(), percent_decode(v))
            })
            .collect()
    }

    fn percent_decode(v: &str) -> String {
        let bytes = v.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                out.push(u8::from_str_radix(&v[i + 1..i + 3], 16).unwrap());
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn the_page_gets_the_challenge_the_loopback_address_and_the_email_in_its_fragment() {
        let flow = SignIn::start(&hosted(), " player@example.test ").unwrap();
        let url = flow.page_url();
        assert!(
            url.starts_with("https://tavernledger.net/desktop-sign-in/#"),
            "{url}"
        );
        let values = fragment(url);
        let keys: Vec<&str> = values.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["challenge", "redirect", "email"]);
        assert_eq!(values[0].1.len(), 43, "S256, base64url");
        assert_eq!(values[1].1, flow.redirect_uri());
        assert_eq!(values[2].1, "player@example.test");
        assert!(
            !url.contains(&flow.pkce.verifier),
            "the verifier stays in the app"
        );
        assert_eq!(challenge_for(&flow.pkce.verifier), values[0].1);
        let local = SignIn::start(&local(), "player@example.test").unwrap();
        assert!(local
            .page_url()
            .starts_with("http://127.0.0.1:3000/desktop-sign-in/#"));
    }

    #[test]
    fn the_full_flow_against_a_fake_server() {
        let http = FakeHttp::default();
        http.on("Post /auth/v1/token", reply(200, session_json("AT", "RT")));
        let server = local();

        let flow = SignIn::start(&server, "player@example.test").unwrap();
        assert!(
            http.routes().is_empty(),
            "the app itself asks for no link: the website page does"
        );
        let challenge = fragment(flow.page_url())[0].1.clone();
        let redirect = flow.redirect_uri();
        let browser = std::thread::spawn(move || {
            open_link(&redirect, "code=0b5c4a1e-1111-4222-8333-944455556666")
        });
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
            challenge,
            "the verifier matches the challenge the page sent"
        );
    }

    #[test]
    fn a_bad_email_starts_nothing() {
        assert!(SignIn::start(&local(), "not an email").is_err());
    }

    #[test]
    fn a_server_with_no_known_website_cannot_sign_in() {
        let other = ServerConfig {
            url: "https://other.example".into(),
            anon_key: "sb_publishable_test".into(),
        };
        let err = SignIn::start(&other, "player@example.test").err().unwrap();
        assert!(err.contains("no sign-in page"), "{err}");
    }

    #[test]
    fn a_cancelled_sign_in_makes_no_exchange() {
        let http = FakeHttp::default();
        let flow = SignIn::start(&local(), "player@example.test").unwrap();
        let cancel = AtomicBool::new(true);
        let err = flow
            .finish(
                &local(),
                &http,
                Instant::now() + Duration::from_secs(5),
                &cancel,
            )
            .unwrap_err();
        assert!(err.contains("cancelled"));
        assert!(http.routes().is_empty());
    }
}
