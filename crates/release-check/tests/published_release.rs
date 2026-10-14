//! The published v0.2.1 release's signature, checked with the public key
//! committed in `tauri.conf.json`. Only public data: the release's
//! `SHA256SUMS` and its minisign signature (key id `CC9CE2C7E6CD31F8`).

use release_check::{check, key_from_conf, Refusal};

const CONF: &str = include_str!("../../desktop/tauri.conf.json");
const SUMS: &[u8] = include_bytes!("published_v0_2_1/SHA256SUMS");
// The release's `SHA256SUMS.minisig`, base64 like the `.sig` files.
const SIGNATURE: &str = include_str!("published_v0_2_1/SHA256SUMS.sig");

#[test]
fn the_published_signature_passes_with_the_committed_key() {
    let key = key_from_conf(CONF).expect("the committed updater key");
    assert_eq!(check(SUMS, SIGNATURE, &key, "0.2.1"), Ok(()));
}

#[test]
fn a_tampered_file_or_another_version_is_refused() {
    let key = key_from_conf(CONF).expect("the committed updater key");
    let mut tampered = SUMS.to_vec();
    tampered[0] = if tampered[0] == b'0' { b'1' } else { b'0' };
    assert_eq!(
        check(&tampered, SIGNATURE, &key, "0.2.1"),
        Err(Refusal::Mismatch)
    );
    assert_eq!(
        check(SUMS, SIGNATURE, &key, "0.2.2"),
        Err(Refusal::Version {
            signed: Some("0.2.1".into())
        })
    );
}

#[test]
fn a_tampered_signature_is_refused() {
    let key = key_from_conf(CONF).expect("the committed updater key");
    // Same length, one base64 character changed inside the signature body.
    let mut bytes = SIGNATURE.as_bytes().to_vec();
    let at = bytes.len() / 2;
    bytes[at] = if bytes[at] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(bytes).unwrap();
    assert!(check(SUMS, &tampered, &key, "0.2.1").is_err());
}
