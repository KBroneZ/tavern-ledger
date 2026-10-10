//! Parser revision 4 (T-206, T-209, T-210, T-214, D-055) on the synthetic
//! logs of tests/bg_log_builder.py (`solo_data_game`, `duo_data_game`), whose
//! combats, hero pick and shop are written out in those functions.

use std::fs;
use std::path::Path;

use bg_parser::report::{CombatResult, GameReport};
use bg_parser::shop::{ActionKind, ShopTurn};

fn read(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(format!("{name}.log"));
    // A Windows checkout may turn the fixture's LF into CRLF.
    fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n")
}

fn report(text: &str) -> GameReport {
    let mut reports = bg_parser::parse_reader(text.as_bytes()).expect("in-memory read");
    assert_eq!(reports.len(), 1);
    reports.remove(0)
}

fn results(r: &GameReport) -> Vec<Option<CombatResult>> {
    r.rounds.iter().map(|round| round.result).collect()
}

#[test]
fn combat_results_are_read_as_the_game_records_them() {
    use CombatResult::*;
    let r = report(&read("solo_data_game"));
    // Round 4: the hero lost health in the shop and won the combat, which the
    // health rule could not tell; the game's own record can.
    assert_eq!(results(&r), [Won, Lost, Tie, Won].map(Some));
    // In Duos the game counts a team's loss once per hero (2 + 2): still a loss.
    let d = report(&read("duo_data_game"));
    assert_eq!(results(&d), [Some(Lost)]);
}

#[test]
fn a_log_without_the_result_tags_has_no_read_results() {
    // An older log, or one a patch changed: every combat would look like a tie.
    let text: String = read("solo_data_game")
        .lines()
        .filter(|l| !l.contains("BACON_WON_LAST_COMBAT") && !l.contains("DAMAGE_DEALT_TO_HERO"))
        .map(|l| format!("{l}\n"))
        .collect();
    let r = report(&text);
    assert_eq!(r.rounds.len(), 4);
    assert!(results(&r).iter().all(Option::is_none));
}

#[test]
fn a_combat_cut_short_by_the_log_has_no_result() {
    let text = read("solo_data_game");
    // Stop in the middle of round 3 (the tie), before the next shop starts.
    let cut = text
        .find("TAG_CHANGE Entity=GameEntity tag=TURN value=7")
        .expect("turn 7");
    let start = text[..cut].rfind('\n').unwrap() + 1;
    let r = report(&text[..start]);
    use CombatResult::*;
    assert_eq!(results(&r), [Some(Won), Some(Lost), None]);
}

#[test]
fn result_tags_that_disagree_are_unknown() {
    // Won and hurt in the same combat: not a state the game gives.
    let text = read("solo_data_game").replacen(
        "tag=BACON_WON_LAST_COMBAT value=1 ",
        "tag=BACON_WON_LAST_COMBAT value=1 \nD 12:00:40.0000000 GameState.DebugPrintPower() -     \
         TAG_CHANGE Entity=2 tag=DAMAGE_DEALT_TO_HERO_LAST_TURN value=3 ",
        1,
    );
    let r = report(&text);
    assert_eq!(r.rounds[0].result, None);
}

#[test]
fn the_hero_pick_has_the_offers_the_reroll_and_the_pick() {
    let r = report(&read("solo_data_game"));
    let pick = r.hero_select.expect("a hero pick");
    assert_eq!(
        pick.offered,
        [
            "TB_BaconShop_HERO_37",
            "BG20_HERO_202_SKIN_B4",
            "TB_BaconShop_HERO_18",
            "TB_BaconShop_HERO_60_SKIN_A",
            "TB_BaconShop_HERO_49",
        ]
    );
    assert_eq!(pick.rerolls, 1);
    assert_eq!(pick.picked.as_deref(), Some("BG20_HERO_202_SKIN_B4"));
    assert_eq!(pick.picked, r.hero);
    let d = report(&read("duo_data_game")).hero_select.unwrap();
    assert_eq!((d.offered.len(), d.rerolls), (2, 0));
}

