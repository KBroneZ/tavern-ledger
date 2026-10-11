//! The player's own shop and logged actions (T-204, T-205) on the synthetic
//! logs of tests/bg_log_builder.py (`solo_shop_game`), whose turns, gold and
//! actions are written out in that function.

use std::fs;
use std::path::Path;

use bg_parser::report::GameReport;
use bg_parser::shop::{ActionKind, Offer, PlayerAction, ShopRecord, TierUp};

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

fn shop(name: &str) -> ShopRecord {
    report(&read(name)).shop.expect("a shop record")
}

fn offer(card: &str, roll: u32, frozen: bool) -> Offer {
    Offer {
        card_id: Some(card.into()),
        roll,
        frozen,
    }
}

fn some(card: &str) -> Option<String> {
    Some(card.into())
}

#[test]
fn each_shop_turn_has_its_gold_tier_and_counts() {
    let s = shop("solo_shop_game");
    assert_eq!(s.turns.len(), 2);
    let [one, two] = [&s.turns[0], &s.turns[1]];
    assert_eq!((one.turn, two.turn), (1, 2));
    assert_eq!((one.tier, two.tier), (Some(1), Some(2)));
    assert_eq!((one.gold, two.gold), (Some(3), Some(10)));
    assert_eq!((one.gold_spent, two.gold_spent), (Some(3), Some(6)));
    assert_eq!((one.rolls, one.free_rolls), (0, 0));
    assert_eq!(
        (two.rolls, two.free_rolls),
        (2, 1),
        "the second roll spent no gold"
    );
    assert_eq!(one.buys, [some("BG20_100")]);
    assert!(two.buys.is_empty());
    assert!(one.sells.is_empty());
    assert_eq!(two.sells, [some("BG20_100")]);
    assert_eq!((one.freezes, two.freezes), (1, 0));
    assert_eq!((one.spell_buys, two.spell_buys), (0, 0));
}

#[test]
fn offers_keep_their_roll_and_what_a_freeze_carried_over() {
    let s = shop("solo_shop_game");
    assert_eq!(
        s.turns[0].offers,
        [
            offer("BG20_100", 0, false),
            offer("BG28_300", 0, false),
            offer("BGS_004", 0, false),
        ]
    );
    assert_eq!(
        s.turns[1].offers,
        [
            offer("BG28_300", 0, true),
            offer("BGS_004", 0, true),
            offer("BG21_009", 0, false),
            offer("BG25_011", 1, false),
            offer("BG26_135", 1, false),
            offer("BG31_806", 1, false),
            // The same card twice in one shop is two offers.
            offer("BG20_100", 2, false),
            offer("BG28_300", 2, false),
            offer("BG20_100", 2, false),
        ]
    );
}

#[test]
fn tier_ups_say_on_which_turn() {
    assert_eq!(
        shop("solo_shop_game").tier_ups,
        [TierUp { turn: 2, tier: 2 }],
        "the tier set to 1 at the start is not a tier-up"
    );
}

#[test]
fn actions_are_the_options_and_choices_sent_with_their_time_and_kind() {
    let s = shop("solo_shop_game");
    let action = |ms, turn, kind| PlayerAction { ms, turn, kind };
    use ActionKind::*;
    assert_eq!(
        s.actions,
        [
            action(15_000, 1, Buy),
            action(20_000, 1, HeroPower),
            action(25_000, 1, Freeze),
            action(75_000, 2, Roll),
            action(80_000, 2, Roll),
            action(85_000, 2, TierUp),
            action(88_000, 2, Move),
            action(90_000, 2, Sell),
            action(95_000, 2, Choose),
        ],
        "the hero pick is not an action of the shop"
    );
    assert_eq!((s.turns[0].actions, s.turns[1].actions), (3, 6));
}

