//! The two values D-063 allows, and nothing else: the lobby's five tribes
//! and the player's own Battlegrounds rating (Solo, Duos). Each has one fixed
//! path of named fields, found on the user's own client (build 253216,
//! `docs/research/memory-reader.md`). Every value is checked before it is
//! returned; a value that fails a check is never returned.

use crate::memory::ReadError;
use crate::mono::{Mono, WalkError};

pub const ASSEMBLY: &str = "Assembly-CSharp";
/// Both classes are in the global namespace.
pub const NAMESPACE: &str = "";

/// The lobby's tribes: `GameState.s_instance`, then two lists of race ids
/// the game shows at hero select (in the lobby, and left out of it).
pub const STATE_CLASS: &str = "GameState";
pub const STATE_INSTANCE: &str = "s_instance";
pub const TRIBES_IN: &str = "m_availableRacesInBattlegroundsExcludingAmalgam";
pub const TRIBES_OUT: &str = "m_missingRacesInBattlegrounds";
/// The type both lists must be (a check, not a lookup).
pub const LIST_CLASS: &str = "List`1";

/// The player's own rating, as the Battlegrounds screen shows it.
pub const RATING_CLASS: &str = "NetCacheBaconRatingInfo";
pub const RATING_SOLO: &str = "<Rating>k__BackingField";
pub const RATING_DUOS: &str = "<DuosRating>k__BackingField";

/// The path to it (T-337, build 253216): the game's service locator
/// (`ServiceManager.s_runtimeServices`, a `ServiceLocator`), its services
/// (`m_services`, a `Dictionary`2` of `ServiceInfo`), the one whose service
/// is the `NetCache`, then its cache (`m_netCache`, a `Map`2`) and the one
/// value there that is a `NetCacheBaconRatingInfo`.
pub const LOCATOR_ASSEMBLY: &str = "Blizzard.T5.ServiceLocator";
pub const LOCATOR_NAMESPACE: &str = "Blizzard.T5.Services";
pub const MANAGER_CLASS: &str = "ServiceManager";
pub const MANAGER_SERVICES: &str = "s_runtimeServices";
pub const LOCATOR_CLASS: &str = "ServiceLocator";
pub const LOCATOR_MAP: &str = "m_services";
pub const DICTIONARY_CLASS: &str = "Dictionary`2";
pub const DICTIONARY_ENTRIES: &str = "_entries";
pub const DICTIONARY_COUNT: &str = "_count";
pub const SERVICE_INFO_CLASS: &str = "ServiceInfo";
pub const SERVICE_INFO_SERVICE: &str = "<Service>k__BackingField";
pub const CACHE_CLASS: &str = "NetCache";
pub const CACHE_MAP: &str = "m_netCache";
pub const MAP_CLASS: &str = "Map`2";
pub const MAP_VALUES: &str = "valueSlots";
pub const MAP_USED: &str = "touchedSlots";
/// The two array types on the path (checks, not lookups).
pub const ENTRIES_ARRAY: &str = "Entry[]";
pub const VALUES_ARRAY: &str = "Object[]";

/// A Battlegrounds lobby has five tribes.
pub const LOBBY_TRIBES: usize = 5;
/// The game's race ids (`TAG_RACE`) of the Battlegrounds tribes, named as
/// the log names them (`CARDRACE`). Update with the tracker's list when the
/// game adds a tribe.
pub const TRIBE_IDS: [(i32, &str); 11] = [
    (126, "ABERRATION"),
    (20, "BEAST"),
    (15, "DEMON"),
    (24, "DRAGON"),
    (18, "ELEMENTAL"),
    (17, "MECHANICAL"),
    (14, "MURLOC"),
    (92, "NAGA"),
    (23, "PIRATE"),
    (43, "QUILBOAR"),
    (11, "UNDEAD"),
];
/// Highest rating accepted as real; the top of the public leaderboard is
/// far below it.
pub const MAX_RATING: i32 = 30_000;
/// Longest race list read (the game has 11 tribes).
const MAX_LIST: usize = 16;
/// Most services and cache values read (the game has a few dozen of each).
const MAX_SERVICES: usize = 512;
const MAX_CACHE: usize = 1024;
/// A `Dictionary<ref, ref>` entry: hash code and next index (4 bytes each),
/// then the key and the value.
const ENTRY_SIZE: u64 = 24;
const ENTRY_VALUE: u64 = 16;

