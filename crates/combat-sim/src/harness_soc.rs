//! The harness on start of combat (T-628): each logged combat played three
//! ways and scored against the game's result. **Today:** the boards at the
//! first attack, start of combat off, the heroes' powers and trinkets not
//! given. **With kits:** the same boards with each hero's powers and
//! trinkets (an unknown one blocks, one that acts later in the combat is
//! approximate). **Leg rule:** the boards as each leg started, before any
//! start-of-combat effect, with start of combat played, when every
//! start-of-combat source on those boards and kits is played; otherwise as
//! "with kits". It knows only what the boards and kits show, as odds at the
//! combat's start would: an effect from a card in a hand or a Secret is not
//! on them, so such a combat is still played from before start of combat,
//! and counted apart.
//!
//! Local only, like the rest of the harness: counts, shares and card ids,
//! never a name.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Instant;

use bg_parser::report::{CombatEntry, GameReport, HeroKit, Round, Side};

use crate::combat::CombatResult;
use crate::harness::CardDb;
use crate::harness_logs::{engine_board, engine_minion, game_values};
use crate::model::{Kit, KitCard, Team, Warband};
use crate::score::{score, Prediction, Report};
use crate::trials::{simulate_teams, Coverage, Options, Verdict};

/// One logged combat with each team's boards in the order they fought, at
/// the first attack and as each leg started.
#[derive(Clone, Debug, PartialEq)]
pub struct SocCombat {
    pub game: usize,
    pub round: i64,
    pub duos: bool,
    /// Today's boards (at the first attack), with each hero's kit.
    pub after: [Team; 2],
    /// The boards before start of combat, with each hero's kit; `None` when
    /// a leg has none.
    pub before: Option<[Team; 2]>,
    pub result: CombatResult,
    /// Card ids of the combat's effects from a card in a hand or a Secret,
    /// which no board shows (the simulator never plays them).
    pub hidden: Vec<String>,
}

/// A hero's kit as the engine takes it, in round `round`; a target is a
/// board index. A held minion only on the player's team (`own`, D-163,
/// D-167), as the overlay.
pub(crate) fn engine_kit(kit: &HeroKit, round: i64, own: bool, db: &CardDb) -> Kit {
    let card = |c: &bg_parser::report::KitCard| KitCard {
        held: c
            .held
            .as_ref()
            .filter(|_| own)
            .and_then(|m| engine_minion(m, db)),
        card_id: c.card_id.clone(),
        slot: u8::try_from(c.slot).unwrap_or(0),
        counters: c.counters.map(|n| i32::try_from(n).unwrap_or(0)),
        target: c
            .target
            .and_then(|p| p.checked_sub(1))
            .and_then(|i| usize::try_from(i).ok()),
    };
    Kit {
        powers: kit.powers.iter().map(card).collect(),
        trinkets: kit.trinkets.iter().map(card).collect(),
        round: u32::try_from(round).ok(),
    }
}

/// An entry's two boards with its kit: (at the first attack, before start of
/// combat). `None` when the entry has no board or a card id is missing.
fn entry_bands(e: &CombatEntry, round: i64, db: &CardDb) -> Option<(Warband, Option<Warband>)> {
    let own = e.side == Side::Own;
    let kit = e.kit.as_ref().map(|k| engine_kit(k, round, own, db));
    let band = |minions| Warband {
        values: game_values(e.values),
        kit: kit.clone(),
        ..Warband::new(minions)
    };
    let after = band(engine_board(e.board.as_deref()?, db)?);
    let before = match e.before.as_deref() {
        Some(b) => Some(band(engine_board(b, db)?)),
        None => None,
    };
    Some((after, before))
}

/// The boards of `side` in the order they fought, at most one per seat (as
/// `harness_duos::legs`).
fn legs(round: &Round, side: Side, db: &CardDb) -> Option<Vec<(Warband, Option<Warband>)>> {
    let mut seats: Vec<Option<i64>> = Vec::new();
    let mut out = Vec::new();
    for e in round
        .entries
        .iter()
        .filter(|e| e.side == side && e.board.is_some())
    {
        if seats.contains(&e.player_id) {
            continue;
        }
        seats.push(e.player_id);
        out.push(entry_bands(e, round.number, db)?);
    }
    (!out.is_empty()).then_some(out)
}

fn team(legs: &[Warband]) -> Team {
    Team {
        first: legs[0].clone(),
        second: legs.get(1).cloned(),
    }
}

fn result_of(r: bg_parser::report::CombatResult) -> CombatResult {
    match r {
        bg_parser::report::CombatResult::Won => CombatResult::Won,
        bg_parser::report::CombatResult::Lost => CombatResult::Lost,
        bg_parser::report::CombatResult::Tie => CombatResult::Tie,
    }
}

