//! Which server to upload to: the hosted project built in, unless the data
//! folder has `server.json` (`{"url": "...", "anon_key": "..."}`, e.g. the
//! local stack for development). Only the public (anon or publishable) key
//! belongs in either; a service-role or secret key is refused.

use std::fs;
use std::io;
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;

pub const FILE_NAME: &str = "server.json";

/// The hosted Supabase project (docs/research/deploy.md, section 10) and its
/// publishable key, which is public by design: what it can reach is limited
/// by row-level security and the upload function's own checks.
const HOSTED_URL: &str = "https://vgttflmexobrqhcyxjks.supabase.co";
// Public key: gitleaks would flag its shape, hence the marker on the line below.
const HOSTED_PUBLISHABLE_KEY: &str = "sb_publishable_pJF8WBoz25Ypn4xAcvp_7Q_slkw7glM"; // gitleaks:allow

#[derive(Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// Project URL without a trailing slash, e.g. https://abc.supabase.co.
    pub url: String,
    pub anon_key: String,
}

impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerConfig")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    Missing,
    Unreadable(String),
    /// What is wrong, in words for the app; never the key itself.
    Invalid(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Missing => write!(f, "no upload server is set up yet"),
            ConfigError::Unreadable(kind) => write!(f, "{FILE_NAME} could not be read ({kind})"),
            ConfigError::Invalid(why) => write!(f, "{FILE_NAME} is not valid: {why}"),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    url: String,
    anon_key: String,
}

/// The built-in server: the hosted project.
pub fn hosted() -> ServerConfig {
    ServerConfig {
        url: HOSTED_URL.to_string(),
        anon_key: HOSTED_PUBLISHABLE_KEY.to_string(),
    }
}

/// `server.json` if the data folder has one, else the hosted project. A file
/// that exists but cannot be read or is wrong stays an error.
pub fn load_or_hosted(dir: &Path) -> Result<ServerConfig, ConfigError> {
    match load(dir) {
        Err(ConfigError::Missing) => Ok(hosted()),
        other => other,
    }
}

pub fn load(dir: &Path) -> Result<ServerConfig, ConfigError> {
    let text = match fs::read_to_string(dir.join(FILE_NAME)) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(ConfigError::Missing),
        Err(e) => return Err(ConfigError::Unreadable(format!("{:?}", e.kind()))),
    };
    let raw: Raw = serde_json::from_str(&text)
        .map_err(|_| ConfigError::Invalid("expected {\"url\": ..., \"anon_key\": ...}"))?;
    parse(&raw.url, &raw.anon_key)
}

pub fn parse(url: &str, anon_key: &str) -> Result<ServerConfig, ConfigError> {
    let url = url.trim_end_matches('/');
    if !is_allowed_url(url) {
        return Err(ConfigError::Invalid(
            "the url must be https://<host>, or http://127.0.0.1:<port> for a local stack",
        ));
    }
    if !is_public_key(anon_key) {
        return Err(ConfigError::Invalid(
            "only the public (anon) key may be used, never a secret or service-role key",
        ));
    }
    Ok(ServerConfig {
        url: url.to_string(),
        anon_key: anon_key.to_string(),
    })
}

/// https with a plain host name, or http on the loopback address (the local
/// development stack). No path, query, user info or spaces.
fn is_allowed_url(url: &str) -> bool {
    let host_ok = |host: &str| {
        !host.is_empty()
            && host.len() <= 253
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    };
    if let Some(rest) = url.strip_prefix("https://") {
        let (host, port) = rest.split_once(':').unwrap_or((rest, ""));
        return host_ok(host) && (port.is_empty() || port.parse::<u16>().is_ok());
    }
    if let Some(port) = url.strip_prefix("http://127.0.0.1:") {
        return port.parse::<u16>().is_ok_and(|p| p != 0);
    }
    false
}