/// The player's own rating. `None` when the game shows none for that mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rating {
    pub solo: Option<u32>,
    pub duos: Option<u32>,
}

/// Why a value could not be read. Shown to the user as a status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadFailure {
    /// The game is not running.
    NotRunning,
    /// Windows refused read access (for example, the game runs as
    /// administrator). Never retried with more rights.
    NoAccess,
    /// The value is not there yet (no game in progress, or hero select has
    /// not loaded it): try again later.
    NotYet,
    /// The game's structure or a check did not match: "unsupported game
    /// version". The reason names only types and checks.
    Unsupported(String),
}

impl std::fmt::Display for ReadFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadFailure::NotRunning => write!(f, "the game is not running"),
            ReadFailure::NoAccess => write!(f, "no read access to the game"),
            ReadFailure::NotYet => write!(f, "not available yet"),
            ReadFailure::Unsupported(why) => write!(f, "unsupported game version ({why})"),
        }
    }
}

fn unsupported(why: impl Into<String>) -> ReadFailure {
    ReadFailure::Unsupported(why.into())
}

/// A failed walk: a missing class, field or bound is an unknown version; an
/// unreadable address is the game changing under us, so try again later.
fn from_walk(e: WalkError) -> ReadFailure {
    match e {
        WalkError::Unsupported(why) => ReadFailure::Unsupported(why),
        // A value where a pointer must be is not the game changing under us.
        WalkError::Read(ReadError::BadPointer(_)) => unsupported("a pointer out of range"),
        WalkError::Read(_) => ReadFailure::NotYet,
    }
}

/// The lobby's tribes, checked: five distinct Battlegrounds tribes in the
/// lobby, a non-empty list of distinct tribes left out, none of them also in
/// the lobby, and every listed id a known tribe. Sorted by name.
pub fn check_tribes(inside: &[i32], outside: &[i32]) -> Result<Vec<&'static str>, ReadFailure> {
    let name = |id: &i32| TRIBE_IDS.iter().find(|(t, _)| t == id).map(|(_, n)| *n);
    if inside.is_empty() || outside.is_empty() {
        return Err(ReadFailure::NotYet);
    }
    if inside.len() != LOBBY_TRIBES {
        return Err(unsupported("lobby does not have five tribes"));
    }
    let mut names: Vec<&'static str> = inside
        .iter()
        .map(|id| name(id).ok_or_else(|| unsupported("unknown tribe id")))
        .collect::<Result<_, _>>()?;
    names.sort_unstable();
    names.dedup();
    if names.len() != LOBBY_TRIBES {
        return Err(unsupported("a tribe is listed twice"));
    }
    for (i, id) in outside.iter().enumerate() {
        if outside[..i].contains(id) {
            return Err(unsupported("a left-out tribe is listed twice"));
        }
        if name(id).is_none() {
            return Err(unsupported("unknown tribe id"));
        }
        if inside.contains(id) {
            return Err(unsupported("a tribe is both in and out of the lobby"));
        }
    }
    Ok(names)
}

/// A rating value, checked: in range, and 0 or less is "none shown".
pub fn check_rating(value: i32) -> Result<Option<u32>, ReadFailure> {
    match value {
        v if v > MAX_RATING => Err(unsupported("rating out of range")),
        v if v < 0 => Err(unsupported("rating out of range")),
        0 => Ok(None),
        v => Ok(Some(v as u32)),
    }
}

