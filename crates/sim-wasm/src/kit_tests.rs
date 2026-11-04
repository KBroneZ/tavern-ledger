//! Hero powers and trinkets in the request (T-628), on made-up boards (no
//! real card data, D-038): what is read, what is refused, start of combat
//! and the turn.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::*;

const PLAIN: &str = "BGS_119";
/// Fire Invocation: at the start of combat the left-most minion's Attack
/// doubles.
const FIRE: &str = "BG22_HERO_001p_t2";
/// Lock and Load: fires the minion it holds at the start of combat.
const LOCK: &str = "BG22_HERO_000p_Alt";
/// Frostwolf Fervor: unlocked from turn 7.
const FROSTWOLF: &str = "BG22_HERO_002p";

fn minion(id: &str, attack: i64, health: i64) -> Value {
    let facts = json!({ "tier": 1, "tribes": [], "attack": 1, "health": 1, "keywords": [] });
    json!({ "card_id": id, "attack": attack, "health": health, "facts": facts })
}

/// A Solo request: the player's 2/3 with `powers` against a 3/3.
fn request(powers: Value, start_of_combat: bool) -> Value {
    json!({
        "player": [{ "tier": 1, "minions": [minion(PLAIN, 2, 3)], "kit": { "powers": powers, "trinkets": [] } }],
        "opponent": [{ "tier": 1, "minions": [minion(PLAIN, 3, 3)] }],
        "start_of_combat": start_of_combat,
    })
}

fn run(v: &Value) -> Value {
    serde_json::from_str(&run_json(&v.to_string())).expect("the answer is JSON")
}

fn share(a: &Value, key: &str) -> f64 {
    a[key].as_f64().unwrap() / a["trials"].as_f64().unwrap()
}

#[test]
fn a_start_of_combat_hero_power_plays_only_when_start_of_combat_does() {
    let powers = json!([{ "card_id": FIRE, "slot": 0 }]);
    let a = run(&request(powers.clone(), true));
    assert_eq!(a["status"], "odds");
    assert_eq!(
        share(&a, "tied"),
        1.0,
        "the doubled 4 Attack trades with the 3/3"
    );
    assert_eq!(a["start_of_combat"], false, "played, not missed");
    // Boards taken at the first attack already carry it.
    let a = run(&request(powers, false));
    assert_eq!(share(&a, "lost"), 1.0);
    assert_eq!(
        a["start_of_combat"], true,
        "its start of combat is on the boards, so the odds are marked (T-632)"
    );
}

/// Holy Mallet acts at the start of combat; Bleeding Heart has no effect in
/// combat; Bronzebeard Portrait acts later in it.
const MALLET: &str = "BG30_MagicItem_902";
const NO_EFFECT: &str = "BG30_MagicItem_713";
const LATER: &str = "BG30_MagicItem_418";

#[test]
fn the_mark_is_set_for_a_trinket_that_acts_at_the_start_of_combat_only() {
    let mark = |id: &str, soc: bool| {
        let mut req = request(json!([]), soc);
        req["player"][0]["kit"]["trinkets"] = json!([{ "card_id": id, "slot": 1 }]);
        run(&req)["start_of_combat"].clone()
    };
    assert_eq!(mark(MALLET, false), true);
    assert_eq!(mark(MALLET, true), false, "played, not missed");
    assert_eq!(mark(NO_EFFECT, false), false);
    assert_eq!(mark(LATER, false), false);
    assert_eq!(run(&request(json!([]), false))["start_of_combat"], false);
}

#[test]
fn a_kit_is_read_into_the_engines_form() {
    let req = json!({
        "player": [{ "minions": [minion(PLAIN, 2, 3), minion(PLAIN, 1, 1)], "kit": {
            "powers": [{ "card_id": LOCK, "slot": 0, "counters": [1, 0, 0, 0, 0], "target": 2, "held": minion(PLAIN, 5, 6) }],
            "trinkets": [{ "card_id": "BG30_Trinket_2nd", "slot": 2 }],
        } }],
        "opponent": [{ "minions": [minion(PLAIN, 3, 3)] }],
        "start_of_combat": true,
        "round": 9,
    });
    let r = input::read(&req.to_string()).expect("read");
    assert!(r.start_of_combat);
    let kit = r.player.first.kit.expect("the player's kit");
    assert_eq!(kit.round, Some(9));
    assert_eq!(kit.powers[0].counters, [1, 0, 0, 0, 0]);
    assert_eq!(kit.powers[0].target, Some(1), "board position 2 is index 1");
    assert_eq!(kit.powers[0].held.as_ref().map(|m| m.attack), Some(5));
    assert_eq!(kit.trinkets[0].slot, 2);
    assert_eq!(r.opponent.first.kit, None, "none given is unknown");
}

#[test]
fn a_held_minion_comes_in_only_with_start_of_combat_played() {
    let lacking = json!({ "card_id": "SYN_HELD", "attack": 5, "health": 6 });
    let powers = json!([{ "card_id": LOCK, "counters": [1, 0, 0, 0, 0], "held": lacking }]);
    let a = run(&request(powers.clone(), true));
    assert_eq!(a["status"], "no_card_data");
    assert_eq!(a["missing"], json!(["SYN_HELD"]));
    // Off, it never comes in: its card data does not matter.
    let r = input::read(&request(powers, false).to_string()).expect("read");
    assert_eq!(r.player.first.kit.unwrap().powers[0].held, None);
    assert!(r.missing.is_empty());
}

