//! Minions an opponent at tavern tier 1 or 2 could have (T-307): the pool
//! minions of their tier or lower whose tribe is in the lobby, plus the
//! neutral ones. Past tier 2 there are too many to be useful, so nothing is
//! listed. Every minion here is labelled `possible`, never `seen`.
//!
//! The lobby's tribes are the ones the user entered (T-303) or, when there
//! are none, the tribes seen in the tavern (inferred). With neither, the list
//! says the tribes are unknown instead of guessing.

use serde::Serialize;

use crate::live::LiveGame;
use crate::lobby_tribes::TRIBES;
use crate::provenance::Source;

/// The highest tavern tier that gets a list.
pub const MAX_TIER: i64 = 2;
/// A minion with this race belongs to every tribe (HearthstoneJSON and the
/// log both say `ALL`).
const ALL_TRIBES: &str = "ALL";

/// One Battlegrounds pool minion from the card data: its id, tier, tribes
/// (empty for a neutral minion) and whether only Duos has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolCard {
    pub id: String,
    pub tier: i64,
    pub tribes: Vec<String>,
    pub duos_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PossibleCard {
    pub id: String,
    pub tier: i64,
}

/// Why there is no list, in plain words; `None` when there is one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoList {
    /// Neither entered nor seen in the tavern: not guessed.
    TribesUnknown,
    /// No card data on this PC yet.
    NoCardData,
}

/// The possible minions of one hero (one seat).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Possible {
    pub seat: i64,
    pub tier: i64,
    /// The lobby tribes the list was made from.
    pub tribes: Vec<String>,
    pub tribes_source: Source,
    /// Lowest tier first, then by card id.
    pub cards: Vec<PossibleCard>,
    pub source: Source,
    pub missing: Option<NoList>,
}

/// The lobby's tribes for the list: entered by the user, else seen in the
/// tavern (only real tribes, not neutral or "all"), else none.
pub fn lobby_tribes(entered: &[String], seen: &[String]) -> (Vec<String>, Source) {
    let real = |list: &[String]| -> Vec<String> {
        list.iter()
            .filter(|t| TRIBES.contains(&t.as_str()))
            .cloned()
            .collect()
    };
    let entered = real(entered);
    if !entered.is_empty() {
        return (entered, Source::Entered);
    }
    let seen = real(seen);
    if !seen.is_empty() {
        return (seen, Source::Inferred);
    }
    (Vec::new(), Source::Unknown)
}

/// The list for one hero, or `None` when its tier is unknown or past
/// [`MAX_TIER`] (nothing is shown then).
pub fn possible_for(
    seat: i64,
    tier: Option<i64>,
    tribes: (&[String], Source),
    pool: &[PoolCard],
    duos: bool,
) -> Option<Possible> {
    let tier = tier.filter(|t| (1..=MAX_TIER).contains(t))?;
    let (lobby, tribes_source) = tribes;
    let missing = if lobby.is_empty() {
        Some(NoList::TribesUnknown)
    } else if pool.is_empty() {
        Some(NoList::NoCardData)
    } else {
        None
    };
    let mut cards: Vec<PossibleCard> = if missing.is_some() {
        Vec::new()
    } else {
        pool.iter()
            .filter(|c| (1..=tier).contains(&c.tier) && (duos || !c.duos_only))
            .filter(|c| {
                c.tribes.is_empty()
                    || c.tribes
                        .iter()
                        .any(|t| t == ALL_TRIBES || lobby.contains(t))
            })
            .map(|c| PossibleCard {
                id: c.id.clone(),
                tier: c.tier,
            })
            .collect()
    };
    cards.sort_by(|a, b| (a.tier, &a.id).cmp(&(b.tier, &b.id)));
    cards.dedup();
    Some(Possible {
        seat,
        tier,
        tribes: lobby.to_vec(),
        tribes_source,
        cards,
        source: Source::Possible,
        missing,
    })
}

