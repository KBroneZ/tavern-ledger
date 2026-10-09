//! Personal stats over the local history (T-102): totals, per-hero numbers
//! and tribes seen in the tavern, per game mode. The app only renders them.
//!
//! Only finished games (`ok`) with a valid place count for places; the rest
//! are counted apart, never as zero. With no counted game the numbers are
//! None ("—" in the window), not 0. Hero names come from the history itself
//! (D-017), never from a guessed mapping.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::provenance::{legend, LegendEntry, Source};

pub const SOLO: &str = "GT_BATTLEGROUNDS";
pub const DUOS: &str = "GT_BATTLEGROUNDS_DUO";
/// Any other Battlegrounds game type: listed so no game is hidden, but its
/// place rules are unknown, so it gets no place numbers.
pub const OTHER: &str = "OTHER";

/// Numbers for a set of games.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Tally {
    /// Every game in the set, counted or not.
    pub games: usize,
    /// Games whose place counts: finished, with a valid place.
    pub placed: usize,
    /// Not finished in the log.
    pub incomplete: usize,
    /// Could not be read (unsupported log format).
    pub unsupported: usize,
    pub average_place: Option<f64>,
    /// Share of counted games in the top half (0.0 to 1.0).
    pub top_half_share: Option<f64>,
    pub wins: Option<usize>,
}

/// Where each number of a `Tally` comes from (T-109): the log when there is
/// a number, unknown when there is none ("—" in the window).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TallySources {
    pub games: Source,
    pub average_place: Source,
    pub top_half_share: Source,
    pub wins: Source,
}

