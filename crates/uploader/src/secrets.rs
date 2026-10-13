//! Where the refresh token lives: Windows Credential Manager (a generic
//! credential, this user and this PC only), never a plain file. The access
//! token stays in memory. The credential's name includes a hash of the data
//! folder, so a test copy of the app with `--data-dir` never reads or
//! overwrites the real sign-in. The token is stored with the URL of the
//! server that issued it ([`bind`]) and is only ever given back for that
//! server ([`token_for`]).

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config;
use crate::pkce::sha256_hex;

/// What the credential holds.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bound {
    url: String,
    refresh_token: String,
}

/// The refresh token with the URL of the server that issued it.
pub fn bind(url: &str, refresh_token: &str) -> String {
    serde_json::json!({"url": url, "refresh_token": refresh_token}).to_string()
}

/// Why a stored credential gives no token for a server.
#[derive(Debug, PartialEq, Eq)]
pub enum Unusable {
    /// It was issued by another server.
    OtherServer,
    /// It is not what this app stores.
    Damaged,
}

/// The refresh token in a stored credential, only when it was issued by the
/// server at `url`. A credential saved before tokens were bound holds the
/// bare token; the hosted project was the only server then.
pub fn token_for(stored: &str, url: &str) -> Result<String, Unusable> {
    if !stored.starts_with('{') {
        return if url == config::hosted().url {
            Ok(stored.to_string())
        } else {
            Err(Unusable::OtherServer)
        };
    }
    let bound: Bound = serde_json::from_str(stored).map_err(|_| Unusable::Damaged)?;
    if bound.url == url {
        Ok(bound.refresh_token)
    } else {
        Err(Unusable::OtherServer)
    }
}

pub trait TokenStore: Send {
    fn load(&self) -> io::Result<Option<String>>;
    fn save(&self, token: &str) -> io::Result<()>;
    fn delete(&self) -> io::Result<()>;
}

/// `TavernLedger/upload/<first 16 hex of SHA-256 of the data folder>`.
pub fn credential_name(data_dir: &Path) -> String {
    let id = sha256_hex(data_dir.to_string_lossy().to_lowercase().as_bytes());
    format!("TavernLedger/upload/{}", &id[..16])
}

#[cfg(windows)]
pub use credman::CredentialManager;

#[cfg(windows)]
mod credman {
    use super::*;
    use std::ptr;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND, FILETIME};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    /// Credential Manager limits a generic credential's secret to 2560 bytes.
    const MAX_BLOB: usize = 5 * 512;

    pub struct CredentialManager {
        target: Vec<u16>,
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    impl CredentialManager {
        pub fn new(name: &str) -> CredentialManager {
            CredentialManager { target: wide(name) }
        }
    }

    impl TokenStore for CredentialManager {
        fn load(&self) -> io::Result<Option<String>> {
            let mut cred: *mut CREDENTIALW = ptr::null_mut();
            // SAFETY: `target` is a NUL-terminated UTF-16 string that outlives
            // the call; `cred` receives a buffer we free with CredFree below.
            let ok = unsafe { CredReadW(self.target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred) };
            if ok == 0 {
                // SAFETY: reads this thread's last-error value; no pointers.
                let err = unsafe { GetLastError() };
                if err == ERROR_NOT_FOUND {
                    return Ok(None);
                }
                return Err(io::Error::from_raw_os_error(err as i32));
            }
            // SAFETY: CredReadW succeeded, so `cred` points to a valid
            // CREDENTIALW whose blob holds CredentialBlobSize bytes.
            let bytes = unsafe {
                let c = &*cred;
                let blob = if c.CredentialBlob.is_null() {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize)
                        .to_vec()
                };
                CredFree(cred as *const _);
                blob
            };
            String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "stored token is not text"))
        }

        fn save(&self, token: &str) -> io::Result<()> {
            if token.len() > MAX_BLOB {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "token too long",
                ));
            }
            let mut blob = token.as_bytes().to_vec();
            let user = wide("TavernLedger");
            let cred = CREDENTIALW {
                Flags: 0,
                Type: CRED_TYPE_GENERIC,
                TargetName: self.target.as_ptr() as *mut u16,
                Comment: ptr::null_mut(),
                LastWritten: FILETIME {
                    dwLowDateTime: 0,
                    dwHighDateTime: 0,
                },
                CredentialBlobSize: blob.len() as u32,
                CredentialBlob: blob.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: ptr::null_mut(),
                TargetAlias: ptr::null_mut(),
                UserName: user.as_ptr() as *mut u16,
            };
            // SAFETY: every pointer in `cred` points into `self.target`,
            // `blob` or `user`, which live until after the call.
            let ok = unsafe { CredWriteW(&cred, 0) };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        fn delete(&self) -> io::Result<()> {
            // SAFETY: `target` is a NUL-terminated UTF-16 string.
            let ok = unsafe { CredDeleteW(self.target.as_ptr(), CRED_TYPE_GENERIC, 0) };
            if ok == 0 {
                // SAFETY: reads this thread's last-error value; no pointers.
                let err = unsafe { GetLastError() };
                if err != ERROR_NOT_FOUND {
                    return Err(io::Error::from_raw_os_error(err as i32));
                }
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_token_round_trips_through_credential_manager_and_is_deleted() {
            let name = format!("TavernLedger/test/{}", std::process::id());
            let store = CredentialManager::new(&name);
            assert_eq!(store.load().unwrap(), None);
            store.save("refresh-token-1").unwrap();
            store.save("refresh-token-2").unwrap();
            assert_eq!(store.load().unwrap().as_deref(), Some("refresh-token-2"));
            store.delete().unwrap();
            assert_eq!(store.load().unwrap(), None);
            store.delete().unwrap();
        }
    }
}

