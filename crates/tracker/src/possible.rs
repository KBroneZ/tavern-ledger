//! Minions an opponent could have (T-307, T-309, T-310): the pool minions of
//! their tier whose tribe is in the lobby, plus the neutral ones. Every
//! minion here is labelled `possible`, never `seen`.
//!
//! When (D-049, D-050): on game turns 2 and 3 every opponent hero gets a list
//! of their tier or lower; from turn 4 on only a hero whose board we have not
//! seen yet gets one, of their tier and the tier below. Nothing on turn 1 or
//! an unknown turn: there is nothing to go on.
//!
//! The lobby's tribes (D-050): the log has no list of them. They are the ones
//! the user entered (T-303); else the tribes the tavern confirmed: a pool
//! minion of exactly one tribe that Bob offered proves its tribe is in the
//! lobby (the card data knows every tribe of a minion, the log's `CARDRACE`
//! only one, so its counts can show tribes that are not in the lobby); else
//! the tribes seen in the tavern by `CARDRACE`. With none, the list says the
//! tribes are unknown instead of guessing.

use serde::Serialize;

use crate::live::LiveGame;
use crate::lobby_tribes::TRIBES;
use crate::provenance::Source;

/// The game turns that list every opponent (the shop record's turn numbers).
pub const TURNS: std::ops::RangeInclusive<i64> = 2..=3;
/// A Battlegrounds lobby has five tribes (D-029), in Solo and in Duos.
pub const LOBBY_SIZE: usize = 5;
/// Tavern tiers the game has; any other value is not a tier.
const TIERS: std::ops::RangeInclusive<i64> = 1..=7;
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

/// Where the lobby's tribes came from (D-050).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TribesBasis {
    /// The user typed them (T-303).
    Entered,
    /// Single-tribe pool minions Bob offered, read with the card data.
    TavernConfirmed,
    /// The log's `CARDRACE` of the offers (no card data): may hold a tribe
    /// that is not in the lobby.
    TavernSeen,
    Unknown,
}

/// The lobby's tribes as far as the app knows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LobbyTribes {
    pub tribes: Vec<String>,
    pub source: Source,
    pub basis: TribesBasis,
    /// All five are known.
    pub complete: bool,
}

impl Default for LobbyTribes {
    fn default() -> Self {
        Self {
            tribes: Vec::new(),
            source: Source::Unknown,
            basis: TribesBasis::Unknown,
            complete: false,
        }
    }
}

/// Why a hero gets a list (D-050).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Game turn 2 or 3: their tier or lower.
    EarlyTurn,
    /// Turn 4 or later and no board of theirs seen yet: their tier and the
    /// tier below.
    NotMetYet,
}

/// The possible minions of one hero (one seat).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Possible {
    pub seat: i64,
    pub tier: i64,
    /// The lowest tier listed (1 on turns 2 and 3).
    pub min_tier: i64,
    pub reason: Reason,
    /// The lobby tribes the list was made from.
    pub tribes: Vec<String>,
    pub tribes_source: Source,
    pub tribes_basis: TribesBasis,
    pub tribes_complete: bool,
    /// Lowest tier first, then by card id.
    pub cards: Vec<PossibleCard>,
    pub source: Source,
    pub missing: Option<NoList>,
}

