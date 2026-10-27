use super::*;
use combat_sim::{BaseCard, Tribes};
use expected_place::model::MIN_GAMES;
use serde_json::json;

// Cards with no combat text in the engine (`combat_sim::cards::NO_COMBAT_EFFECT`).
const WEAK: &str = "BG28_603t";
const STRONG: &str = "BG26_361";
const OTHER: &str = "BG_BOT_911";
const FOE: &str = "BGS_119";
const FILLER: &str = "BGS_034";
/// A card the card data lacks.
const UNKNOWN: &str = "BG_NOT_A_CARD";
/// A spell: no health.
const SPELL: &str = "BG28_500";
/// Acts at the start of combat (`cards_round1`).
const STARTS: &str = "BG26_963_G";
/// Trinkets: no combat effect, and one acting at the start of combat.
const TRINKET_A: &str = "BG30_MagicItem_406";
const TRINKET_B: &str = "BG30_MagicItem_416";
const TRINKET_SOC: &str = "BG30_MagicItem_902";

fn stats(attack: i32, health: i32) -> Option<Facts> {
    Some(Facts {
        tier: Some(1),
        tribes: Tribes::NONE,
        base: BaseCard {
            attack,
            health,
            keywords: Keywords::NONE,
        },
    })
}

fn facts(id: &str) -> Option<Facts> {
    match id {
        WEAK => stats(1, 1),
        STRONG => stats(20, 20),
        OTHER => stats(2, 2),
        FOE => stats(3, 3),
        FILLER => stats(1, 1),
        SPELL => stats(0, 0),
        STARTS => stats(4, 4),
        _ => None,
    }
}

fn minion(card: &str, atk: i64, health: i64) -> Value {
    json!({"card_id": card, "atk": atk, "health": health, "golden": false, "keywords": []})
}

fn offer(card: &str) -> Value {
    json!({"card_id": card, "roll": 0, "frozen": false})
}

fn shop_turn(n: i64, buys: &[&str], sells: &[&str], offers: &[&str]) -> Value {
    let offers: Vec<Value> = offers.iter().map(|c| offer(c)).collect();
    json!({
        "turn": n, "start_ms": n * 1000, "end_ms": n * 1000 + 999, "tier": 1, "gold": 3,
        "gold_spent": 3, "rolls": 0, "free_rolls": 0, "buys": buys, "spell_buys": 0,
        "sells": sells, "freezes": 0, "actions": 0, "offers": offers,
    })
}

fn act(turn: i64, k: usize, kind: &str) -> Value {
    json!({"ms": turn * 1000 + 100 + k as i64, "turn": turn, "kind": kind})
}

fn round(n: i64, own: Vec<Value>, opponent: Vec<Value>) -> Value {
    json!({"number": n, "result": "won", "entries": [
        {"side": "own", "player_id": 1, "board": own},
        {"side": "opponent", "player_id": 2, "board": opponent},
    ], "health_after": {"1": 30, "2": 25}})
}

fn report(turns: Vec<Value>, actions: Vec<Value>, rounds: Vec<Value>) -> Value {
    json!({
        "status": "ok", "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1,
        "final_place": 4, "start_health": {"1": 30, "2": 30},
        "shop": {"turns": turns, "tier_ups": [], "actions": actions,
                 "start_ms": 0, "end_ms": 99_000, "ended": true},
        "discovers": [], "rounds": rounds,
    })
}

/// Turn 1: the shop shows WEAK and STRONG; the player buys WEAK and loses
/// the combat with it to a 3/3.
fn bought_weak() -> Value {
    report(
        vec![shop_turn(1, &[WEAK], &[], &[WEAK, STRONG])],
        vec![act(1, 0, "buy")],
        vec![round(1, vec![minion(WEAK, 1, 1)], vec![minion(FOE, 3, 3)])],
    )
}

fn run(report: &Value) -> GameReview {
    review(report, &facts, None, &Budget::default()).unwrap()
}

fn rebuilt(r: &GameReview, i: usize) -> &Rebuilt {
    r.decisions[i].rebuilt.as_ref().unwrap()
}

fn reason(r: &GameReview, i: usize) -> NotRebuilt {
    r.decisions[i].rebuilt.clone().unwrap_err()
}

