//! The game build of the running client, before any game is played (T-337).
//!
//! The own rating is read on the Battlegrounds lobby screen, often before the
//! session's first game, so the build a game writes to `Power.log` is not
//! there yet. The client writes its version once at start in `Net.log` (when
//! `[Net]` is on in `log.config`), as part of a user agent:
//!
//! ```text
//! D 04:58:45.6710837 Hearthstone/36.6.253216 (PC;…)
//! ```
//!
//! Only the build number (the third part of the version) is kept, never
//! anything else from the line. A session's build is the one its `Power.log`
//! games say, else the one in its `Net.log`; neither means unknown, and the
//! rating is then not read (D-063's fail-safe rule).

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub const NET_LOG: &str = "Net.log";
const AGENT: &str = "Hearthstone/";
const POWER_BUILD: &str = "DebugPrintGame() - BuildNumber=";
/// The user agent is written in the first seconds; only the start of the
/// file is read.
const NET_LOG_HEAD: u64 = 64 * 1024;
/// Builds have six digits today; anything far outside is not a build.
const MIN_BUILD: i64 = 1_000;
/// Looks at a `Net.log` that has no user agent before giving up on it for
/// the session (one per poll, about a second each).
const NET_LOG_LOOKS: u32 = 30;
const MAX_BUILD: i64 = 100_000_000;

/// The build in a `Power.log` line, if it is the game's build line.
pub fn from_power_line(line: &str) -> Option<i64> {
    let at = line.find(POWER_BUILD)?;
    plausible(line[at + POWER_BUILD.len()..].trim().parse().ok()?)
}

/// The build in a `Net.log` line that is the client's user agent and
/// nothing else: `D <time> Hearthstone/<major>.<minor>.<build> (…)`.
pub fn from_net_line(line: &str) -> Option<i64> {
    let mut words = line.splitn(4, ' ');
    let (Some("D"), Some(_time), Some(agent), Some(rest)) =
        (words.next(), words.next(), words.next(), words.next())
    else {
        return None;
    };
    if !rest.starts_with('(') {
        return None;
    }
    let version = agent.strip_prefix(AGENT)?;
    let parts: Vec<&str> = version.split('.').collect();
    let [major, minor, build] = parts.as_slice() else {
        return None;
    };
    let digits = |s: &str| !s.is_empty() && s.len() <= 9 && s.bytes().all(|b| b.is_ascii_digit());
    if !(digits(major) && digits(minor) && digits(build)) {
        return None;
    }
    plausible(build.parse().ok()?)
}

fn plausible(build: i64) -> Option<i64> {
    (MIN_BUILD..MAX_BUILD).contains(&build).then_some(build)
}

/// The build in the start of a session folder's `Net.log`. A missing file
/// is `Ok(None)` (the `[Net]` section is off, or the client has not written
/// it yet).
pub fn from_net_log(session_dir: &Path) -> io::Result<Option<i64>> {
    let file = match File::open(session_dir.join(NET_LOG)) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut head = Vec::new();
    file.take(NET_LOG_HEAD).read_to_end(&mut head)?;
    // Only whole lines: the last one may be cut by the limit or still being
    // written.
    let whole = head
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(&head[..0], |end| &head[..end]);
    let text = String::from_utf8_lossy(whole);
    let mut builds = text.lines().filter_map(from_net_line);
    let first = builds.next();
    // Two user agents with different builds in one session: unknown.
    Ok(first.filter(|b| builds.all(|other| other == *b)))
}

/// The client build of the session being followed.
#[derive(Clone, Debug, Default)]
pub struct ClientBuild {
    session: Option<String>,
    from_power: Option<i64>,
    from_net: Option<i64>,
    /// Looks at an existing `Net.log` that gave no build.
    net_looks: u32,
}

impl ClientBuild {
    /// A new session folder: nothing known about its build yet.
    pub fn start(&mut self, session: &str) {
        *self = ClientBuild {
            session: Some(session.to_string()),
            ..ClientBuild::default()
        };
    }

    pub fn feed_power(&mut self, line: &str) {
        if let Some(build) = from_power_line(line) {
            self.from_power = Some(build);
        }
    }

