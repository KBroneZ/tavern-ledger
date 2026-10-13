//! Which Hearthstone folder the app follows (T-579): found automatically
//! (the running game, the Windows install records, Battle.net, the usual
//! folders) or chosen by the player in the settings. This module decides what
//! the windows are told about it and runs the player's choices. It only
//! reads: the game's own files are never written (D-004, D-006).
//!
//! The windows never send a path: "Choose…" opens the system's folder
//! picker here, and "Use this folder" takes the folder of the running game
//! the app itself saw, so a page cannot point the app at an arbitrary place.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, State, WebviewWindow};
use tracker::discover::{
    folder_of_exe, gather, resolve, same_folder_key, scan_drives, validate_chosen, Candidate,
    GameFolder, How, Resolution, Source,
};
use tracker::setup::folder_messages;

use crate::applog::Logged;
use crate::overlay::lock;
use crate::settings::{Language, Settings};
use crate::{set_status, AppState};

/// How often the running game's folder is read again.
const RUNNING_EVERY: Duration = Duration::from_secs(5);
/// How often the places the game may be are gathered again (an install
/// made while the app runs).
const REGATHER_EVERY: Duration = Duration::from_secs(60);

/// What the windows know about the folder in use.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct FolderStatus {
    /// The game folder in use, or the folder the player chose when it cannot
    /// be used; `None` when none was found.
    pub path: Option<String>,
    /// `chosen` or how it was found (`running`, `registry`, `battlenet`,
    /// `usual`, `scan`); `None` when none was found.
    pub how: Option<&'static str>,
    /// The player chose a folder (it may be unusable now).
    pub chosen: bool,
}

/// The outcome of looking again.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub folder: FolderStatus,
    /// For the setup banner when no folder can be used.
    pub lines: Vec<String>,
    /// The running game's folder when it is not the one in use and the
    /// player has not said no to it.
    pub offer: Option<PathBuf>,
    pub in_use: Option<GameFolder>,
}

/// What the status says about the choice of folder. `running` is the folder
/// of the running `Hearthstone.exe`, if any.
pub fn decide(resolution: &Resolution, running: Option<&Path>, declined: &[PathBuf]) -> Decision {
    let (folder, in_use) = match resolution {
        Resolution::Found(found) => (
            FolderStatus {
                path: Some(found.dir.display().to_string()),
                how: Some(found.how.id()),
                chosen: found.how == How::Chosen,
            },
            Some(found.clone()),
        ),
        Resolution::ChosenUnusable { dir, .. } => (
            FolderStatus {
                path: Some(dir.display().to_string()),
                how: Some(How::Chosen.id()),
                chosen: true,
            },
            None,
        ),
        Resolution::NotFound => (FolderStatus::default(), None),
    };
    let differs = |dir: &Path| {
        in_use
            .as_ref()
            .is_none_or(|used| same_folder_key(&used.dir) != same_folder_key(dir))
    };
    let said_no = |dir: &Path| {
        declined
            .iter()
            .any(|no| same_folder_key(no) == same_folder_key(dir))
    };
    let offer = running
        .filter(|dir| validate_chosen(dir).is_ok() && differs(dir) && !said_no(dir))
        .map(Path::to_path_buf);
    Decision {
        folder,
        lines: folder_messages(resolution),
        offer,
        in_use,
    }
}

/// Shared with the windows' commands: the tracker thread looks again when
/// the generation moves, and the commands read the folder it offered.
#[derive(Default)]
pub struct FolderState {
    generation: AtomicU64,
    offer: Mutex<Option<PathBuf>>,
    /// A picker is open: a second one is not opened over it.
    picking: AtomicBool,
}

