//! Where the game writes its logs: `<install>\Logs\Hearthstone_<date>\`.
//!
//! Every place the game might be is gathered as a candidate (the running
//! game's own folder, both Uninstall keys, Battle.net's install data, the
//! usual folders and, when nothing else fits, the common folders on every
//! drive). A candidate without `Hearthstone.exe` or without a `Logs` folder is
//! dropped; of the rest the one whose `Logs` has the newest session wins, so
//! an old install with a leftover `Logs` folder never beats the one being
//! played. A folder the player chose always wins over all of this (T-579).
//! Read-only: `reg query` and plain file reads, nothing is written.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const GAME_EXE: &str = "Hearthstone.exe";
const LOGS_FOLDER: &str = "Logs";

const UNINSTALL_KEYS: [&str; 2] = [
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Hearthstone",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Hearthstone",
];
const FALLBACK_INSTALL_DIRS: [&str; 2] = [
    r"C:\Program Files (x86)\Hearthstone",
    r"C:\Program Files\Hearthstone",
];
/// Where an install usually sits under a drive's root, for the drive scan.
const DRIVE_SUBFOLDERS: [&str; 5] = [
    "Hearthstone",
    r"Games\Hearthstone",
    r"Battle.net\Hearthstone",
    r"Program Files (x86)\Hearthstone",
    r"Program Files\Hearthstone",
];
/// Battle.net's install data; only read, and only the first few MiB of it.
const BATTLE_NET_DB: &str = r"Battle.net\Agent\product.db";
const BATTLE_NET_DB_LIMIT: u64 = 8 << 20;
/// Most install paths taken from Battle.net's data: a real one has a few.
const MAX_BATTLE_NET_PATHS: usize = 16;

/// How a candidate folder was found, best first: when two candidates have the
/// same newest session (or none) the earlier kind wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    /// The folder of the running `Hearthstone.exe`.
    RunningGame,
    /// An Uninstall key's `InstallLocation`.
    Registry,
    /// An install path in Battle.net's own data.
    BattleNet,
    /// `C:\Program Files*\Hearthstone`.
    UsualFolder,
    /// A common folder name at the root of some drive.
    DriveScan,
}

impl Source {
    /// The word the window shows (it is translated there).
    pub fn id(self) -> &'static str {
        match self {
            Source::RunningGame => "running",
            Source::Registry => "registry",
            Source::BattleNet => "battlenet",
            Source::UsualFolder => "usual",
            Source::DriveScan => "scan",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub dir: PathBuf,
    pub source: Source,
}

/// How the folder in use was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// The player chose it in the settings.
    Chosen,
    Automatic(Source),
}

impl How {
    /// `chosen`, or the source's word.
    pub fn id(self) -> &'static str {
        match self {
            How::Chosen => "chosen",
            How::Automatic(source) => source.id(),
        }
    }
}

/// The game's install folder (where `Hearthstone.exe` and `client.config`
/// live) and how it was decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameFolder {
    pub dir: PathBuf,
    pub how: How,
}

impl GameFolder {
    pub fn logs_dir(&self) -> PathBuf {
        self.dir.join(LOGS_FOLDER)
    }
}

/// Why a folder cannot be the game's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderProblem {
    /// The path is not there (an unplugged drive, a deleted folder).
    Missing,
    /// The path is a file, and not the game's.
    NotAFolder,
    /// A folder with neither `Hearthstone.exe` nor game sessions in `Logs`.
    NotTheGame,
}

impl FolderProblem {
    /// In plain English; the window has the Spanish for each.
    pub fn message(self) -> &'static str {
        match self {
            FolderProblem::Missing => "That folder was not found.",
            FolderProblem::NotAFolder => "That is a file, not a folder.",
            FolderProblem::NotTheGame => {
                "Hearthstone.exe is not in that folder, and it has no Logs folder with game sessions."
            }
        }
    }
}

/// What the choice of folder comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Found(GameFolder),
    /// The player chose a folder that cannot be used now (the drive is gone,
    /// the game was moved). Never replaced by a guess: the window says so.
    ChosenUnusable {
        dir: PathBuf,
        problem: FolderProblem,
    },
    NotFound,
}

/// The game's Logs folder, found automatically. Read-only: runs `reg query`,
/// writes nothing.
pub fn find_logs_dir() -> Option<PathBuf> {
    match resolve(None, [gather(), scan_drives()].concat()) {
        Resolution::Found(folder) => Some(folder.logs_dir()),
        _ => None,
    }
}

