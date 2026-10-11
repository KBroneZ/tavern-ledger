//! A test build never draws its overlay over the user's game (T-336
//! follow-up). Sessions 056 and 058 showed a test overlay over a real game
//! for a few seconds; now a debug build, or any run with a development flag,
//! keeps the overlay window hidden while `Hearthstone.exe` runs, unless it
//! was started with `--allow-over-game`. Release builds started without a
//! development flag are unchanged. The game is found by listing process
//! names only (`game_memory::process::is_running`): no handle on it is opened.

use std::time::{Duration, Instant};

/// Shows the overlay of a test build over a running game anyway.
pub const ALLOW_OVER_GAME_ARG: &str = "--allow-over-game";

/// Flags only used to try a build: any of them makes the run a test run.
pub const DEV_ARGS: [&str; 4] = [
    "--overlay-dev",
    "--overlay-unlock",
    "--logs-dir",
    "--data-dir",
];

/// How long a "the game runs" answer is kept: listing every process each
/// half second would be wasteful, and a game starts slower than this.
const RECHECK_EVERY: Duration = Duration::from_secs(2);

/// Whether this run is guarded: a debug build or a development flag, and no
/// `--allow-over-game`.
pub fn guarded(debug_build: bool, args: impl IntoIterator<Item = String>) -> bool {
    let args: Vec<String> = args.into_iter().collect();
    let dev = args.iter().skip(1).any(|a| DEV_ARGS.contains(&a.as_str()));
    let allowed = args.iter().skip(1).any(|a| a == ALLOW_OVER_GAME_ARG);
    (debug_build || dev) && !allowed
}

/// Whether the overlay must stay hidden now.
pub fn blocks(guarded: bool, game_running: bool) -> bool {
    guarded && game_running
}

/// Says whether the game runs, asking the system at most every
/// [`RECHECK_EVERY`]; an unguarded run never asks.
pub struct GameWatch {
    guarded: bool,
    last: Option<(Instant, bool)>,
}

impl GameWatch {
    pub fn new(guarded: bool) -> Self {
        Self {
            guarded,
            last: None,
        }
    }

    /// Whether the overlay must stay hidden now; `running` asks the system.
    pub fn blocks_with(&mut self, now: Instant, running: impl FnOnce() -> bool) -> bool {
        if !self.guarded {
            return false;
        }
        let fresh = matches!(self.last, Some((at, _)) if now.duration_since(at) < RECHECK_EVERY);
        if !fresh {
            self.last = Some((now, running()));
        }
        blocks(self.guarded, self.last.is_some_and(|(_, on)| on))
    }

    pub fn blocks(&mut self) -> bool {
        self.blocks_with(Instant::now(), game_memory::process::is_running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_debug_build_is_guarded_unless_allowed() {
        assert!(guarded(true, args(&["desktop.exe"])));
        assert!(!guarded(true, args(&["desktop.exe", ALLOW_OVER_GAME_ARG])));
    }

    #[test]
    fn a_release_build_is_guarded_only_with_a_dev_flag() {
        assert!(!guarded(false, args(&["desktop.exe"])));
        assert!(!guarded(false, args(&["desktop.exe", "--minimized"])));
        for flag in DEV_ARGS {
            assert!(guarded(false, args(&["desktop.exe", flag, "x"])), "{flag}");
            assert!(
                !guarded(
                    false,
                    args(&["desktop.exe", flag, "x", ALLOW_OVER_GAME_ARG])
                ),
                "{flag}"
            );
        }
        // The program's own path is not a flag.
        assert!(!guarded(false, args(&["--overlay-dev"])));
    }

    #[test]
    fn a_guarded_run_hides_the_overlay_only_while_the_game_runs() {
        assert!(blocks(true, true));
        assert!(!blocks(true, false));
        assert!(!blocks(false, true), "release builds unchanged");
    }

    #[test]
    fn the_process_list_is_read_at_most_every_two_seconds() {
        let start = Instant::now();
        let mut watch = GameWatch::new(true);
        assert!(watch.blocks_with(start, || true));
        assert!(
            watch.blocks_with(start + Duration::from_millis(500), || panic!("kept")),
            "the answer is kept"
        );
        assert!(
            !watch.blocks_with(start + RECHECK_EVERY, || false),
            "asked again"
        );
    }

    #[test]
    fn an_unguarded_run_never_lists_processes() {
        let mut watch = GameWatch::new(false);
        assert!(!watch.blocks_with(Instant::now(), || panic!("must not ask")));
    }
}
