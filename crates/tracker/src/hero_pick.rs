//! The hero pick (T-209): the heroes the game offered the player, the
//! rerolls used and the hero picked, for the recap, and per hero over the
//! history (times offered, times picked, pick rate, average place when
//! picked) for the stats. The window only renders them.
//!
//! Only the player's own offers: the log has no one else's (D-004). Unknown
//! is not zero: a game saved before parser revision 4 says "not recorded",
//! a game whose log has no hero choice or no pick says "no data", and
//! neither is counted.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::provenance::Source;
use crate::recap::{text, HeroRef, View};
use crate::stats::Names;

/// The parser never writes more heroes than this for one pick.
const MAX_OFFERED: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PickState {
    Recorded,
    /// Saved by a parser older than revision 4: re-read with `--reparse`.
    NotRecorded,
    /// The log has no hero choice for this game, or the record is malformed.
    NoData,
}

/// The hero pick of one game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeroPick {
    pub state: PickState,
    pub source: Source,
    /// Every hero shown: the first offer, then each reroll's hero.
    pub offered: Vec<HeroRef>,
    pub rerolls: Option<u32>,
    /// None when the log does not say which hero was picked.
    pub picked: Option<HeroRef>,
}

/// What a saved report's `hero_select` holds: the heroes offered, the
/// rerolls and the pick; Err when there is none to read.
fn read(report: &Value) -> Result<(Vec<String>, u32, Option<String>), PickState> {
    let select = match report.get("hero_select") {
        None => return Err(PickState::NotRecorded),
        Some(Value::Null) => return Err(PickState::NoData),
        Some(select) => select,
    };
    let offered = select
        .get("offered")
        .and_then(Value::as_array)
        .ok_or(PickState::NoData)?;
    let offered: Vec<String> = offered
        .iter()
        .filter_map(Value::as_str)
        .take(MAX_OFFERED)
        .map(String::from)
        .collect();
    let rerolls = select
        .get("rerolls")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(PickState::NoData)?;
    // A pick that is not among the offers is not one the parser wrote.
    let picked = text(select, "picked").filter(|p| offered.contains(p));
    Ok((offered, rerolls, picked))
}

/// The hero pick part of a game's recap.
pub(crate) fn hero_pick(view: &View, report: &Value) -> HeroPick {
    match read(report) {
        Ok((offered, rerolls, picked)) => HeroPick {
            state: PickState::Recorded,
            source: Source::Log,
            offered: offered.iter().map(|id| view.hero_ref(id)).collect(),
            rerolls: Some(rerolls),
            picked: picked.map(|id| view.hero_ref(&id)),
        },
        Err(state) => HeroPick {
            state,
            source: Source::Unknown,
            offered: Vec::new(),
            rerolls: None,
            picked: None,
        },
    }
}

/// One hero over the games whose pick the log has.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HeroPickStats {
    /// The base hero (skins grouped as in the per-hero stats).
    pub hero: String,
    pub name: Option<String>,
    /// Every card id offered under this hero.
    pub variants: Vec<String>,
    /// Games where the game offered it (once per game).
    pub offered: usize,
    /// Games where it was picked.
    pub picked: usize,
    /// Picked over offered (0.0 to 1.0).
    pub pick_rate: Option<f64>,
    /// Picked games with a place that counts: the denominator of the average.
    pub placed_when_picked: usize,
    pub average_place_when_picked: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct HeroPicks {
    /// Games counted: the pick is in the log.
    pub games: usize,
    /// Saved by an older parser (re-read with `--reparse`).
    pub not_recorded: usize,
    /// The log has no hero choice or no pick for these games.
    pub no_data: usize,
    /// Offered in most games first.
    pub heroes: Vec<HeroPickStats>,
    pub source: Source,
}

#[derive(Default)]
struct HeroCounts {
    variants: BTreeSet<String>,
    offered: usize,
    picked: usize,
    places: Vec<i64>,
}

/// Sums for [`HeroPicks`], filled game by game.
#[derive(Default)]
pub(crate) struct PickCounts {
    stats: HeroPicks,
    heroes: BTreeMap<String, HeroCounts>,
}

impl PickCounts {
    /// `place` is the game's place when it counts; `base` puts a card id
    /// under its hero.
    pub(crate) fn add(
        &mut self,
        report: &Value,
        place: Option<i64>,
        base: &dyn Fn(&str) -> String,
    ) {
        let (offered, picked) = match read(report) {
            Ok((offered, _, Some(picked))) => (offered, picked),
            // A pick the log does not have would make every offer look passed over.
            Ok(_) | Err(PickState::NoData) => {
                self.stats.no_data += 1;
                return;
            }
            Err(_) => {
                self.stats.not_recorded += 1;
                return;
            }
        };
        self.stats.games += 1;
        let mut seen = BTreeSet::new();
        for card in &offered {
            let hero = base(card);
            let counts = self.heroes.entry(hero.clone()).or_default();
            counts.variants.insert(card.clone());
            // A hero offered twice in one game (a reroll back to it) counts once.
            if seen.insert(hero) {
                counts.offered += 1;
            }
        }
        let counts = self.heroes.entry(base(&picked)).or_default();
        counts.picked += 1;
        counts.places.extend(place);
    }