#[cfg(test)]
pub mod memory {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// For tests: shared, so a test can look at what the engine stored.
    #[derive(Clone, Default)]
    pub struct MemoryStore {
        pub token: Arc<Mutex<Option<String>>>,
        pub fail_save: Arc<Mutex<bool>>,
    }

    impl TokenStore for MemoryStore {
        fn load(&self) -> io::Result<Option<String>> {
            Ok(self.token.lock().unwrap().clone())
        }
        fn save(&self, token: &str) -> io::Result<()> {
            if *self.fail_save.lock().unwrap() {
                return Err(io::Error::other("locked"));
            }
            *self.token.lock().unwrap() = Some(token.to_string());
            Ok(())
        }
        fn delete(&self) -> io::Result<()> {
            *self.token.lock().unwrap() = None;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_given_back_only_for_the_server_that_issued_it() {
        let stored = bind("https://a.supabase.co", "RT");
        assert_eq!(
            token_for(&stored, "https://a.supabase.co").as_deref(),
            Ok("RT")
        );
        for other in [
            "https://b.supabase.co",
            "https://a.supabase.co.example",
            "http://127.0.0.1:54321",
            "",
        ] {
            assert_eq!(
                token_for(&stored, other),
                Err(Unusable::OtherServer),
                "{other}"
            );
        }
    }

    #[test]
    fn a_token_saved_before_binding_belongs_to_the_hosted_project_only() {
        let hosted = config::hosted().url;
        assert_eq!(token_for("RT", &hosted).as_deref(), Ok("RT"));
        assert_eq!(
            token_for("RT", "https://b.supabase.co"),
            Err(Unusable::OtherServer)
        );
        assert_eq!(
            token_for("RT", "http://127.0.0.1:54321"),
            Err(Unusable::OtherServer)
        );
    }

    #[test]
    fn a_damaged_credential_gives_no_token() {
        for stored in ["{", "{}", "{\"url\": \"https://a.supabase.co\"}"] {
            assert_eq!(
                token_for(stored, "https://a.supabase.co"),
                Err(Unusable::Damaged),
                "{stored}"
            );
        }
        let extra = r#"{"url":"https://a.supabase.co","refresh_token":"RT","x":1}"#;
        assert_eq!(
            token_for(extra, "https://a.supabase.co"),
            Err(Unusable::Damaged)
        );
    }

    #[test]
    fn a_url_with_quotes_cannot_forge_another_field() {
        let stored = bind("https://a\",\"url\":\"https://b", "RT");
        assert_eq!(token_for(&stored, "https://b"), Err(Unusable::OtherServer));
    }

    #[test]
    fn each_data_folder_has_its_own_credential() {
        let a = credential_name(Path::new(r"C:\Users\x\AppData\Roaming\TavernLedger"));
        let b = credential_name(Path::new(r"C:\Temp\test-data"));
        assert_ne!(a, b);
        assert!(a.starts_with("TavernLedger/upload/"));
        assert_eq!(a.len(), "TavernLedger/upload/".len() + 16);
        assert_eq!(
            a,
            credential_name(Path::new(r"c:\users\x\appdata\roaming\tavernledger"))
        );
    }
}