/// The folder in use: the player's choice when there is one, else the best
/// of the candidates.
pub fn resolve(chosen: Option<&Path>, candidates: Vec<Candidate>) -> Resolution {
    match chosen {
        Some(path) => match validate_chosen(path) {
            Ok(dir) => Resolution::Found(GameFolder {
                dir,
                how: How::Chosen,
            }),
            Err(problem) => Resolution::ChosenUnusable {
                dir: path.to_path_buf(),
                problem,
            },
        },
        None => pick_best(candidates).map_or(Resolution::NotFound, Resolution::Found),
    }
}

/// The game folder a player's pick stands for. Whatever was picked is
/// accepted if it leads to the game: the install folder itself (it has
/// `Hearthstone.exe`, or a `Logs` folder with sessions), `Hearthstone.exe`,
/// the `Logs` folder, one session folder, or a folder that holds a
/// `Hearthstone` install.
pub fn validate_chosen(path: &Path) -> Result<PathBuf, FolderProblem> {
    // A relative path would mean the app's own folder; a network path can
    // stall the app for a long time and sign in to that host.
    if !path.is_absolute() || is_network_path(path) {
        return Err(FolderProblem::Missing);
    }
    let meta = std::fs::metadata(path).map_err(|_| FolderProblem::Missing)?;
    if meta.is_file() {
        let is_exe = path
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(GAME_EXE));
        return match path.parent() {
            Some(dir) if is_exe => Ok(dir.to_path_buf()),
            _ => Err(FolderProblem::NotAFolder),
        };
    }
    if is_game_folder(path) || has_sessions(&path.join(LOGS_FOLDER)) {
        return Ok(path.to_path_buf());
    }
    let parent = path.parent();
    let name_is = |dir: &Path, wanted: &str| {
        dir.file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case(wanted))
    };
    if let Some(parent) = parent {
        // The Logs folder itself.
        if name_is(path, LOGS_FOLDER) && (has_sessions(path) || is_game_folder(parent)) {
            return Ok(parent.to_path_buf());
        }
        // One session folder inside it.
        let session = path
            .file_name()
            .is_some_and(|n| is_session_name(&n.to_string_lossy()));
        if let (true, true, Some(install)) =
            (session, name_is(parent, LOGS_FOLDER), parent.parent())
        {
            return Ok(install.to_path_buf());
        }
    }
    let inner = path.join("Hearthstone");
    if is_game_folder(&inner) {
        return Ok(inner);
    }
    Err(FolderProblem::NotTheGame)
}

/// `\\host\share` and `\\?\UNC\...` paths (but not `\\?\C:\...`).
fn is_network_path(path: &Path) -> bool {
    let text = path.as_os_str().to_string_lossy().to_ascii_uppercase();
    text.starts_with(r"\\?\UNC\") || (text.starts_with(r"\\") && !text.starts_with(r"\\?\"))
}

fn is_game_folder(dir: &Path) -> bool {
    dir.join(GAME_EXE).is_file()
}

fn has_sessions(logs: &Path) -> bool {
    latest_session(logs).is_some()
}

/// Of the candidates, the install that is being played: it has
/// `Hearthstone.exe` and a `Logs` folder, and its newest session is the
/// latest; then the better source decides. The same folder found twice counts
/// once, under its best source.
pub fn pick_best(candidates: Vec<Candidate>) -> Option<GameFolder> {
    let mut seen = HashSet::new();
    let mut usable: Vec<(Option<String>, Source, PathBuf)> = Vec::new();
    let mut ordered = candidates;
    ordered.sort_by_key(|c| c.source);
    for candidate in ordered {
        if is_network_path(&candidate.dir) || !seen.insert(same_folder_key(&candidate.dir)) {
            continue;
        }
        let logs = candidate.dir.join(LOGS_FOLDER);
        if is_game_folder(&candidate.dir) && logs.is_dir() {
            let newest = latest_session(&logs)
                .and_then(|s| s.file_name().map(|n| n.to_string_lossy().into_owned()));
            usable.push((newest, candidate.source, candidate.dir));
        }
    }
    // Newest session first (session names sort by start time; a folder with
    // none comes last), then the better source; the sort is stable.
    usable.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    usable
        .into_iter()
        .next()
        .map(|(_, source, dir)| GameFolder {
            dir,
            how: How::Automatic(source),
        })
}

/// Two spellings of one folder (case, a trailing slash) are the same.
pub fn same_folder_key(dir: &Path) -> String {
    // By text, so both slashes and a trailing one count the same on any system.
    dir.to_string_lossy()
        .to_lowercase()
        .split(['\\', '/'])
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("\\")
}

/// Every place the game might be, but the running game (the app adds that
/// one from the process list, see [`folder_of_exe`]) and the drive scan
/// ([`scan_drives`], which touches every drive and is done once).
pub fn gather() -> Vec<Candidate> {
    let mut out = Vec::new();
    out.extend(
        UNINSTALL_KEYS
            .iter()
            .filter_map(|key| install_location(key))
            .map(|dir| Candidate {
                dir,
                source: Source::Registry,
            }),
    );
    out.extend(battle_net_candidates());
    out.extend(FALLBACK_INSTALL_DIRS.iter().map(|dir| Candidate {
        dir: PathBuf::from(dir),
        source: Source::UsualFolder,
    }));
    // Only installs with the game in them are kept, so looking again does
    // not touch every guess each time.
    out.retain(|c| !is_network_path(&c.dir) && is_game_folder(&c.dir));
    out
}

/// The common folder names on every drive, `C:` to `Z:`.
pub fn scan_drives() -> Vec<Candidate> {
    scan_roots(&drive_roots())
}

/// The folder of a running game's `Hearthstone.exe`, from its full path.
pub fn folder_of_exe(exe: &Path) -> Option<PathBuf> {
    let named_right = exe
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case(GAME_EXE));
    if named_right {
        exe.parent().map(Path::to_path_buf)
    } else {
        None
    }
}

