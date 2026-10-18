//! Facts about the package the Store edition runs from, and where Windows
//! keeps what a package writes to a virtualized folder.
//!
//! A packaged app's new files under `%LOCALAPPDATA%` and `%APPDATA%` go to the
//! package's private store unless the folder exists on the real disk or the
//! manifest excludes it. The app cannot see the difference from inside, so
//! after a write it looks for a copy in the private store: finding one means
//! Windows redirected the write and the game will not see the file.

use std::path::{Path, PathBuf};

/// Where the copy of `target` would be in the package's private store:
/// `private_root` is the store's `LocalCache\Local` folder, which stands in
/// for `%LOCALAPPDATA%`. `None` when `target` is not under `local_app_data`.
pub fn private_copy(private_root: &Path, local_app_data: &Path, target: &Path) -> Option<PathBuf> {
    let relative = relative_to(target, local_app_data)?;
    Some(private_root.join(relative))
}

/// `path` below `base`, comparing the parts the way Windows does (ignoring
/// case). `Path::strip_prefix` is exact, so a differently cased spelling of
/// the same folder would read as "not under it".
fn relative_to(path: &Path, base: &Path) -> Option<PathBuf> {
    let mut parts = path.components();
    for want in base.components() {
        let got = parts.next()?;
        let same = got.as_os_str().to_string_lossy().to_lowercase()
            == want.as_os_str().to_string_lossy().to_lowercase();
        if !same {
            return None;
        }
    }
    Some(parts.as_path().to_path_buf())
}

/// The package's private `LocalCache\Local` folder, when this process runs
/// from a package.
pub fn private_local_root(local_app_data: &Path) -> Option<PathBuf> {
    let family = family_name()?;
    Some(
        local_app_data
            .join("Packages")
            .join(family)
            .join("LocalCache")
            .join("Local"),
    )
}

/// The Windows build number (`CurrentBuildNumber`), when it can be read.
#[cfg(windows)]
pub fn windows_build() -> Option<u32> {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    let key = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", KEY_READ)
        .ok()?;
    let text: String = key.get_value("CurrentBuildNumber").ok()?;
    text.trim().parse().ok()
}

#[cfg(not(windows))]
pub fn windows_build() -> Option<u32> {
    None
}

#[cfg(feature = "store")]
mod win {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
    use windows::Win32::Storage::Packaging::Appx::{
        GetCurrentPackageFamilyName, GetCurrentPackageFullName,
    };

    /// Whether this process runs from an installed package. Asking for the
    /// name's length is enough: a process without identity gets
    /// `APPMODEL_ERROR_NO_PACKAGE`.
    pub fn has_identity() -> bool {
        let mut length = 0u32;
        // SAFETY: `length` is a valid, writable u32 and no buffer is passed.
        let code = unsafe { GetCurrentPackageFullName(&mut length, None) };
        code == ERROR_INSUFFICIENT_BUFFER
    }

    pub fn family_name() -> Option<String> {
        let mut length = 0u32;
        // SAFETY: as above; this call only reports the needed length.
        let code = unsafe { GetCurrentPackageFamilyName(&mut length, None) };
        if code != ERROR_INSUFFICIENT_BUFFER || length == 0 {
            return None;
        }
        let mut buffer = vec![0u16; length as usize];
        // SAFETY: `buffer` holds `length` u16, the size Windows asked for.
        let code =
            unsafe { GetCurrentPackageFamilyName(&mut length, Some(PWSTR(buffer.as_mut_ptr()))) };
        if code != ERROR_SUCCESS {
            return None;
        }
        // `length` counts the terminating zero.
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        String::from_utf16(&buffer[..end]).ok()
    }
}

#[cfg(feature = "store")]
pub use win::{family_name, has_identity};

/// Without the `store` feature the app is never packaged.
#[cfg(not(feature = "store"))]
pub fn has_identity() -> bool {
    false
}

#[cfg(not(feature = "store"))]
pub fn family_name() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_under_local_app_data_has_its_copy_at_the_same_place_in_the_private_store() {
        let local = Path::new(r"C:\Users\A\AppData\Local");
        let private = Path::new(r"C:\Users\A\AppData\Local\Packages\TL_x\LocalCache\Local");
        let target = local
            .join("Blizzard")
            .join("Hearthstone")
            .join("log.config");
        assert_eq!(
            private_copy(private, local, &target),
            Some(
                private
                    .join("Blizzard")
                    .join("Hearthstone")
                    .join("log.config")
            )
        );
    }

    #[test]
    fn the_folders_are_compared_the_way_windows_does_ignoring_case() {
        let local = Path::new(r"C:\Users\A\AppData\Local");
        let private = Path::new(r"C:\x\Local");
        let target = Path::new(r"c:\users\a\APPDATA\local\Blizzard\Hearthstone\log.config");
        assert_eq!(
            private_copy(private, local, target),
            Some(
                private
                    .join("Blizzard")
                    .join("Hearthstone")
                    .join("log.config")
            )
        );
    }

    #[test]
    fn a_file_elsewhere_has_no_private_copy() {
        let local = Path::new(r"C:\Users\A\AppData\Local");
        let private = Path::new(r"C:\x\Local");
        assert_eq!(
            private_copy(
                private,
                local,
                Path::new(r"D:\Games\Hearthstone\client.config")
            ),
            None
        );
        assert_eq!(
            private_copy(private, local, Path::new(r"C:\Users\A\AppData\LocalLow\x")),
            None,
            "a sibling folder with the same start is not under it"
        );
    }

    #[test]
    fn a_test_process_has_no_package_identity() {
        assert!(!has_identity());
        assert!(family_name().is_none());
        assert!(private_local_root(Path::new(r"C:\L")).is_none());
    }

    #[test]
    fn this_windows_has_a_build_number() {
        if cfg!(windows) {
            assert!(windows_build().is_some_and(|build| build >= 10240));
        }
    }
}