impl TallySources {
    fn of(tally: &Tally) -> Self {
        TallySources {
            games: Source::log_if(tally.games > 0),
            average_place: Source::log_if(tally.average_place.is_some()),
            top_half_share: Source::log_if(tally.top_half_share.is_some()),
            wins: Source::log_if(tally.wins.is_some()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HeroStats {
    /// Hero card id without its skin suffix; None when the log gave no hero.
    pub hero: Option<String>,
    /// Name as the game's log printed it; None when no game in the history
    /// names this hero.
    pub name: Option<String>,
    /// Every card id (skins included) grouped under this hero.
    pub variants: Vec<String>,
    /// The name is printed by the log, or unknown (the id is shown instead).
    pub name_source: Source,
    /// Several card ids under one hero are grouped by us (D-019): inferred.
    pub grouping: Source,
    pub tally: Tally,
    pub tally_sources: TallySources,
}

/// A tribe offered in the tavern. Not the exact lobby tribes: the log does
/// not give those (see docs/research/parser-hslog.md).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TribeSeen {
    pub tribe: String,
    /// Games where the tavern offered it at least once.
    pub games: usize,
    /// Tavern offers with this tribe, over all those games.
    pub offers: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModeStats {
    /// SOLO, DUOS or OTHER.
    pub mode: String,
    /// Places that count as top half (4 in Solo, 2 in Duos); None for OTHER.
    pub top_half: Option<i64>,
    pub totals: Tally,
    pub totals_sources: TallySources,
    /// Most played first.
    pub heroes: Vec<HeroStats>,
    /// Games with any tavern data, the denominator for `tribes`.
    pub games_with_tribes: usize,
    /// Seen in most games first.
    pub tribes: Vec<TribeSeen>,
    /// Every tribe row is our reading of the tavern offers: inferred.
    pub tribes_source: Source,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Stats {
    /// Solo and Duos always (maybe with 0 games); OTHER only when it has games.
    pub modes: Vec<ModeStats>,
    /// What each source label means; the window shows it as its legend.
    pub legend: Vec<LegendEntry>,
}

/// The hero a card id belongs to: the id without a trailing `_SKIN_<x>`.
/// Ids with other prefixes or suffixes stay as they are.
pub fn base_hero(card_id: &str) -> &str {
    match card_id.rfind(SKIN) {
        Some(at) if at > 0 && at + SKIN.len() < card_id.len() => {
            let suffix = &card_id[at + SKIN.len()..];
            if suffix.chars().all(|c| c.is_ascii_alphanumeric()) {
                &card_id[..at]
            } else {
                card_id
            }
        }
        _ => card_id,
    }
}

const SKIN: &str = "_SKIN_";

/// Place rules of a mode: the top half, and the worst possible place.
pub(crate) fn rules(mode: &str) -> Option<(i64, i64)> {
    match mode {
        SOLO => Some((4, 8)),
        DUOS => Some((2, 4)), // Duos places are per team
        _ => None,
    }
}

pub(crate) fn mode_of(report: &Value) -> &'static str {
    match report.get("game_type").and_then(Value::as_str) {
        Some(SOLO) => SOLO,
        Some(DUOS) => DUOS,
        _ => OTHER,
    }
}

/// The place of a game if it counts: a finished game with a place inside
/// the mode's range. None for anything else.
pub(crate) fn counted_place(mode: &str, status: Option<&str>, report: &Value) -> Option<i64> {
    let (_, worst) = rules(mode)?;
    if status != Some("ok") {
        return None;
    }
    report
        .get("final_place")
        .and_then(Value::as_i64)
        .filter(|p| (1..=worst).contains(p))
}

/// Games of one set, before turning them into a Tally.
#[derive(Default)]
struct Counts {
    games: usize,
    incomplete: usize,
    unsupported: usize,
    places: Vec<i64>,
}

impl Counts {
    /// `counted_place` is the place only when it counts (see `counted_place`).
    fn add(&mut self, status: Option<&str>, counted_place: Option<i64>) {
        self.games += 1;
        match status {
            Some("incomplete") => self.incomplete += 1,
            Some("unsupported") => self.unsupported += 1,
            _ => self.places.extend(counted_place),
        }
    }

    fn tally(&self, top_half: Option<i64>) -> Tally {
        let placed = self.places.len();
        let share = |hits: usize| (placed > 0).then(|| hits as f64 / placed as f64);
        let top = top_half.unwrap_or(0);
        Tally {
            games: self.games,
            placed,
            incomplete: self.incomplete,
            unsupported: self.unsupported,
            average_place: (placed > 0)
                .then(|| self.places.iter().sum::<i64>() as f64 / placed as f64),
            top_half_share: share(self.places.iter().filter(|&&p| p <= top).count()),
            wins: (placed > 0).then(|| self.places.iter().filter(|&&p| p == 1).count()),
        }
    }
}

#[derive(Default)]
struct ModeCounts {
    totals: Counts,
    heroes: BTreeMap<Option<String>, (BTreeSet<String>, Counts)>,
    games_with_tribes: usize,
    tribes: BTreeMap<String, (usize, u64)>,
}

impl ModeCounts {
    fn add(&mut self, mode: &str, report: &Value) {
        let status = report.get("status").and_then(Value::as_str);
        let place = counted_place(mode, status, report);
        self.totals.add(status, place);
        let hero = report.get("hero").and_then(Value::as_str);
        let (variants, counts) = self
            .heroes
            .entry(hero.map(|h| base_hero(h).to_string()))
            .or_default();
        variants.extend(hero.map(String::from));
        counts.add(status, place);

        let offered: Vec<(&String, u64)> = report
            .get("shop_tribes")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(tribe, n)| n.as_u64().filter(|&n| n > 0).map(|n| (tribe, n)))
            .collect();
        if !offered.is_empty() {
            self.games_with_tribes += 1;
        }
        for (tribe, n) in offered {
            let (games, offers) = self.tribes.entry(tribe.clone()).or_default();
            *games += 1;
            *offers = offers.saturating_add(n);
        }
    }

    fn finish(self, mode: &str, names: &Names) -> ModeStats {
        let top_half = rules(mode).map(|(top, _)| top);
        let mut heroes: Vec<HeroStats> = self
            .heroes
            .into_iter()
            .map(|(hero, (variants, counts))| {
                let name = hero.as_deref().and_then(|h| names.for_hero(h, &variants));
                let tally = counts.tally(top_half);
                HeroStats {
                    name_source: Source::log_if(name.is_some()),
                    grouping: if variants.len() > 1 {
                        Source::Inferred
                    } else {
                        Source::Log
                    },
                    tally_sources: TallySources::of(&tally),
                    name,
                    hero,
                    variants: variants.into_iter().collect(),
                    tally,
                }
            })
            .collect();
        // Most played first; ties by id, unknown hero last, so the order is stable.
        heroes.sort_by(|a, b| {
            b.tally
                .games
                .cmp(&a.tally.games)
                .then_with(|| a.hero.is_none().cmp(&b.hero.is_none()))
                .then_with(|| a.hero.cmp(&b.hero))
        });
        let mut tribes: Vec<TribeSeen> = self
            .tribes
            .into_iter()
            .map(|(tribe, (games, offers))| TribeSeen {
                tribe,
                games,
                offers,
            })
            .collect();
        tribes.sort_by(|a, b| {
            b.games
                .cmp(&a.games)
                .then(b.offers.cmp(&a.offers))
                .then_with(|| a.tribe.cmp(&b.tribe))
        });
        let totals = self.totals.tally(top_half);
        ModeStats {
            mode: mode.to_string(),
            top_half,
            totals_sources: TallySources::of(&totals),
            totals,
            heroes,
            games_with_tribes: self.games_with_tribes,
            tribes,
            tribes_source: Source::Inferred,
        }
    }
}

/// Hero names printed by the log, from every game in the history.
#[derive(Default)]
struct Names(BTreeMap<String, BTreeMap<String, usize>>);

impl Names {
    fn add(&mut self, report: &Value) {
        let printed = report.get("card_names").and_then(Value::as_object);
        for (id, name) in printed.into_iter().flatten() {
            if let Some(name) = name.as_str() {
                *self
                    .0
                    .entry(id.clone())
                    .or_default()
                    .entry(name.to_string())
                    .or_default() += 1;
            }
        }
    }

    /// The most common name of a card id; a tie takes the first in
    /// alphabetical order, so the name never flips between refreshes.
    fn of(&self, id: &str) -> Option<String> {
        let seen = self.0.get(id)?;
        let most = seen.values().max()?;
        seen.iter()
            .find(|(_, n)| *n == most)
            .map(|(name, _)| name.clone())
    }

    /// The base hero's name if any game names it, else its first named skin.
    fn for_hero(&self, base: &str, variants: &BTreeSet<String>) -> Option<String> {
        self.of(base)
            .or_else(|| variants.iter().find_map(|id| self.of(id)))
    }
}

/// Stats for the latest report of every game in the history.
pub fn compute<'a>(reports: impl IntoIterator<Item = &'a Value>) -> Stats {
    let mut names = Names::default();
    let mut modes: BTreeMap<&'static str, ModeCounts> = BTreeMap::new();
    for report in reports {
        names.add(report);
        let mode = mode_of(report);
        modes.entry(mode).or_default().add(mode, report);
    }
    let other = modes.remove(OTHER);
    let mut take = |mode| modes.remove(mode).unwrap_or_default().finish(mode, &names);
    let mut out = vec![take(SOLO), take(DUOS)];
    out.extend(other.map(|counts| counts.finish(OTHER, &names)));
    Stats {
        modes: out,
        legend: legend(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(mode: &str, status: &str, hero: &str, place: Option<i64>) -> Value {
        json!({
            "status": status,
            "game_type": mode,
            "hero": hero,
            "final_place": place,
            "card_names": {},
            "shop_tribes": {},
        })
    }

    fn mode<'a>(stats: &'a Stats, name: &str) -> &'a ModeStats {
        stats.modes.iter().find(|m| m.mode == name).unwrap()
    }

    #[test]
    fn empty_history_has_solo_and_duos_with_no_numbers() {
        let stats = compute(&[]);
        assert_eq!(
            stats
                .modes
                .iter()
                .map(|m| m.mode.as_str())
                .collect::<Vec<_>>(),
            [SOLO, DUOS]
        );
        for m in &stats.modes {
            assert_eq!(m.totals, Tally::default());
            assert_eq!(m.totals.average_place, None);
            assert_eq!(m.totals.wins, None, "no games is unknown, not 0 wins");
            assert!(m.heroes.is_empty() && m.tribes.is_empty());
        }
        assert_eq!(mode(&stats, SOLO).top_half, Some(4));
        assert_eq!(mode(&stats, DUOS).top_half, Some(2));
    }

    #[test]
    fn solo_and_duos_are_counted_apart() {
        let games = [
            game(SOLO, "ok", "H1", Some(1)),
            game(SOLO, "ok", "H1", Some(5)),
            game(DUOS, "ok", "H1", Some(2)),
            game(DUOS, "ok", "H2", Some(3)),
            game(DUOS, "ok", "H2", Some(4)),
        ];
        let stats = compute(&games);
        let solo = &mode(&stats, SOLO).totals;
        assert_eq!((solo.games, solo.placed), (2, 2));
        assert_eq!(solo.average_place, Some(3.0));
        assert_eq!(solo.top_half_share, Some(0.5)); // 1 is top 4, 5 is not
        assert_eq!(solo.wins, Some(1));
        let duos = &mode(&stats, DUOS).totals;
        assert_eq!((duos.games, duos.placed), (3, 3));
        assert_eq!(duos.average_place, Some(3.0));
        assert_eq!(duos.top_half_share, Some(1.0 / 3.0)); // top 2 in Duos
        assert_eq!(duos.wins, Some(0), "counted games with no win is a real 0");
    }

    #[test]
    fn incomplete_and_unsupported_games_are_counted_apart_not_as_places() {
        let games = [
            game(SOLO, "ok", "H1", Some(2)),
            game(SOLO, "incomplete", "H1", None),
            game(SOLO, "incomplete", "H1", Some(8)), // a place, but unfinished
            json!({"status": "unsupported", "game_type": SOLO, "problems": ["x"]}),
        ];
        let t = mode(&compute(&games), SOLO).totals.clone();
        assert_eq!(
            (t.games, t.placed, t.incomplete, t.unsupported),
            (4, 1, 2, 1)
        );
        assert_eq!(t.average_place, Some(2.0));
        assert_eq!(t.wins, Some(0));
    }

    #[test]
    fn only_incomplete_games_give_no_numbers() {
        let games = [game(DUOS, "incomplete", "H1", None)];
        let t = mode(&compute(&games), DUOS).totals.clone();
        assert_eq!((t.games, t.placed, t.incomplete), (1, 0, 1));
        assert_eq!(
            (t.average_place, t.top_half_share, t.wins),
            (None, None, None)
        );
    }

    #[test]
    fn places_outside_the_mode_range_do_not_count() {
        let games = [
            game(DUOS, "ok", "H1", Some(5)), // Duos teams place 1-4
            game(SOLO, "ok", "H1", Some(0)),
            game(SOLO, "ok", "H1", Some(9)),
            json!({"status": "ok", "game_type": SOLO, "hero": "H1", "final_place": "1"}),
        ];
        let stats = compute(&games);
        assert_eq!(mode(&stats, DUOS).totals.placed, 0);
        assert_eq!(mode(&stats, SOLO).totals.placed, 0);
        assert_eq!(mode(&stats, SOLO).totals.games, 3);
    }

    #[test]
    fn other_game_types_are_listed_without_place_numbers() {
        let games = [game("GT_BATTLEGROUNDS_FRIENDLY", "ok", "H1", Some(1))];
        let stats = compute(&games);
        let other = mode(&stats, OTHER);
        assert_eq!(other.top_half, None);
        assert_eq!((other.totals.games, other.totals.placed), (1, 0));
        assert_eq!(other.totals.wins, None);
        assert_eq!(other.heroes[0].tally.games, 1);
    }

    #[test]
    fn skin_suffix_is_dropped_and_nothing_else() {
        assert_eq!(base_hero("BG20_HERO_202_SKIN_B4"), "BG20_HERO_202");
        assert_eq!(
            base_hero("TB_BaconShop_HERO_102_SKIN_G"),
            "TB_BaconShop_HERO_102"
        );
        assert_eq!(base_hero("BG20_HERO_202"), "BG20_HERO_202");
        // Not a skin suffix we have seen: kept as is, never guessed.
        assert_eq!(base_hero("BG20_HERO_102pe"), "BG20_HERO_102pe");
        assert_eq!(base_hero("BG20_HERO_202_SKIN_"), "BG20_HERO_202_SKIN_");
        assert_eq!(base_hero("_SKIN_A"), "_SKIN_A");
    }

    #[test]
    fn heroes_group_skins_and_sort_by_games() {
        let games = [
            game(DUOS, "ok", "BG20_HERO_202_SKIN_C", Some(1)),
            game(DUOS, "ok", "BG20_HERO_202", Some(3)),
            game(DUOS, "incomplete", "BG20_HERO_202_SKIN_B4", None),
            game(DUOS, "ok", "TB_BaconShop_HERO_37_SKIN_C", Some(2)),
            game(SOLO, "ok", "BG20_HERO_202", Some(1)),
        ];
        let duos = mode(&compute(&games), DUOS).clone();
        let ids: Vec<_> = duos.heroes.iter().map(|h| h.hero.as_deref()).collect();
        assert_eq!(ids, [Some("BG20_HERO_202"), Some("TB_BaconShop_HERO_37")]);
        let nguyen = &duos.heroes[0];
        assert_eq!(
            nguyen.variants,
            [
                "BG20_HERO_202",
                "BG20_HERO_202_SKIN_B4",
                "BG20_HERO_202_SKIN_C"
            ]
        );
        assert_eq!((nguyen.tally.games, nguyen.tally.placed), (3, 2));
        assert_eq!(nguyen.tally.average_place, Some(2.0));
        assert_eq!(nguyen.tally.top_half_share, Some(0.5));
        assert_eq!(nguyen.tally.wins, Some(1));
    }

    #[test]
    fn hero_ties_are_sorted_by_id_so_the_order_is_stable() {
        let games = [
            game(SOLO, "ok", "B", Some(1)),
            game(SOLO, "ok", "A", Some(1)),
        ];
        let solo = mode(&compute(&games), SOLO).clone();
        let ids: Vec<_> = solo
            .heroes
            .iter()
            .map(|h| h.hero.clone().unwrap())
            .collect();
        assert_eq!(ids, ["A", "B"]);
    }

    #[test]
    fn games_without_a_hero_are_one_row_not_hidden() {
        let games = [json!({"status": "ok", "game_type": SOLO, "final_place": 3})];
        let solo = mode(&compute(&games), SOLO).clone();
        assert_eq!(solo.heroes.len(), 1);
        assert_eq!(solo.heroes[0].hero, None);
        assert_eq!(solo.heroes[0].name, None);
        assert_eq!(solo.heroes[0].tally.placed, 1);
    }

    #[test]
    fn hero_names_come_from_any_game_in_the_history() {
        let mut solo_game = game(SOLO, "ok", "BG20_HERO_202_SKIN_C", Some(2));
        solo_game["card_names"] = json!({"BG20_HERO_202_SKIN_C": "Sunwell Nguyen"});
        // The base hero is only named in a Duos game's lobby.
        let mut duos_game = game(DUOS, "ok", "X", Some(1));
        duos_game["card_names"] = json!({"BG20_HERO_202": "Master Nguyen"});
        let stats = compute(&[solo_game, duos_game]);
        let hero = &mode(&stats, SOLO).heroes[0];
        assert_eq!(hero.name.as_deref(), Some("Master Nguyen"));
    }

    #[test]
    fn a_skin_name_is_used_when_the_base_hero_is_never_named() {
        let mut g = game(SOLO, "ok", "BG27_HERO_801_SKIN_C", Some(2));
        g["card_names"] = json!({"BG27_HERO_801_SKIN_C": "Axe Thorim"});
        let stats = compute(&[g, game(SOLO, "ok", "NO_NAME", Some(1))]);
        let solo = mode(&stats, SOLO);
        let named: Vec<_> = solo.heroes.iter().map(|h| h.name.as_deref()).collect();
        assert_eq!(named, [Some("Axe Thorim"), None]);
    }

    #[test]
    fn an_id_printed_with_two_names_takes_the_most_common_one() {
        let named = |n: &str| {
            let mut g = game(SOLO, "ok", "H", Some(1));
            g["card_names"] = json!({ "H": n });
            g
        };
        let stats = compute(&[named("Skin name"), named("Hero"), named("Hero")]);
        assert_eq!(mode(&stats, SOLO).heroes[0].name.as_deref(), Some("Hero"));
        // A tie picks the first in alphabetical order, so it never flips.
        let stats = compute(&[named("B"), named("A")]);
        assert_eq!(mode(&stats, SOLO).heroes[0].name.as_deref(), Some("A"));
    }

    #[test]
    fn stats_carry_the_legend() {
        let stats = compute(&[]);
        assert_eq!(stats.legend.len(), 5);
        assert_eq!(stats.legend[0].source, Source::Log);
    }

    #[test]
    fn counted_numbers_are_from_the_log_and_missing_ones_unknown() {
        let stats = compute(&[game(SOLO, "ok", "H", Some(2))]);
        let solo = mode(&stats, SOLO);
        assert_eq!(solo.totals_sources.games, Source::Log);
        assert_eq!(solo.totals_sources.average_place, Source::Log);
        assert_eq!(solo.totals_sources.wins, Source::Log, "0 wins is a real 0");
        // Duos has no games: every number is unknown, not 0.
        let duos = mode(&stats, DUOS);
        assert_eq!(duos.totals_sources.games, Source::Unknown);
        assert_eq!(duos.totals_sources.average_place, Source::Unknown);
        assert_eq!(duos.totals_sources.wins, Source::Unknown);
    }

    #[test]
    fn modes_without_place_rules_have_unknown_place_numbers() {
        let stats = compute(&[game("GT_BATTLEGROUNDS_FRIENDLY", "ok", "H", Some(1))]);
        let other = mode(&stats, OTHER);
        assert_eq!(other.totals_sources.games, Source::Log);
        assert_eq!(other.totals_sources.average_place, Source::Unknown);
        assert_eq!(
            other.heroes[0].tally_sources.top_half_share,
            Source::Unknown
        );
    }

    #[test]
    fn grouped_skins_are_inferred_and_a_single_id_is_from_the_log() {
        let stats = compute(&[
            game(SOLO, "ok", "BG20_HERO_202", Some(1)),
            game(SOLO, "ok", "BG20_HERO_202_SKIN_C", Some(2)),
            game(SOLO, "ok", "BG21_HERO_1", Some(3)),
        ]);
        let heroes = &mode(&stats, SOLO).heroes;
        let grouped = heroes.iter().find(|h| h.variants.len() == 2).unwrap();
        assert_eq!(grouped.grouping, Source::Inferred);
        let single = heroes.iter().find(|h| h.variants.len() == 1).unwrap();
        assert_eq!(single.grouping, Source::Log);
    }

    #[test]
    fn a_hero_name_is_from_the_log_and_a_missing_one_unknown() {
        let mut named = game(SOLO, "ok", "A", Some(1));
        named["card_names"] = json!({"A": "Alpha"});
        let stats = compute(&[named, game(SOLO, "ok", "B", Some(1))]);
        let heroes = &mode(&stats, SOLO).heroes;
        assert_eq!(heroes[0].name_source, Source::Log);
        assert_eq!(heroes[1].name_source, Source::Unknown);
    }

    #[test]
    fn tribes_are_always_inferred() {
        let stats = compute(&[]);
        assert!(stats
            .modes
            .iter()
            .all(|m| m.tribes_source == Source::Inferred));
    }

    #[test]
    fn tribes_count_games_and_offers_per_mode() {
        let mut a = game(SOLO, "ok", "H", Some(1));
        a["shop_tribes"] = json!({"Beast": 10, "Murloc": 2});
        let mut b = game(SOLO, "incomplete", "H", None);
        b["shop_tribes"] = json!({"Beast": 5, "Naga": 7});
        let c = game(SOLO, "ok", "H", Some(1)); // no tavern data
        let mut d = game(DUOS, "ok", "H", Some(1));
        d["shop_tribes"] = json!({"Dragon": 1});
        let stats = compute(&[a, b, c, d]);
        let solo = mode(&stats, SOLO);
        assert_eq!(solo.games_with_tribes, 2);
        let tribe = |t: &str, g, o| TribeSeen {
            tribe: t.into(),
            games: g,
            offers: o,
        };
        assert_eq!(
            solo.tribes,
            [
                tribe("Beast", 2, 15),
                tribe("Naga", 1, 7),
                tribe("Murloc", 1, 2)
            ]
        );
        assert_eq!(mode(&stats, DUOS).tribes, [tribe("Dragon", 1, 1)]);
    }

    #[test]
    fn huge_offer_counts_saturate_instead_of_overflowing() {
        let mut g = game(SOLO, "ok", "H", Some(1));
        g["shop_tribes"] = json!({ "Beast": u64::MAX });
        let stats = compute(&[g.clone(), g]);
        assert_eq!(mode(&stats, SOLO).tribes[0].offers, u64::MAX);
    }

    #[test]
    fn hand_edited_records_do_not_break_the_stats() {
        let games = [
            json!({}),
            json!({"status": 3, "game_type": [], "hero": {}, "card_names": "x", "shop_tribes": {"Beast": "a", "Naga": -1}}),
            json!("not an object"),
        ];
        let stats = compute(&games);
        let total: usize = stats.modes.iter().map(|m| m.totals.games).sum();
        assert_eq!(total, 3, "every record is still counted as a game");
        assert!(stats.modes.iter().all(|m| m.tribes.is_empty()));
    }
}