#[test]
fn a_power_the_turn_unlocks_needs_the_turn() {
    let powers = json!([{ "card_id": FROSTWOLF, "counters": [1, 0, 0, 0, 0] }]);
    let a = run(&request(powers.clone(), true));
    assert_eq!(
        a["status"], "not_modelled",
        "without the turn it never guesses"
    );
    assert_eq!(a["not_modelled"], json!([FROSTWOLF]));
    let mut req = request(powers, true);
    req["round"] = json!(7);
    assert_eq!(run(&req)["status"], "odds");
}

#[test]
fn an_unreviewed_hero_power_is_named_never_guessed() {
    let a = run(&request(json!([{ "card_id": "SYN_POWER" }]), false));
    assert_eq!(a["status"], "not_modelled");
    assert_eq!(a["not_modelled"], json!(["SYN_POWER"]));
    assert_eq!(a["reasons"][0]["card_id"], "SYN_POWER");
}

#[test]
fn bad_kits_are_refused_with_a_reason() {
    let bad = |powers: Value| run(&request(powers, true));
    for (powers, words) in [
        (json!([{ "card_id": "" }]), "card id"),
        (json!([{ "card_id": FIRE, "slot": 4 }]), "slot"),
        (json!([{ "card_id": FIRE, "counters": [1, 2] }]), "five"),
        (
            json!([{ "card_id": FIRE, "counters": [-1, 0, 0, 0, 0] }]),
            "counter",
        ),
        (json!([{ "card_id": FIRE, "target": 8 }]), "target"),
        (json!([{ "card_id": FIRE, "target": 2 }]), "past the board"),
        (
            json!([{ "card_id": FIRE }, { "card_id": FIRE }, { "card_id": FIRE }, { "card_id": FIRE }, { "card_id": FIRE }]),
            "short list",
        ),
    ] {
        let a = bad(powers);
        assert_eq!(a["status"], "bad_input");
        assert!(a["error"].as_str().unwrap().contains(words), "{a}");
    }
    let mut req = request(json!([]), true);
    req["player"][0]["kit"]["trinkets"] = json!([{ "card_id": FIRE, "target": 1 }]);
    assert!(run(&req)["error"]
        .as_str()
        .unwrap()
        .contains("only a hero power"));
    req["player"][0]["kit"]["trinkets"] = json!([{ "card_id": FIRE, "held": minion(PLAIN, 1, 1) }]);
    assert!(run(&req)["error"]
        .as_str()
        .unwrap()
        .contains("only a hero power"));
    let mut req = request(json!([]), true);
    req["start_of_combat"] = json!("yes");
    assert_eq!(run(&req)["status"], "bad_input");
    req["start_of_combat"] = json!(true);
    req["round"] = json!(0);
    assert_eq!(run(&req)["status"], "bad_input");
    req["round"] = json!(7);
    req["player"][0]["kit"] = json!([]);
    assert_eq!(run(&req)["status"], "bad_input");
}

#[test]
fn a_request_without_the_new_fields_plays_as_before() {
    // No kit, no turn, no start of combat: boards taken at the first attack.
    let req = json!({
        "player": [{ "minions": [minion(PLAIN, 2, 3)] }],
        "opponent": [{ "minions": [minion(PLAIN, 3, 3)] }],
    });
    let r = input::read(&req.to_string()).expect("read");
    assert!(!r.start_of_combat);
    assert_eq!(r.player.first.kit, None);
}

#[test]
fn the_pages_table_of_what_hero_powers_need_matches_the_engine() {
    use combat_sim::kit_effects::{acts, needs, played, played_ids};
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../web/src/lib/sim/kitNeeds.json"
    );
    let page: BTreeMap<String, Vec<String>> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("kitNeeds.json")).expect("JSON");
    let mut engine = BTreeMap::new();
    for id in played_ids() {
        let play = played(id).expect("played");
        let n = needs(play);
        let words: Vec<String> = [
            (n.target, "target"),
            (n.held, "held"),
            (n.round, "round"),
            (n.teammate, "teammate"),
            // It acts only while its first number is not 0.
            (acts(play, &[0; 5], Some(99)) == Some(false), "first"),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, w)| w.to_string())
        .collect();
        if !words.is_empty() {
            engine.insert(id.to_string(), words);
        }
    }
    assert_eq!(page, engine, "web/src/lib/sim/kitNeeds.json");
    // The turn those powers unlock at is the page's UNLOCK_TURN (web/src/lib/sim/kit.ts).
    for id in played_ids() {
        if let Some(combat_sim::kit_effects::SpaceAction::CopyHighest { unlock, .. }) =
            played(id).and_then(|p| p.space)
        {
            assert_eq!(unlock, 7, "{id}: web/src/lib/sim/kit.ts UNLOCK_TURN");
        }
    }
}