    /// Looks in the session's `Net.log` until a build is found there.
    pub fn look_in_net_log(&mut self, logs_dir: &Path) {
        if self.from_net.is_some() || self.net_looks >= NET_LOG_LOOKS {
            return;
        }
        if let Some(session) = &self.session {
            let dir = logs_dir.join(session);
            self.from_net = from_net_log(&dir).ok().flatten();
            if self.from_net.is_none() && dir.join(NET_LOG).is_file() {
                self.net_looks += 1;
            }
        }
    }

    /// The build: a game's own line first, else the client's user agent.
    /// When both are known and differ, neither is trusted.
    pub fn now(&self) -> Option<i64> {
        match (self.from_power, self.from_net) {
            (Some(p), Some(n)) if p != n => None,
            (p, n) => p.or(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_agent_line_gives_the_build_and_nothing_else() {
        let line =
            "D 04:58:45.6710837 Hearthstone/36.6.253216 (PC;System_Product_Name;Desktop;abc123)";
        assert_eq!(from_net_line(line), Some(253216));
    }

    #[test]
    fn lines_that_are_not_a_version_give_nothing() {
        for line in [
            "D 1 Hearthstone/36.6 (PC)",
            "D 1 Hearthstone/36.6.x253216 (PC)",
            "D 1 Hearthstone/36.6.253216.1 (PC)",
            "D 1 Hearthstone/36.6.12 (PC)",
            "D 1 Hearthstone/a.b.253216 (PC)",
            "D 1 Hearthstone/36.6.253216",
            "D 1 unrelated Hearthstone/36.6.253216 (PC)",
            "D 1 x Hearthstone/36.6.253216 (PC)",
            "E 1 Hearthstone/36.6.253216 (PC)",
            "D 1 Some other line 253216",
            "",
        ] {
            assert_eq!(from_net_line(line), None, "{line}");
        }
    }

    #[test]
    fn the_power_log_build_line_gives_the_build() {
        assert_eq!(
            from_power_line("D 19:11:43.1 GameState.DebugPrintGame() - BuildNumber=253216"),
            Some(253216)
        );
        assert_eq!(
            from_power_line("D 1 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS"),
            None
        );
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tl-build-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_session_build_comes_from_its_net_log_until_a_game_says_it() {
        let logs = temp_dir("net");
        let session = "Hearthstone_2026_10_11_04_58_30";
        std::fs::create_dir_all(logs.join(session)).unwrap();
        let mut b = ClientBuild::default();
        b.start(session);
        b.look_in_net_log(&logs);
        assert_eq!(b.now(), None, "no Net.log: unknown");
        // A line still being written is not read.
        std::fs::write(
            logs.join(session).join(NET_LOG),
            "D 04:58:45.1 Network.Connect\nD 04:58:45.6 Hearthstone/36.6.253216 (PC;x)",
        )
        .unwrap();
        b.look_in_net_log(&logs);
        assert_eq!(b.now(), None);
        std::fs::write(
            logs.join(session).join(NET_LOG),
            "D 04:58:45.1 Network.Connect\nD 04:58:45.6 Hearthstone/36.6.253216 (PC;x)\n",
        )
        .unwrap();
        b.look_in_net_log(&logs);
        assert_eq!(b.now(), Some(253216));
        b.feed_power("D 1 GameState.DebugPrintGame() - BuildNumber=253216");
        assert_eq!(b.now(), Some(253216));
        b.feed_power("D 2 GameState.DebugPrintGame() - BuildNumber=253999");
        assert_eq!(b.now(), None, "two builds disagree: unknown");
        b.start("Hearthstone_2026_10_12_00_00_00");
        assert_eq!(b.now(), None, "a new session starts unknown");
    }

    #[test]
    fn two_user_agents_with_different_builds_are_unknown() {
        let logs = temp_dir("net-two");
        let session = "Hearthstone_2026_10_11_04_58_30";
        std::fs::create_dir_all(logs.join(session)).unwrap();
        std::fs::write(
            logs.join(session).join(NET_LOG),
            "D 1 Hearthstone/36.6.253216 (PC)\nD 2 Hearthstone/36.6.253999 (PC)\n",
        )
        .unwrap();
        assert_eq!(from_net_log(&logs.join(session)).unwrap(), None);
    }
}
