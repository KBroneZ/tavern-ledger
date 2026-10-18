//! Which edition of the app this binary is.
//!
//! The Microsoft Store edition is the same code built with the `store` cargo
//! feature (D-158, T-379). That feature is the only switch: nothing is
//! inferred at run time from the package identity, which the app only logs.

/// Built with the `store` feature: the edition submitted to the Microsoft
/// Store. It checks for no updates itself and starts with Windows through the
/// package's start-up task.
pub const STORE: bool = cfg!(feature = "store");

/// Written to the app log at start-up, and the string the package script looks
/// for in the exe: a Store package is refused unless it holds the Store name
/// and not the installer's, so CI cannot pack the other edition's binary.
pub const NAME: &str = if STORE {
    "microsoft-store-edition"
} else {
    "installer-edition"
};
