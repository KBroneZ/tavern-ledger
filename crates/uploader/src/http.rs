//! The one thing the uploader needs from an HTTP client: send a request, get
//! the status, a few headers and the body. The real client (reqwest over
//! Windows' TLS) is behind the `http` feature; tests use a fake server.

use std::fmt;
use std::time::Duration;

/// Every request gives up after this long (T-104d: 15 s).
pub const TIMEOUT: Duration = Duration::from_secs(15);
/// Larger answers are cut off; the server's answers are a few hundred bytes.
pub const MAX_ANSWER: usize = 64 * 1024;
pub const USER_AGENT: &str = concat!("TavernLedger/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Never prints header values: they hold tokens and keys.
impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url)
            .field(
                "headers",
                &self.headers.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            )
            .field("body_len", &self.body.len())
            .finish()
    }
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    /// Only `Retry-After`, the one header the uploader reads.
    pub retry_after: Option<String>,
    pub body: Vec<u8>,
}

/// Never prints the body: auth answers hold tokens.
impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("retry_after", &self.retry_after)
            .field("body_len", &self.body.len())
            .finish()
    }
}

impl Response {
    pub fn json(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&self.body).ok()
    }
}

/// Could not get an answer at all. Never holds the URL's query or a header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetError {
    Timeout,
    /// Connection refused, DNS, TLS: the kind only, no addresses.
    Connect(String),
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetError::Timeout => write!(f, "the server did not answer in time"),
            NetError::Connect(kind) => write!(f, "could not reach the server ({kind})"),
        }
    }
}

pub trait Http: Send + Sync {
    fn send(&self, request: Request) -> Result<Response, NetError>;
}

#[cfg(feature = "http")]
pub use real::ReqwestHttp;

#[cfg(feature = "http")]
mod real {
    use super::*;
    use std::io::Read;

    /// Blocking reqwest client: Windows' TLS and certificate store
    /// (native-tls), no redirects (a redirect could carry the token
    /// elsewhere), 15 s timeout, a small answer cap.
    pub struct ReqwestHttp {
        client: reqwest::blocking::Client,
    }

    impl ReqwestHttp {
        pub fn new() -> Result<ReqwestHttp, NetError> {
            let client = reqwest::blocking::Client::builder()
                .timeout(TIMEOUT)
                .connect_timeout(TIMEOUT)
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(USER_AGENT)
                .https_only(false) // checked per server in config.rs: https, or http on loopback
                .build()
                .map_err(|e| NetError::Connect(kind(&e)))?;
            Ok(ReqwestHttp { client })
        }
    }

    fn kind(e: &reqwest::Error) -> String {
        if e.is_timeout() {
            "timeout".into()
        } else if e.is_connect() {
            "connect".into()
        } else if e.is_request() {
            "request".into()
        } else {
            "other".into()
        }
    }

    impl Http for ReqwestHttp {
        fn send(&self, request: Request) -> Result<Response, NetError> {
            let mut builder = match request.method {
                Method::Get => self.client.get(&request.url),
                Method::Post => self.client.post(&request.url),
                Method::Put => self.client.put(&request.url),
            };
            for (k, v) in &request.headers {
                builder = builder.header(k, v);
            }
            if !request.body.is_empty() || request.method != Method::Get {
                builder = builder.body(request.body);
            }
            let res = builder.send().map_err(|e| {
                if e.is_timeout() {
                    NetError::Timeout
                } else {
                    NetError::Connect(kind(&e))
                }
            })?;
            let status = res.status().as_u16();
            let retry_after = res
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let mut body = Vec::new();
            res.take(MAX_ANSWER as u64)
                .read_to_end(&mut body)
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        NetError::Timeout
                    } else {
                        NetError::Connect("read".into())
                    }
                })?;
            Ok(Response {
                status,
                retry_after,
                body,
            })
        }
    }
}

#[cfg(test)]
pub mod fake {
    //! A scripted server: answers each request with the next reply queued for
    //! its method and path, and records every request.
    use super::*;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    pub type Reply = Result<Response, NetError>;

    #[derive(Default)]
    pub struct FakeHttp {
        replies: Mutex<HashMap<String, VecDeque<Reply>>>,
        pub requests: Mutex<Vec<Request>>,
    }

    pub fn reply(status: u16, body: serde_json::Value) -> Reply {
        Ok(Response {
            status,
            retry_after: None,
            body: body.to_string().into_bytes(),
        })
    }

    fn route(method: &Method, url: &str) -> String {
        let path = url.split('?').next().unwrap_or(url);
        let path = path
            .split_once("://")
            .map_or(path, |(_, rest)| rest.find('/').map_or("/", |i| &rest[i..]));
        format!("{method:?} {path}")
    }

    impl FakeHttp {
        /// Queues a reply for `"Post /auth/v1/otp"`-style routes.
        pub fn on(&self, route: &str, reply: Reply) {
            self.replies
                .lock()
                .unwrap()
                .entry(route.to_string())
                .or_default()
                .push_back(reply);
        }

        pub fn routes(&self) -> Vec<String> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .map(|r| route(&r.method, &r.url))
                .collect()
        }

        pub fn last(&self) -> Request {
            self.requests.lock().unwrap().last().cloned().unwrap()
        }
    }

    impl Http for FakeHttp {
        fn send(&self, request: Request) -> Result<Response, NetError> {
            let key = route(&request.method, &request.url);
            self.requests.lock().unwrap().push(request);
            // A route matches its own path and every path below it.
            let mut replies = self.replies.lock().unwrap();
            replies
                .iter_mut()
                .filter(|(r, q)| (key == **r || key.starts_with(&format!("{r}/"))) && !q.is_empty())
                .max_by_key(|(r, _)| r.len())
                .and_then(|(_, q)| q.pop_front())
                .unwrap_or_else(|| panic!("no reply queued for {key}"))
        }
    }
}
