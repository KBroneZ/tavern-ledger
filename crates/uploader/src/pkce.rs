//! Random values for the sign-in flow: the PKCE verifier and challenge
//! (RFC 7636, S256) and the `state` of the loopback redirect.

use std::io;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

/// `bytes` random bytes from the operating system, base64url without padding.
pub fn random_token(bytes: usize) -> io::Result<String> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

/// A uniform number in [0, 1) for retry jitter.
pub fn random_fraction() -> f64 {
    let mut buf = [0u8; 8];
    // Jitter only spreads retries out; a failure here just means no spread.
    if getrandom::fill(&mut buf).is_err() {
        return 0.5;
    }
    (u64::from_le_bytes(buf) >> 11) as f64 / (1u64 << 53) as f64
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Compares two secrets in time that does not depend on where they differ.
pub fn same_secret(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

/// One sign-in attempt's proof key. The verifier never leaves this process
/// except in the final code exchange.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> io::Result<Pkce> {
        // 32 bytes: a 43-character verifier, the minimum RFC 7636 allows.
        let verifier = random_token(32)?;
        let challenge = challenge_for(&verifier);
        Ok(Pkce {
            verifier,
            challenge,
        })
    }
}

/// S256: base64url(SHA-256(verifier)).
pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_the_rfc_7636_example() {
        // RFC 7636, appendix B.
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn verifiers_are_random_and_long_enough() {
        let a = Pkce::new().unwrap();
        let b = Pkce::new().unwrap();
        assert_ne!(a.verifier, b.verifier);
        assert_eq!(a.verifier.len(), 43);
        assert!(a
            .verifier
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
        assert_eq!(a.challenge, challenge_for(&a.verifier));
    }

    #[test]
    fn fractions_stay_in_range() {
        for _ in 0..100 {
            let f = random_fraction();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn secrets_compare_by_value() {
        assert!(same_secret("abc", "abc"));
        assert!(!same_secret("abc", "abd"));
        assert!(!same_secret("abc", "abcd"));
        assert!(!same_secret("", "a"));
    }

    #[test]
    fn sha256_is_lower_case_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