impl LiveGame {
    /// Adds the possible minions of every opponent hero at tier 1 or 2. Call
    /// it after the entered tribes and the leaderboard are in.
    pub fn with_possible(mut self, pool: &[PoolCard]) -> Self {
        let seen: Vec<String> = self.tribes.iter().map(|t| t.tribe.clone()).collect();
        let (lobby, source) = lobby_tribes(&self.entered_tribes, &seen);
        let duos = self.is_duos;
        for opponent in &mut self.opponents {
            opponent.possible = opponent
                .seats
                .iter()
                .zip(&opponent.tiers)
                .filter_map(|(&seat, tier)| {
                    possible_for(seat, tier.value, (&lobby, source), pool, duos)
                })
                .collect();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, tier: i64, tribes: &[&str]) -> PoolCard {
        PoolCard {
            id: id.into(),
            tier,
            tribes: tribes.iter().map(|t| t.to_string()).collect(),
            duos_only: false,
        }
    }

    fn pool() -> Vec<PoolCard> {
        vec![
            card("BEAST_1", 1, &["BEAST"]),
            card("MURLOC_1", 1, &["MURLOC"]),
            card("NEUTRAL_1", 1, &[]),
            card("BEAST_2", 2, &["BEAST"]),
            card("DEMON_2", 2, &["DEMON"]),
            card("AMALGAM_2", 2, &["ALL"]),
            card("MURLOC_PIRATE_2", 2, &["MURLOC", "PIRATE"]),
            card("BEAST_3", 3, &["BEAST"]),
            PoolCard {
                duos_only: true,
                ..card("DUOS_1", 1, &[])
            },
        ]
    }

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    fn ids(p: &Possible) -> Vec<&str> {
        p.cards.iter().map(|c| c.id.as_str()).collect()
    }

    #[test]
    fn tier_one_lists_tier_one_minions_of_the_lobby_tribes_and_neutral_ones() {
        let lobby = strings(&["BEAST", "DEMON"]);
        let p = possible_for(3, Some(1), (&lobby, Source::Entered), &pool(), false).unwrap();
        assert_eq!(ids(&p), ["BEAST_1", "NEUTRAL_1"]);
        assert_eq!(p.source, Source::Possible);
        assert_eq!(p.tribes_source, Source::Entered);
        assert_eq!(p.missing, None);
    }

    #[test]
    fn tier_two_adds_tier_two_and_keeps_tier_one_lowest_first() {
        let lobby = strings(&["BEAST", "PIRATE"]);
        let p = possible_for(3, Some(2), (&lobby, Source::Entered), &pool(), false).unwrap();
        assert_eq!(
            ids(&p),
            [
                "BEAST_1",
                "NEUTRAL_1",
                "AMALGAM_2",
                "BEAST_2",
                "MURLOC_PIRATE_2"
            ],
            "a minion of two tribes counts when either is in the lobby; 'all' always"
        );
    }

    #[test]
    fn tier_three_or_more_or_unknown_lists_nothing() {
        let lobby = strings(&["BEAST"]);
        for tier in [Some(3), Some(6), None, Some(0), Some(-1)] {
            assert_eq!(
                possible_for(3, tier, (&lobby, Source::Entered), &pool(), false),
                None,
                "{tier:?}"
            );
        }
    }

    #[test]
    fn duos_only_minions_are_left_out_of_solo() {
        let lobby = strings(&["BEAST"]);
        let solo = possible_for(3, Some(1), (&lobby, Source::Entered), &pool(), false).unwrap();
        let duos = possible_for(3, Some(1), (&lobby, Source::Entered), &pool(), true).unwrap();
        assert!(!ids(&solo).contains(&"DUOS_1"));
        assert!(ids(&duos).contains(&"DUOS_1"));
    }

    #[test]
    fn unknown_tribes_are_said_not_guessed() {
        let p = possible_for(3, Some(1), (&[], Source::Unknown), &pool(), false).unwrap();
        assert!(p.cards.is_empty(), "not even neutral minions");
        assert_eq!(p.missing, Some(NoList::TribesUnknown));
        assert_eq!(p.tribes_source, Source::Unknown);
    }

    #[test]
    fn no_card_data_is_said() {
        let lobby = strings(&["BEAST"]);
        let p = possible_for(3, Some(1), (&lobby, Source::Entered), &[], false).unwrap();
        assert!(p.cards.is_empty());
        assert_eq!(p.missing, Some(NoList::NoCardData));
    }

    #[test]
    fn entered_tribes_win_over_the_tavern_and_neither_is_unknown() {
        let entered = strings(&["BEAST", "DEMON", "DRAGON", "MURLOC", "NAGA"]);
        let seen = strings(&["PIRATE", "NEUTRAL", "ALL"]);
        assert_eq!(
            lobby_tribes(&entered, &seen),
            (entered.clone(), Source::Entered)
        );
        assert_eq!(
            lobby_tribes(&[], &seen),
            (strings(&["PIRATE"]), Source::Inferred),
            "neutral and 'all' are not lobby tribes"
        );
        assert_eq!(
            lobby_tribes(&[], &strings(&["NEUTRAL"])),
            (Vec::new(), Source::Unknown)
        );
    }
}