impl FolderState {
    fn changed(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

/// The tracker thread's side. The places the game may be are gathered once
/// (with a scan of the drives) and again every [`REGATHER_EVERY`], on a
/// worker thread so a dead mapped drive or a slow disk never holds up the
/// tracker or the window (the answer is used when it arrives); the
/// running game is read every [`RUNNING_EVERY`] (a process list, no file
/// access). The folders are ranked again only when something moved: the
/// player changed the choice, the running game's folder changed, or the
/// places were gathered again. So a sleeping or offline disk is not touched
/// every few seconds.
pub struct Finder {
    candidates: Vec<Candidate>,
    drives: Vec<Candidate>,
    /// The worker gathering the places: the places and, the first time, the
    /// drive scan.
    gathering: Option<Receiver<Gathered>>,
    gathered_at: Option<Instant>,
    /// A gathering has been taken in (the first one always counts as news,
    /// even when it found nothing: the window still has to say so).
    has_gathered: bool,
    running_read_at: Option<Instant>,
    running: Option<PathBuf>,
    seen_generation: u64,
    /// Not looking at all: the folder was set on the command line.
    fixed: bool,
}

impl Finder {
    pub fn new(fixed: bool) -> Self {
        Self {
            candidates: Vec::new(),
            drives: Vec::new(),
            gathering: None,
            gathered_at: None,
            has_gathered: false,
            running_read_at: None,
            running: None,
            seen_generation: 0,
            fixed,
        }
    }

    /// Takes in what the worker found, if it has finished.
    fn collect_gathered(&mut self) -> bool {
        let Some(receiver) = &self.gathering else {
            return false;
        };
        match receiver.try_recv() {
            Ok(found) => {
                // The same places as before: nothing to rank again.
                let mut changed = !self.has_gathered || found.places != self.candidates;
                self.has_gathered = true;
                self.candidates = found.places;
                if let Some(drives) = found.drives {
                    changed |= drives != self.drives;
                    self.drives = drives;
                }
                self.gathering = None;
                changed
            }
            Err(TryRecvError::Empty) => false,
            Err(TryRecvError::Disconnected) => {
                self.gathering = None;
                false
            }
        }
    }

    /// Looks again when something moved, tells the windows and returns what
    /// it found; `None` when nothing moved.
    pub fn look(
        &mut self,
        app: &AppHandle,
        state: &AppState,
        settings: &Settings,
    ) -> Option<Decision> {
        if self.fixed {
            return None;
        }
        let generation = state.folder.generation();
        let mut moved = generation != self.seen_generation;
        self.seen_generation = generation;
        moved |= self.collect_gathered();
        let gather_due = self.gathering.is_none()
            && self
                .gathered_at
                .is_none_or(|at| at.elapsed() >= REGATHER_EVERY);
        if gather_due {
            self.gathering = Some(start_gathering(self.gathered_at.is_none()));
            self.gathered_at = Some(Instant::now());
        }
        let running_due = self
            .running_read_at
            .is_none_or(|at| at.elapsed() >= RUNNING_EVERY);
        if running_due {
            self.running_read_at = Some(Instant::now());
            let running = game_memory::location::game_exe_path()
                .as_deref()
                .and_then(folder_of_exe);
            moved |= running != self.running;
            self.running = running;
        }
        if !moved {
            return None;
        }
        let mut candidates = [self.candidates.clone(), self.drives.clone()].concat();
        candidates.extend(self.running.iter().map(|dir| Candidate {
            dir: dir.clone(),
            source: Source::RunningGame,
        }));
        let resolution = resolve(settings.game_folder().as_deref(), candidates);
        let decision = decide(
            &resolution,
            self.running.as_deref(),
            &settings.declined_folders(),
        );
        *lock(&state.folder.offer) = decision.offer.clone();
        let offer = decision.offer.as_ref().map(|dir| dir.display().to_string());
        let folder = decision.folder.clone();
        set_status(app, state, |s| {
            s.folder = folder;
            s.folder_offer = offer;
        });
        Some(decision)
    }
}

/// What a gathering worker found.
struct Gathered {
    places: Vec<Candidate>,
    /// The drive scan, done the first time only.
    drives: Option<Vec<Candidate>>,
}

/// Gathers the places on another thread: file checks on a dead network
/// drive can wait a long time, and nothing here may wait for them.
fn start_gathering(with_drives: bool) -> Receiver<Gathered> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let found = Gathered {
            places: gather(),
            drives: with_drives.then(scan_drives),
        };
        // The finder may be gone by now; nobody needs the answer then.
        let _ = sender.send(found);
    });
    receiver
}

/// The picker's title.
fn picker_title(language: Language) -> &'static str {
    match language {
        Language::En => "Choose the Hearthstone folder",
        Language::Es => "Elige la carpeta de Hearthstone",
    }
}

/// Opens the system's folder picker, and keeps what the player picks when it
/// leads to the game. `Ok(false)` when the picker was closed without a pick;
/// an error says why the pick was not used.
#[tauri::command]
pub async fn game_folder_choose(
    window: WebviewWindow,
    settings: State<'_, Arc<Settings>>,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    choose_folder(window, settings, state)
        .await
        .logged("game_folder_choose")
}

async fn choose_folder(
    window: WebviewWindow,
    settings: State<'_, Arc<Settings>>,
    state: State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    if state.folder.picking.swap(true, Ordering::SeqCst) {
        return Ok(false);
    }
    let title = picker_title(settings.language());
    let picked = tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title(title)
            .set_parent(&window)
            .pick_folder()
    })
    .await;
    state.folder.picking.store(false, Ordering::SeqCst);
    let picked =
        picked.map_err(|_| "The folder picker stopped after an internal error.".to_string())?;
    let Some(picked) = picked else {
        return Ok(false);
    };
    let dir = validate_chosen(&picked).map_err(|problem| problem.message().to_string())?;
    keep(&settings, &state, Some(dir)).map(|()| true)
}

/// Back to finding the folder automatically.
#[tauri::command]
pub fn game_folder_reset(
    settings: State<'_, Arc<Settings>>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    keep(&settings, &state, None).logged("game_folder_reset")
}

/// Yes to the notice "Hearthstone is running from <folder>. Use this folder?".
#[tauri::command]
pub fn game_folder_use_running(
    settings: State<'_, Arc<Settings>>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let offered = lock(&state.folder.offer).clone();
    let dir = offered
        .ok_or_else(|| "Hearthstone is not running from another folder now.".to_string())
        .and_then(|dir| validate_chosen(&dir).map_err(|p| p.message().to_string()))
        .logged("game_folder_use_running")?;
    keep(&settings, &state, Some(dir)).logged("game_folder_use_running")
}

