//! Child processes without a console window. The desktop app is a GUI
//! program, so a console program it starts would flash a window; every
//! process the app or the tracker starts goes through [`command`].

use std::ffi::OsStr;
use std::process::Command;

/// Windows' `CREATE_NO_WINDOW` process creation flag.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command that opens no console window (a plain command elsewhere).
pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}