/// The combats of one game with a result and a board on each side.
pub fn soc_combats(report: &GameReport, game: usize, db: &CardDb) -> Vec<SocCombat> {
    let duos = report.game_type.as_deref() == Some("GT_BATTLEGROUNDS_DUO");
    let mut out = Vec::new();
    for round in &report.rounds {
        let Some(result) = round.result.map(result_of) else {
            continue;
        };
        let (Some(own), Some(opp)) = (legs(round, Side::Own, db), legs(round, Side::Opponent, db))
        else {
            continue;
        };
        if !duos && (own.len() > 1 || opp.len() > 1) {
            continue;
        }
        let after = |legs: &[(Warband, Option<Warband>)]| -> Vec<Warband> {
            legs.iter().map(|(a, _)| a.clone()).collect()
        };
        let before = |legs: &[(Warband, Option<Warband>)]| -> Option<Vec<Warband>> {
            legs.iter().map(|(_, b)| b.clone()).collect()
        };
        out.push(SocCombat {
            game,
            round: round.number,
            duos,
            after: [team(&after(&own)), team(&after(&opp))],
            before: before(&own)
                .zip(before(&opp))
                .map(|(o, p)| [team(&o), team(&p)]),
            result,
            hidden: round
                .hidden_triggers
                .iter()
                .map(|(_, id)| id.clone())
                .collect(),
        });
    }
    out
}

/// A team without its heroes' kits: today's input.
fn without_kits(t: &Team) -> Team {
    let strip = |b: &Warband| Warband {
        kit: None,
        ..b.clone()
    };
    Team {
        first: strip(&t.first),
        second: t.second.as_ref().map(strip),
    }
}

/// How one combat was played.
enum Played {
    Blocked(Coverage),
    Odds(Prediction),
}

fn play(teams: &[Team; 2], start_of_combat: bool, options: &Options) -> Played {
    let opts = Options {
        start_of_combat,
        allow_unmodelled: false,
        ..*options
    };
    match simulate_teams(&teams[0], &teams[1], &opts) {
        Ok(Verdict::Odds(o)) => Played::Odds(Prediction {
            won: o.share(CombatResult::Won),
            tied: o.share(CombatResult::Tie),
            lost: o.share(CombatResult::Lost),
        }),
        Ok(Verdict::NotModelled(c)) => Played::Blocked(c),
        Err(_) => Played::Blocked(Coverage::default()),
    }
}

/// The boards the leg rule plays: before start of combat when every
/// start-of-combat source on them is played, else today's boards.
pub fn leg_rule(c: &SocCombat) -> Option<&[Team; 2]> {
    let before = c.before.as_ref()?;
    Coverage::of_teams(&before[0], &before[1], true)
        .not_modelled
        .is_empty()
        .then_some(before)
}

/// Scores of one way of playing, over the combats with a result.
#[derive(Default)]
struct Tally {
    combats: u32,
    blocked: u32,
    items: Vec<(Prediction, CombatResult)>,
    /// Card ids that blocked a combat -> combats.
    blockers: BTreeMap<String, u32>,
}

impl Tally {
    fn add(&mut self, played: Played, result: CombatResult) {
        self.combats += 1;
        match played {
            Played::Odds(p) => self.items.push((p, result)),
            Played::Blocked(c) => {
                self.blocked += 1;
                for id in c.not_modelled {
                    *self.blockers.entry(id).or_insert(0) += 1;
                }
            }
        }
    }

    fn line(&self, out: &mut String, mode: &str, label: &str) {
        let r: Report = score(&self.items, 5);
        let share = if self.combats > 0 {
            100.0 * f64::from(self.blocked) / f64::from(self.combats)
        } else {
            0.0
        };
        let _ = writeln!(
            out,
            "| {mode} | {label} | {} | {} ({share:.0}%) | {} | {:.0}% | {:.3} | {} |",
            self.combats,
            self.blocked,
            r.combats,
            r.mean_p_actual * 100.0,
            r.brier,
            if r.combats > 0 {
                format!("{} of {}", r.top_pick_hits, r.combats)
            } else {
                "-".into()
            }
        );
    }
}

/// One mode's combats played the three ways.
#[derive(Default)]
struct ModeRun {
    today: Tally,
    kits: Tally,
    rule: Tally,
    /// Combats the leg rule played from before start of combat; of them,
    /// those whose boards differ from today's and those with an effect from
    /// a hand or a Secret; combats with every leg's boards before.
    from_before: u32,
    differs: u32,
    hidden: u32,
    with_before: u32,
    /// What kept combats on today's boards (card id -> combats), and one
    /// line per combat moved whose boards differ.
    kept: BTreeMap<String, u32>,
    moved: Vec<String>,
    /// How long each run that gave odds took (microseconds), as the
    /// overlay's two moments (D-165): with kits at the first attack, and the
    /// leg rule's from before start of combat with it played.
    micros_kits: Vec<u128>,
    micros_before: Vec<u128>,
}