#[test]
fn a_buy_is_played_again_with_each_other_card_on_offer() {
    let r = run(&bought_weak());

    let b = rebuilt(&r, 0);
    assert_eq!(b.round, 1);
    assert_eq!(b.actual.lost, 1.0);
    assert_eq!(b.alternatives.len(), 1);
    let alt = &b.alternatives[0];
    assert_eq!(alt.card, STRONG);
    assert_eq!(alt.approx, [Approx::BaseStats]);
    let fight = alt.fight.as_ref().unwrap();
    assert_eq!(fight.won, 1.0);
    let delta = alt.delta.as_ref().unwrap();
    assert_eq!(delta.score, 1.0);
    assert!(delta.clear);
    // A sure result is clear after one batch each.
    assert_eq!(alt.stop, Some(Stop::Clear));
    assert_eq!(fight.trials, Budget::default().batch);
    // The 20/20 stands: its tier is the damage; the 3/3 dealt its tier.
    assert_eq!(fight.minion_damage_dealt, Some(1.0));
    assert_eq!(b.actual.minion_damage_taken, Some(1.0));
    assert_eq!(delta.minion_damage_taken, Some(-1.0));
    // Without a model: a marker, never a number.
    assert_eq!(fight.place, Place::NoModel);
    assert_eq!(b.actual.place, Place::NoModel);
    assert_eq!(delta.place, None);
    assert!(!r.out_of_time);
    assert_eq!(r.boards, 2);
}

#[test]
fn the_same_game_gives_the_same_review() {
    let mut g = bought_weak();
    // A close fight, so the trials run long.
    g["shop"]["turns"][0]["offers"] = json!([offer(WEAK), offer(OTHER)]);
    g["rounds"][0]["entries"][0]["board"] = json!([minion(WEAK, 2, 1), minion(FILLER, 1, 2)]);
    g["rounds"][0]["entries"][1]["board"] = json!([minion(FOE, 1, 1), minion(FOE, 2, 2)]);

    assert_eq!(run(&g), run(&g));
}

#[test]
fn even_boards_play_every_trial_and_stay_unclear() {
    let mut g = bought_weak();
    // FILLER is a 1/1 like WEAK: the same fight with another card id.
    g["shop"]["turns"][0]["offers"] = json!([offer(WEAK), offer(FILLER)]);
    g["rounds"][0]["entries"][1]["board"] = json!([minion(FOE, 1, 1)]);

    let r = run(&g);

    let alt = &rebuilt(&r, 0).alternatives[0];
    let delta = alt.delta.as_ref().unwrap();
    assert_eq!(delta.score, 0.0);
    assert!(!delta.clear);
    // Both boards trade 1/1s: a sure tie is settled at once.
    assert_eq!(alt.stop, Some(Stop::Close));
}

#[test]
fn an_unclear_difference_runs_to_the_most_trials() {
    let mut g = bought_weak();
    let budget = Budget {
        max_trials: 1_000,
        batch: 250,
        ..Budget::default()
    };
    // FILLER is a 1/1 like WEAK, on a board whose fight is random: the
    // same dice give the same results, never a clear difference, and a
    // thousand trials are too few to call it surely small.
    g["shop"]["turns"][0]["offers"] = json!([offer(WEAK), offer(FILLER)]);
    g["rounds"][0]["entries"][0]["board"] =
        json!([minion(WEAK, 1, 1), minion(OTHER, 4, 3), minion(OTHER, 2, 5)]);
    g["rounds"][0]["entries"][1]["board"] =
        json!([minion(FOE, 3, 3), minion(FOE, 1, 6), minion(FOE, 5, 2)]);

    let r = review(&g, &facts, None, &budget).unwrap();

    let b = rebuilt(&r, 0);
    let alt = &b.alternatives[0];
    assert_eq!(alt.stop, Some(Stop::MaxTrials));
    assert_eq!(alt.fight.as_ref().unwrap().trials, budget.max_trials);
    assert_eq!(b.actual.trials, budget.max_trials);
    let delta = alt.delta.as_ref().unwrap();
    assert_eq!(delta.score, 0.0);
    assert!(delta.standard_error > 0.0);
    assert!(!delta.clear);
}

#[test]
fn decisions_of_one_turn_share_the_actual_board() {
    // Two buys in turn 1: both played against the same actual board.
    let g = report(
        vec![shop_turn(1, &[WEAK, FILLER], &[], &[WEAK, FILLER, STRONG])],
        vec![act(1, 0, "buy"), act(1, 1, "buy")],
        vec![round(
            1,
            vec![minion(WEAK, 1, 1), minion(FILLER, 1, 1)],
            vec![minion(FOE, 3, 3)],
        )],
    );

    let r = run(&g);

    assert_eq!(r.decisions.len(), 2);
    // The actual board once, and STRONG in each slot, FILLER for WEAK: 4.
    assert_eq!(r.boards, 4);
    assert_eq!(rebuilt(&r, 0).actual.trials, rebuilt(&r, 1).actual.trials);
    let cards: Vec<&str> = rebuilt(&r, 0)
        .alternatives
        .iter()
        .map(|a| a.card.as_str())
        .collect();
    assert_eq!(cards, [FILLER, STRONG]);
}

