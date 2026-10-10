//! Where the game client is, from the scene changes in `LoadingScreen.log`
//! (T-314): the main menu, the Battlegrounds lobby, a Battlegrounds game
//! loading, or a game. For the app's status and the overlay's start and end.
//!
//! The lines read (checked on the user's 6 sessions, 11 games; every game
//! has a `BACON` to `GAMEPLAY` change about 2 s before its `CREATE_GAME` in
//! `Power.log`, and a `GAMEPLAY` to `BACON` or `HUB` change when the player
//! leaves the game screen):
//!
//! ```text
//! LoadingScreen.OnScenePreUnload() - prevMode=BACON nextMode=GAMEPLAY m_phase=INVALID
//! LoadingScreen.OnSceneLoaded() - prevMode=BACON currMode=GAMEPLAY
//! ```
//!
//! Only the mode names are kept, never anything else from the file. The
//! file exists only when `log.config` has a `[LoadingScreen]` section;
//! without it the scene is unknown and the app works from `Power.log` alone.

use serde::Serialize;

const LOADED: &str = "LoadingScreen.OnSceneLoaded() - ";
const PRE_UNLOAD: &str = "LoadingScreen.OnScenePreUnload() - ";
const BATTLEGROUNDS: &str = "BACON";
const GAMEPLAY: &str = "GAMEPLAY";
const HUB: &str = "HUB";
/// A mode name is a short word in capitals; anything else is not kept.
const MAX_MODE_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene {
    /// The main menu (`HUB`).
    Menu,
    /// The Battlegrounds lobby (`BACON`), also while leaving a game for it.
    BattlegroundsLobby,
    /// The lobby is closing to load a game (`BACON` to `GAMEPLAY`).
    BattlegroundsLoading,
    /// A game is on screen (`GAMEPLAY`), of any mode.
    Game,
    /// Any other part of the client (login, collection, other modes…).
    Elsewhere,
}

impl Scene {
    /// The words the app's status shows.
    pub fn describe(self) -> &'static str {
        match self {
            Scene::Menu => "In the main menu",
            Scene::BattlegroundsLobby => "In the Battlegrounds lobby",
            Scene::BattlegroundsLoading => "Battlegrounds game loading",
            Scene::Game => "In a game",
            Scene::Elsewhere => "Elsewhere in the game",
        }
    }

    /// The player is on a screen that is not a game: a game the log still
    /// has open was left (cut off without an end).
    pub fn left_game(self) -> bool {
        matches!(
            self,
            Scene::Menu | Scene::BattlegroundsLobby | Scene::Elsewhere
        )
    }
}

/// Follows the scene changes of one session's `LoadingScreen.log`.
#[derive(Clone, Debug, Default)]
pub struct SceneTracker {
    now: Option<Scene>,
}

impl SceneTracker {
    pub fn feed(&mut self, line: &str) {
        if let Some(rest) = after(line, LOADED) {
            if let Some(curr) = mode(rest, "currMode=") {
                self.now = Some(scene_of(&curr));
            }
        } else if let Some(rest) = after(line, PRE_UNLOAD) {
            let (Some(prev), Some(next)) = (mode(rest, "prevMode="), mode(rest, "nextMode="))
            else {
                return;
            };
            if prev == BATTLEGROUNDS && next == GAMEPLAY {
                self.now = Some(Scene::BattlegroundsLoading);
            } else if prev == GAMEPLAY && next != GAMEPLAY {
                // The game screen is closing: the game is over for the player.
                self.now = Some(scene_of(&next));
            }
        }
    }

    /// The scene now; None before the file said anything.
    pub fn now(&self) -> Option<Scene> {
        self.now
    }
}

fn after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker).map(|at| &line[at + marker.len()..])
}

/// The value after `key`, when it looks like a mode name.
fn mode(rest: &str, key: &str) -> Option<String> {
    let value = rest
        .split_whitespace()
        .find_map(|word| word.strip_prefix(key))?;
    let ok = !value.is_empty()
        && value.len() <= MAX_MODE_LEN
        && value.chars().all(|c| c.is_ascii_uppercase() || c == '_');
    ok.then(|| value.to_string())
}

