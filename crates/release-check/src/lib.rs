//! Checks a release file's updater signature the way the app does before it
//! keeps an update (T-105b): minisign, against the public key in the app's
//! `tauri.conf.json`, and the version named in the signed trusted comment
//! must be the release's. The release workflow runs it on the signed files
//! before anything is drafted, so a wrong key or a wrong version stops the
//! release instead of reaching players.

use base64::Engine as _;
use minisign_verify::{PublicKey, Signature};

/// Why a file is refused, in words for the workflow's log.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The config has no updater key, or it is not a minisign public key.
    NoKey,
    /// The signature text is not a minisign signature.
    BadSignature,
    /// The file is not what the key signed.
    Mismatch,
    /// The signature names no version, or another one.
    Version { signed: Option<String> },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoKey => write!(f, "the updater public key is missing or not a minisign key"),
            Self::BadSignature => write!(f, "the signature is not a minisign signature"),
            Self::Mismatch => write!(f, "the signature does not match the file and the key"),
            Self::Version { signed: None } => write!(f, "the signature names no version"),
            Self::Version { signed: Some(v) } => write!(f, "the signature is for version {v}"),
        }
    }
}

fn decode(text: &str) -> Option<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// The updater public key (base64, as in the config) from `tauri.conf.json`'s text.
pub fn key_from_conf(conf: &str) -> Result<String, Refusal> {
    let conf: serde_json::Value = serde_json::from_str(conf).map_err(|_| Refusal::NoKey)?;
    let key = conf["plugins"]["updater"]["pubkey"]
        .as_str()
        .ok_or(Refusal::NoKey)?;
    public_key(key)?;
    Ok(key.to_string())
}

fn public_key(key: &str) -> Result<PublicKey, Refusal> {
    let text = decode(key).ok_or(Refusal::NoKey)?;
    PublicKey::decode(&text).map_err(|_| Refusal::NoKey)
}

/// Checks `data` against `signature` (the `.sig` file's text: base64 of a
/// minisign signature) and the public key `key`, and that the signature was
/// made for `version`.
pub fn check(data: &[u8], signature: &str, key: &str, version: &str) -> Result<(), Refusal> {
    let key = public_key(key)?;
    let text = decode(signature).ok_or(Refusal::BadSignature)?;
    let signature = Signature::decode(&text).map_err(|_| Refusal::BadSignature)?;
    // Legacy (not prehashed) signatures are refused: the Tauri CLI never makes them.
    key.verify(data, &signature, false)
        .map_err(|_| Refusal::Mismatch)?;
    // Only now is the trusted comment known to be the signer's.
    let signed = signature
        .trusted_comment()
        .split('\t')
        .find_map(|field| field.strip_prefix("version:"));
    if signed == Some(version) {
        Ok(())
    } else {
        Err(Refusal::Version {
            signed: signed.map(String::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minisign::KeyPair;

    fn b64(text: String) -> String {
        base64::engine::general_purpose::STANDARD.encode(text)
    }

    fn key_of(pair: &KeyPair) -> String {
        b64(pair.pk.to_box().unwrap().into_string())
    }

    fn sign(pair: &KeyPair, data: &[u8], trusted: &str) -> String {
        let sig = minisign::sign(Some(&pair.pk), &pair.sk, data, Some(trusted), None).unwrap();
        b64(sig.into_string())
    }

    const TRUSTED: &str =
        "timestamp:1760000000\tfile:TavernLedger_0.2.0_x64-setup.exe\tversion:0.2.0";

    #[test]
    fn a_file_signed_by_the_key_for_its_version_passes() {
        let pair = KeyPair::generate_unencrypted_keypair().unwrap();
        let sig = sign(&pair, b"installer", TRUSTED);
        assert_eq!(check(b"installer", &sig, &key_of(&pair), "0.2.0"), Ok(()));
        // The .sig file may end with a newline.
        assert_eq!(
            check(b"installer", &format!("{sig}\n"), &key_of(&pair), "0.2.0"),
            Ok(())
        );
    }

    #[test]
    fn another_key_other_bytes_or_another_version_are_refused() {
        let pair = KeyPair::generate_unencrypted_keypair().unwrap();
        let other = KeyPair::generate_unencrypted_keypair().unwrap();
        let sig = sign(&pair, b"installer", TRUSTED);
        assert_eq!(
            check(b"installer", &sig, &key_of(&other), "0.2.0"),
            Err(Refusal::Mismatch)
        );
        assert_eq!(
            check(b"installer!", &sig, &key_of(&pair), "0.2.0"),
            Err(Refusal::Mismatch)
        );
        assert_eq!(
            check(b"installer", &sig, &key_of(&pair), "0.3.0"),
            Err(Refusal::Version {
                signed: Some("0.2.0".into())
            })
        );
        let unbound = sign(&pair, b"installer", "timestamp:1760000000\tfile:x.exe");
        assert_eq!(
            check(b"installer", &unbound, &key_of(&pair), "0.2.0"),
            Err(Refusal::Version { signed: None })
        );
        assert_eq!(
            check(b"installer", "not base64!", &key_of(&pair), "0.2.0"),
            Err(Refusal::BadSignature)
        );
    }

    #[test]
    fn the_key_comes_from_the_config_and_the_placeholder_is_no_key() {
        let pair = KeyPair::generate_unencrypted_keypair().unwrap();
        let conf = serde_json::json!({ "plugins": { "updater": { "pubkey": key_of(&pair) } } });
        assert_eq!(key_from_conf(&conf.to_string()), Ok(key_of(&pair)));
        let committed = include_str!("../../desktop/tauri.conf.json");
        assert_eq!(key_from_conf(committed), Err(Refusal::NoKey));
        assert_eq!(key_from_conf("{}"), Err(Refusal::NoKey));
    }
}