#[test]
fn a_sale_is_played_again_with_the_minion_kept() {
    let g = report(
        vec![
            shop_turn(1, &[], &[], &[]),
            shop_turn(2, &[], &[STRONG], &[]),
        ],
        vec![act(2, 0, "sell")],
        vec![
            round(1, vec![minion(STRONG, 20, 20)], vec![minion(FOE, 3, 3)]),
            round(2, vec![minion(WEAK, 1, 1)], vec![minion(FOE, 3, 3)]),
        ],
    );

    let r = run(&g);

    let b = rebuilt(&r, 0);
    assert_eq!(b.round, 2);
    let alt = &b.alternatives[0];
    assert_eq!(alt.card, STRONG);
    assert_eq!(alt.approx, [Approx::EarlierStats]);
    assert_eq!(alt.fight.as_ref().unwrap().won, 1.0);
    assert!(alt.delta.as_ref().unwrap().score > 0.9);
}

#[test]
fn a_trinket_choice_is_played_again_with_the_other_trinket() {
    let mut g = bought_weak();
    g["shop"]["actions"] = json!([act(1, 0, "choose")]);
    g["shop"]["turns"][0]["buys"] = json!([]);
    g["discovers"] = json!([{"turn": 1, "source": null, "picked": [0], "offered": [
        {"card_id": TRINKET_A, "gift": null}, {"card_id": TRINKET_B, "gift": null},
        {"card_id": TRINKET_SOC, "gift": null},
    ]}]);
    g["rounds"][0]["entries"][0]["kit"] =
        json!({"powers": [], "trinkets": [{"card_id": TRINKET_A, "slot": 1}]});

    let r = run(&g);

    let b = rebuilt(&r, 0);
    assert_eq!(b.alternatives.len(), 2);
    assert_eq!(b.alternatives[0].card, TRINKET_B);
    assert!(b.alternatives[0].approx.is_empty());
    assert_eq!(b.alternatives[0].delta.as_ref().unwrap().score, 0.0);
    assert_eq!(b.alternatives[1].fight, Err(Gap::StartOfCombat));
    assert_eq!(b.alternatives[1].stop, None);
}

#[test]
fn shop_decisions_say_they_need_shops_the_log_never_showed() {
    let mut g = bought_weak();
    g["shop"]["actions"] = json!([act(1, 0, "roll"), act(1, 1, "freeze")]);
    g["shop"]["turns"][0]["buys"] = json!([]);
    g["shop"]["turns"][0]["rolls"] = json!(1);

    let r = run(&g);

    assert_eq!(r.decisions.len(), 2);
    assert_eq!(reason(&r, 0), NotRebuilt::NeedsShopPlayout);
    assert_eq!(reason(&r, 1), NotRebuilt::NeedsShopPlayout);
    assert_eq!(r.boards, 0);
}

