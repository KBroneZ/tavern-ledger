//! Cards proven on real combats (T-646): a modelled card is proven on a game
//! build when a combat of that build had it on a board that fought (before
//! some attack: at the start, summoned during the combat, or a Duos
//! teammate's board once it came in) and the engine replayed that combat as
//! the game did, attack by attack, to the end. An attack the engine matched
//! only with another roll of its dice still counts (chance, not a rule:
//! T-643), and so does a card with no combat effect, which is modelled as
//! such. Every other modelled card plays from its text only on that build.

use std::collections::{BTreeMap, BTreeSet};

use crate::cards;
use crate::harness_logs::{LogCombat, Replayed};

/// Build -> the modelled cards proven on it. `saved` and `replays` are the
/// saved combats and their replays in the same order (`replay_all`);
/// `game_builds[g - 1]` is game `g`'s build.
pub fn proven_cards(
    saved: &[LogCombat],
    replays: &[Replayed],
    game_builds: &[Option<i64>],
) -> BTreeMap<i64, BTreeSet<String>> {
    let mut out: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    for (combat, replayed) in saved.iter().zip(replays) {
        let r = &replayed.replay;
        let followed = r.difference.is_none() && r.attacks > 0 && r.matched == r.attacks;
        let build = combat
            .game
            .checked_sub(1)
            .and_then(|g| game_builds.get(g).copied().flatten());
        let (true, Some(build)) = (followed, build) else {
            continue;
        };
        let ids = out.entry(build).or_default();
        for id in card_ids(combat) {
            if cards::rules(id).is_some() {
                ids.insert(id.to_string());
            }
        }
    }
    out
}

/// The modelled cards not proven: those that play from their text only.
pub fn text_only(proven: &BTreeSet<String>) -> usize {
    cards::modelled_ids()
        .iter()
        .filter(|id| !proven.contains(**id))
        .count()
}

/// Every card id on a board that fought: the boards before each attack. A
/// Duos teammate's board that never came in is not there.
fn card_ids(c: &LogCombat) -> impl Iterator<Item = &str> {
    c.steps
        .iter()
        .flat_map(|s| s.before.iter().flatten())
        .map(|m| m.card_id.as_str())
}

#[cfg(test)]
#[path = "harness_proven_tests.rs"]
mod tests;
