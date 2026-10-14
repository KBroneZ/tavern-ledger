//! The heroes on offer at hero select as the log has them now (T-365): the
//! offer, each reroll's replacement, and nothing once the hero is picked.
//! Live only: the saved report does not change.

use std::fs;
use std::path::Path;

use bg_parser::LogReader;

fn lines(name: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(format!("{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .lines()
        .map(String::from)
        .collect()
}

/// A reader fed every line before the first one holding `needle`.
fn reader_before(name: &str, needle: &str) -> LogReader {
    let all = lines(name);
    let at = all
        .iter()
        .position(|l| l.contains(needle))
        .expect("fixture has the line");
    let mut reader = LogReader::default();
    all[..at].iter().for_each(|l| reader.feed(l));
    reader
}

fn cards(reader: &LogReader) -> Vec<String> {
    reader
        .hero_offer_now()
        .map(|o| o.heroes.into_iter().map(|h| h.card_id).collect())
        .unwrap_or_default()
}

#[test]
fn no_game_no_offer() {
    assert_eq!(LogReader::default().hero_offer_now(), None);
}

#[test]
fn the_four_heroes_on_offer_show_before_any_reroll() {
    let reader = reader_before("solo_data_game", "CHANGE_ENTITY");
    let offer = reader.hero_offer_now().expect("an offer");
    assert_eq!(offer.rerolls, 0);
    assert_eq!(
        cards(&reader),
        [
            "TB_BaconShop_HERO_37",
            "BG20_HERO_202_SKIN_B4",
            "TB_BaconShop_HERO_18",
            "TB_BaconShop_HERO_60_SKIN_A",
        ]
    );
}

#[test]
fn a_reroll_replaces_the_hero_in_its_place() {
    // The game's next power line closes the reroll's block.
    let mut all = lines("solo_data_game");
    let at = all.iter().position(|l| l.contains("SendChoices")).unwrap();
    all.truncate(at);
    all.push(
        "D 12:00:00.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=GameEntity tag=NEXT_STEP value=MAIN_ACTION "
            .into(),
    );
    let mut reader = LogReader::default();
    all.iter().for_each(|l| reader.feed(l));
    let offer = reader.hero_offer_now().expect("an offer");
    assert_eq!(offer.rerolls, 1);
    assert_eq!(
        cards(&reader),
        [
            "TB_BaconShop_HERO_37",
            "BG20_HERO_202_SKIN_B4",
            "TB_BaconShop_HERO_49",
            "TB_BaconShop_HERO_60_SKIN_A",
        ],
        "the rerolled-away hero is gone, the new one is in its place"
    );
}

#[test]
fn once_the_hero_is_picked_there_is_no_offer() {
    let reader = reader_before("solo_data_game", "tag=TURN value=1 ");
    assert_eq!(reader.hero_offer_now(), None);
}

#[test]
fn duos_offers_two_heroes() {
    let reader = reader_before("duo_data_game", "DebugPrintEntitiesChosen");
    assert_eq!(
        cards(&reader),
        ["BG24_HERO_100_SKIN_E", "TB_BaconShop_HERO_37"]
    );
}

#[test]
fn a_game_with_no_hero_choice_has_no_offer() {
    let all = lines("solo_shop_game");
    let mut reader = LogReader::default();
    all.iter().for_each(|l| reader.feed(l));
    assert_eq!(reader.hero_offer_now(), None);
}

/// The user's own games: just before the pick the offer is what the saved
/// report shows without the heroes rerolled away, and after it there is none.
#[test]
fn real_games_offer_what_the_report_says_before_the_pick() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real");
    let mut checked = 0;
    for name in [
        "b253216_solo",
        "b253216_duos",
        "b253216_duos_gifts",
        "b253216_duos_rally",
        "b254477_duos",
    ] {
        let all: Vec<String> = fs::read_to_string(dir.join(format!("{name}.log")))
            .expect("fixture")
            .lines()
            .map(String::from)
            .collect();
        let mut full = LogReader::default();
        all.iter().for_each(|l| full.feed(l));
        assert_eq!(full.hero_offer_now(), None, "{name}: none after the pick");
        let report = full.finish().remove(0);
        let select = report.hero_select.expect("a hero choice");
        let at = all
            .iter()
            .position(|l| l.contains("DebugPrintEntitiesChosen") && l.contains("EntitiesCount="))
            .expect("a pick");
        let mut before = LogReader::default();
        all[..at].iter().for_each(|l| before.feed(l));
        let offer = before.hero_offer_now().expect(name);
        assert_eq!(offer.rerolls, select.rerolls, "{name}");
        assert_eq!(
            offer.heroes.len() + select.rerolls as usize,
            select.offered.len(),
            "{name}: one hero per place, each reroll adds one to the log's list"
        );
        let cards: Vec<_> = offer.heroes.iter().map(|h| &h.card_id).collect();
        assert!(cards.iter().all(|c| select.offered.contains(c)), "{name}");
        assert!(
            select.picked.as_ref().is_some_and(|p| cards.contains(&p)),
            "{name}: the pick was on offer"
        );
        checked += 1;
    }
    assert_eq!(checked, 5);
}
