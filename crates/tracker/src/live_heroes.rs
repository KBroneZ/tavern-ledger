//! The heroes on offer at hero select, with the player's own numbers for each
//! (T-365): how many games they played it, their average place and the share
//! of those games in the top half, from the local history only. A hero never
//! played has no numbers ("no games yet"), never a zero. A tier and a tribe
//! affinity from the community's games come later (they need data this app
//! does not have); nothing stands in their place.

use bg_parser::hero_select::HeroOffer;
use serde::Serialize;

use crate::live::{LiveGame, Phase};
use crate::recap::HeroRef;
use crate::stats::{base_hero, ModeStats, Stats, DUOS, SOLO};

/// The player's own history with one hero in this mode.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OwnHeroNumbers {
    /// Games played with it (counted or not).
    pub games: usize,
    /// Games with a place that counts: what the two numbers below are over.
    pub placed: usize,
    pub average_place: Option<f64>,
    /// Share of the placed games in the top half (0.0 to 1.0).
    pub top_half_share: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OfferedHeroView {
    pub hero: HeroRef,
    /// None: no game with this hero in the history yet.
    pub own: Option<OwnHeroNumbers>,
}

/// The hero select panel's data.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LiveHeroOffer {
    pub heroes: Vec<OfferedHeroView>,
    pub rerolls: u32,
}

fn own_numbers(mode: Option<&ModeStats>, card: &str) -> Option<OwnHeroNumbers> {
    let base = base_hero(card);
    let hero = mode?
        .heroes
        .iter()
        .find(|h| h.hero.as_deref() == Some(base) || h.variants.iter().any(|v| v == card))?;
    (hero.tally.games > 0).then_some(OwnHeroNumbers {
        games: hero.tally.games,
        placed: hero.tally.placed,
        average_place: hero.tally.average_place,
        top_half_share: hero.tally.top_half_share,
    })
}

impl LiveGame {
    /// Notes the heroes the log says are on offer (from the watcher); the
    /// panel's data is made from them by [`LiveGame::with_hero_offer`].
    pub fn with_offered_heroes(mut self, offer: Option<HeroOffer>) -> Self {
        self.offered_heroes = offer;
        self
    }

    /// Makes the heroes on offer the panel shows, with the player's own
    /// numbers for each from `stats` (the history of this PC), while the game
    /// is at hero select.
    pub fn with_hero_offer(mut self, stats: &Stats) -> Self {
        let offer = self
            .offered_heroes
            .as_ref()
            .filter(|_| self.phase == Phase::Starting);
        let wanted = if self.is_duos { DUOS } else { SOLO };
        let mode = stats.modes.iter().find(|m| m.mode == wanted);
        self.hero_offer = offer.map(|offer| LiveHeroOffer {
            heroes: offer
                .heroes
                .iter()
                .map(|h| OfferedHeroView {
                    hero: HeroRef {
                        id: h.card_id.clone(),
                        name: h.name.clone(),
                    },
                    own: own_numbers(mode, &h.card_id),
                })
                .collect(),
            rerolls: offer.rerolls,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::live_game;
    use crate::stats::compute;
    use bg_parser::hero_select::OfferedHero;
    use serde_json::{json, Value};

    const DUOS_TYPE: &str = "GT_BATTLEGROUNDS_DUO";

    fn played(mode: &str, hero: &str, place: i64) -> Value {
        json!({
            "status": "ok", "game_type": mode, "local_player_id": 1,
            "hero": hero, "final_place": place, "rounds": [], "lobby": [],
        })
    }

    fn offer(cards: &[&str]) -> HeroOffer {
        HeroOffer {
            heroes: cards
                .iter()
                .map(|c| OfferedHero {
                    card_id: c.to_string(),
                    name: Some(format!("Name of {c}")),
                })
                .collect(),
            rerolls: 1,
        }
    }

    fn starting(game_type: &str) -> LiveGame {
        live_game(&json!({"game_type": game_type, "problems": ["unexpected lobby size: 0"]}))
    }

    #[test]
    fn each_offered_hero_has_the_players_own_numbers_or_none() {
        let history = [
            played(SOLO, "H_A", 2),
            played(SOLO, "H_A", 6),
            played(SOLO, "H_A_SKIN_B", 1),
            played(DUOS, "H_B", 1),
        ];
        let stats = compute(&history);
        let game = starting(SOLO)
            .with_offered_heroes(Some(offer(&["H_A", "H_B", "H_C"])))
            .with_hero_offer(&stats);
        let shown = game.hero_offer.expect("an offer");
        assert_eq!(shown.rerolls, 1);
        let ids: Vec<_> = shown.heroes.iter().map(|h| h.hero.id.as_str()).collect();
        assert_eq!(ids, ["H_A", "H_B", "H_C"]);
        // The skin counts under its hero.
        let a = shown.heroes[0].own.as_ref().expect("played");
        assert_eq!((a.games, a.placed), (3, 3));
        assert_eq!(a.average_place, Some(3.0));
        assert_eq!(a.top_half_share, Some(2.0 / 3.0));
        // Played only in Duos: no games yet in Solo. Never played: none.
        assert_eq!(shown.heroes[1].own, None);
        assert_eq!(shown.heroes[2].own, None);
    }

    #[test]
    fn duos_looks_at_the_duos_history() {
        let stats = compute(&[played(SOLO, "H_A", 1), played(DUOS, "H_B", 2)]);
        let game = starting(DUOS_TYPE)
            .with_offered_heroes(Some(offer(&["H_A", "H_B"])))
            .with_hero_offer(&stats);
        let shown = game.hero_offer.unwrap();
        assert_eq!(shown.heroes[0].own, None);
        let b = shown.heroes[1].own.as_ref().unwrap();
        assert_eq!(b.top_half_share, Some(1.0));
    }

    #[test]
    fn nothing_once_the_game_is_on_or_when_there_is_no_offer() {
        let stats = compute(&[played(SOLO, "H_A", 1)]);
        let mut playing = starting(SOLO);
        playing.phase = Phase::Playing;
        let shown = playing
            .with_offered_heroes(Some(offer(&["H_A"])))
            .with_hero_offer(&stats);
        assert_eq!(shown.hero_offer, None);
        let none = starting(SOLO)
            .with_offered_heroes(None)
            .with_hero_offer(&stats);
        assert_eq!(none.hero_offer, None);
    }
}
