//! Which edition of the app this binary is.
//!
//! The Microsoft Store edition is the same code built with the `store` cargo
//! feature (D-158, T-379). That feature is the only switch: nothing is
//! inferred at run time from the package identity, which the app only logs.

/// Built with the `store` feature: the edition submitted to the Microsoft
/// Store. It checks for no updates itself and starts with Windows through the
/// package's start-up task.
pub const STORE: bool = cfg!(feature = "store");

/// Built with the `store-manual-setup` feature on top of `store`: the edition
/// to ship if Microsoft refuses the `unvirtualizedResources` capability. It
/// never writes the game's two logging files; the setup box lists the lines to
/// write by hand instead of offering Fix.
pub const MANUAL_SETUP: bool = cfg!(feature = "store-manual-setup");

/// The first Windows 11 build. The manifest's excluded folders (what lets the
/// Store edition write `log.config` where the game reads it) are documented as
/// Windows 11 behaviour; older builds ignore them.
pub const WINDOWS_11_BUILD: u32 = 22000;

/// Whether the Fix button may write the game's logging files from this
/// edition. The installer edition always may. The Store edition may only
/// where the excluded folder works: not in the manual-setup build, and not in
/// a package on a Windows older than 11, where a write would end up in the
/// package's private store. A process with no package identity is not
/// virtualized at all (a Store build run from a build folder), so it may.
pub fn fix_allowed(store: bool, manual_setup: bool, identity: bool, build: Option<u32>) -> bool {
    if !store {
        return true;
    }
    if manual_setup {
        return false;
    }
    !identity || build.is_some_and(|build| build >= WINDOWS_11_BUILD)
}

/// [`fix_allowed`] for this process.
pub fn fix_allowed_here() -> bool {
    fix_allowed(
        STORE,
        MANUAL_SETUP,
        crate::package::has_identity(),
        crate::package::windows_build(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_installer_edition_always_fixes() {
        for identity in [false, true] {
            for build in [None, Some(19045), Some(26100)] {
                assert!(fix_allowed(false, false, identity, build));
            }
        }
    }

    #[test]
    fn the_store_edition_fixes_only_where_the_excluded_folder_works() {
        // In a package: Windows 11 yes; Windows 10 and an unknown build no.
        assert!(fix_allowed(true, false, true, Some(22000)));
        assert!(fix_allowed(true, false, true, Some(26100)));
        assert!(!fix_allowed(true, false, true, Some(21999)));
        assert!(!fix_allowed(true, false, true, Some(19045)));
        assert!(!fix_allowed(true, false, true, None));
        // No package, no virtualization, nothing to fear.
        assert!(fix_allowed(true, false, false, Some(19045)));
        assert!(fix_allowed(true, false, false, None));
    }

    #[test]
    fn the_manual_setup_build_never_fixes() {
        for identity in [false, true] {
            assert!(!fix_allowed(true, true, identity, Some(26100)));
        }
    }
}