/// A legacy anon JWT (role "anon") or a new-style publishable key.
fn is_public_key(key: &str) -> bool {
    if key.len() > 4096 || key.bytes().any(|b| !b.is_ascii_graphic()) {
        return false;
    }
    if key.starts_with("sb_publishable_") {
        return true;
    }
    let mut parts = key.split('.');
    let (Some(_), Some(payload), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .is_some_and(|claims| claims.get("role").and_then(|r| r.as_str()) == Some("anon"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt(role: &str) -> String {
        let payload = URL_SAFE_NO_PAD.encode(format!("{{\"role\":\"{role}\"}}"));
        format!("eyJhbGciOiJIUzI1NiJ9.{payload}.c2ln")
    }

    #[test]
    fn accepts_https_and_the_local_stack() {
        for url in [
            "https://abc.supabase.co",
            "https://abc.supabase.co/",
            "http://127.0.0.1:54321",
        ] {
            assert!(parse(url, &jwt("anon")).is_ok(), "{url}");
        }
        assert_eq!(
            parse("https://abc.supabase.co/", "sb_publishable_x")
                .unwrap()
                .url,
            "https://abc.supabase.co"
        );
    }

    #[test]
    fn refuses_plain_http_and_odd_urls() {
        for url in [
            "http://abc.supabase.co",
            "http://localhost:54321",
            "https://user@abc.supabase.co",
            "https://abc.supabase.co/path",
            "https://abc.supabase.co?x=1",
            "https://",
            "ftp://abc",
            "http://127.0.0.1:0",
        ] {
            assert!(parse(url, &jwt("anon")).is_err(), "{url}");
        }
    }

    #[test]
    fn refuses_secret_and_service_role_keys() {
        for key in [
            jwt("service_role"),
            "sb_secret_abc".into(),
            "plain".into(),
            String::new(),
        ] {
            assert!(matches!(
                parse("https://a.supabase.co", &key),
                Err(ConfigError::Invalid(_))
            ));
        }
    }

    #[test]
    fn the_key_never_shows_in_debug_or_errors() {
        let config = parse("https://a.supabase.co", &jwt("anon")).unwrap();
        assert!(!format!("{config:?}").contains(&config.anon_key));
        let err = parse("https://a.supabase.co", &jwt("service_role")).unwrap_err();
        assert!(!err.to_string().contains("eyJ"));
    }

    #[test]
    fn loads_the_file_or_says_it_is_missing() {
        let dir = std::env::temp_dir().join(format!("tl-config-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let _ = fs::remove_file(dir.join(FILE_NAME));
        assert_eq!(load(&dir), Err(ConfigError::Missing));
        fs::write(dir.join(FILE_NAME), "{\"url\": \"https://a.supabase.co\"}").unwrap();
        assert!(matches!(load(&dir), Err(ConfigError::Invalid(_))));
        let body = serde_json::json!({"url": "https://a.supabase.co", "anon_key": jwt("anon")});
        fs::write(dir.join(FILE_NAME), body.to_string()).unwrap();
        assert_eq!(load(&dir).unwrap().url, "https://a.supabase.co");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_built_in_server_is_the_hosted_project_with_its_public_key() {
        let server = hosted();
        assert_eq!(server.url, "https://vgttflmexobrqhcyxjks.supabase.co");
        assert_eq!(parse(&server.url, &server.anon_key), Ok(server.clone()));
        assert!(server.anon_key.starts_with("sb_publishable_"));
    }

    #[test]
    fn without_the_file_the_hosted_project_is_used_and_the_file_still_wins() {
        let dir = std::env::temp_dir().join(format!("tl-config-hosted-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let _ = fs::remove_file(dir.join(FILE_NAME));
        assert_eq!(load_or_hosted(&dir), Ok(hosted()));
        let body = serde_json::json!({"url": "http://127.0.0.1:54321", "anon_key": jwt("anon")});
        fs::write(dir.join(FILE_NAME), body.to_string()).unwrap();
        assert_eq!(load_or_hosted(&dir).unwrap().url, "http://127.0.0.1:54321");
        // A broken file is an error to show, never a silent switch to the hosted project.
        fs::write(dir.join(FILE_NAME), "{").unwrap();
        assert!(matches!(load_or_hosted(&dir), Err(ConfigError::Invalid(_))));
        fs::remove_dir_all(&dir).unwrap();
    }
}