/// No to that notice: the folder is not offered again.
#[tauri::command]
pub fn game_folder_decline_running(
    settings: State<'_, Arc<Settings>>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let offered = lock(&state.folder.offer).clone();
    let result = match offered {
        Some(dir) => settings.decline_folder(dir),
        None => Ok(()),
    };
    state.folder.changed();
    result.logged("game_folder_decline_running")
}

/// Saves the choice and makes the tracker look again, also when saving
/// failed: the choice then holds until the app restarts, and the error says so.
fn keep(settings: &Settings, state: &AppState, folder: Option<PathBuf>) -> Result<(), String> {
    let saved = settings.set_game_folder(folder);
    state.folder.changed();
    saved
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicU32;
    use tracker::discover::FolderProblem;

    fn temp_dir() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tl-game-folder-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn game(root: &Path, name: &str) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(dir.join("Logs")).unwrap();
        fs::write(dir.join("Hearthstone.exe"), b"").unwrap();
        dir
    }

    fn found(dir: &Path, how: How) -> Resolution {
        Resolution::Found(GameFolder {
            dir: dir.to_path_buf(),
            how,
        })
    }

    #[test]
    fn a_folder_found_automatically_is_said_with_how() {
        let root = temp_dir();
        let dir = game(&root, "D");
        let d = decide(&found(&dir, How::Automatic(Source::Registry)), None, &[]);
        assert_eq!(d.folder.path, Some(dir.display().to_string()));
        assert_eq!(d.folder.how, Some("registry"));
        assert!(!d.folder.chosen);
        assert!(d.lines.is_empty() && d.offer.is_none());
    }

    #[test]
    fn a_chosen_folder_is_said_as_chosen() {
        let root = temp_dir();
        let dir = game(&root, "D");
        let d = decide(&found(&dir, How::Chosen), None, &[]);
        assert_eq!(d.folder.how, Some("chosen"));
        assert!(d.folder.chosen);
    }

    #[test]
    fn a_chosen_folder_that_is_gone_stays_shown_with_the_banner_text() {
        let gone = PathBuf::from(r"E:\Hearthstone");
        let d = decide(
            &Resolution::ChosenUnusable {
                dir: gone.clone(),
                problem: FolderProblem::Missing,
            },
            None,
            &[],
        );
        assert_eq!(d.folder.path, Some(gone.display().to_string()));
        assert!(d.folder.chosen);
        assert!(d.in_use.is_none());
        assert_eq!(d.lines.len(), 1);
        assert!(d.lines[0].contains(r"E:\Hearthstone"));
    }

    #[test]
    fn nothing_found_says_so_in_the_banner() {
        let d = decide(&Resolution::NotFound, None, &[]);
        assert_eq!(d.folder, FolderStatus::default());
        assert!(d.lines[0].contains("could not find Hearthstone"));
    }

    #[test]
    fn a_running_game_elsewhere_is_offered_until_the_player_says_no() {
        let root = temp_dir();
        let used = game(&root, "E");
        let running = game(&root, "D");
        let in_use = found(&used, How::Chosen);
        let d = decide(&in_use, Some(&running), &[]);
        assert_eq!(d.offer, Some(running.clone()));
        // The same folder is not offered; a "no" is remembered (any spelling).
        assert_eq!(decide(&in_use, Some(&used), &[]).offer, None);
        let no = PathBuf::from(running.display().to_string().to_uppercase());
        assert_eq!(decide(&in_use, Some(&running), &[no]).offer, None);
        // With nothing in use, the running game is offered too.
        assert_eq!(
            decide(&Resolution::NotFound, Some(&running), &[]).offer,
            Some(running)
        );
    }

    #[test]
    fn a_running_folder_that_is_not_a_game_folder_is_not_offered() {
        let root = temp_dir();
        let used = game(&root, "E");
        let odd = root.join("odd");
        fs::create_dir_all(&odd).unwrap();
        assert_eq!(
            decide(&found(&used, How::Chosen), Some(&odd), &[]).offer,
            None
        );
    }

    #[test]
    fn the_picker_has_a_title_in_each_language() {
        assert_ne!(picker_title(Language::En), picker_title(Language::Es));
    }

    #[test]
    fn a_choice_is_kept_and_makes_the_tracker_look_again() {
        let dir = temp_dir();
        let settings = Settings::default();
        settings.load(&dir).unwrap();
        let state = AppState::default();
        let before = state.folder.generation();
        keep(&settings, &state, Some(PathBuf::from(r"D:\Hearthstone"))).unwrap();
        assert_eq!(
            settings.game_folder(),
            Some(PathBuf::from(r"D:\Hearthstone"))
        );
        assert!(state.folder.generation() > before);
        keep(&settings, &state, None).unwrap();
        assert_eq!(settings.game_folder(), None);
    }
}
