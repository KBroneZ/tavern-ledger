//! Parser revision 5 (T-207, T-208, T-211, T-212, D-058) on the synthetic
//! logs of tests/bg_log_builder.py (`solo_round_two_game`,
//! `duo_round_two_game`), whose leaderboard, boards and choices are written
//! out in those functions.

use std::fs;
use std::path::Path;

use bg_parser::discover::Discover;
use bg_parser::leaderboard::TrinketSlot;
use bg_parser::report::{GameReport, LobbyPlayer, Side};

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

fn seat(r: &GameReport, pid: i64) -> &LobbyPlayer {
    r.lobby.iter().find(|p| p.player_id == pid).expect("seat")
}

fn cards(d: &Discover) -> Vec<(Option<&str>, Option<&str>)> {
    d.offered
        .iter()
        .map(|o| (o.card_id.as_deref(), o.gift.as_deref()))
        .collect()
}

#[test]
fn the_leaderboard_triples_and_trinkets_are_kept_per_hero_and_turn() {
    let r = report(&read("solo_round_two_game"));
    let opp = seat(&r, 3);
    let triples: Vec<_> = opp.triples.iter().map(|t| (t.turn, t.count)).collect();
    // The combat copy's 5 is not the leaderboard's.
    assert_eq!(triples, [(2, 1), (3, 2)]);
    let trinkets: Vec<_> = opp
        .trinkets
        .iter()
        .map(|t| (t.turn, t.slot, t.dbf_id))
        .collect();
    assert_eq!(
        trinkets,
        [
            (2, TrinketSlot::First, 111001),
            (3, TrinketSlot::First, 111002)
        ]
    );
    let me = seat(&r, 2);
    assert!(
        me.triples.is_empty(),
        "no triple: empty, the log has the hero"
    );
    assert_eq!(me.trinkets.len(), 1);
    assert_eq!(me.trinkets[0].dbf_id, 112002);

    let d = report(&read("duo_round_two_game"));
    let mate = seat(&d, 1);
    assert_eq!((mate.triples.len(), mate.trinkets.len()), (1, 1));
    assert_eq!(seat(&d, 4).trinkets[0].slot, TrinketSlot::Second);
    assert!(seat(&d, 3).trinkets.is_empty());
}

#[test]
fn board_keywords_are_the_tags_the_minion_entered_combat_with() {
    let r = report(&read("solo_round_two_game"));
    let opponent = r.rounds[0]
        .entries
        .iter()
        .find(|e| e.side == Side::Opponent);
    let board = opponent.and_then(|e| e.board.clone()).expect("a board");
    let keywords: Vec<_> = board.iter().map(|m| m.keywords.clone()).collect();
    assert_eq!(keywords, [vec!["taunt", "divine_shield", "rally"], vec![]]);
    let d = report(&read("duo_round_two_game"));
    let own = d.rounds[0].entries[0]
        .board
        .clone()
        .expect("the teammate's board");
    assert_eq!(
        own[0].keywords,
        ["reborn", "venomous", "windfury", "stealth", "deathrattle"]
    );
}

#[test]
fn choices_keep_their_source_offers_gifts_and_pick() {
    let r = report(&read("solo_round_two_game"));
    assert_eq!(r.discovers.len(), 3);
    let trinket = &r.discovers[0];
    assert_eq!(
        (trinket.turn, trinket.source.as_deref()),
        (2, Some("BG30_Trinket_1st"))
    );
    assert_eq!(trinket.offered.len(), 4);
    assert_eq!(trinket.picked, [1]);
    let gift = &r.discovers[1];
    assert_eq!(gift.source.as_deref(), Some("BG36_MidGameEffect_010"));
    assert_eq!(
        cards(gift),
        [
            (Some("BG36_100"), Some("BG36_MidGameEffect_000t1")),
            (Some("BG36_101"), Some("BG36_MidGameEffect_000t2")),
            (Some("BG36_102"), Some("BG36_MidGameEffect_000t3"))
        ]
    );
    assert_eq!(gift.picked, [2]);
    // The game ended before this one was answered: no pick, never a guess.
    let open = &r.discovers[2];
    assert_eq!(open.source.as_deref(), Some("TB_BaconShop_Triples_01"));
    assert!(open.picked.is_empty());
    assert_eq!(r.old_god, Some(130610));

    let d = report(&read("duo_round_two_game"));
    assert_eq!(d.discovers.len(), 1);
    assert_eq!(d.discovers[0].turn, 1);
    assert_eq!(d.discovers[0].picked, [0]);
    assert_eq!(d.old_god, None, "no Old God in the log: unknown");
}

