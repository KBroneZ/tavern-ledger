//! Read-only reader for two values the game shows the player (D-063): the
//! lobby's five tribes at hero select and the player's own Battlegrounds
//! rating (Solo, Duos).
//!
//! - **Read-only.** The game is opened with query (limited) and read-memory
//!   rights only ([`process::ACCESS_RIGHTS`]); never write, thread, debug or
//!   injection rights (D-006). The handle is closed after each read.
//! - **Narrow.** One fixed path of named fields per value ([`game`]); no
//!   scans, no other data. Names only from Mono's metadata.
//! - **Fail safe.** Every value is checked; on any doubt the caller gets
//!   "unsupported game version" and keeps its log-based values.
//! - The app calls this only when the user turned the switch on (off by
//!   default); with it off, nothing here reads the game. The one thing asked of
//!   the running game without the switch is where its `Hearthstone.exe` is
//!   ([`location`], the limited query right only, no memory access).

pub mod game;
pub mod location;
pub mod memory;
pub mod mono;
pub mod pe;
pub mod process;

pub use game::{Rating, ReadFailure};

#[cfg(windows)]
fn with_mono<T>(
    read: impl FnOnce(&mono::Mono) -> Result<T, ReadFailure>,
) -> Result<T, ReadFailure> {
    use process::{GameProcess, OpenError};
    let game = GameProcess::open().map_err(|e| match e {
        OpenError::NotRunning => ReadFailure::NotRunning,
        OpenError::AccessDenied => ReadFailure::NoAccess,
        // Mono loads a moment after the game starts.
        OpenError::NoMono => ReadFailure::NotYet,
        OpenError::Unsupported => ReadFailure::Unsupported("not Windows".into()),
    })?;
    let mono = mono::Mono::attach(&game, game.mono_base(), mono::LAYOUT_BDWGC_X64).map_err(
        |e| match e {
            mono::WalkError::Unsupported(why) => ReadFailure::Unsupported(why),
            mono::WalkError::Read(memory::ReadError::BadPointer(_)) => {
                ReadFailure::Unsupported("a pointer out of range".into())
            }
            mono::WalkError::Read(_) => ReadFailure::NotYet,
        },
    )?;
    read(&mono)
    // `game` drops here: the handle is closed.
}

#[cfg(not(windows))]
fn with_mono<T>(
    _read: impl FnOnce(&mono::Mono) -> Result<T, ReadFailure>,
) -> Result<T, ReadFailure> {
    Err(ReadFailure::NotRunning)
}

/// The lobby's five tribes (names as the log writes them), checked.
pub fn lobby_tribes() -> Result<Vec<&'static str>, ReadFailure> {
    with_mono(game::read_lobby_tribes)
}

/// The player's own Battlegrounds rating, checked.
pub fn own_rating() -> Result<Rating, ReadFailure> {
    with_mono(game::read_own_rating)
}

/// What one check run found, for the app's status and the `game-memory-check`
/// tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub tribes: Result<Vec<&'static str>, ReadFailure>,
    pub rating: Result<Rating, ReadFailure>,
}

/// Tries both reads once.
pub fn check() -> Check {
    Check {
        tribes: lobby_tribes(),
        rating: own_rating(),
    }
}

#[cfg(test)]
mod scope_tests {
    //! The reader walks only the fields D-063 names (D-004 as amended).

    /// Every name the walker may look up or check a type against, and
    /// nothing else.
    const ALLOWED: [&str; 28] = [
        "Assembly-CSharp",
        "",
        "GameState",
        "s_instance",
        "m_availableRacesInBattlegroundsExcludingAmalgam",
        "m_missingRacesInBattlegrounds",
        "List`1",
        "NetCacheBaconRatingInfo",
        "<Rating>k__BackingField",
        "<DuosRating>k__BackingField",
        "Blizzard.T5.ServiceLocator",
        "Blizzard.T5.Services",
        "ServiceManager",
        "s_runtimeServices",
        "ServiceLocator",
        "m_services",
        "Dictionary`2",
        "_entries",
        "_count",
        "ServiceInfo",
        "<Service>k__BackingField",
        "NetCache",
        "m_netCache",
        "Map`2",
        "valueSlots",
        "touchedSlots",
        "Entry[]",
        "Object[]",
    ];

    fn code(src: &str) -> String {
        src.lines()
            .take_while(|l| !l.starts_with("#[cfg(test)]"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_names_the_reader_looks_up_are_only_d063s() {
        let game = code(include_str!("game.rs"));
        let names: Vec<&str> = game
            .lines()
            .filter(|l| l.contains("const ") && l.contains(": &str = \""))
            .filter_map(|l| l.split('"').nth(1))
            .collect();
        assert_eq!(names, ALLOWED);
    }

    #[test]
    fn lookups_never_take_a_name_written_in_place() {
        for (file, src) in [
            ("game.rs", include_str!("game.rs")),
            ("lib.rs", include_str!("lib.rs")),
        ] {
            for line in code(src).lines() {
                let lookup = [
                    ".image(",
                    ".class(",
                    ".field(",
                    "static_object(",
                    "object_field(",
                    "int_field(",
                ]
                .iter()
                .any(|call| line.contains(call));
                assert!(
                    !(lookup && line.contains('"')),
                    "{file}: a lookup with a literal name: {line}"
                );
            }
        }
    }
}