/// `play`, and how long it took in microseconds.
fn timed(teams: &[Team; 2], start_of_combat: bool, options: &Options) -> (Played, u128) {
    let start = Instant::now();
    let played = play(teams, start_of_combat, options);
    (played, start.elapsed().as_micros())
}

fn run_mode(combats: &[SocCombat], duos: bool, options: &Options) -> ModeRun {
    let mut m = ModeRun::default();
    for c in combats.iter().filter(|c| c.duos == duos) {
        let bare = [without_kits(&c.after[0]), without_kits(&c.after[1])];
        m.today.add(play(&bare, false, options), c.result);
        let (played, micros) = timed(&c.after, false, options);
        if matches!(played, Played::Odds(_)) {
            m.micros_kits.push(micros);
        }
        m.kits.add(played, c.result);
        if let Some(before) = &c.before {
            m.with_before += 1;
            for id in Coverage::of_teams(&before[0], &before[1], true).not_modelled {
                *m.kept.entry(id).or_insert(0) += 1;
            }
        }
        let Some(before) = leg_rule(c) else {
            m.rule.add(play(&c.after, false, options), c.result);
            continue;
        };
        m.from_before += 1;
        m.hidden += u32::from(!c.hidden.is_empty());
        let (played, micros) = timed(before, true, options);
        if matches!(played, Played::Odds(_)) {
            m.micros_before.push(micros);
        }
        if *before != c.after {
            m.differs += 1;
            m.moved.push(moved_line(c, before, &played, options));
        }
        m.rule.add(played, c.result);
    }
    m
}

fn write_mode(out: &mut String, mode: &str, m: &ModeRun) {
    m.today.line(out, mode, "today");
    m.kits.line(out, mode, "with kits");
    m.rule.line(out, mode, "leg rule");
    let _ = writeln!(
        out,
        "| {mode} | played from before start of combat | {} (boards differ: {}; with an effect \
         from a hand or a Secret: {}; combats with every leg's boards before: {}) | | | | | |",
        m.from_before, m.differs, m.hidden, m.with_before
    );
    for (label, t) in [("with kits", &m.kits), ("leg rule", &m.rule)] {
        if !t.blockers.is_empty() {
            let _ = writeln!(
                out,
                "\n{mode}, {label}, blocked by: {}",
                counts(&t.blockers, 30)
            );
        }
    }
}

/// One mode's speed per combat, as the overlay's two moments (D-165).
fn speed_line(mode: &str, m: &ModeRun) -> String {
    let speed = |micros: &[u128]| {
        let max = micros.iter().max().copied().unwrap_or(0) as f64 / 1000.0;
        let mean = micros.iter().sum::<u128>() as f64 / 1000.0 / micros.len().max(1) as f64;
        format!(
            "{mean:.1} ms mean, {max:.1} ms at most ({} combats)",
            micros.len()
        )
    };
    format!(
        "{mode}, speed per combat: with kits at the first attack {}; from before start of \
         combat with it played {}.",
        speed(&m.micros_kits),
        speed(&m.micros_before)
    )
}

/// `card id` n, most first.
fn counts(map: &BTreeMap<String, u32>, at_most: usize) -> String {
    let mut rows: Vec<(&String, &u32)> = map.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let ids: Vec<String> = rows
        .iter()
        .take(at_most)
        .map(|(id, n)| format!("`{id}` {n}"))
        .collect();
    if ids.is_empty() {
        "none".into()
    } else {
        ids.join(", ")
    }
}

/// The report: the three ways, per mode, the combats moved and what kept
/// combats on today's boards.
pub fn render_start_of_combat(combats: &[SocCombat], options: &Options) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n## Start of combat, hero powers and trinkets (T-628)\n"
    );
    let _ = writeln!(
        out,
        "Each combat with a result played three ways, {} trials, seed {}: today's boards at \
         the first attack (start of combat off, no hero powers or trinkets); the same with each \
         hero's powers and trinkets; the leg rule (the boards before start of combat with it \
         played, when every start-of-combat source on those boards and kits is played, else the \
         second way). A card, power or trinket not modelled blocks, as in the app.\n",
        options.trials, options.seed
    );
    let _ = writeln!(
        out,
        "| Mode | Way | With a result | Blocked | Scored | Mean p(real) | Brier | Most likely = real |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|");
    let mut kept: BTreeMap<String, u32> = BTreeMap::new();
    let mut moved: Vec<String> = Vec::new();
    let mut speeds: Vec<String> = Vec::new();
    for (duos, mode) in [(false, "Solo"), (true, "Duos")] {
        let m = run_mode(combats, duos, options);
        write_mode(&mut out, mode, &m);
        speeds.push(speed_line(mode, &m));
        for (id, n) in &m.kept {
            *kept.entry(id.clone()).or_insert(0) += n;
        }
        moved.extend(m.moved);
    }
    let _ = writeln!(out);
    for line in &speeds {
        let _ = writeln!(out, "{line}");
    }
    if !moved.is_empty() {
        let _ = writeln!(
            out,
            "\nCombats played from before start of combat whose boards differ from today's \
             (game, round: start-of-combat cards; p(real) today -> leg rule):\n"
        );
        for line in &moved {
            let _ = writeln!(out, "- {line}");
        }
    }
    let _ = writeln!(
        out,
        "\nWhat keeps combats on today's boards (a start-of-combat source not played yet, or \
         not reviewed; card id: combats): {}",
        counts(&kept, 30)
    );
    out
}

