//! Three combats of real Duos games whose odds the simulator refused because
//! one card was not modelled (T-616): Ghastcoiler, Dark Paradox in its tier 5
//! form and Timewarped Jazzer. The boards are card ids, stats, keywords and
//! Dark Gift ids only.

use combat_sim::{simulate, GameValues, Keywords, Minion, Options, Verdict, Warband};

/// (card id, Attack, Health, keywords as letters: T aunt, D ivine shield,
/// R eborn, V enomous, W indfury, S tealth, Dark Gift ids).
type Row = (
    &'static str,
    i32,
    i32,
    &'static str,
    &'static [&'static str],
);

fn warband(rows: &[Row]) -> Warband {
    let minions = rows
        .iter()
        .map(|&(id, attack, health, letters, gifts)| {
            let keywords = letters.chars().fold(Keywords::NONE, |k, c| {
                k.union(match c {
                    'T' => Keywords::TAUNT,
                    'D' => Keywords::DIVINE_SHIELD,
                    'R' => Keywords::REBORN,
                    'V' => Keywords::VENOMOUS,
                    'W' => Keywords::WINDFURY,
                    'S' => Keywords::STEALTH,
                    other => panic!("unknown keyword letter {other}"),
                })
            });
            let mut minion = Minion::new(id, attack, health).with(keywords);
            minion.golden = id.ends_with("_G");
            for gift in gifts {
                minion = minion.gift(*gift);
            }
            minion
        })
        .collect();
    Warband {
        minions,
        tavern_tier: Some(5),
        values: GameValues::default(),
    }
}

fn has_odds(player: &Warband, opponent: &Warband) {
    let options = Options {
        trials: 200,
        ..Options::default()
    };
    match simulate(player, opponent, &options).unwrap() {
        Verdict::Odds(o) => assert_eq!(o.won + o.tied + o.lost, o.trials),
        Verdict::NotModelled(c) => panic!("no odds, not modelled: {:?}", c.not_modelled),
    }
}

#[test]
fn a_combat_against_ghastcoiler_has_odds() {
    // Round 8: the opponent's right-most minion is Ghastcoiler.
    let own = warband(&[
        ("BG33_886_G", 5, 9, "", &[]),
        ("BG36_356", 10, 10, "", &[]),
        ("BGDUO_109", 4, 5, "", &[]),
        ("BGDUO_109", 52, 52, "", &[]),
        ("BG34_405", 5, 4, "", &[]),
        ("BG36_354", 3, 4, "", &[]),
        ("BG31_803", 1, 1, "", &[]),
    ]);
    let opponent = warband(&[
        ("BG36_204", 5, 7, "", &[]),
        ("BG36_367", 21, 34, "", &[]),
        ("BG34_170_G", 6, 6, "", &[]),
        ("BG22_403", 8, 5, "", &[]),
        ("BG34_170t2", 5, 19, "", &[]),
        ("BG_LOE_077", 3, 5, "", &[]),
        ("BGS_008", 7, 7, "", &[]),
    ]);
    has_odds(&own, &opponent);
    has_odds(&opponent, &own);
}

#[test]
fn a_combat_against_dark_paradox_with_a_slow_shield_has_odds() {
    // Round 9: Dark Paradox 49/40 with a Divine Shield that takes 3 hits.
    let own = warband(&[
        ("BG36_362", 186, 187, "T", &[]),
        ("BG36_730", 309, 309, "T", &[]),
        ("BG32_873", 93, 93, "", &[]),
        ("BG36_762", 73, 76, "", &[]),
        ("BG26_524_G", 220, 227, "", &["BG36_MidGameEffect_000t2"]),
        ("BG36_503", 161, 163, "", &[]),
        ("BG25_354_G", 2, 14, "", &[]),
    ]);
    let opponent = warband(&[
        ("BG31_149", 37, 38, "V", &[]),
        ("BG31_149", 24, 25, "D", &[]),
        ("BG36_360t4", 49, 40, "D", &[]),
        ("BG32_821", 32, 28, "", &[]),
        ("BG32_821", 22, 21, "", &[]),
        ("BG36_764", 48, 46, "", &["BG36_MidGameEffect_000t4"]),
        ("BG36_701", 25, 25, "", &["BG36_MidGameEffect_000t78"]),
    ]);
    has_odds(&own, &opponent);
    has_odds(&opponent, &own);
}

#[test]
fn a_combat_with_timewarped_jazzer_has_odds() {
    // Round 6: Timewarped Jazzer 5/6 with Taunt on the player's board.
    let own = warband(&[
        ("BG34_Giant_306", 5, 6, "T", &[]),
        ("BG34_682", 5, 3, "", &["BG36_MidGameEffect_000t16"]),
        ("BGDUO_111", 2, 2, "", &[]),
        ("BG36_345", 4, 4, "", &[]),
        ("BG31_326", 35, 7, "", &["BG36_MidGameEffect_000t74"]),
        ("BGFYM_002t", 9, 11, "T", &[]),
    ]);
    let opponent = warband(&[
        ("BG35_814", 13, 11, "D", &[]),
        ("BG29_813", 5, 6, "D", &["BG36_MidGameEffect_000t15"]),
        ("BGS_034", 12, 8, "DR", &[]),
        ("BG32_341", 5, 5, "D", &["BG36_MidGameEffect_000t15"]),
        ("BG36_345", 5, 3, "", &[]),
    ]);
    has_odds(&own, &opponent);
}

#[test]
fn stitched_salvager_is_still_refused_and_says_why() {
    let own = warband(&[("BG31_999", 16, 4, "", &[]), ("BG36_345", 4, 4, "", &[])]);
    let opponent = warband(&[("BG36_345", 5, 3, "", &[])]);
    let options = Options {
        trials: 10,
        ..Options::default()
    };
    let Verdict::NotModelled(c) = simulate(&own, &opponent, &options).unwrap() else {
        panic!("the Salvager's copy is unknown: the odds must be refused");
    };
    assert_eq!(c.not_modelled, vec!["BG31_999".to_string()]);
    let reasons = c.reasons();
    assert_eq!(reasons.len(), 1);
    assert!(reasons[0].1.contains("gone and not known"), "{:?}", reasons);
}