/// The common folder names under each root (a drive's root).
pub fn scan_roots(roots: &[PathBuf]) -> Vec<Candidate> {
    roots
        .iter()
        .flat_map(|root| {
            DRIVE_SUBFOLDERS
                .iter()
                // `Games\Hearthstone` is two folders, whatever the system.
                .map(|sub| {
                    sub.split('\\')
                        .fold(root.clone(), |dir, part| dir.join(part))
                })
        })
        .filter(|dir| is_game_folder(dir))
        .map(|dir| Candidate {
            dir,
            source: Source::DriveScan,
        })
        .collect()
}

/// `C:\` to `Z:\` that are there.
fn drive_roots() -> Vec<PathBuf> {
    ('C'..='Z')
        .map(|letter| PathBuf::from(format!("{letter}:\\")))
        .filter(|root| root.is_dir())
        .collect()
}

fn battle_net_candidates() -> Vec<Candidate> {
    let Some(data) = std::env::var_os("ProgramData") else {
        return Vec::new();
    };
    let Ok(file) = std::fs::File::open(PathBuf::from(data).join(BATTLE_NET_DB)) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    if file
        .take(BATTLE_NET_DB_LIMIT)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Vec::new();
    }
    game_paths_in(&bytes)
        .into_iter()
        .map(|dir| Candidate {
            dir,
            source: Source::BattleNet,
        })
        .collect()
}

/// Install paths for Hearthstone in Battle.net's binary data: any text that
/// starts like `D:\` or `D:/` and has a `Hearthstone` folder in it, cut right
/// after that folder. A wrong guess is dropped later (no `Hearthstone.exe`).
pub fn game_paths_in(bytes: &[u8]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut seen = HashSet::new();
    for run in bytes.split(|b| *b < 0x20 || *b == 0x7f) {
        if out.len() >= MAX_BATTLE_NET_PATHS {
            break;
        }
        let text = String::from_utf8_lossy(run);
        let chars: Vec<char> = text.chars().collect();
        let start = (0..chars.len().saturating_sub(2)).find(|&at| {
            chars[at].is_ascii_alphabetic()
                && chars[at + 1] == ':'
                && (chars[at + 2] == '\\' || chars[at + 2] == '/')
        });
        let Some(start) = start else { continue };
        let path: String = chars[start..].iter().collect();
        if let Some(cut) = cut_after_hearthstone(&path) {
            let dir = PathBuf::from(cut.replace('/', "\\"));
            if seen.insert(same_folder_key(&dir)) {
                out.push(dir);
            }
        }
    }
    out
}

fn cut_after_hearthstone(path: &str) -> Option<&str> {
    let after_separator = |at: usize| at > 0 && matches!(path.as_bytes()[at - 1], b'\\' | b'/');
    path.to_ascii_lowercase()
        .match_indices("hearthstone")
        .map(|(at, _)| at)
        .find(|&at| after_separator(at))
        .map(|at| &path[..at + "hearthstone".len()])
}