/// One combat moved to the boards before start of combat, for the list.
fn moved_line(c: &SocCombat, before: &[Team; 2], played: &Played, options: &Options) -> String {
    let p_real = |p: &Played| match p {
        Played::Odds(p) => format!("{:.0}%", p.of(c.result) * 100.0),
        Played::Blocked(_) => "blocked".into(),
    };
    let today = play(&c.after, false, options);
    let mut ids: Vec<String> = Vec::new();
    for band in before.iter().flat_map(Team::boards) {
        for m in &band.minions {
            let soc = crate::cards::has_start_of_combat(&m.card_id)
                || m.gifts.iter().any(|g| crate::gifts::has_start_of_combat(g));
            if soc && !ids.contains(&m.card_id) {
                ids.push(m.card_id.clone());
            }
        }
    }
    let hidden = if c.hidden.is_empty() {
        String::new()
    } else {
        format!(" (from a hand or a Secret: {})", c.hidden.join(", "))
    };
    format!(
        "game {} round {}: {}{hidden}; {} -> {}",
        c.game,
        c.round,
        if ids.is_empty() {
            "none".into()
        } else {
            ids.join(", ")
        },
        p_real(&today),
        p_real(played)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Minion;

    fn kit_card(id: &str, target: Option<i64>) -> bg_parser::report::KitCard {
        bg_parser::report::KitCard {
            card_id: id.into(),
            slot: 1,
            counters: [3, 0, 0, 0, 0],
            target,
            held: None,
        }
    }

    #[test]
    fn a_named_minion_becomes_a_board_index() {
        let kit = HeroKit {
            powers: vec![
                kit_card("A", Some(1)),
                kit_card("B", Some(3)),
                kit_card("C", Some(0)),
                kit_card("D", Some(i64::MIN)),
                kit_card("E", None),
            ],
            trinkets: vec![kit_card("BG30_Trinket_1st", None)],
        };
        let kit = engine_kit(&kit, 7, true, &CardDb::default());
        let targets: Vec<_> = kit.powers.iter().map(|p| p.target).collect();
        assert_eq!(targets, [Some(0), Some(2), None, None, None]);
        assert_eq!(kit.powers[0].counters, [3, 0, 0, 0, 0]);
        assert_eq!(kit.trinkets[0].slot, 1);
    }

    fn band(minions: Vec<Minion>, powers: &[&str]) -> Warband {
        Warband {
            kit: Some(Kit {
                powers: powers.iter().map(|id| KitCard::new(*id)).collect(),
                trinkets: Vec::new(),
                round: Some(7),
            }),
            tavern_tier: Some(1),
            ..Warband::new(minions)
        }
    }

    fn combat(before: Option<[Team; 2]>, hidden: &[&str]) -> SocCombat {
        let quiet = Team::solo(band(vec![Minion::new("BGS_119", 1, 1)], &[]));
        SocCombat {
            game: 1,
            round: 1,
            duos: false,
            after: [quiet.clone(), quiet],
            before,
            result: CombatResult::Won,
            hidden: hidden.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn the_leg_rule_takes_the_boards_before_only_when_every_source_is_played() {
        let plain = Team::solo(band(vec![Minion::new("BGS_119", 2, 2)], &[]));
        let quiet = [plain.clone(), plain.clone()];
        assert_eq!(leg_rule(&combat(Some(quiet.clone()), &[])), Some(&quiet));
        // Rapid Reanimation acts at the start of combat and is not played yet.
        let wax = Team::solo(band(
            vec![Minion::new("BGS_119", 2, 2)],
            &["BG25_HERO_103p"],
        ));
        assert_eq!(leg_rule(&combat(Some([wax, plain.clone()]), &[])), None);
        // No boards before start of combat: today's.
        assert_eq!(leg_rule(&combat(None, &[])), None);
        // A hand's effect is on no board: the boards before are still taken.
        assert_eq!(
            leg_rule(&combat(Some(quiet.clone()), &["BG32_330"])),
            Some(&quiet)
        );
    }
}
