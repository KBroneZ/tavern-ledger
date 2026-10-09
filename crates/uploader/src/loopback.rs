//! The loopback redirect of the sign-in flow (RFC 8252). The app listens on
//! 127.0.0.1 only, on a port the system picks, at
//! `/desktop-callback/<state>`, where `state` is random per attempt. The sign-in
//! link in the user's email ends there with `?code=<one-time code>`. A request
//! with another state (an old link, another program) is answered and ignored;
//! the first request with the right state ends the wait, so the code is used
//! once. Supabase Auth accepts any loopback IP redirect on any port without an
//! allow-list entry (checked on GoTrue v2.197.0, the local stack).

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::pkce::{random_token, same_secret};

pub const CALLBACK_PATH: &str = "/desktop-callback/";
const MAX_REQUEST: usize = 8 * 1024;
/// One connection gets this long in total to send its request line.
const CONNECTION_DEADLINE: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(100);

#[derive(Debug, PartialEq, Eq)]
pub enum WaitError {
    Cancelled,
    TimedOut,
    /// The link came back with an error (expired, already used): the
    /// server's error code, letters and `_` only.
    LinkFailed(String),
    Io(String),
}

impl std::fmt::Display for WaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitError::Cancelled => write!(f, "sign-in cancelled"),
            WaitError::TimedOut => write!(f, "the sign-in link was not opened in time"),
            WaitError::LinkFailed(code) => {
                write!(
                    f,
                    "the sign-in link did not work ({code}); ask for a new one"
                )
            }
            WaitError::Io(kind) => write!(f, "the sign-in listener failed ({kind})"),
        }
    }
}

pub struct Loopback {
    listener: TcpListener,
    state: String,
    port: u16,
}

impl Loopback {
    pub fn bind() -> io::Result<Loopback> {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        Ok(Loopback {
            listener,
            state: random_token(32)?,
            port,
        })
    }

    pub fn redirect_uri(&self) -> String {
        format!(
            "http://127.0.0.1:{}{CALLBACK_PATH}{}",
            self.port, self.state
        )
    }

    /// Waits for the browser to bring back the one-time code.
    pub fn wait(&self, deadline: Instant, cancel: &AtomicBool) -> Result<String, WaitError> {
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(WaitError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(WaitError::TimedOut);
            }
            match self.listener.accept() {
                Ok((stream, _)) => {
                    if let Some(outcome) = self.serve(stream) {
                        return outcome;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => thread::sleep(POLL),
                Err(e) => return Err(WaitError::Io(format!("{:?}", e.kind()))),
            }
        }
    }

    /// Answers one connection. `None`: not our callback, keep waiting.
    fn serve(&self, mut stream: TcpStream) -> Option<Result<String, WaitError>> {
        let target = match read_request_target(&mut stream) {
            Ok(Some(t)) => t,
            Ok(None) => {
                respond(&mut stream, 405, PAGE_NOT_FOUND);
                return None;
            }
            Err(_) => return None,
        };
        let (path, query) = target.split_once('?').unwrap_or((&target, ""));
        let state_ok = path
            .strip_prefix(CALLBACK_PATH)
            .is_some_and(|s| same_secret(s, &self.state));
        if !state_ok {
            let page = if path.starts_with(CALLBACK_PATH) {
                PAGE_OLD_LINK
            } else {
                PAGE_NOT_FOUND
            };
            respond(
                &mut stream,
                if page == PAGE_OLD_LINK { 400 } else { 404 },
                page,
            );
            return None;
        }
        let param = |name: &str| {
            query
                .split('&')
                .find_map(|kv| kv.strip_prefix(name).and_then(|v| v.strip_prefix('=')))
        };
        if let Some(code) = param("code").filter(|c| is_code(c)) {
            respond(&mut stream, 200, PAGE_DONE);
            return Some(Ok(code.to_string()));
        }
        let error = param("error_code")
            .or_else(|| param("error"))
            .filter(|e| {
                !e.is_empty()
                    && e.len() <= 64
                    && e.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            })
            .unwrap_or("no_code")
            .to_string();
        respond(&mut stream, 400, PAGE_FAILED);
        Some(Err(WaitError::LinkFailed(error)))
    }
}

/// Supabase's auth codes are UUIDs; anything else is refused.
fn is_code(c: &str) -> bool {
    (16..=128).contains(&c.len()) && c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// The target of a `GET` request line, or None for another method. At most
/// 8 KiB is read, within CONNECTION_DEADLINE in total, so a stuck or
/// trickling client cannot hold the listener.
fn read_request_target(stream: &mut TcpStream) -> io::Result<Option<String>> {
    stream.set_nonblocking(false)?;
    stream.set_write_timeout(Some(CONNECTION_DEADLINE))?;
    let deadline = Instant::now() + CONNECTION_DEADLINE;
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    // The request line is all we need: stop at the end of the first line.
    while !buf.contains(&b'\n') && buf.len() < MAX_REQUEST {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::from(io::ErrorKind::TimedOut));
        }
        stream.set_read_timeout(Some(left))?;
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let text = String::from_utf8_lossy(&buf);
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.split(' ');
    match (parts.next(), parts.next()) {
        (Some("GET"), Some(target)) if target.starts_with('/') => Ok(Some(target.to_string())),
        _ => Ok(None),
    }
}

const PAGE_DONE: &str = "You are signed in to Tavern Ledger. You can close this tab.";
const PAGE_FAILED: &str =
    "This sign-in link did not work (it may have expired or been used). Ask for a new one in Tavern Ledger.";
const PAGE_OLD_LINK: &str =
    "This sign-in link is from an earlier attempt. Open the newest email from Tavern Ledger.";
const PAGE_NOT_FOUND: &str = "Not found.";

/// A fixed page: nothing from the request is echoed back.
fn respond(stream: &mut TcpStream, status: u16, message: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    let body = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Tavern Ledger</title><p>{message}</p>"
    );
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\n\
         Content-Security-Policy: default-src 'none'\r\nX-Content-Type-Options: nosniff\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// What a browser does: one GET, then read the answer.
    pub fn browse(port: u16, target: &str) -> (u16, String) {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        let status = out[9..12].parse().unwrap();
        (status, out)
    }