/// Only real tribes (not neutral or "all"), each once, sorted.
fn real(list: &[String]) -> Vec<String> {
    let mut out: Vec<String> = list
        .iter()
        .filter(|t| TRIBES.contains(&t.as_str()))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The tribes the tavern proved are in the lobby: those of the pool minions
/// Bob offered that have exactly one tribe (D-050). `None` when no such
/// offer is in the card data, or when more than five tribes come out (the
/// card data and the game disagree): then nothing is claimed.
pub fn confirmed_tribes(offered: &[String], pool: &[PoolCard]) -> Option<Vec<String>> {
    let single: Vec<String> = offered
        .iter()
        .filter_map(|id| pool.iter().find(|c| &c.id == id))
        .filter(|c| c.tribes.len() == 1)
        .map(|c| c.tribes[0].clone())
        .collect();
    let tribes = real(&single);
    (!tribes.is_empty() && tribes.len() <= LOBBY_SIZE).then_some(tribes)
}

/// The lobby's tribes for the lists: entered by the user, else confirmed by
/// the tavern with the card data, else seen in the tavern, else none.
pub fn lobby_tribes(
    entered: &[String],
    confirmed: Option<&[String]>,
    seen: &[String],
) -> LobbyTribes {
    let entered = real(entered);
    if !entered.is_empty() {
        return LobbyTribes {
            complete: entered.len() == LOBBY_SIZE,
            tribes: entered,
            source: Source::Entered,
            basis: TribesBasis::Entered,
        };
    }
    if let Some(confirmed) = confirmed.map(real).filter(|c| !c.is_empty()) {
        return LobbyTribes {
            complete: confirmed.len() == LOBBY_SIZE,
            tribes: confirmed,
            source: Source::Inferred,
            basis: TribesBasis::TavernConfirmed,
        };
    }
    let seen = real(seen);
    if !seen.is_empty() {
        return LobbyTribes {
            tribes: seen,
            source: Source::Inferred,
            basis: TribesBasis::TavernSeen,
            complete: false,
        };
    }
    LobbyTribes::default()
}

/// Why a hero gets a list on this turn, if it does: every hero on turns 2
/// and 3; from turn 4 on only one whose board we have not seen.
pub fn reason_for(turn: Option<i64>, board_seen: bool) -> Option<Reason> {
    match turn? {
        t if TURNS.contains(&t) => Some(Reason::EarlyTurn),
        t if t > *TURNS.end() && !board_seen => Some(Reason::NotMetYet),
        _ => None,
    }
}

/// The list for one hero, or `None` when its tier is unknown (nothing is
/// shown then).
pub fn possible_for(
    seat: i64,
    tier: Option<i64>,
    reason: Reason,
    lobby: &LobbyTribes,
    pool: &[PoolCard],
    duos: bool,
) -> Option<Possible> {
    let tier = tier.filter(|t| TIERS.contains(t))?;
    let min_tier = match reason {
        Reason::EarlyTurn => 1,
        Reason::NotMetYet => (tier - 1).max(1),
    };
    let missing = if lobby.tribes.is_empty() {
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
            .filter(|c| (min_tier..=tier).contains(&c.tier) && (duos || !c.duos_only))
            .filter(|c| {
                c.tribes.is_empty()
                    || c.tribes
                        .iter()
                        .any(|t| t == ALL_TRIBES || lobby.tribes.contains(t))
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
        min_tier,
        reason,
        tribes: lobby.tribes.clone(),
        tribes_source: lobby.source,
        tribes_basis: lobby.basis,
        tribes_complete: lobby.complete,
        cards,
        source: Source::Possible,
        missing,
    })
}

impl LiveGame {
    /// Adds the lobby's tribes as far as the user, the tavern and the card
    /// data tell, and the possible minions of every opponent hero the turn
    /// rule lets through ([`reason_for`]). Call it after the entered tribes
    /// and the leaderboard are in.
    pub fn with_possible(mut self, pool: &[PoolCard]) -> Self {
        let seen: Vec<String> = self.tribes.iter().map(|t| t.tribe.clone()).collect();
        let confirmed = confirmed_tribes(&self.offered, pool);
        let lobby = lobby_tribes(&self.entered_tribes, confirmed.as_deref(), &seen);
        let duos = self.is_duos;
        let turn = self.turn.value;
        for opponent in &mut self.opponents {
            let lists: Vec<Possible> = opponent
                .seats
                .iter()
                .zip(&opponent.tiers)
                .filter_map(|(&seat, tier)| {
                    let met = opponent.boards.iter().any(|b| b.seat == seat);
                    let reason = reason_for(turn, met)?;
                    possible_for(seat, tier.value, reason, &lobby, pool, duos)
                })
                .collect();
            opponent.possible = lists;
        }
        self.lobby_tribes = lobby;
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

    fn entered(tribes: &[String]) -> LobbyTribes {
        lobby_tribes(tribes, None, &[])
    }

    fn ids(p: &Possible) -> Vec<&str> {
        p.cards.iter().map(|c| c.id.as_str()).collect()
    }

    #[test]
    fn tier_one_lists_tier_one_minions_of_the_lobby_tribes_and_neutral_ones() {
        let lobby = strings(&["BEAST", "DEMON"]);
        let p = possible_for(
            3,
            Some(1),
            Reason::EarlyTurn,
            &entered(&lobby),
            &pool(),
            false,
        )
        .unwrap();
        assert_eq!(ids(&p), ["BEAST_1", "NEUTRAL_1"]);
        assert_eq!(p.source, Source::Possible);
        assert_eq!(p.tribes_source, Source::Entered);
        assert_eq!(p.missing, None);
    }

    #[test]
    fn tier_two_adds_tier_two_and_keeps_tier_one_lowest_first() {
        let lobby = strings(&["BEAST", "PIRATE"]);
        let p = possible_for(
            3,
            Some(2),
            Reason::EarlyTurn,
            &entered(&lobby),
            &pool(),
            false,
        )
        .unwrap();
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
    fn tier_three_adds_tier_three() {
        let lobby = strings(&["BEAST"]);
        let p = possible_for(
            3,
            Some(3),
            Reason::EarlyTurn,
            &entered(&lobby),
            &pool(),
            false,
        )
        .unwrap();
        assert_eq!(
            ids(&p),
            ["BEAST_1", "NEUTRAL_1", "AMALGAM_2", "BEAST_2", "BEAST_3"]
        );
    }

    #[test]
    fn an_unknown_tier_lists_nothing() {
        let lobby = strings(&["BEAST"]);
        for tier in [None, Some(0), Some(-1), Some(8)] {
            assert_eq!(
                possible_for(3, tier, Reason::EarlyTurn, &entered(&lobby), &pool(), false),
                None,
                "{tier:?}"
            );
        }
    }

    #[test]
    fn duos_only_minions_are_left_out_of_solo() {
        let lobby = strings(&["BEAST"]);
        let solo = possible_for(
            3,
            Some(1),
            Reason::EarlyTurn,
            &entered(&lobby),
            &pool(),
            false,
        )
        .unwrap();
        let duos = possible_for(
            3,
            Some(1),
            Reason::EarlyTurn,
            &entered(&lobby),
            &pool(),
            true,
        )
        .unwrap();
        assert!(!ids(&solo).contains(&"DUOS_1"));
        assert!(ids(&duos).contains(&"DUOS_1"));
    }

    #[test]
    fn unknown_tribes_are_said_not_guessed() {
        let p = possible_for(
            3,
            Some(1),
            Reason::EarlyTurn,
            &LobbyTribes::default(),
            &pool(),
            false,
        )
        .unwrap();
        assert!(p.cards.is_empty(), "not even neutral minions");
        assert_eq!(p.missing, Some(NoList::TribesUnknown));
        assert_eq!(p.tribes_source, Source::Unknown);
    }

    #[test]
    fn no_card_data_is_said() {
        let lobby = strings(&["BEAST"]);
        let p = possible_for(3, Some(1), Reason::EarlyTurn, &entered(&lobby), &[], false).unwrap();
        assert!(p.cards.is_empty());
        assert_eq!(p.missing, Some(NoList::NoCardData));
    }

    #[test]
    fn entered_tribes_win_over_the_tavern_and_neither_is_unknown() {
        let entered = strings(&["BEAST", "DEMON", "DRAGON", "MURLOC", "NAGA"]);
        let confirmed = strings(&["MURLOC"]);
        let seen = strings(&["PIRATE", "NEUTRAL", "ALL"]);
        let lobby = lobby_tribes(&entered, Some(&confirmed), &seen);
        assert_eq!(lobby.tribes, entered);
        assert_eq!(
            (lobby.source, lobby.basis, lobby.complete),
            (Source::Entered, TribesBasis::Entered, true)
        );
        let lobby = lobby_tribes(&[], Some(&confirmed), &seen);
        assert_eq!(
            lobby.tribes, confirmed,
            "the tavern's confirmed tribes next"
        );
        assert_eq!(
            (lobby.source, lobby.basis, lobby.complete),
            (Source::Inferred, TribesBasis::TavernConfirmed, false)
        );
        let lobby = lobby_tribes(&[], None, &seen);
        assert_eq!(
            lobby.tribes,
            strings(&["PIRATE"]),
            "neutral and 'all' are not lobby tribes"
        );
        assert_eq!(
            (lobby.source, lobby.basis, lobby.complete),
            (Source::Inferred, TribesBasis::TavernSeen, false)
        );
        assert_eq!(
            lobby_tribes(&[], None, &strings(&["NEUTRAL"])),
            LobbyTribes::default()
        );
        assert_eq!(lobby_tribes(&[], Some(&[]), &[]).source, Source::Unknown);
    }

    #[test]
    fn single_tribe_offers_confirm_their_tribe_and_dual_ones_do_not() {
        let pool = vec![
            card("BEAST_1", 1, &["BEAST"]),
            card("DEMON_QUIL_2", 2, &["DEMON", "QUILBOAR"]),
            card("AMALGAM_2", 2, &["ALL"]),
            card("NEUTRAL_1", 1, &[]),
            card("MURLOC_1", 1, &["MURLOC"]),
        ];
        let offered = strings(&[
            "BEAST_1",
            "DEMON_QUIL_2",
            "AMALGAM_2",
            "NEUTRAL_1",
            "BEAST_1",
            "NOT_IN_THE_DATA",
        ]);
        assert_eq!(
            confirmed_tribes(&offered, &pool),
            Some(strings(&["BEAST"])),
            "a demon-quilboar offer proves neither; 'all', neutral and unknown cards prove nothing"
        );
        assert_eq!(confirmed_tribes(&strings(&["NEUTRAL_1"]), &pool), None);
        assert_eq!(confirmed_tribes(&offered, &[]), None, "no card data");
    }

    #[test]
    fn more_than_five_confirmed_tribes_claims_nothing() {
        let tribes = ["BEAST", "DEMON", "DRAGON", "MURLOC", "NAGA", "PIRATE"];
        let pool: Vec<PoolCard> = tribes.iter().map(|t| card(t, 1, &[t])).collect();
        let offered = strings(&tribes);
        assert_eq!(
            confirmed_tribes(&offered[..5], &pool).map(|t| t.len()),
            Some(5)
        );
        assert_eq!(confirmed_tribes(&offered, &pool), None);
    }

    #[test]
    fn five_confirmed_tribes_make_the_lobby_complete() {
        let five = strings(&["BEAST", "DEMON", "DRAGON", "MURLOC", "NAGA"]);
        assert!(lobby_tribes(&[], Some(&five), &[]).complete);
        assert!(!lobby_tribes(&[], Some(&five[..4]), &[]).complete);
    }

    #[test]
    fn turn_rule_lists_everyone_on_two_and_three_and_only_unmet_heroes_later() {
        for met in [false, true] {
            assert_eq!(reason_for(Some(2), met), Some(Reason::EarlyTurn));
            assert_eq!(reason_for(Some(3), met), Some(Reason::EarlyTurn));
            for turn in [None, Some(0), Some(1), Some(-2)] {
                assert_eq!(reason_for(turn, met), None, "{turn:?}");
            }
        }
        for turn in [4, 9, 20] {
            assert_eq!(reason_for(Some(turn), false), Some(Reason::NotMetYet));
            assert_eq!(reason_for(Some(turn), true), None, "a board was seen");
        }
    }

    #[test]
    fn a_hero_not_met_yet_gets_their_tier_and_the_tier_below() {
        let lobby = strings(&["BEAST"]);
        let p = possible_for(
            3,
            Some(3),
            Reason::NotMetYet,
            &entered(&lobby),
            &pool(),
            false,
        )
        .unwrap();
        assert_eq!(ids(&p), ["AMALGAM_2", "BEAST_2", "BEAST_3"]);
        assert_eq!((p.min_tier, p.tier, p.reason), (2, 3, Reason::NotMetYet));
        let p = possible_for(
            3,
            Some(1),
            Reason::NotMetYet,
            &entered(&lobby),
            &pool(),
            false,
        )
        .unwrap();
        assert_eq!(p.min_tier, 1, "never below tier 1");
        assert_eq!(ids(&p), ["BEAST_1", "NEUTRAL_1"]);
    }
}