fn scene_of(mode: &str) -> Scene {
    match mode {
        HUB => Scene::Menu,
        BATTLEGROUNDS => Scene::BattlegroundsLobby,
        GAMEPLAY => Scene::Game,
        _ => Scene::Elsewhere,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(prev: &str, curr: &str) -> String {
        format!("D 19:15:48.8947131 LoadingScreen.OnSceneLoaded() - prevMode={prev} currMode={curr}")
    }

    fn pre_unload(prev: &str, next: &str) -> String {
        format!(
            "D 19:15:46.8339075 LoadingScreen.OnScenePreUnload() - prevMode={prev} nextMode={next} m_phase=INVALID"
        )
    }

    fn after_lines(lines: &[String]) -> Option<Scene> {
        let mut t = SceneTracker::default();
        lines.iter().for_each(|l| t.feed(l));
        t.now()
    }

    #[test]
    fn nothing_read_is_unknown() {
        assert_eq!(after_lines(&[]), None);
        let noise = "D 19:11:43.1 LoadingScreen.OnSceneLoaded() - m_assetLoadStartTimestamp=123".to_string();
        assert_eq!(after_lines(&[noise]), None);
    }

    #[test]
    fn a_battlegrounds_game_goes_lobby_loading_game_lobby() {
        let mut lines = vec![loaded("STARTUP", "LOGIN"), loaded("LOGIN", "HUB")];
        assert_eq!(after_lines(&lines), Some(Scene::Menu));
        lines.push(pre_unload("HUB", "BACON"));
        assert_eq!(after_lines(&lines), Some(Scene::Menu), "still the menu until it loads");
        lines.push(loaded("HUB", "BACON"));
        assert_eq!(after_lines(&lines), Some(Scene::BattlegroundsLobby));
        lines.push(pre_unload("BACON", "GAMEPLAY"));
        assert_eq!(after_lines(&lines), Some(Scene::BattlegroundsLoading));
        lines.push(loaded("BACON", "GAMEPLAY"));
        assert_eq!(after_lines(&lines), Some(Scene::Game));
        lines.push(pre_unload("GAMEPLAY", "BACON"));
        assert_eq!(after_lines(&lines), Some(Scene::BattlegroundsLobby), "left as soon as it closes");
        lines.push(loaded("GAMEPLAY", "BACON"));
        assert_eq!(after_lines(&lines), Some(Scene::BattlegroundsLobby));
    }

    #[test]
    fn other_modes_are_elsewhere_and_a_game_from_them_is_a_game() {
        let lines = vec![loaded("HUB", "DRAFT")];
        assert_eq!(after_lines(&lines), Some(Scene::Elsewhere));
        let lines = vec![loaded("HUB", "DRAFT"), pre_unload("DRAFT", "GAMEPLAY")];
        assert_eq!(after_lines(&lines), Some(Scene::Elsewhere), "not a Battlegrounds load");
        let lines = vec![loaded("DRAFT", "GAMEPLAY")];
        assert_eq!(after_lines(&lines), Some(Scene::Game));
    }

    #[test]
    fn only_a_scene_that_is_not_a_game_counts_as_left() {
        assert!(Scene::BattlegroundsLobby.left_game());
        assert!(Scene::Menu.left_game());
        assert!(Scene::Elsewhere.left_game());
        assert!(!Scene::Game.left_game());
        assert!(!Scene::BattlegroundsLoading.left_game());
    }

    #[test]
    fn a_value_that_is_not_a_mode_name_is_ignored() {
        let odd = "D 1 LoadingScreen.OnSceneLoaded() - prevMode=HUB currMode=Some#1234".to_string();
        assert_eq!(after_lines(&[loaded("LOGIN", "HUB"), odd]), Some(Scene::Menu));
    }
}
