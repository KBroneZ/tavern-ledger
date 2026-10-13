//! Where the running game is installed (T-579): the full path of its
//! `Hearthstone.exe`, as Task Manager shows it.
//!
//! The process is opened with `PROCESS_QUERY_LIMITED_INFORMATION` only: the
//! right Windows gives for reading a process's name and path, which does not
//! allow reading its memory or changing it. This runs without the memory
//! switch (it is not a memory read) and is asked for the folder only, never
//! for any value of the game. If Windows refuses (the game runs as
//! administrator) the answer is simply "unknown".

use std::path::PathBuf;

use crate::process::GAME_EXE;

/// The path of the running game's `Hearthstone.exe`; `None` when it does not
/// run, when Windows says no, and off Windows.
pub fn game_exe_path() -> Option<PathBuf> {
    win::image_path(find_pid(GAME_EXE)?)
}

#[cfg(windows)]
use crate::process::find_pid;

#[cfg(not(windows))]
fn find_pid(_exe: &str) -> Option<u32> {
    None
}

#[cfg(windows)]
mod win {
    use std::path::PathBuf;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW};

    use crate::process::QUERY_LIMITED;

    /// Longest path Windows allows (32,767 units) plus the end.
    const MAX_PATH_UNITS: usize = 32_768;

    pub fn image_path(pid: u32) -> Option<PathBuf> {
        // SAFETY: plain call with the limited query right only; the handle
        // is closed below.
        let handle = unsafe { OpenProcess(QUERY_LIMITED, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; MAX_PATH_UNITS];
        let mut size = buffer.len() as u32;
        // SAFETY: `buffer` holds `size` UTF-16 units and the call writes at
        // most that many, setting `size` to the length written.
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
        // SAFETY: the handle came from OpenProcess and is closed once.
        unsafe { CloseHandle(handle) };
        if ok == 0 {
            return None;
        }
        let units = buffer.get(..size as usize)?;
        Some(PathBuf::from(String::from_utf16_lossy(units)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reads_the_path_of_a_running_program() {
            let own = image_path(std::process::id()).expect("this test's own path");
            let expected = std::env::current_exe().unwrap();
            assert_eq!(
                own.to_string_lossy().to_lowercase(),
                expected.to_string_lossy().to_lowercase()
            );
        }
    }
}

#[cfg(not(windows))]
mod win {
    use std::path::PathBuf;

    pub fn image_path(_pid: u32) -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    /// Only the limited query right is asked for here, and nothing that
    /// reads memory or writes to the game.
    #[test]
    fn source_asks_for_the_limited_query_right_only() {
        let src = include_str!("location.rs");
        let code: String = src
            .lines()
            .take_while(|l| !l.starts_with("#[cfg(test)]"))
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(code.matches("OpenProcess(").count(), 1);
        assert!(code.contains("OpenProcess(QUERY_LIMITED, 0, pid)"));
        assert_eq!(crate::process::QUERY_LIMITED, 0x1000);
        for banned in [
            "ReadProcessMemory",
            "WriteProcessMemory",
            "VM_READ",
            "ACCESS_RIGHTS",
            "PROCESS_ALL_ACCESS",
            "CreateRemoteThread",
            "DebugActiveProcess",
        ] {
            assert!(!code.contains(banned), "{banned} in location.rs");
        }
    }
}
