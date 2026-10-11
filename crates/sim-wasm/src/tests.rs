//! The wrapper on made-up boards (no real card data, D-038): what it
//! accepts, what it refuses, and the C ABI round trip.

use serde_json::{json, Value};

use super::*;

/// A plain tier 1 card with no combat effect, and the Skeleton token.
const PLAIN: &str = "BGS_119";
const SKELETON: &str = "BG_ICC_026t";

fn facts() -> Value {
    json!({ "tier": 1, "tribes": [], "attack": 1, "health": 1, "keywords": [] })
}

fn minion(id: &str, attack: i64, health: i64) -> Value {
    json!({ "card_id": id, "attack": attack, "health": health, "facts": facts() })
}

fn board(minions: Vec<Value>, tier: Option<i64>) -> Value {
    json!({ "tier": tier, "minions": minions })
}

fn request(player: Vec<Value>, opponent: Vec<Value>) -> Value {
    json!({ "player": player, "opponent": opponent, "damage_cap": 15 })
}

fn run(v: &Value) -> Value {
    serde_json::from_str(&run_json(&v.to_string())).expect("the answer is JSON")
}

#[test]
fn a_sure_win_gives_every_trial_and_the_damage() {
    let req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(3))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(2))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "odds");
    assert_eq!(a["engine"], combat_sim::VERSION);
    assert_eq!(a["trials"], 10_000);
    assert_eq!(
        (a["won"].as_u64(), a["tied"].as_u64(), a["lost"].as_u64()),
        (Some(10_000), Some(0), Some(0))
    );
    // Tier 3 hero plus one tier 1 minion left.
    assert_eq!(a["dealt"], json!({ "low": 4, "likely": 4, "high": 4 }));
    assert_eq!(a["taken"], Value::Null);
    assert_eq!(a["damage_known"], true);
}

#[test]
fn the_damage_cap_applies_and_an_unknown_tier_hides_the_damage() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(6))],
        vec![board(vec![minion(SKELETON, 1, 1)], None)],
    );
    req["damage_cap"] = json!(5);
    assert_eq!(
        run(&req)["dealt"],
        json!({ "low": 5, "likely": 5, "high": 5 })
    );
    req["player"][0]["tier"] = Value::Null;
    let a = run(&req);
    assert_eq!(a["dealt"], Value::Null);
    assert_eq!(a["damage_known"], false);
}

#[test]
fn the_same_request_gives_the_same_answer() {
    let req = request(
        vec![board(
            vec![minion(PLAIN, 3, 3), minion(PLAIN, 1, 4)],
            Some(2),
        )],
        vec![board(
            vec![minion(PLAIN, 2, 5), minion(SKELETON, 1, 1)],
            Some(2),
        )],
    );
    assert_eq!(run(&req), run(&req));
}