#[test]
fn a_game_without_a_hero_choice_in_the_log_has_no_hero_pick() {
    let r = report(&read("solo_shop_game"));
    assert_eq!(r.hero_select, None);
    assert!(r.skin_parents.is_empty());
}

#[test]
fn skins_carry_their_base_hero_link_and_base_heroes_none() {
    let r = report(&read("solo_data_game"));
    let links: Vec<_> = r
        .skin_parents
        .iter()
        .map(|(card, id)| (card.as_str(), *id))
        .collect();
    assert_eq!(
        links,
        [
            ("BG20_HERO_202_SKIN_B4", 60011),
            ("TB_BaconShop_HERO_102_SKIN_G", 59999),
            ("TB_BaconShop_HERO_60_SKIN_A", 58000),
        ]
    );
    let d = report(&read("duo_data_game"));
    assert_eq!(d.skin_parents.len(), 2, "own and teammate skins");
}

fn turns(name: &str) -> Vec<ShopTurn> {
    report(&read(name)).shop.expect("a shop record").turns
}

#[test]
fn gold_beyond_the_turns_own_is_counted_by_where_it_came_from() {
    let t = turns("solo_data_game");
    let gold = |f: fn(&ShopTurn) -> Option<u32>| t.iter().map(f).collect::<Vec<_>>();
    // Turn 1: a coin (1) and a trigger (2). Turn 2: a hero power that takes
    // the gold and gives it back is not extra gold.
    assert_eq!(gold(|t| t.extra_gold), [3, 0, 0, 0].map(Some));
    assert_eq!(gold(|t| t.sell_gold), [0, 2, 0, 0].map(Some));
    assert_eq!(
        gold(|t| t.buy_gold),
        [2, 0, 0, 0].map(Some),
        "override cost 2"
    );
    assert_eq!(gold(|t| t.spell_gold), [0, 0, 1, 0].map(Some));
    // The turn's own gold is never extra.
    assert_eq!(
        t.iter().map(|t| t.gold).collect::<Vec<_>>(),
        [3, 4, 5, 6].map(Some)
    );
}

#[test]
fn a_roll_that_uses_a_free_roll_of_the_button_is_counted() {
    let t = turns("solo_data_game");
    assert_eq!((t[1].rolls, t[1].free_rolls), (2, 1));
    assert_eq!(t[1].free_refreshes, Some(1));
    assert!(t
        .iter()
        .filter(|t| t.turn != 2)
        .all(|t| t.free_refreshes == Some(0)));
}

#[test]
fn a_card_passed_to_the_teammate_is_a_pass() {
    let r = report(&read("duo_data_game"));
    let shop = r.shop.unwrap();
    assert_eq!(
        shop.turns[0].passes,
        Some(vec![Some("BG20_100".to_string())])
    );
    let kinds: Vec<_> = shop.actions.iter().map(|a| a.kind).collect();
    assert_eq!(kinds, [ActionKind::Pass]);
}

#[test]
fn a_deck_action_of_a_card_not_the_players_is_not_a_pass() {
    let text = read("duo_data_game").replace(
        "BlockType=DECK_ACTION Entity=30",
        "BlockType=DECK_ACTION Entity=11",
    );
    let shop = report(&text).shop.unwrap();
    assert_eq!(shop.turns[0].passes, Some(Vec::new()));
    let kinds: Vec<_> = shop.actions.iter().map(|a| a.kind).collect();
    assert_eq!(kinds, [ActionKind::Other]);
}

#[test]
fn free_rolls_are_unknown_when_the_log_never_gives_the_buttons_count() {
    // The Duos game has no roll button count at all: unknown, not "none used".
    let t = turns("duo_data_game");
    assert!(t.iter().all(|t| t.free_refreshes.is_none()));
    // The rolls themselves are still counted.
    assert!(t.iter().all(|t| t.rolls == 0));
}

const POWER: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() -     ";

fn skin_links(text: &str) -> Vec<(String, i64)> {
    report(text).skin_parents.into_iter().collect()
}