#[test]
fn every_decision_not_rebuilt_says_why() {
    // Nothing else on offer.
    let mut g = bought_weak();
    g["shop"]["turns"][0]["offers"] = json!([offer(WEAK)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::NoAlternative);

    // No combat after it.
    let mut g = bought_weak();
    g["rounds"] = json!([]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::NoNextCombat);

    // Bought but not played.
    let mut g = bought_weak();
    g["rounds"][0]["entries"][0]["board"] = json!([minion(OTHER, 2, 2)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::TakenNotOnBoard);

    // Two copies: which one was bought?
    let mut g = bought_weak();
    g["rounds"][0]["entries"][0]["board"] = json!([minion(WEAK, 1, 1), minion(WEAK, 2, 2)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::Ambiguous);

    // The card taken acts at the start of combat.
    let mut g = bought_weak();
    g["shop"]["turns"][0]["buys"] = json!([STARTS]);
    g["shop"]["turns"][0]["offers"] = json!([offer(STARTS), offer(STRONG)]);
    g["rounds"][0]["entries"][0]["board"] = json!([minion(STARTS, 4, 4)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::StartOfCombat);

    // A card on the actual board the card data lacks.
    let mut g = bought_weak();
    g["rounds"][0]["entries"][1]["board"] = json!([minion(UNKNOWN, 3, 3)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::Actual(Gap::NoCardData));
}

#[test]
fn a_sale_not_rebuilt_says_why() {
    let sale = |before: Vec<Value>, after: Vec<Value>| {
        report(
            vec![
                shop_turn(1, &[], &[], &[]),
                shop_turn(2, &[], &[STRONG], &[]),
            ],
            vec![act(2, 0, "sell")],
            vec![
                round(1, before, vec![minion(FOE, 3, 3)]),
                round(2, after, vec![minion(FOE, 3, 3)]),
            ],
        )
    };
    let full: Vec<Value> = (0..7).map(|_| minion(FILLER, 1, 1)).collect();
    let mut before = full.clone();
    before[0] = minion(STRONG, 20, 20);

    let g = sale(vec![minion(WEAK, 1, 1)], vec![]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::SoldNotOnBoard);
    let g = sale(before, full);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::BoardFull);
    let g = sale(
        vec![minion(STRONG, 20, 20), minion(STRONG, 2, 2)],
        vec![minion(STRONG, 2, 2)],
    );
    assert_eq!(reason(&run(&g), 0), NotRebuilt::Ambiguous);
}

#[test]
fn an_alternative_that_cannot_be_played_says_why() {
    let mut g = bought_weak();
    g["shop"]["turns"][0]["offers"] = json!([
        offer(WEAK),
        offer(UNKNOWN),
        offer(SPELL),
        offer(STARTS),
        offer(STRONG)
    ]);
    // A card the log hides is counted, not played.
    g["shop"]["turns"][0]["offers"]
        .as_array_mut()
        .unwrap()
        .push(json!({"card_id": null, "roll": 0, "frozen": false}));

    let r = run(&g);

    let b = rebuilt(&r, 0);
    let fights: Vec<(&str, Result<(), Gap>)> = b
        .alternatives
        .iter()
        .map(|a| {
            (
                a.card.as_str(),
                a.fight.as_ref().map(|_| ()).map_err(|g| *g),
            )
        })
        .collect();
    assert_eq!(
        fights,
        [
            (UNKNOWN, Err(Gap::NoCardData)),
            (SPELL, Err(Gap::NotAMinion)),
            (STARTS, Err(Gap::StartOfCombat)),
            (STRONG, Ok(())),
        ]
    );
    assert_eq!(b.unknown_offers, 1);
}

#[test]
fn a_start_of_combat_card_elsewhere_on_the_boards_is_noted() {
    let mut g = bought_weak();
    g["rounds"][0]["entries"][1]["board"] = json!([minion(FOE, 3, 3), minion(STARTS, 4, 4)]);

    let r = run(&g);

    let alt = &rebuilt(&r, 0).alternatives[0];
    assert_eq!(
        alt.approx,
        [Approx::BaseStats, Approx::StartOfCombatOnBoard]
    );
}

#[test]
fn a_minion_gifted_with_no_gift_named_makes_the_fights_approximate() {
    let mut g = bought_weak();
    g["rounds"][0]["entries"][1]["board"][0]["gift_unknown"] = json!(true);

    let r = run(&g);

    let b = rebuilt(&r, 0);
    assert_eq!(b.actual.approximate, [FOE]);
    assert_eq!(b.alternatives[0].fight.as_ref().unwrap().approximate, [FOE]);
}

#[test]
fn a_game_that_is_not_solo_gets_no_review() {
    let mut g = bought_weak();
    g["game_type"] = json!("GT_BATTLEGROUNDS_DUO");
    assert_eq!(
        review(&g, &facts, None, &Budget::default()),
        Err(NoReview::NotSolo)
    );
    let mut g = bought_weak();
    g["shop"] = Value::Null;
    assert_eq!(
        review(&g, &facts, None, &Budget::default()),
        Err(NoReview::NoShop(ShopState::NotAvailable))
    );
}

/// A Solo model that only reads the number of minions and the sum of their
/// attack: more attack, a better place.
fn model(games: usize, solo: bool) -> Model {
    let k = places(solo);
    let width = expected_place::WIDTH;
    let attack = expected_place::NAMES
        .iter()
        .position(|n| *n == "attack_sum")
        .unwrap();
    let weights = (0..k)
        .map(|place| {
            let mut w = vec![0.0; INPUTS];
            // The best place gains most from attack.
            w[attack] = 0.1 * (k - place) as f64;
            w
        })
        .collect();
    Model {
        solo,
        encoder: expected_place::VERSION,
        mean: vec![0.0; width],
        spread: vec![1.0; width],
        weights,
        intercepts: vec![0.0; k],
        games,
        rows: games,
    }
}

#[test]
fn with_a_model_each_board_gets_its_expected_place() {
    let m = model(MIN_GAMES, true);

    let r = review(&bought_weak(), &facts, Some(&m), &Budget::default()).unwrap();

    let b = rebuilt(&r, 0);
    let Place::Expected(actual) = b.actual.place else {
        panic!("no place: {:?}", b.actual.place);
    };
    let alt = &b.alternatives[0];
    let Place::Expected(better) = alt.fight.as_ref().unwrap().place else {
        panic!("no place");
    };
    assert!(better < actual, "{better} vs {actual}");
    let d = alt.delta.as_ref().unwrap().place.unwrap();
    assert!((d - (better - actual)).abs() < 1e-12);
}

#[test]
fn a_model_players_may_not_see_gives_a_marker_not_a_number() {
    let place = |m: &Model| {
        let r = review(&bought_weak(), &facts, Some(m), &Budget::default()).unwrap();
        rebuilt(&r, 0).actual.place
    };
    assert_eq!(place(&model(MIN_GAMES - 1, true)), Place::TooFewGames);
    assert_eq!(place(&model(MIN_GAMES, false)), Place::OtherMode);
    let mut other = model(MIN_GAMES, true);
    other.encoder += 1;
    assert_eq!(place(&other), Place::OtherFormat);
    other = model(MIN_GAMES, true);
    other.weights.pop();
    assert_eq!(place(&other), Place::OtherFormat);
}

/// A long synthetic game: every turn a buy with five other cards on offer,
/// on full boards. The card bought alternates, so each is
/// new on its turn's board.
fn long_game(turns: i64) -> Value {
    let offers = [WEAK, STRONG, OTHER, FILLER, FOE, STRONG];
    let bought = |t: i64| if t % 2 == 1 { WEAK } else { OTHER };
    let shop: Vec<Value> = (1..=turns)
        .map(|t| shop_turn(t, &[bought(t)], &[], &offers))
        .collect();
    let actions: Vec<Value> = (1..=turns).map(|t| act(t, 0, "buy")).collect();
    let rounds: Vec<Value> = (1..=turns)
        .map(|t| {
            let mut own: Vec<Value> = (0..6).map(|i| minion(FILLER, 2 + i, 3)).collect();
            own.push(minion(bought(t), 2, 2));
            // Another opponent each turn: no board is played twice.
            let theirs: Vec<Value> = (0..7).map(|i| minion(FOE, 3 + i % 3, 2 + t)).collect();
            round(t, own, theirs)
        })
        .collect();
    report(shop, actions, rounds)
}

#[test]
fn the_time_budget_is_respected_and_what_it_did_not_reach_says_so() {
    let g = long_game(12);
    let budget = Budget {
        time: Duration::from_millis(10),
        max_trials: 2_000,
        batch: 100,
    };
    // A clock that moves one millisecond each time it is read.
    let mut now = 0u64;
    let mut clock = || {
        now += 1;
        Duration::from_millis(now)
    };

    let r = review_with_clock(&g, &facts, None, &budget, &mut clock).unwrap();

    assert!(r.out_of_time);
    // Every decision is there, rebuilt or with its reason.
    assert_eq!(r.decisions.len(), 12);
    // The clock is read before each batch, and never after the time is up:
    // at most 9 batches ran.
    assert!(r.trials <= 9 * u64::from(budget.batch), "{}", r.trials);
    let last = r.decisions.last().unwrap();
    assert_eq!(last.rebuilt, Err(NotRebuilt::Actual(Gap::OutOfTime)));
    let out_of_time = r
        .decisions
        .iter()
        .filter(|d| d.rebuilt == Err(NotRebuilt::Actual(Gap::OutOfTime)))
        .count();
    assert!(out_of_time > 0);
}

#[test]
fn a_real_clock_budget_stops_the_review_in_time() {
    let g = long_game(16);
    let budget = Budget {
        time: Duration::from_millis(50),
        max_trials: 1_000_000,
        batch: 200,
    };
    let started = Instant::now();

    let r = review(&g, &facts, None, &budget).unwrap();

    assert!(r.out_of_time);
    assert_eq!(r.decisions.len(), 16);
    // One batch may run past the budget; a batch is far below a second.
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}