fn install_location(key: &str) -> Option<PathBuf> {
    // Full path: never a reg.exe found next to the app or on PATH.
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let reg = PathBuf::from(system_root).join("System32").join("reg.exe");
    let output = Command::new(reg)
        .args(["query", key, "/v", "InstallLocation"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_reg_value(&String::from_utf8_lossy(&output.stdout), "InstallLocation")
}

/// Parses `    InstallLocation    REG_SZ    C:\Path` from `reg query` output.
pub fn parse_reg_value(output: &str, name: &str) -> Option<PathBuf> {
    output.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(name)?;
        let (_, value) = rest.trim_start().split_once("REG_SZ")?;
        let value = value.trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

/// True for folder names like `Hearthstone_2026_10_09_15_26_25`.
pub fn is_session_name(name: &str) -> bool {
    let Some(stamp) = name.strip_prefix("Hearthstone_") else {
        return false;
    };
    let parts: Vec<&str> = stamp.split('_').collect();
    let widths = [4, 2, 2, 2, 2, 2];
    parts.len() == widths.len()
        && parts
            .iter()
            .zip(widths)
            .all(|(p, w)| p.len() == w && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The newest session folder (names sort by start time).
pub fn latest_session(logs_dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(logs_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter(|e| is_session_name(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tavern-ledger-discover-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A fake install under `root/<name>`: the exe (unless `exe` is false)
    /// and a `Logs` folder with these sessions.
    fn install(root: &Path, name: &str, exe: bool, sessions: &[&str]) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(dir.join(LOGS_FOLDER)).unwrap();
        if exe {
            fs::write(dir.join(GAME_EXE), b"").unwrap();
        }
        for session in sessions {
            fs::create_dir_all(dir.join(LOGS_FOLDER).join(session)).unwrap();
        }
        dir
    }

    fn candidate(dir: &Path, source: Source) -> Candidate {
        Candidate {
            dir: dir.to_path_buf(),
            source,
        }
    }

    const OLD: &str = "Hearthstone_2026_10_01_10_00_00";
    const NEW: &str = "Hearthstone_2026_10_12_00_05_51";

    #[test]
    fn reads_install_location_from_reg_output() {
        let out = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\X\r\n    InstallLocation    REG_SZ    C:\\Battle.net\\Hearthstone\r\n\r\n";
        assert_eq!(
            parse_reg_value(out, "InstallLocation"),
            Some(PathBuf::from(r"C:\Battle.net\Hearthstone"))
        );
        assert_eq!(parse_reg_value("nothing here", "InstallLocation"), None);
    }

    #[test]
    fn recognizes_session_folders() {
        assert!(is_session_name("Hearthstone_2026_10_09_15_26_25"));
        assert!(!is_session_name("Hearthstone_2026_10_09"));
        assert!(!is_session_name("Hearthstone_2026_10_09_15_26_2x"));
        assert!(!is_session_name("Other_2026_10_09_15_26_25"));
    }

    #[test]
    fn the_install_with_the_newest_session_wins_over_a_better_source() {
        // The tester's case: a stale install named by the registry, the
        // played one found only in the usual folders.
        let root = temp_dir();
        let stale = install(&root, "stale", true, &[OLD]);
        let played = install(&root, "played", true, &[NEW]);
        let best = pick_best(vec![
            candidate(&stale, Source::Registry),
            candidate(&played, Source::UsualFolder),
        ])
        .unwrap();
        assert_eq!(best.dir, played);
        assert_eq!(best.how, How::Automatic(Source::UsualFolder));
    }

    #[test]
    fn a_folder_without_the_game_exe_is_dropped_even_with_newer_logs() {
        let root = temp_dir();
        let leftover = install(&root, "leftover", false, &[NEW]);
        let real = install(&root, "real", true, &[OLD]);
        let best = pick_best(vec![
            candidate(&leftover, Source::Registry),
            candidate(&real, Source::UsualFolder),
        ])
        .unwrap();
        assert_eq!(best.dir, real);
        assert_eq!(
            pick_best(vec![candidate(&leftover, Source::Registry)]),
            None
        );
    }

    #[test]
    fn a_folder_without_a_logs_folder_is_dropped() {
        let root = temp_dir();
        let dir = root.join("noLogs");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(GAME_EXE), b"").unwrap();
        assert_eq!(pick_best(vec![candidate(&dir, Source::Registry)]), None);
    }

    #[test]
    fn equal_sessions_go_to_the_better_source_and_a_folder_counts_once() {
        let root = temp_dir();
        let a = install(&root, "a", true, &[NEW]);
        let b = install(&root, "b", true, &[NEW]);
        let best = pick_best(vec![
            candidate(&b, Source::UsualFolder),
            candidate(&a, Source::BattleNet),
            candidate(&a, Source::RunningGame),
        ])
        .unwrap();
        assert_eq!(best.dir, a);
        assert_eq!(best.how, How::Automatic(Source::RunningGame));
    }

    #[test]
    fn an_install_that_has_logs_but_no_session_yet_loses_to_one_with_a_session() {
        let root = temp_dir();
        let empty = install(&root, "empty", true, &[]);
        let played = install(&root, "played", true, &[OLD]);
        let best = pick_best(vec![
            candidate(&empty, Source::RunningGame),
            candidate(&played, Source::DriveScan),
        ])
        .unwrap();
        assert_eq!(best.dir, played);
        // Alone, the empty one is still the answer.
        assert_eq!(
            pick_best(vec![candidate(&empty, Source::RunningGame)])
                .unwrap()
                .dir,
            empty
        );
    }

    #[test]
    fn spellings_of_one_folder_are_the_same_folder() {
        assert_eq!(
            same_folder_key(Path::new(r"D:\Hearthstone")),
            same_folder_key(Path::new(r"d:\hearthstone\"))
        );
        assert_ne!(
            same_folder_key(Path::new(r"D:\Hearthstone")),
            same_folder_key(Path::new(r"E:\Hearthstone"))
        );
    }

    #[test]
    fn no_candidates_is_not_found() {
        assert_eq!(resolve(None, Vec::new()), Resolution::NotFound);
    }

    #[test]
    fn a_chosen_folder_wins_over_every_candidate() {
        let root = temp_dir();
        let played = install(&root, "played", true, &[NEW]);
        let mine = install(&root, "mine", true, &[OLD]);
        let found = resolve(Some(&mine), vec![candidate(&played, Source::RunningGame)]);
        assert_eq!(
            found,
            Resolution::Found(GameFolder {
                dir: mine,
                how: How::Chosen
            })
        );
    }

    #[test]
    fn a_chosen_folder_that_is_gone_is_said_not_replaced_by_a_guess() {
        let root = temp_dir();
        let played = install(&root, "played", true, &[NEW]);
        let gone = root.join("unplugged");
        let found = resolve(Some(&gone), vec![candidate(&played, Source::RunningGame)]);
        assert_eq!(
            found,
            Resolution::ChosenUnusable {
                dir: gone,
                problem: FolderProblem::Missing
            }
        );
    }

    #[test]
    fn the_pick_may_be_the_folder_the_exe_the_logs_or_a_session() {
        let root = temp_dir();
        let game = install(&root, "Hearthstone", true, &[NEW]);
        let logs = game.join(LOGS_FOLDER);
        assert_eq!(validate_chosen(&game), Ok(game.clone()));
        assert_eq!(validate_chosen(&game.join(GAME_EXE)), Ok(game.clone()));
        assert_eq!(validate_chosen(&logs), Ok(game.clone()));
        assert_eq!(validate_chosen(&logs.join(NEW)), Ok(game.clone()));
        // The folder that holds the install.
        assert_eq!(validate_chosen(&root), Ok(game));
    }

    #[test]
    fn a_game_with_sessions_but_no_exe_is_accepted_by_its_logs() {
        let root = temp_dir();
        let moved = install(&root, "moved", false, &[NEW]);
        assert_eq!(validate_chosen(&moved), Ok(moved.clone()));
        assert_eq!(validate_chosen(&moved.join(LOGS_FOLDER)), Ok(moved));
    }

    #[test]
    fn a_pick_that_is_not_the_game_says_why() {
        let root = temp_dir();
        let nothing = root.join("Documents");
        fs::create_dir_all(&nothing).unwrap();
        let note = nothing.join("note.txt");
        fs::write(&note, b"x").unwrap();
        let empty_logs = install(&root, "tl", false, &[]);
        assert_eq!(validate_chosen(&nothing), Err(FolderProblem::NotTheGame));
        assert_eq!(validate_chosen(&note), Err(FolderProblem::NotAFolder));
        assert_eq!(
            validate_chosen(&root.join("nope")),
            Err(FolderProblem::Missing)
        );
        assert_eq!(validate_chosen(&empty_logs), Err(FolderProblem::NotTheGame));
        assert_eq!(
            validate_chosen(&empty_logs.join(LOGS_FOLDER)),
            Err(FolderProblem::NotTheGame)
        );
        for problem in [
            FolderProblem::Missing,
            FolderProblem::NotAFolder,
            FolderProblem::NotTheGame,
        ] {
            assert!(!problem.message().is_empty());
        }
    }

    #[test]
    fn validating_never_creates_or_changes_anything() {
        let root = temp_dir();
        let dir = root.join("Hearthstone");
        fs::create_dir_all(&dir).unwrap();
        let _ = validate_chosen(&dir);
        let _ = pick_best(vec![candidate(&dir, Source::Registry)]);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    }

    #[test]
    fn a_running_game_gives_its_exe_folder() {
        let game = std::env::temp_dir().join("Games").join("Hearthstone");
        assert_eq!(
            folder_of_exe(&game.join("Hearthstone.exe")),
            Some(game.clone())
        );
        assert_eq!(
            folder_of_exe(&game.join("hearthstone.EXE")),
            Some(game.clone())
        );
        assert_eq!(folder_of_exe(&game.with_file_name("Other.exe")), None);
    }

    #[test]
    fn the_drive_scan_finds_the_common_folders_with_the_exe() {
        let drive = temp_dir();
        let other = temp_dir();
        let nested = drive.join("Games").join("Hearthstone");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join(GAME_EXE), b"").unwrap();
        // A folder with the right name but no game in it is not a candidate.
        fs::create_dir_all(other.join("Hearthstone")).unwrap();
        let found = scan_roots(&[drive, other]);
        assert_eq!(found, vec![candidate(&nested, Source::DriveScan)]);
    }

    #[test]
    fn network_and_relative_picks_are_refused_without_touching_them() {
        for path in [
            r"\\server\share\Hearthstone",
            r"\\?\UNC\server\share",
            "Hearthstone",
            r"..\x",
        ] {
            assert_eq!(
                validate_chosen(Path::new(path)),
                Err(FolderProblem::Missing),
                "{path}"
            );
        }
        let network = Path::new(r"\\server\share\Hearthstone");
        assert_eq!(pick_best(vec![candidate(network, Source::Registry)]), None);
        assert!(!is_network_path(Path::new(r"\\?\C:\Games\Hearthstone")));
    }

    #[test]
    fn a_crafted_battle_net_file_gives_few_distinct_paths() {
        let mut data = Vec::new();
        for n in 0..5000 {
            data.extend_from_slice(format!("\x1aD:\\{n}\\Hearthstone\x00").as_bytes());
        }
        assert_eq!(game_paths_in(&data).len(), MAX_BATTLE_NET_PATHS);
        let same = b"\x1aD:\\Games\\Hearthstone\x00\x1ad:/games/hearthstone\x00".repeat(100);
        assert_eq!(game_paths_in(&same).len(), 1);
    }

    #[test]
    fn a_logs_pick_is_accepted_when_the_game_is_next_to_it_even_with_no_session() {
        let root = temp_dir();
        let game = install(&root, "Hearthstone", true, &[]);
        assert_eq!(validate_chosen(&game.join(LOGS_FOLDER)), Ok(game));
    }

    #[test]
    fn the_cut_skips_a_hearthstone_that_is_not_a_whole_folder_start() {
        assert_eq!(
            cut_after_hearthstone(r"D:\NotHearthstone\Games\Hearthstone\x"),
            Some(r"D:\NotHearthstone\Games\Hearthstone")
        );
        assert_eq!(cut_after_hearthstone(r"D:\Overwatch"), None);
    }

    #[test]
    fn install_paths_are_read_from_battle_net_data() {
        let mut data = b"\x0a\x0bhearthstone\x12".to_vec();
        data.extend_from_slice(b"\x1aD:\\Games\\Hearthstone\x22\x05extra");
        data.extend_from_slice(b"\x00\x01e:/Blizzard/Hearthstone\x00");
        data.extend_from_slice(b"\x1bC:\\Other\\Overwatch\x00");
        data.extend_from_slice(b"\x1bF:\\NotHearthstone\\x\x00");
        assert_eq!(
            game_paths_in(&data),
            vec![
                PathBuf::from(r"D:\Games\Hearthstone"),
                PathBuf::from(r"e:\Blizzard\Hearthstone"),
            ]
        );
        assert!(game_paths_in(b"no paths in here").is_empty());
    }
}
