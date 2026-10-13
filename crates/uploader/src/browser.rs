//! Opens the website's sign-in page in the user's default browser (T-562).
//! Only a web address the app made itself: https, or the local website on
//! this PC; nothing that could read as a file, a program or a switch.

use std::io;

const MAX_URL: usize = 4096;

/// Whether `url` is a web address this module will hand to the system: an
/// https host name, or this PC's address with a port, then a path; no user
/// part, nothing outside printable ASCII and no character a shell or a
/// browser reads specially.
pub fn is_openable(url: &str) -> bool {
    let rest = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://127.0.0.1:") {
        rest
    } else {
        return false;
    };
    let Some((authority, _path)) = rest.split_once('/') else {
        return false;
    };
    let authority_ok = if url.starts_with("https://") {
        let (host, port) = authority.split_once(':').unwrap_or((authority, ""));
        !host.is_empty()
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            && port.bytes().all(|b| b.is_ascii_digit())
    } else {
        !authority.is_empty() && authority.bytes().all(|b| b.is_ascii_digit())
    };
    authority_ok
        && url.len() <= MAX_URL
        && url.bytes().all(|b| {
            b.is_ascii_graphic() && !matches!(b, b'"' | b'<' | b'>' | b'\\' | b'`' | b'^' | b'|')
        })
}

/// Opens `url` in the default browser.
pub fn open(url: &str) -> io::Result<()> {
    if !is_openable(url) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a web address",
        ));
    }
    imp::open(url)
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::iter::once;

    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(once(0)).collect()
    }

    pub fn open(url: &str) -> io::Result<()> {
        let verb = wide("open");
        let target = wide(url);
        // SAFETY: both strings are NUL-terminated UTF-16 that outlive the
        // call; no window, parameters or directory are passed.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // Above 32 is success (the Win32 contract for ShellExecute).
        if result as isize > 32 {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "the browser could not be opened ({})",
                result as isize
            )))
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::io;

    pub fn open(_url: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "opening a browser is only built for Windows",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_addresses_the_app_makes_are_opened() {
        for ok in [
            "https://tavernledger.net/desktop-sign-in/#challenge=abc&redirect=http%3A%2F%2F127.0.0.1%3A5%2Fx&email=a%40b.c",
            "http://127.0.0.1:3000/desktop-sign-in/#x=1",
        ] {
            assert!(is_openable(ok), "{ok}");
        }
        for bad in [
            "",
            "C:\\Windows\\System32\\calc.exe",
            "file:///C:/x",
            "http://example.test/",
            "http://localhost:3000/",
            "https://a b.test/",
            "https://a.test/\"x",
            "https://a.test/\n",
            "https://a.test/é",
            "javascript:alert(1)",
            "ms-settings:",
            "https://tavernledger.net",
            "http://127.0.0.1:80@evil.test/",
            "https://user@tavernledger.net/",
            "https://tavernledger.net:x/",
            "http://127.0.0.1:/x",
        ] {
            assert!(!is_openable(bad), "{bad:?}");
        }
        assert!(!is_openable(&format!(
            "https://a.test/{}",
            "x".repeat(MAX_URL)
        )));
        assert_eq!(
            open("file:///C:/x").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