#[test]
fn times_run_from_the_game_start_to_its_end() {
    let s = shop("solo_shop_game");
    assert_eq!(s.start_ms, Some(10_000));
    assert_eq!(s.end_ms, Some(150_000));
    assert!(s.ended);
    assert_eq!((s.turns[0].start_ms, s.turns[0].end_ms), (10_000, 70_000));
    assert_eq!((s.turns[1].start_ms, s.turns[1].end_ms), (70_000, 150_000));
}

#[test]
fn a_game_the_log_cuts_short_says_so_and_ends_at_the_last_line() {
    let s = shop("solo_shop_game_cut");
    assert!(!s.ended);
    assert_eq!(s.end_ms, Some(120_000));
    assert_eq!(s.turns.len(), 2);
}

#[test]
fn a_game_that_could_not_be_read_has_no_shop_record() {
    let r = report(&read("missing_entity"));
    assert!(r.shop.is_none());
    let r = report(&read("not_battlegrounds"));
    assert!(r.shop.is_none());
}

#[test]
fn a_turn_without_gold_in_the_log_has_unknown_gold_not_zero() {
    // The older synthetic games never set the player's gold.
    let s = report(&read("solo_game")).shop.expect("a shop record");
    assert_eq!(s.turns[0].gold, None);
    assert_eq!(s.turns[0].gold_spent, None);
    assert!(s.turns[0].offers.is_empty());
}

/// `solo_shop_game` with turn 1 as on 2026-10-11 (T-332): the log sets no
/// gold tag of the player during it and the player only picks a discover;
/// `turn_one` replaces turn 1's three actions.
fn late_gold_game(turn_one: &str) -> String {
    let text = read("solo_shop_game");
    let gold = "D 12:00:10.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=2 tag=RESOURCES value=3 \n";
    let first = "D 12:00:15.0000000 GameState.SendOption()";
    let turn_two = "D 12:00:40.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=GameEntity tag=TURN value=2 ";
    let (from, to) = (text.find(first).unwrap(), text.find(turn_two).unwrap());
    let text = format!("{}{turn_one}{}", &text[..from], &text[to..]);
    assert!(text.contains(gold));
    text.replacen(gold, "", 1)
}

const PICK: &str = "D 12:00:15.0000000 GameState.SendChoices() - id=4 ChoiceType=GENERAL\n";

#[test]
fn a_turn_with_no_gold_tags_spent_nothing_when_the_log_proves_it() {
    let s = report(&late_gold_game(PICK)).shop.expect("a shop record");
    let one = &s.turns[0];
    assert_eq!(one.gold, None, "the turn's own gold stays unknown");
    assert_eq!(one.gold_spent, Some(0));
    assert_eq!(
        (one.extra_gold, one.sell_gold, one.buy_gold),
        (None, None, None)
    );
    assert_eq!(s.turns[1].gold, Some(10));
    assert_eq!(s.actions[0].kind, ActionKind::Choose);
}

#[test]
fn a_turn_with_no_gold_tags_stays_unknown_without_that_proof() {
    // A roll could have spent gold the log does not show.
    let roll = "D 12:00:15.0000000 GameState.SendOption() - selectedOption=1 selectedSubOption=-1 selectedTarget=0 selectedPosition=0\n\
                D 12:00:15.0000000 GameState.DebugPrintPower() - BLOCK_START BlockType=PLAY Entity=60 EffectCardId=X EffectIndex=0 Target=0 SubOption=-1 \n\
                D 12:00:15.0000000 GameState.DebugPrintPower() - BLOCK_END\n";
    let s = report(&late_gold_game(roll)).shop.expect("a shop record");
    assert_eq!(s.turns[0].rolls, 1);
    assert_eq!(s.turns[0].gold_spent, None);
    // The next turn has no gold tags either: nothing proves turn 1.
    let text = late_gold_game(PICK).replace("tag=RESOURCES value=10 ", "tag=ARMOR value=0 ");
    let s = report(&text).shop.expect("a shop record");
    assert_eq!(s.turns[1].gold, None);
    assert_eq!(s.turns[0].gold_spent, None);
    // Gold spent so far moved during the turn.
    let spend = "D 12:00:15.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=2 tag=NUM_RESOURCES_SPENT_THIS_GAME value=1 \n";
    let s = report(&late_gold_game(&format!("{PICK}{spend}")))
        .shop
        .unwrap();
    assert_eq!(s.turns[0].gold_spent, None);
    // The last turn of a log has no next turn to prove it.
    let cut = late_gold_game(PICK);
    let cut = &cut[..cut.find("D 12:00:40.0000000 ").unwrap()];
    let s = report(cut).shop.expect("a shop record");
    assert_eq!(s.turns.len(), 1);
    assert_eq!(s.turns[0].gold_spent, None);
}