/// Reads the lobby's tribes through `mono`.
pub fn read_lobby_tribes(mono: &Mono) -> Result<Vec<&'static str>, ReadFailure> {
    let image = mono.image(ASSEMBLY).map_err(from_walk)?;
    let class = mono
        .class(image, NAMESPACE, STATE_CLASS)
        .map_err(from_walk)?;
    // No game state outside a game: not yet, not unsupported.
    let state = match mono.static_object(class, STATE_INSTANCE) {
        Ok(s) => s,
        Err(WalkError::Unsupported(why)) if why.ends_with("is not set") => {
            return Err(ReadFailure::NotYet)
        }
        Err(e) => return Err(from_walk(e)),
    };
    // The object must be a GameState of this image, not one of another
    // class that happens to sit there.
    if mono.object_class(state).map_err(from_walk)? != class {
        return Err(unsupported("game state of another type"));
    }
    let list = |field: &str| -> Result<Vec<i32>, ReadFailure> {
        match mono.object_field(state, class, field) {
            Ok(l) => mono
                .typed_int_list(l, LIST_CLASS, MAX_LIST)
                .map_err(from_walk),
            Err(WalkError::Unsupported(why)) if why.ends_with("is not set") => Ok(Vec::new()),
            Err(e) => Err(from_walk(e)),
        }
    };
    let inside = list(TRIBES_IN)?;
    let outside = list(TRIBES_OUT)?;
    check_tribes(&inside, &outside)
}

/// Reads the player's own rating through `mono`, along the path above.
/// Every object on it must be of the expected class.
pub fn read_own_rating(mono: &Mono) -> Result<Rating, ReadFailure> {
    let cache = net_cache(mono)?;
    let image = mono.image(ASSEMBLY).map_err(from_walk)?;
    let cache_class = mono
        .class(image, NAMESPACE, CACHE_CLASS)
        .map_err(from_walk)?;
    let rating_class = mono
        .class(image, NAMESPACE, RATING_CLASS)
        .map_err(from_walk)?;
    if mono.object_class(cache).map_err(from_walk)? != cache_class {
        return Err(unsupported("cache of another type"));
    }
    let map = mono
        .object_field(cache, cache_class, CACHE_MAP)
        .map_err(not_yet_if_unset)?;
    if mono.object_class_name(map).map_err(from_walk)? != MAP_CLASS {
        return Err(unsupported("cache map of another type"));
    }
    let map_class = mono.object_class(map).map_err(from_walk)?;
    let used = count(
        mono.int_field(map, map_class, MAP_USED)
            .map_err(from_walk)?,
    )?;
    let values = mono
        .object_field(map, map_class, MAP_VALUES)
        .map_err(not_yet_if_unset)?;
    let mut found = None;
    for value in mono
        .ref_array(values, VALUES_ARRAY, used, MAX_CACHE)
        .map_err(from_walk)?
        .into_iter()
        .flatten()
    {
        if mono.object_class(value).map_err(from_walk)? == rating_class {
            if found.is_some() {
                return Err(unsupported("two rating objects"));
            }
            found = Some(value);
        }
    }
    // The cache fills a moment after the client logs in.
    let info = found.ok_or(ReadFailure::NotYet)?;
    read_rating_info(mono, info)
}

