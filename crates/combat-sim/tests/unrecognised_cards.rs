//! Two combats of a real Duos game whose odds the simulator refused because
//! it did not know one form of a card: Disguised Graverobber (normal) and
//! Spark Snapper (golden). The boards are card ids, stats and keywords only.

use combat_sim::{simulate, GameValues, Keywords, Minion, Options, Verdict, Warband};

/// (card id, Attack, Health, keywords as letters: T aunt, D ivine shield,
/// R eborn).
type Row = (&'static str, i32, i32, &'static str);

fn warband(rows: &[Row], values: GameValues) -> Warband {
    let minions = rows
        .iter()
        .map(|&(id, attack, health, letters)| {
            let keywords = letters.chars().fold(Keywords::NONE, |k, c| {
                k.union(match c {
                    'T' => Keywords::TAUNT,
                    'D' => Keywords::DIVINE_SHIELD,
                    'R' => Keywords::REBORN,
                    other => panic!("unknown keyword letter {other}"),
                })
            });
            let mut minion = Minion::new(id, attack, health).with(keywords);
            minion.golden = id.ends_with("_G");
            minion
        })
        .collect();
    Warband {
        kit: None,
        minions,
        tavern_tier: Some(6),
        values,
    }
}

fn odds_exist(player: &Warband, opponent: &Warband) {
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
fn a_combat_against_disguised_graverobber_has_odds() {
    // Round 6: the opponent's board holds the normal Disguised Graverobber.
    let own = warband(
        &[
            ("BG36_354", 10, 10, ""),
            ("BG36_345", 4, 3, ""),
            ("BG36_345", 3, 3, ""),
            ("BG27_005", 4, 6, ""),
            ("BG34_405", 6, 4, ""),
        ],
        GameValues::default(),
    );
    let opponent = warband(
        &[
            ("BG25_010t", 6, 1, "R"),
            ("BG28_303", 4, 4, ""),
            ("BG25_011_G", 6, 8, ""),
            ("BG25_010", 6, 1, ""),
        ],
        GameValues {
            undead_attack: 4,
            ..GameValues::default()
        },
    );
    odds_exist(&own, &opponent);
}

#[test]
fn a_combat_with_a_golden_spark_snapper_has_odds() {
    // Round 16: the player's board holds a golden Spark Snapper.
    let own = warband(
        &[
            ("BG26_ICC_901_G", 11, 16, "T"),
            ("BG36_851_G", 8931, 11459, "TD"),
            ("BG36_367_G", 1822, 2048, ""),
            ("BG36_366_G", 3812, 3936, ""),
            ("BG33_823", 4, 5, ""),
            ("BG36_366_G", 5182, 5308, ""),
            ("BG26_152", 121, 161, "T"),
        ],
        GameValues::default(),
    );
    let opponent = warband(
        &[
            ("BG36_312_G", 1407, 3224, ""),
            ("BG36_300_G", 1432, 3403, ""),
            ("BG26_ICC_901", 607, 1591, ""),
            ("BG36_311_G", 1294, 3085, ""),
            ("BG26_ICC_901", 289, 741, ""),
            ("BG36_108_G", 1646, 3751, "T"),
            ("BG36_320_G", 786, 1802, ""),
        ],
        GameValues {
            spell: (17, 45),
            ..GameValues::default()
        },
    );
    odds_exist(&own, &opponent);
}