#[test]
fn older_logs_without_the_new_lines_give_empty_lists_not_guesses() {
    let r = report(&read("solo_data_game"));
    assert!(r.discovers.is_empty());
    assert_eq!(r.old_god, None);
    assert!(r
        .lobby
        .iter()
        .all(|p| p.triples.is_empty() && p.trinkets.is_empty()));
}

/// The parser's reports of the synthetic round-two games, saved for
/// upload-game's Deno tests. `BLESS=1 cargo test -p bg-parser --test
/// round_two` writes it again.
#[test]
fn the_saved_reports_for_upload_game_are_current() {
    let reports: Vec<GameReport> = ["solo_round_two_game", "duo_round_two_game"]
        .iter()
        .map(|name| report(&read(name)))
        .collect();
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rust/round_two_games.json");
    if std::env::var_os("BLESS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &text).unwrap();
    }
    let saved = fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        saved, text,
        "run BLESS=1 cargo test -p bg-parser --test round_two"
    );
}

#[test]
fn a_hero_packet_with_a_count_set_keeps_it_and_strange_values_are_left_out() {
    let text = read("solo_round_two_game");
    // The opponent's leaderboard hero arrives with a triple already counted.
    let created = "tag=PLAYER_LEADERBOARD_PLACE value=2\n";
    let at = text.find(created).expect("the opponent's hero packet") + created.len();
    let line = text[..at].lines().last().unwrap();
    let prefix = &line[..line.find("tag=").unwrap()];
    let mut text = format!(
        "{}{prefix}tag=PLAYER_TRIPLES value=1\n{}",
        &text[..at],
        &text[at..]
    );
    // A value that is not a number, and a turn past any game's.
    text = text.replace(
        "TAG_CHANGE Entity=21 tag=PLAYER_TRIPLES value=2",
        "TAG_CHANGE Entity=21 tag=PLAYER_TRIPLES value=MANY",
    );
    let r = report(&text);
    let triples: Vec<_> = seat(&r, 3)
        .triples
        .iter()
        .map(|t| (t.turn, t.count))
        .collect();
    assert_eq!(
        triples,
        [(0, 1)],
        "set at the start, then nothing new or readable"
    );

    let far = read("solo_round_two_game").replace(
        "TAG_CHANGE Entity=GameEntity tag=TURN value=5 ",
        "TAG_CHANGE Entity=GameEntity tag=TURN value=205 ",
    );
    let r = report(&far);
    assert_eq!(
        r.problems,
        ["parser error: UnsupportedTurn"],
        "turn 103 is past any game's: the game is not read"
    );
}

#[test]
fn a_hero_packet_with_trinkets_gives_the_same_order_on_every_parse() {
    let text = read("solo_round_two_game");
    // The opponent's leaderboard hero arrives with all three trinket slots set.
    let created = "tag=PLAYER_LEADERBOARD_PLACE value=2\n";
    let at = text.find(created).expect("the opponent's hero packet") + created.len();
    let line = text[..at].lines().last().unwrap();
    let prefix = &line[..line.find("tag=").unwrap()];
    let tags = [
        ("BACON_HEROPOWER_TRINKET_DATABASE_ID", 30),
        ("BACON_SECOND_TRINKET_DATABASE_ID", 20),
        ("BACON_FIRST_TRINKET_DATABASE_ID", 10),
    ];
    let added: String = tags
        .iter()
        .map(|(tag, value)| format!("{prefix}tag={tag} value={value}\n"))
        .collect();
    let text = format!("{}{added}{}", &text[..at], &text[at..]);

    let first = report(&text);
    let slots: Vec<_> = seat(&first, 3)
        .trinkets
        .iter()
        .filter(|t| t.turn == 0)
        .map(|t| (t.turn, t.slot, t.dbf_id))
        .collect();
    assert_eq!(
        slots,
        [
            (0, TrinketSlot::First, 10),
            (0, TrinketSlot::Second, 20),
            (0, TrinketSlot::HeroPower, 30)
        ],
        "slot order within a turn"
    );
    // Each parse hashes the tags with a new seed.
    for _ in 0..50 {
        assert_eq!(report(&text).lobby, first.lobby);
    }
}