/// The `NetCache` service, from the service locator.
fn net_cache(mono: &Mono) -> Result<u64, ReadFailure> {
    let image = mono.image(LOCATOR_ASSEMBLY).map_err(from_walk)?;
    let manager = mono
        .class(image, LOCATOR_NAMESPACE, MANAGER_CLASS)
        .map_err(from_walk)?;
    let locator_class = mono
        .class(image, LOCATOR_NAMESPACE, LOCATOR_CLASS)
        .map_err(from_walk)?;
    let locator = mono
        .static_object(manager, MANAGER_SERVICES)
        .map_err(not_yet_if_unset)?;
    if mono.object_class(locator).map_err(from_walk)? != locator_class {
        return Err(unsupported("service locator of another type"));
    }
    let services = mono
        .object_field(locator, locator_class, LOCATOR_MAP)
        .map_err(not_yet_if_unset)?;
    if mono.object_class_name(services).map_err(from_walk)? != DICTIONARY_CLASS {
        return Err(unsupported("services of another type"));
    }
    let dict_class = mono.object_class(services).map_err(from_walk)?;
    let used = count(
        mono.int_field(services, dict_class, DICTIONARY_COUNT)
            .map_err(from_walk)?,
    )?;
    let entries = mono
        .object_field(services, dict_class, DICTIONARY_ENTRIES)
        .map_err(not_yet_if_unset)?;
    let infos = mono
        .struct_array_refs(
            entries,
            ENTRIES_ARRAY,
            used,
            MAX_SERVICES,
            ENTRY_SIZE,
            ENTRY_VALUE,
        )
        .map_err(from_walk)?;
    let mut found = None;
    for info in infos.into_iter().flatten() {
        if mono.object_class_name(info).map_err(from_walk)? != SERVICE_INFO_CLASS {
            return Err(unsupported("a service entry of another type"));
        }
        let info_class = mono.object_class(info).map_err(from_walk)?;
        let service = match mono.object_field(info, info_class, SERVICE_INFO_SERVICE) {
            Ok(s) => s,
            // A service registered but not made yet.
            Err(WalkError::Unsupported(why)) if why.ends_with("is not set") => continue,
            Err(e) => return Err(from_walk(e)),
        };
        if mono.object_class_name(service).map_err(from_walk)? == CACHE_CLASS {
            if found.is_some() {
                return Err(unsupported("two caches"));
            }
            found = Some(service);
        }
    }
    found.ok_or(ReadFailure::NotYet)
}

/// A count read from the game: never negative.
fn count(value: i32) -> Result<usize, ReadFailure> {
    usize::try_from(value).map_err(|_| unsupported("negative count"))
}

/// A reference that is not set yet is "not yet"; anything else as usual.
fn not_yet_if_unset(e: WalkError) -> ReadFailure {
    match e {
        WalkError::Unsupported(why) if why.ends_with("is not set") => ReadFailure::NotYet,
        e => from_walk(e),
    }
}