#[test]
fn a_game_past_midnight_keeps_counting() {
    let text = read("solo_shop_game")
        .replace("D 12:00:", "D 23:59:")
        .replace("D 12:01:", "D 00:00:")
        .replace("D 12:02:", "D 00:01:");
    let s = report(&text).shop.expect("a shop record");
    assert_eq!(s.end_ms, Some(150_000));
    assert_eq!(s.turns[1].start_ms, 70_000);
}

#[test]
fn reading_the_shop_live_does_not_change_the_final_report() {
    let text = read("solo_shop_game");
    let mut watched = bg_parser::LogReader::default();
    let mut seen = Vec::new();
    for line in text.lines() {
        watched.feed(line);
        seen.extend(watched.snapshot_current().and_then(|r| r.shop));
    }
    assert!(seen.iter().any(|s| s.turns.len() == 1));
    assert_eq!(
        watched.finish(),
        bg_parser::parse_reader(text.as_bytes()).unwrap()
    );
}

#[test]
fn a_second_press_of_the_freeze_button_is_an_unfreeze() {
    let text = read("solo_shop_game");
    let freeze_end =
        "tag=FROZEN value=1 \nD 12:00:25.0000000 GameState.DebugPrintPower() - BLOCK_END\n";
    let at = text.find(freeze_end).expect("the freeze block") + freeze_end.len();
    let p = "D 12:00:30.0000000 GameState.";
    let unfreeze = [
        format!("{p}SendOption() - selectedOption=1 selectedSubOption=-1 selectedTarget=0 selectedPosition=0"),
        format!("{p}DebugPrintPower() - BLOCK_START BlockType=PLAY Entity=62 EffectCardId=X EffectIndex=0 Target=0 SubOption=-1 "),
        format!("{p}DebugPrintPower() -     TAG_CHANGE Entity=31 tag=FROZEN value=0 "),
        format!("{p}DebugPrintPower() - BLOCK_END"),
    ]
    .join("\n");
    let text = format!("{}{unfreeze}\n{}", &text[..at], &text[at..]);
    let s = report(&text).shop.expect("a shop record");
    let kinds: Vec<_> = s.actions.iter().take(4).map(|a| a.kind).collect();
    use ActionKind::*;
    assert_eq!(kinds, [Buy, HeroPower, Freeze, Unfreeze]);
    assert_eq!(s.turns[0].freezes, 1);
}

/// The parser's reports of the synthetic shop games, saved for upload-game's
/// Deno tests (the JSON next to each log is the Python prototype's, without
/// the shop). `BLESS=1 cargo test -p bg-parser --test shop` writes it again.
#[test]
fn the_saved_reports_for_upload_game_are_current() {
    let reports: Vec<GameReport> = ["solo_shop_game", "solo_shop_game_cut"]
        .iter()
        .map(|name| report(&read(name)))
        .collect();
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rust/solo_shop_games.json");
    if std::env::var_os("BLESS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &text).unwrap();
    }
    let saved = fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        saved, text,
        "run BLESS=1 cargo test -p bg-parser --test shop"
    );
}