#[test]
fn a_card_given_two_different_links_has_none() {
    let reroll = "CHANGE_ENTITY - Updating Entity=103";
    let text = read("solo_data_game").replace(
        &format!("{POWER}{reroll}"),
        &format!(
            "{POWER}TAG_CHANGE Entity=102 tag=BACON_SKIN_PARENT_ID value=12345 \n{POWER}{reroll}"
        ),
    );
    let links = skin_links(&text);
    assert!(
        links
            .iter()
            .all(|(card, _)| card != "BG20_HERO_202_SKIN_B4"),
        "{links:?}"
    );
    assert_eq!(links.len(), 2);
}

#[test]
fn a_hero_changed_to_another_card_does_not_keep_the_old_cards_link() {
    // 104 is a skin linked to 58000; the reroll turns it into a base hero
    // whose packet has no link.
    let text = read("solo_data_game").replace(
        "CHANGE_ENTITY - Updating Entity=103 CardID=TB_BaconShop_HERO_49",
        "CHANGE_ENTITY - Updating Entity=104 CardID=TB_BaconShop_HERO_49",
    );
    let r = report(&text);
    assert!(r
        .hero_select
        .unwrap()
        .offered
        .contains(&"TB_BaconShop_HERO_49".to_string()));
    assert!(!r.skin_parents.contains_key("TB_BaconShop_HERO_49"));
    assert_eq!(
        r.skin_parents.get("TB_BaconShop_HERO_60_SKIN_A"),
        Some(&58000)
    );
    // A reroll into a skin brings that skin's own link.
    let text = read("solo_data_game").replace(
        "tag=BACON_NUM_MULLIGAN_REFRESH_USED value=1",
        &format!("tag=BACON_NUM_MULLIGAN_REFRESH_USED value=1\n{POWER}    tag=BACON_SKIN_PARENT_ID value=77"),
    );
    assert_eq!(
        report(&text).skin_parents.get("TB_BaconShop_HERO_49"),
        Some(&77)
    );
}

#[test]
fn a_link_of_zero_or_out_of_range_is_ignored() {
    let reroll = "CHANGE_ENTITY - Updating Entity=103";
    let text = read("solo_data_game").replace(
        &format!("{POWER}{reroll}"),
        &format!(
            "{POWER}TAG_CHANGE Entity=21 tag=BACON_SKIN_PARENT_ID value=0 \n\
             {POWER}TAG_CHANGE Entity=104 tag=BACON_SKIN_PARENT_ID value=99999999 \n{POWER}{reroll}"
        ),
    );
    assert_eq!(skin_links(&text), skin_links(&read("solo_data_game")));
}

#[test]
fn a_turn_without_gold_in_the_log_has_unknown_gold_numbers_not_zero() {
    let text: String = read("solo_data_game")
        .lines()
        .filter(|l| !l.contains("tag=RESOURCES value=3"))
        .map(|l| format!("{l}\n"))
        .collect();
    let t = turns_of(&text);
    assert_eq!(t[0].gold, None);
    assert_eq!((t[0].extra_gold, t[0].sell_gold), (None, None));
    assert_eq!((t[0].buy_gold, t[0].spell_gold), (None, None));
    // Counts do not need the gold.
    assert_eq!(t[0].free_refreshes, Some(0));
}

fn turns_of(text: &str) -> Vec<ShopTurn> {
    report(text).shop.expect("a shop record").turns
}

/// The parser's reports of the synthetic data games, saved for upload-game's
/// Deno tests (the JSON next to each log is the Python prototype's, without
/// the new fields). `BLESS=1 cargo test -p bg-parser --test data_round`
/// writes it again.
#[test]
fn the_saved_reports_for_upload_game_are_current() {
    let reports: Vec<GameReport> = ["solo_data_game", "duo_data_game"]
        .iter()
        .map(|name| report(&read(name)))
        .collect();
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rust/data_round_games.json");
    if std::env::var_os("BLESS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &text).unwrap();
    }
    let saved = fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        saved, text,
        "run BLESS=1 cargo test -p bg-parser --test data_round"
    );
}