/// Reads the rating from a `NetCacheBaconRatingInfo` object, after checking
/// that the object is one.
pub fn read_rating_info(mono: &Mono, info: u64) -> Result<Rating, ReadFailure> {
    let class_name = mono.object_class_name(info).map_err(from_walk)?;
    if class_name != RATING_CLASS {
        return Err(unsupported("rating object of another type"));
    }
    let image = mono.image(ASSEMBLY).map_err(from_walk)?;
    let class = mono
        .class(image, NAMESPACE, RATING_CLASS)
        .map_err(from_walk)?;
    let solo = mono
        .int_field(info, class, RATING_SOLO)
        .map_err(from_walk)?;
    let duos = mono
        .int_field(info, class, RATING_DUOS)
        .map_err(from_walk)?;
    Ok(Rating {
        solo: check_rating(solo)?,
        duos: check_rating(duos)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mono::fake::{FakeField, FakeMono, MONO_BASE};
    use crate::mono::LAYOUT_BDWGC_X64;

    const IN: [i32; 5] = [20, 24, 14, 11, 23];
    const OUT: [i32; 5] = [126, 15, 18, 43, 17];

    #[test]
    fn five_known_distinct_tribes_pass_sorted_by_name() {
        assert_eq!(
            check_tribes(&IN, &OUT).unwrap(),
            ["BEAST", "DRAGON", "MURLOC", "PIRATE", "UNDEAD"]
        );
        // The list of tribes left out must be there too: empty is not yet.
        assert_eq!(check_tribes(&IN, &[]), Err(ReadFailure::NotYet));
    }

    #[test]
    fn tribes_that_fail_a_check_are_never_returned() {
        let bad = |inside: &[i32], outside: &[i32]| {
            matches!(
                check_tribes(inside, outside),
                Err(ReadFailure::Unsupported(_))
            )
        };
        assert!(bad(&[20, 24, 14, 11], &OUT));
        assert!(bad(&[20, 24, 14, 11, 23, 15], &OUT));
        assert!(bad(&[20, 24, 14, 11, 11], &OUT));
        assert!(bad(&[20, 24, 14, 11, 99], &OUT));
        assert!(bad(&IN, &[126, 15, 15]));
        assert!(bad(&[20, 24, 14, 11, 23], &[23]));
        assert!(bad(&[20, 24, 14, 11, 23], &[7]));
        assert_eq!(check_tribes(&[], &[]), Err(ReadFailure::NotYet));
    }

    #[test]
    fn ratings_are_checked() {
        assert_eq!(check_rating(6543), Ok(Some(6543)));
        assert_eq!(check_rating(0), Ok(None));
        assert!(check_rating(-1).is_err());
        assert!(check_rating(MAX_RATING + 1).is_err());
    }

    #[test]
    fn tribe_ids_match_the_tracker_names_once_each() {
        let mut names: Vec<&str> = TRIBE_IDS.iter().map(|(_, n)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TRIBE_IDS.len());
    }

    fn game_state(f: &mut FakeMono, inside: &[i32], outside: &[i32]) -> u64 {
        let image = f.assembly(ASSEMBLY);
        let (class, data) = f.class(
            image,
            "",
            STATE_CLASS,
            &[
                FakeField {
                    name: STATE_INSTANCE,
                    offset: 0,
                    is_static: true,
                },
                FakeField {
                    name: TRIBES_IN,
                    offset: 0x10,
                    is_static: false,
                },
                FakeField {
                    name: TRIBES_OUT,
                    offset: 0x18,
                    is_static: false,
                },
            ],
        );
        let state = f.object(class, 0x20);
        f.mem.put_u64(data, state);
        let l_in = f.int_list(inside);
        let l_out = f.int_list(outside);
        f.mem.put_u64(state + 0x10, l_in);
        f.mem.put_u64(state + 0x18, l_out);
        image
    }

    #[test]
    fn reads_the_lobby_tribes_from_a_synthetic_game() {
        let mut f = FakeMono::new();
        game_state(&mut f, &IN, &OUT);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert_eq!(
            read_lobby_tribes(&mono).unwrap(),
            ["BEAST", "DRAGON", "MURLOC", "PIRATE", "UNDEAD"]
        );
    }

    #[test]
    fn no_game_in_progress_is_not_yet() {
        let mut f = FakeMono::new();
        let image = f.assembly(ASSEMBLY);
        f.class(
            image,
            "",
            STATE_CLASS,
            &[FakeField {
                name: STATE_INSTANCE,
                offset: 0,
                is_static: true,
            }],
        );
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert_eq!(read_lobby_tribes(&mono), Err(ReadFailure::NotYet));
    }

    #[test]
    fn a_renamed_field_is_an_unsupported_version() {
        let mut f = FakeMono::new();
        let image = f.assembly(ASSEMBLY);
        let (class, data) = f.class(
            image,
            "",
            STATE_CLASS,
            &[
                FakeField {
                    name: STATE_INSTANCE,
                    offset: 0,
                    is_static: true,
                },
                FakeField {
                    name: "m_availableRaces",
                    offset: 0x10,
                    is_static: false,
                },
            ],
        );
        let state = f.object(class, 0x20);
        f.mem.put_u64(data, state);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(matches!(
            read_lobby_tribes(&mono),
            Err(ReadFailure::Unsupported(_))
        ));
    }

    #[test]
    fn a_bad_lobby_in_memory_is_refused() {
        let mut f = FakeMono::new();
        game_state(&mut f, &[20, 24, 14, 11, 20], &OUT);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(matches!(
            read_lobby_tribes(&mono),
            Err(ReadFailure::Unsupported(_))
        ));
    }

    /// The objects on the walk must be of the expected types: a value read
    /// through an object of another type is never returned.
    #[test]
    fn objects_of_another_type_are_refused() {
        let unsupported = |f: &FakeMono| {
            let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
            matches!(read_lobby_tribes(&mono), Err(ReadFailure::Unsupported(_)))
        };
        use crate::memory::Memory;
        // `s_instance` holding an object that is not a GameState.
        let mut f = FakeMono::new();
        let image = game_state(&mut f, &IN, &OUT);
        let state = static_state(&f, image);
        f.set_class_name(state, "GameState");
        assert!(unsupported(&f), "a GameState of another class");
        // A list that is not a List`1.
        let mut f = FakeMono::new();
        let image = game_state(&mut f, &IN, &OUT);
        let state = static_state(&f, image);
        let list = f.mem.read_u64(state + 0x10).unwrap();
        f.set_class_name(list, "Dictionary`2");
        assert!(unsupported(&f), "a list of another type");
        // Items that are not an array.
        let mut f = FakeMono::new();
        let image = game_state(&mut f, &IN, &OUT);
        let state = static_state(&f, image);
        let list = f.mem.read_u64(state + 0x18).unwrap();
        let items = f.mem.read_u64(list + 0x10).unwrap();
        f.set_class_name(items, "String");
        assert!(unsupported(&f), "items that are not an array");
    }

    /// The GameState object the fake's static field holds.
    fn static_state(f: &FakeMono, image: u64) -> u64 {
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        let class = mono.class(image, NAMESPACE, STATE_CLASS).unwrap();
        mono.static_object(class, STATE_INSTANCE).unwrap()
    }

    fn rating_object(f: &mut FakeMono, class_name: &str, solo: i32, duos: i32) -> u64 {
        let image = f.assembly(ASSEMBLY);
        let fields = [
            FakeField {
                name: RATING_SOLO,
                offset: 0x10,
                is_static: false,
            },
            FakeField {
                name: RATING_DUOS,
                offset: 0x14,
                is_static: false,
            },
        ];
        let (class, _) = f.class(image, "", RATING_CLASS, &fields);
        let (other, _) = f.class(image, "", "Other", &fields);
        let obj = f.object(
            if class_name == RATING_CLASS {
                class
            } else {
                other
            },
            0x10,
        );
        f.mem.put_u32(obj + 0x10, solo as u32);
        f.mem.put_u32(obj + 0x14, duos as u32);
        obj
    }

    fn field(name: &'static str, offset: u64, is_static: bool) -> FakeField<'static> {
        FakeField {
            name,
            offset,
            is_static,
        }
    }

    /// An array object of class `name` with `len` slots of `stride` bytes.
    fn array(f: &mut FakeMono, name: &str, len: u64, stride: u64) -> u64 {
        let a = f.alloc((0x20 + len * stride) as usize);
        f.set_class_name(a, name);
        f.mem.put_u64(a + 0x18, len);
        a
    }

    /// What the synthetic game's rating path is made of, to break one part.
    struct RatingGame {
        f: FakeMono,
        services: u64,
        cache_values: u64,
        info_entries: u64,
    }

    /// The path as found on build 253216: ServiceManager → ServiceLocator →
    /// Dictionary of ServiceInfo (another service first, then NetCache) →
    /// NetCache → Map → values (another value first, then the rating).
    fn rating_game(solo: i32, duos: i32) -> RatingGame {
        let mut f = FakeMono::new();
        let csharp = f.assembly(ASSEMBLY);
        let locator_image = f.assembly(LOCATOR_ASSEMBLY);
        let core = f.assembly("mscorlib");
        let (manager, statics) = f.class(
            locator_image,
            LOCATOR_NAMESPACE,
            MANAGER_CLASS,
            &[field(MANAGER_SERVICES, 0, true)],
        );
        let _ = manager;
        let (locator_class, _) = f.class(
            locator_image,
            LOCATOR_NAMESPACE,
            LOCATOR_CLASS,
            &[field(LOCATOR_MAP, 0x20, false)],
        );
        let (dict_class, _) = f.class(
            core,
            "System.Collections.Generic",
            DICTIONARY_CLASS,
            &[
                field(DICTIONARY_ENTRIES, 0x18, false),
                field(DICTIONARY_COUNT, 0x40, false),
            ],
        );
        let (info_class, _) = f.class(
            locator_image,
            "",
            SERVICE_INFO_CLASS,
            &[field(SERVICE_INFO_SERVICE, 0x20, false)],
        );
        let (cache_class, _) = f.class(csharp, "", CACHE_CLASS, &[field(CACHE_MAP, 0x10, false)]);
        let (other_service, _) = f.class(csharp, "", "OtherService", &[]);
        let (map_class, _) = f.class(
            core,
            "Blizzard.T5.Core",
            MAP_CLASS,
            &[field(MAP_VALUES, 0x28, false), field(MAP_USED, 0x38, false)],
        );
        let (rating_class, _) = f.class(
            csharp,
            "",
            RATING_CLASS,
            &[
                field(RATING_SOLO, 0x10, false),
                field(RATING_DUOS, 0x14, false),
            ],
        );
        let (other_value, _) = f.class(csharp, "", "NetCacheOther", &[]);

        let locator = f.object(locator_class, 0x50);
        f.mem.put_u64(statics, locator);
        let services = f.object(dict_class, 0x40);
        f.mem.put_u64(locator + 0x20, services);
        let entries = array(&mut f, "Entry[]", 4, ENTRY_SIZE);
        f.mem.put_u64(services + 0x18, entries);
        f.mem.put_u32(services + 0x40, 3);
        // Entry 0: another service; entry 1: freed (no value); entry 2: NetCache.
        let other = f.object(other_service, 0x10);
        let info0 = f.object(info_class, 0x40);
        f.mem.put_u64(info0 + 0x20, other);
        let cache = f.object(cache_class, 0x20);
        let info2 = f.object(info_class, 0x40);
        f.mem.put_u64(info2 + 0x20, cache);
        f.mem.put_u64(entries + 0x20 + ENTRY_VALUE, info0);
        f.mem
            .put_u64(entries + 0x20 + 2 * ENTRY_SIZE + ENTRY_VALUE, info2);

        let map = f.object(map_class, 0x40);
        f.mem.put_u64(cache + 0x10, map);
        let values = array(&mut f, "Object[]", 4, 8);
        f.mem.put_u64(map + 0x28, values);
        f.mem.put_u32(map + 0x38, 3);
        let other_obj = f.object(other_value, 0x10);
        let rating = f.object(rating_class, 0x10);
        f.mem.put_u32(rating + 0x10, solo as u32);
        f.mem.put_u32(rating + 0x14, duos as u32);
        f.mem.put_u64(values + 0x20, other_obj);
        f.mem.put_u64(values + 0x20 + 2 * 8, rating);
        RatingGame {
            f,
            services,
            cache_values: values,
            info_entries: entries,
        }
    }

    use crate::memory::Memory as _;

    fn own_rating_of(g: &RatingGame) -> Result<Rating, ReadFailure> {
        let mono = Mono::attach(&g.f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        read_own_rating(&mono)
    }

    #[test]
    fn reads_the_own_rating_along_the_service_locator_path() {
        let g = rating_game(6651, 7116);
        assert_eq!(
            own_rating_of(&g),
            Ok(Rating {
                solo: Some(6651),
                duos: Some(7116)
            })
        );
    }

    #[test]
    fn an_object_of_another_type_on_the_rating_path_is_refused() {
        let unsupported =
            |g: &RatingGame| matches!(own_rating_of(g), Err(ReadFailure::Unsupported(_)));
        let mut g = rating_game(6651, 7116);
        let services = g.services;
        g.f.set_class_name(services, "List`1");
        assert!(unsupported(&g), "services that are not a dictionary");
        let mut g = rating_game(6651, 7116);
        let first =
            g.f.mem
                .read_u64(g.info_entries + 0x20 + ENTRY_VALUE)
                .unwrap();
        g.f.set_class_name(first, "Something");
        assert!(unsupported(&g), "a service entry that is not a ServiceInfo");
        let mut g = rating_game(6651, 7116);
        let values = g.cache_values;
        g.f.set_class_name(values, "Int32");
        assert!(unsupported(&g), "cache values that are not an array");
        let mut g = rating_game(6651, 7116);
        let services = g.services;
        g.f.mem.put_u32(services + 0x40, 99);
        assert!(unsupported(&g), "a count longer than the entries");
        let mut g = rating_game(6651, 7116);
        let services = g.services;
        g.f.mem.put_u32(services + 0x40, u32::MAX);
        assert!(unsupported(&g), "a negative count");
    }

    #[test]
    fn arrays_of_another_type_and_pointers_out_of_range_are_refused() {
        let unsupported =
            |g: &RatingGame| matches!(own_rating_of(g), Err(ReadFailure::Unsupported(_)));
        let mut g = rating_game(6651, 7116);
        let entries = g.info_entries;
        g.f.set_class_name(entries, "Int64[]");
        assert!(unsupported(&g), "entries that are not Entry[]");
        let mut g = rating_game(6651, 7116);
        let values = g.cache_values;
        g.f.set_class_name(values, "Int64[]");
        assert!(unsupported(&g), "values that are not Object[]");
        // A value where the cache map pointer must be: not "not yet".
        let mut g = rating_game(6651, 7116);
        let entries = g.info_entries;
        let info =
            g.f.mem
                .read_u64(entries + 0x20 + 2 * ENTRY_SIZE + ENTRY_VALUE)
                .unwrap();
        let cache = g.f.mem.read_u64(info + 0x20).unwrap();
        g.f.mem.put_u64(cache + 0x10, 0xffff_ffff_ffff_fff0);
        assert!(unsupported(&g), "a pointer out of range");
    }

    #[test]
    fn a_rating_field_that_is_not_an_int_is_refused() {
        let mut f = FakeMono::new();
        let image = f.assembly(ASSEMBLY);
        let (class, _) = f.class(
            image,
            "",
            RATING_CLASS,
            &[
                field(RATING_SOLO, 0x10, false),
                field("m_other", 0x14, false),
            ],
        );
        let obj = f.object(class, 0x10);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(mono.int_field(obj, class, RATING_SOLO).is_ok());
        assert!(matches!(
            mono.int_field(obj, class, "m_other"),
            Err(WalkError::Unsupported(_))
        ));
    }

    #[test]
    fn a_rating_out_of_range_or_twice_in_the_cache_is_refused() {
        let g = rating_game(99_999, 7116);
        assert!(matches!(
            own_rating_of(&g),
            Err(ReadFailure::Unsupported(_))
        ));
        let mut g = rating_game(6651, 7116);
        let values = g.cache_values;
        let rating = g.f.mem.read_u64(values + 0x20 + 2 * 8).unwrap();
        g.f.mem.put_u64(values + 0x20 + 8, rating);
        assert_eq!(
            own_rating_of(&g),
            Err(ReadFailure::Unsupported("two rating objects".into()))
        );
    }

    #[test]
    fn no_rating_in_the_cache_yet_is_not_yet() {
        let mut g = rating_game(6651, 7116);
        let values = g.cache_values;
        g.f.mem.put_u64(values + 0x20 + 2 * 8, 0);
        assert_eq!(own_rating_of(&g), Err(ReadFailure::NotYet));
        let mut g = rating_game(6651, 7116);
        let entries = g.info_entries;
        g.f.mem
            .put_u64(entries + 0x20 + 2 * ENTRY_SIZE + ENTRY_VALUE, 0);
        assert_eq!(
            own_rating_of(&g),
            Err(ReadFailure::NotYet),
            "no NetCache service yet"
        );
    }

    #[test]
    fn reads_the_own_rating_from_its_object() {
        let mut f = FakeMono::new();
        let obj = rating_object(&mut f, RATING_CLASS, 6543, 0);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert_eq!(
            read_rating_info(&mono, obj).unwrap(),
            Rating {
                solo: Some(6543),
                duos: None
            }
        );
    }

    #[test]
    fn a_rating_from_another_type_or_out_of_range_is_refused() {
        let mut f = FakeMono::new();
        let obj = rating_object(&mut f, "Other", 6543, 0);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(matches!(
            read_rating_info(&mono, obj),
            Err(ReadFailure::Unsupported(_))
        ));
        let mut f = FakeMono::new();
        let obj = rating_object(&mut f, RATING_CLASS, 99_999, 0);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(matches!(
            read_rating_info(&mono, obj),
            Err(ReadFailure::Unsupported(_))
        ));
    }
}
