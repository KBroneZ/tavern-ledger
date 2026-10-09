//! The one thing this crate needs from an HTTP client: a GET with a size cap,
//! and the status, three headers and the body back. The real client (reqwest
//! over Windows' TLS) is behind the `http` feature; tests use a fake server.

use std::fmt;
use std::time::Duration;

/// Every request gives up after this long. The card data file is about
/// 1.6 MB with gzip, so this leaves room for a slow connection.
pub const TIMEOUT: Duration = Duration::from_secs(30);
pub const USER_AGENT: &str = concat!(
    "TavernLedger/",
    env!("CARGO_PKG_VERSION"),
    " (+https://tavernledger.net)"
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Get {
    pub url: String,
    /// Ask for gzip; the body then comes back compressed and the caller
    /// inflates it (reqwest is built without its own decompression).
    pub gzip: bool,
    /// The ETag of the copy we have: the server answers 304 if it is current.
    pub if_none_match: Option<String>,
    /// Bodies longer than this are cut off and marked `truncated`.
    pub max_bytes: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Reply {
    pub status: u16,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub content_encoding: Option<String>,
    pub body: Vec<u8>,
    /// The body was longer than `max_bytes`.
    pub truncated: bool,
}

impl fmt::Debug for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reply")
            .field("status", &self.status)
            .field("content_type", &self.content_type)
            .field("content_encoding", &self.content_encoding)
            .field("body_len", &self.body.len())
            .field("truncated", &self.truncated)
            .finish()
    }
}

/// Could not get an answer at all.
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

pub trait Fetch: Send + Sync {
    fn get(&self, request: &Get) -> Result<Reply, NetError>;
}

#[cfg(feature = "http")]
pub use real::ReqwestFetch;

#[cfg(feature = "http")]
mod real {
    use super::*;
    use std::io::Read;

    /// Blocking reqwest client: Windows' TLS and certificate store
    /// (native-tls), HTTPS only, no redirects, a timeout and a User-Agent
    /// that says who we are.
    pub struct ReqwestFetch {
        client: reqwest::blocking::Client,
    }

    impl ReqwestFetch {
        pub fn new() -> Result<ReqwestFetch, NetError> {
            let client = reqwest::blocking::Client::builder()
                .timeout(TIMEOUT)
                .connect_timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(USER_AGENT)
                .https_only(true)
                .build()
                .map_err(|e| NetError::Connect(kind(&e)))?;
            Ok(ReqwestFetch { client })
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

    fn header(res: &reqwest::blocking::Response, name: &str) -> Option<String> {
        res.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    }

    impl Fetch for ReqwestFetch {
        fn get(&self, request: &Get) -> Result<Reply, NetError> {
            let mut builder = self.client.get(&request.url);
            if request.gzip {
                builder = builder.header("Accept-Encoding", "gzip");
            }
            if let Some(etag) = &request.if_none_match {
                builder = builder.header("If-None-Match", etag);
            }
            let res = builder.send().map_err(|e| {
                if e.is_timeout() {
                    NetError::Timeout
                } else {
                    NetError::Connect(kind(&e))
                }
            })?;
            let status = res.status().as_u16();
            let etag = header(&res, "etag");
            let content_type = header(&res, "content-type");
            let content_encoding = header(&res, "content-encoding");
            let mut body = Vec::new();
            res.take(request.max_bytes as u64 + 1)
                .read_to_end(&mut body)
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        NetError::Timeout
                    } else {
                        NetError::Connect("read".into())
                    }
                })?;
            let truncated = body.len() > request.max_bytes;
            body.truncate(request.max_bytes);
            Ok(Reply {
                status,
                etag,
                content_type,
                content_encoding,
                body,
                truncated,
            })
        }
    }
}

#[cfg(test)]
pub mod fake {
    //! A scripted server: answers each URL with the next reply queued for it,
    //! and records every request.
    use super::*;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct FakeFetch {
        replies: Mutex<HashMap<String, VecDeque<Result<Reply, NetError>>>>,
        pub requests: Mutex<Vec<Get>>,
    }

    pub fn ok(content_type: &str, body: Vec<u8>) -> Result<Reply, NetError> {
        Ok(Reply {
            status: 200,
            etag: None,
            content_type: Some(content_type.into()),
            content_encoding: None,
            body,
            truncated: false,
        })
    }

    pub fn status(code: u16) -> Result<Reply, NetError> {
        Ok(Reply {
            status: code,
            etag: None,
            content_type: None,
            content_encoding: None,
            body: Vec::new(),
            truncated: false,
        })
    }

    impl FakeFetch {
        pub fn on(&self, url: &str, reply: Result<Reply, NetError>) {
            self.replies
                .lock()
                .unwrap()
                .entry(url.to_string())
                .or_default()
                .push_back(reply);
        }

        pub fn urls(&self) -> Vec<String> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .map(|r| r.url.clone())
                .collect()
        }
    }

    impl Fetch for FakeFetch {
        fn get(&self, request: &Get) -> Result<Reply, NetError> {
            self.requests.lock().unwrap().push(request.clone());
            let mut reply = self
                .replies
                .lock()
                .unwrap()
                .get_mut(&request.url)
                .and_then(VecDeque::pop_front)
                .unwrap_or_else(|| panic!("no reply queued for {}", request.url))?;
            // Like the real client: cut at the cap and say so.
            if reply.body.len() > request.max_bytes {
                reply.body.truncate(request.max_bytes);
                reply.truncated = true;
            }
            Ok(reply)
        }
    }
}