#[test]
fn a_card_the_engine_does_not_know_is_named_never_guessed() {
    let req = request(
        vec![board(vec![minion("BG_NOT_A_CARD", 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "not_modelled");
    assert_eq!(a["not_modelled"], json!(["BG_NOT_A_CARD"]));
    assert!(a.get("won").is_none());
}

#[test]
fn a_card_without_facts_is_named_as_missing() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    req["opponent"][0]["minions"][0]["facts"] = Value::Null;
    let a = run(&req);
    assert_eq!(a["status"], "no_card_data");
    assert_eq!(a["missing"], json!([SKELETON]));
}

#[test]
fn approximate_cards_come_with_their_reason() {
    // Round 1: a Deathrattle whose numbers grow during the game.
    let req = request(
        vec![board(vec![minion("BG25_009", 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "odds");
    let approx = a["approximate"].as_array().unwrap();
    assert_eq!(approx[0]["card_id"], "BG25_009");
    assert!(approx[0]["reason"].as_str().unwrap().contains("grow"));
}

#[test]
fn a_start_of_combat_card_is_flagged() {
    let req = request(
        vec![board(vec![minion("BG26_963_G", 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    assert_eq!(run(&req)["start_of_combat"], true);
}

#[test]
fn duos_teams_bring_two_boards() {
    let req = request(
        vec![
            board(vec![minion(PLAIN, 1, 1)], Some(1)),
            board(vec![minion(PLAIN, 5, 5)], Some(1)),
        ],
        vec![
            board(vec![minion(PLAIN, 1, 1)], Some(1)),
            board(vec![minion(PLAIN, 1, 1)], Some(1)),
        ],
    );
    let a = run(&req);
    assert_eq!(a["status"], "odds");
    assert!(a["won"].as_u64().unwrap() > a["lost"].as_u64().unwrap());
}

#[test]
fn board_keywords_are_read_and_card_text_words_dropped() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
    );
    req["player"][0]["minions"][0]["keywords"] = json!(["divine_shield", "deathrattle", "rally"]);
    let a = run(&req);
    // A shielded 1/1 against a plain 1/1 always wins.
    assert_eq!(a["won"], 10_000);
    req["player"][0]["minions"][0]["keywords"] = json!(["flying"]);
    assert_eq!(run(&req)["status"], "bad_input");
}

/// One way to spoil a good request.
type Change = Box<dyn Fn(&mut Value)>;

#[test]
fn bad_requests_are_refused_with_a_reason() {
    let ok = request(
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
    );
    let cases: Vec<(&str, Change)> = vec![
        ("no player", Box::new(|r| r["player"] = json!([]))),
        (
            "three boards",
            Box::new(|r| {
                r["player"] = json!([r["opponent"][0], r["opponent"][0], r["opponent"][0]])
            }),
        ),
        (
            "bad id",
            Box::new(|r| r["player"][0]["minions"][0]["card_id"] = json!("<b>")),
        ),
        (
            "no health",
            Box::new(|r| r["player"][0]["minions"][0]["health"] = json!(0)),
        ),
        (
            "negative attack",
            Box::new(|r| r["player"][0]["minions"][0]["attack"] = json!(-1)),
        ),
        (
            "fraction",
            Box::new(|r| r["player"][0]["minions"][0]["attack"] = json!(1.5)),
        ),
        (
            "huge",
            Box::new(|r| r["player"][0]["minions"][0]["health"] = json!(1_i64 << 40)),
        ),
        ("tier 8", Box::new(|r| r["player"][0]["tier"] = json!(8))),
        (
            "golden text",
            Box::new(|r| r["player"][0]["minions"][0]["golden"] = json!("yes")),
        ),
        ("cap 0", Box::new(|r| r["damage_cap"] = json!(0))),
        (
            "facts tribe",
            Box::new(|r| r["player"][0]["minions"][0]["facts"]["tribes"] = json!(["<x>"])),
        ),
        (
            "facts health",
            Box::new(|r| r["player"][0]["minions"][0]["facts"]["health"] = json!(0)),
        ),
        (
            "not a list",
            Box::new(|r| r["player"][0]["minions"] = json!({})),
        ),
    ];
    for (name, change) in cases {
        let mut r = ok.clone();
        change(&mut r);
        let a = run(&r);
        assert_eq!(a["status"], "bad_input", "{name}: {a}");
        assert!(a["error"].as_str().is_some_and(|e| !e.is_empty()), "{name}");
    }
    let full = board((0..8).map(|_| minion(PLAIN, 1, 1)).collect(), Some(1));
    let mut r = ok.clone();
    r["player"][0] = full;
    assert_eq!(
        run(&r)["status"],
        "bad_board",
        "eight minions: the engine's own refusal"
    );
    for text in ["", "null", "[]", "{", "\"x\""] {
        assert_eq!(run_json_value(text)["status"], "bad_input", "{text:?}");
    }
    let big = format!("{{\"x\":\"{}\"}}", "a".repeat(input::MAX_INPUT_BYTES));
    assert_eq!(run_json_value(&big)["error"], "the request is too large");
}

fn run_json_value(text: &str) -> Value {
    serde_json::from_str(&run_json(text)).unwrap()
}

#[test]
fn the_c_abi_round_trip_gives_the_same_answer() {
    let req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    )
    .to_string();
    let ptr = sim_alloc(req.len());
    // SAFETY: `ptr` holds `req.len()` writable bytes from sim_alloc.
    unsafe { std::ptr::copy_nonoverlapping(req.as_ptr(), ptr, req.len()) };
    // SAFETY: as sim_run asks: from sim_alloc with this length, written above.
    let len = unsafe { sim_run(ptr, req.len()) };
    // SAFETY: the answer lives until the next run; `len` bytes long.
    let bytes = unsafe { std::slice::from_raw_parts(sim_answer(), len) };
    assert_eq!(std::str::from_utf8(bytes).unwrap(), run_json(&req));
    // Not UTF-8: refused, not a panic.
    let ptr = sim_alloc(2);
    // SAFETY: two writable bytes from sim_alloc.
    unsafe { std::ptr::copy_nonoverlapping([0xff, 0xfe].as_ptr(), ptr, 2) };
    // SAFETY: as above.
    let len = unsafe { sim_run(ptr, 2) };
    // SAFETY: as above.
    let bytes = unsafe { std::slice::from_raw_parts(sim_answer(), len) };
    assert!(std::str::from_utf8(bytes).unwrap().contains("not UTF-8"));
    // An empty request: an allocation of one byte, read as zero.
    let ptr = sim_alloc(0);
    // SAFETY: from sim_alloc(0), nothing written, length 0.
    let len = unsafe { sim_run(ptr, 0) };
    assert!(len > 0);
}