    fn start() -> (
        Arc<Loopback>,
        thread::JoinHandle<Result<String, WaitError>>,
        Arc<AtomicBool>,
    ) {
        let lb = Arc::new(Loopback::bind().unwrap());
        let cancel = Arc::new(AtomicBool::new(false));
        let (l, c) = (lb.clone(), cancel.clone());
        let handle = thread::spawn(move || l.wait(Instant::now() + Duration::from_secs(10), &c));
        (lb, handle, cancel)
    }

    fn path_of(lb: &Loopback) -> String {
        let uri = lb.redirect_uri();
        uri[uri.find(CALLBACK_PATH).unwrap()..].to_string()
    }

    const CODE: &str = "0b5c4a1e-1111-4222-8333-944455556666";

    #[test]
    fn listens_on_loopback_only_with_a_random_state() {
        let a = Loopback::bind().unwrap();
        let b = Loopback::bind().unwrap();
        assert!(a.redirect_uri().starts_with("http://127.0.0.1:"));
        assert_ne!(a.state, b.state);
        assert_eq!(a.state.len(), 43);
        assert!(a.listener.local_addr().unwrap().ip().is_loopback());
    }

    #[test]
    fn the_right_state_and_a_code_end_the_wait() {
        let (lb, handle, _) = start();
        let (status, page) = browse(lb.port, &format!("{}?code={CODE}", path_of(&lb)));
        assert_eq!(status, 200);
        assert!(page.contains("signed in"));
        assert!(page.contains("Cache-Control: no-store"));
        assert_eq!(handle.join().unwrap(), Ok(CODE.to_string()));
    }

    #[test]
    fn a_wrong_state_or_path_is_answered_and_ignored() {
        let (lb, handle, _) = start();
        let (status, page) = browse(lb.port, &format!("{CALLBACK_PATH}someoneelse?code={CODE}"));
        assert_eq!(status, 400);
        assert!(!page.contains(CODE), "nothing from the request is echoed");
        assert_eq!(browse(lb.port, "/favicon.ico").0, 404);
        assert!(!handle.is_finished(), "still waiting for the right state");
        browse(lb.port, &format!("{}?code={CODE}", path_of(&lb)));
        assert_eq!(handle.join().unwrap(), Ok(CODE.to_string()));
    }

    #[test]
    fn other_methods_and_garbage_do_not_end_the_wait() {
        let (lb, handle, cancel) = start();
        let mut s = TcpStream::connect(("127.0.0.1", lb.port)).unwrap();
        write!(s, "POST {}?code={CODE} HTTP/1.1\r\n\r\n", path_of(&lb)).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        assert!(out.starts_with("HTTP/1.1 405"));
        let mut junk = TcpStream::connect(("127.0.0.1", lb.port)).unwrap();
        junk.write_all(&[0xff; 100]).unwrap();
        drop(junk);
        thread::sleep(Duration::from_millis(300));
        assert!(!handle.is_finished());
        cancel.store(true, Ordering::SeqCst);
        assert_eq!(handle.join().unwrap(), Err(WaitError::Cancelled));
    }

    #[test]
    fn a_client_trickling_bytes_is_dropped_and_the_link_still_works() {
        let (lb, handle, _) = start();
        let mut slow = TcpStream::connect(("127.0.0.1", lb.port)).unwrap();
        let trickle = thread::spawn(move || {
            for _ in 0..30 {
                if slow.write_all(b"G").is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            }
        });
        thread::sleep(Duration::from_millis(300));
        let started = Instant::now();
        let (status, _) = browse(lb.port, &format!("{}?code={CODE}", path_of(&lb)));
        assert_eq!(status, 200);
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "{:?}",
            started.elapsed()
        );
        assert_eq!(handle.join().unwrap(), Ok(CODE.to_string()));
        trickle.join().unwrap();
    }

    #[test]
    fn an_error_from_the_link_ends_the_wait_with_its_code() {
        let (lb, handle, _) = start();
        let target = format!(
            "{}?error=access_denied&error_code=otp_expired&error_description=Email+link+is+invalid",
            path_of(&lb)
        );
        assert_eq!(browse(lb.port, &target).0, 400);
        assert_eq!(
            handle.join().unwrap(),
            Err(WaitError::LinkFailed("otp_expired".into()))
        );
    }

    #[test]
    fn a_code_that_is_not_shaped_like_one_is_not_used() {
        let (lb, handle, _) = start();
        browse(lb.port, &format!("{}?code=<script>", path_of(&lb)));
        assert_eq!(
            handle.join().unwrap(),
            Err(WaitError::LinkFailed("no_code".into()))
        );
    }

    #[test]
    fn the_wait_ends_at_the_deadline() {
        let lb = Loopback::bind().unwrap();
        let cancel = AtomicBool::new(false);
        let started = Instant::now();
        let result = lb.wait(Instant::now() + Duration::from_millis(250), &cancel);
        assert_eq!(result, Err(WaitError::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