    pub(crate) fn finish(self, names: &Names) -> HeroPicks {
        let mut heroes: Vec<HeroPickStats> = self
            .heroes
            .into_iter()
            .map(|(hero, c)| {
                let placed = c.places.len();
                HeroPickStats {
                    name: names.for_hero(&hero, &c.variants),
                    variants: c.variants.into_iter().collect(),
                    pick_rate: (c.offered > 0).then(|| c.picked as f64 / c.offered as f64),
                    average_place_when_picked: (placed > 0)
                        .then(|| c.places.iter().sum::<i64>() as f64 / placed as f64),
                    placed_when_picked: placed,
                    offered: c.offered,
                    picked: c.picked,
                    hero,
                }
            })
            .collect();
        heroes.sort_by(|a, b| {
            b.offered
                .cmp(&a.offered)
                .then(b.picked.cmp(&a.picked))
                .then_with(|| a.hero.cmp(&b.hero))
        });
        HeroPicks {
            source: Source::log_if(self.stats.games > 0),
            heroes,
            ..self.stats
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recap::recap;
    use crate::stats::{compute, SOLO};
    use serde_json::json;

    fn game(offered: &[&str], rerolls: u32, picked: Option<&str>, place: i64) -> Value {
        json!({
            "status": "ok", "game_type": SOLO, "local_player_id": 1,
            "hero": picked, "final_place": place,
            "card_names": {"H_A": "Hero A"},
            "hero_select": {"offered": offered, "rerolls": rerolls, "picked": picked},
            "rounds": [], "lobby": [],
        })
    }

    #[test]
    fn the_recap_shows_the_offers_the_rerolls_and_the_pick() {
        let r = recap(&game(
            &["H_A", "H_B", "H_C", "H_D", "H_E"],
            1,
            Some("H_A"),
            3,
        ));
        let pick = r.hero_pick;
        assert_eq!(
            (pick.state, pick.source),
            (PickState::Recorded, Source::Log)
        );
        assert_eq!(pick.offered.len(), 5);
        assert_eq!(pick.rerolls, Some(1));
        let picked = pick.picked.unwrap();
        assert_eq!(
            (picked.id.as_str(), picked.name.as_deref()),
            ("H_A", Some("Hero A"))
        );
    }

    #[test]
    fn an_older_record_says_not_recorded_and_a_missing_choice_no_data() {
        let mut old = game(&["H_A"], 0, Some("H_A"), 1);
        old.as_object_mut().unwrap().remove("hero_select");
        let r = recap(&old).hero_pick;
        assert_eq!(
            (r.state, r.source),
            (PickState::NotRecorded, Source::Unknown)
        );
        assert!(r.offered.is_empty() && r.rerolls.is_none());
        let mut none = game(&["H_A"], 0, Some("H_A"), 1);
        none["hero_select"] = Value::Null;
        assert_eq!(recap(&none).hero_pick.state, PickState::NoData);
        // Hand-edited: a pick that was never offered is not shown as a pick.
        let odd = game(&["H_A"], 0, Some("H_Z"), 1);
        assert_eq!(recap(&odd).hero_pick.picked, None);
    }

    fn picks(stats: &crate::stats::Stats) -> &HeroPicks {
        &stats.modes[0].hero_picks
    }

    #[test]
    fn stats_count_offers_picks_and_the_place_when_picked() {
        let games = [
            game(&["H_A", "H_B", "H_C", "H_D"], 0, Some("H_A"), 2),
            game(&["H_A", "H_B", "H_E", "H_F"], 0, Some("H_B"), 5),
            game(&["H_A", "H_G", "H_H", "H_I"], 0, Some("H_A"), 4),
        ];
        let stats = compute(games.iter());
        let p = picks(&stats);
        assert_eq!((p.games, p.not_recorded, p.no_data), (3, 0, 0));
        let a = &p.heroes[0];
        assert_eq!(a.hero, "H_A");
        assert_eq!((a.offered, a.picked), (3, 2));
        assert_eq!(a.pick_rate, Some(2.0 / 3.0));
        assert_eq!(
            (a.placed_when_picked, a.average_place_when_picked),
            (2, Some(3.0))
        );
        let c = p.heroes.iter().find(|h| h.hero == "H_C").unwrap();
        assert_eq!((c.offered, c.picked, c.pick_rate), (1, 0, Some(0.0)));
        assert_eq!(c.average_place_when_picked, None);
        assert_eq!(p.source, Source::Log);
    }

    #[test]
    fn skins_of_one_hero_are_one_row_and_count_once_per_game() {
        let games = [game(
            &["H_A", "H_A_SKIN_B", "H_C", "H_D"],
            1,
            Some("H_A_SKIN_B"),
            1,
        )];
        let stats = compute(games.iter());
        let a = &picks(&stats).heroes[0];
        assert_eq!(a.hero, "H_A");
        assert_eq!(a.variants, ["H_A", "H_A_SKIN_B"]);
        assert_eq!((a.offered, a.picked), (1, 1));
    }

    #[test]
    fn games_without_a_pick_are_counted_apart_not_as_passed_over() {
        let mut old = game(&["H_A"], 0, Some("H_A"), 1);
        old.as_object_mut().unwrap().remove("hero_select");
        let no_pick = game(&["H_A", "H_B"], 0, None, 1);
        let stats = compute([old, no_pick].iter());
        let p = picks(&stats);
        assert_eq!((p.games, p.not_recorded, p.no_data), (0, 1, 1));
        assert!(p.heroes.is_empty());
        assert_eq!(p.source, Source::Unknown);
    }

    #[test]
    fn an_unfinished_game_counts_its_pick_but_not_a_place() {
        let mut g = game(&["H_A", "H_B"], 0, Some("H_A"), 3);
        g["status"] = json!("incomplete");
        let stats = compute([g].iter());
        let a = &picks(&stats).heroes[0];
        assert_eq!(
            (a.picked, a.placed_when_picked, a.average_place_when_picked),
            (1, 0, None)
        );
    }
}
