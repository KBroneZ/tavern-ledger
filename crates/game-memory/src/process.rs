//! The game process, opened read-only (D-063).
//!
//! The process is opened with exactly two access rights: query (limited) and
//! read memory. Never write, never create threads, never debug: those rights
//! are not asked for, so Windows would refuse such calls even by mistake.
//! If the game runs with more privileges than the app, opening fails and the
//! reader reports it; it never asks for elevation or retries with more rights.

use crate::memory::{Memory, ReadError};

/// `PROCESS_QUERY_LIMITED_INFORMATION` (0x1000).
pub const QUERY_LIMITED: u32 = 0x1000;
/// `PROCESS_VM_READ` (0x0010).
pub const VM_READ: u32 = 0x0010;
/// The only rights the reader ever asks for.
pub const ACCESS_RIGHTS: u32 = QUERY_LIMITED | VM_READ;

pub const GAME_EXE: &str = "Hearthstone.exe";
pub const MONO_MODULE: &str = "mono-2.0-bdwgc.dll";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    NotRunning,
    /// Windows refused the read rights (for example, the game runs as
    /// administrator). Never retried with more rights.
    AccessDenied,
    /// The game's Mono runtime module is not loaded (yet).
    NoMono,
    /// Not on Windows.
    Unsupported,
}

#[cfg(windows)]
pub use win::GameProcess;

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::c_void;
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        EnumProcessModulesEx, GetModuleBaseNameW, LIST_MODULES_64BIT,
    };
    use windows_sys::Win32::System::Threading::OpenProcess;

    /// Most modules a process is expected to load; more are ignored.
    const MAX_MODULES: usize = 1024;

    pub struct GameProcess {
        handle: HANDLE,
        mono_base: u64,
    }

    impl GameProcess {
        /// Finds the game and opens it with [`ACCESS_RIGHTS`] only.
        pub fn open() -> Result<GameProcess, OpenError> {
            let pid = find_pid(GAME_EXE).ok_or(OpenError::NotRunning)?;
            // SAFETY: plain call; the rights are the two constants above and
            // the handle is closed in Drop.
            let handle = unsafe { OpenProcess(ACCESS_RIGHTS, 0, pid) };
            if handle.is_null() {
                return Err(OpenError::AccessDenied);
            }
            let mut process = GameProcess {
                handle,
                mono_base: 0,
            };
            process.mono_base = process.module_base(MONO_MODULE).ok_or(OpenError::NoMono)?;
            Ok(process)
        }

        pub fn mono_base(&self) -> u64 {
            self.mono_base
        }

        fn module_base(&self, name: &str) -> Option<u64> {
            let mut modules: Vec<HMODULE> = vec![std::ptr::null_mut(); MAX_MODULES];
            let mut needed = 0u32;
            let bytes = (modules.len() * size_of::<HMODULE>()) as u32;
            // SAFETY: `modules` holds `bytes` bytes; `needed` receives a count.
            let ok = unsafe {
                EnumProcessModulesEx(
                    self.handle,
                    modules.as_mut_ptr(),
                    bytes,
                    &mut needed,
                    LIST_MODULES_64BIT,
                )
            };
            if ok == 0 {
                return None;
            }
            let count = (needed as usize / size_of::<HMODULE>()).min(MAX_MODULES);
            modules[..count].iter().copied().find_map(|module| {
                let mut buf = [0u16; 260];
                // SAFETY: `buf` holds 260 UTF-16 units; the call writes at most that.
                let len = unsafe {
                    GetModuleBaseNameW(self.handle, module, buf.as_mut_ptr(), buf.len() as u32)
                } as usize;
                let base = String::from_utf16_lossy(&buf[..len.min(buf.len())]);
                base.eq_ignore_ascii_case(name).then_some(module as u64)
            })
        }
    }

    impl Drop for GameProcess {
        fn drop(&mut self) {
            // SAFETY: the handle came from OpenProcess and is closed once.
            unsafe { CloseHandle(self.handle) };
        }
    }

    impl Memory for GameProcess {
        fn read(&self, addr: u64, buf: &mut [u8]) -> Result<(), ReadError> {
            let mut got = 0usize;
            // SAFETY: `buf` is a valid writable buffer of `buf.len()` bytes;
            // ReadProcessMemory only reads the other process.
            let ok = unsafe {
                ReadProcessMemory(
                    self.handle,
                    addr as *const c_void,
                    buf.as_mut_ptr() as *mut c_void,
                    buf.len(),
                    &mut got,
                )
            };
            if ok == 0 || got != buf.len() {
                return Err(ReadError::Unreadable(addr));
            }
            Ok(())
        }
    }

    /// Lists processes without opening any of them.
    fn find_pid(exe: &str) -> Option<u32> {
        // SAFETY: plain call; the snapshot handle is closed below.
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snap == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut found = None;
        // SAFETY: `entry` is a PROCESSENTRY32W with dwSize set.
        let mut more = unsafe { Process32FirstW(snap, &mut entry) } != 0;
        while more {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).eq_ignore_ascii_case(exe) {
                found = Some(entry.th32ProcessID);
                break;
            }
            // SAFETY: as above.
            more = unsafe { Process32NextW(snap, &mut entry) } != 0;
        }
        // SAFETY: the snapshot handle is closed once.
        unsafe { CloseHandle(snap) };
        found
    }

    /// Whether the game is running, without opening it.
    pub fn is_running() -> bool {
        find_pid(GAME_EXE).is_some()
    }
}

#[cfg(windows)]
pub use win::is_running;

#[cfg(not(windows))]
pub fn is_running() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_for_read_and_limited_query_rights_only() {
        assert_eq!(ACCESS_RIGHTS, 0x1010);
        // Write (0x20), operation (0x08), create thread (0x02), all access,
        // duplicate handle (0x40), set information (0x200), suspend (0x800),
        // full query (0x400) and terminate (0x01) are never asked for.
        for forbidden in [
            0x0001, 0x0002, 0x0008, 0x0020, 0x0040, 0x0200, 0x0400, 0x0800,
        ] {
            assert_eq!(ACCESS_RIGHTS & forbidden, 0, "right {forbidden:#x}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn rights_match_the_windows_constants() {
        use windows_sys::Win32::System::Threading::{
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE,
        };
        assert_eq!(
            ACCESS_RIGHTS,
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ
        );
        assert_eq!(ACCESS_RIGHTS & (PROCESS_VM_WRITE | PROCESS_VM_OPERATION), 0);
    }

    /// The source asks Windows for no other right: only `ACCESS_RIGHTS` is
    /// passed to `OpenProcess`, and no write, thread or debug API is used.
    #[test]
    fn source_opens_with_access_rights_only() {
        let src = include_str!("process.rs");
        let code: String = src
            .lines()
            .take_while(|l| !l.starts_with("#[cfg(test)]"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(code.matches("OpenProcess(").count(), 1);
        assert!(code.contains("OpenProcess(ACCESS_RIGHTS, 0, pid)"));
        for banned in [
            "WriteProcessMemory",
            "VirtualAllocEx",
            "VirtualProtectEx",
            "CreateRemoteThread",
            "DebugActiveProcess",
            "PROCESS_ALL_ACCESS",
            "PROCESS_VM_WRITE",
            "PROCESS_VM_OPERATION",
            "PROCESS_CREATE_THREAD",
        ] {
            assert!(!code.contains(banned), "{banned} in process.rs");
        }
    }
}
