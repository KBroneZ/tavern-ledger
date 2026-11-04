//! The harness's inventory of hero powers and trinkets (T-631): for each
//! one that acts in combat or that nobody reviewed, in how many logged
//! combats it was on a board that fought, how many it blocks (alone or with
//! other cards), how many it makes approximate, and how many it keeps on
//! today's boards under the leg rule. A combat can have several blockers,
//! so "alone" is the count that playing this one card would unblock.
//!
//! Counts and card ids only, never a name.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::harness_soc::SocCombat;
use crate::kit::{self, KitEffect};
use crate::model::Team;
use crate::trials::Coverage;

/// One hero power or trinket over one mode's combats.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KitRow {
    /// Boards that fought with it (one per leg and side).
    pub boards: u32,
    /// Combats with it on a board that fought.
    pub combats: u32,
    /// Combats it blocks on the boards at the first attack with kits, and of
    /// them those it alone blocks.
    pub blocked: u32,
    pub blocked_alone: u32,
    /// Combats it makes approximate there.
    pub approximate: u32,
    /// Combats the leg rule keeps on today's boards because of it, and of
    /// them those it alone keeps there.
    pub kept: u32,
    pub kept_alone: u32,
}

/// The ids of every hero power and trinket on the team's boards, one entry
/// per board that has it.
fn kit_ids(team: &Team) -> Vec<String> {
    let mut out = Vec::new();
    for band in std::iter::once(&team.first).chain(team.second.as_ref()) {
        let Some(kit) = &band.kit else { continue };
        let ids: BTreeSet<&String> = kit.cards().map(|c| &c.card_id).collect();
        out.extend(ids.into_iter().cloned());
    }
    out
}

/// True when the card can change a combat or nobody reviewed it.
fn listed(id: &str) -> bool {
    kit::effect(id) != Some(KitEffect::None)
}

/// The inventory of one mode's combats, by card id.
pub fn kit_rows(combats: &[SocCombat], duos: bool) -> BTreeMap<String, KitRow> {
    let mut rows: BTreeMap<String, KitRow> = BTreeMap::new();
    for c in combats.iter().filter(|c| c.duos == duos) {
        let per_board: Vec<String> = c.after.iter().flat_map(kit_ids).collect();
        let ids: BTreeSet<&String> = per_board.iter().filter(|id| listed(id)).collect();
        if ids.is_empty() {
            continue;
        }
        let with_kits = Coverage::of_teams(&c.after[0], &c.after[1], false);
        let before = c
            .before
            .as_ref()
            .map(|b| Coverage::of_teams(&b[0], &b[1], true).not_modelled);
        for id in ids {
            let row = rows.entry(id.clone()).or_default();
            row.combats += 1;
            row.boards += u32::try_from(per_board.iter().filter(|b| *b == id).count()).unwrap_or(0);
            if with_kits.not_modelled.contains(id) {
                row.blocked += 1;
                row.blocked_alone += u32::from(with_kits.not_modelled.len() == 1);
            }
            if with_kits.approximate.iter().any(|(a, _)| a == id) {
                row.approximate += 1;
            }
            if let Some(kept) = &before {
                if kept.contains(id) {
                    row.kept += 1;
                    row.kept_alone += u32::from(kept.len() == 1);
                }
            }
        }
    }
    rows
}

/// The inventory as a table per mode, most blocking first.
pub fn render_kit_inventory(combats: &[SocCombat]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n## Hero powers and trinkets that act in combat (T-631)\n"
    );
    let _ = writeln!(
        out,
        "Every hero power or trinket on a board that fought whose text acts in combat, or that \
         the simulator has not reviewed: combats with it; blocked with kits at the first attack \
         (of them, by it alone); approximate there; kept on today's boards by the leg rule (of \
         them, by it alone). A combat counts once per card.\n"
    );
    for (duos, mode) in [(false, "Solo"), (true, "Duos")] {
        let rows = kit_rows(combats, duos);
        let n = combats.iter().filter(|c| c.duos == duos).count();
        if rows.is_empty() {
            let _ = writeln!(out, "{mode} ({n} combats): none.\n");
            continue;
        }
        let _ = writeln!(out, "{mode} ({n} combats):\n");
        let _ = writeln!(
            out,
            "| Card | When | Boards | Combats | Blocked (alone) | Approximate | Kept (alone) |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|---|---|");
        let mut sorted: Vec<(&String, &KitRow)> = rows.iter().collect();
        sorted.sort_by(|a, b| {
            (b.1.blocked, b.1.kept, b.1.combats)
                .cmp(&(a.1.blocked, a.1.kept, a.1.combats))
                .then(a.0.cmp(b.0))
        });
        for (id, r) in sorted {
            let _ = writeln!(
                out,
                "| `{id}` | {} | {} | {} | {} ({}) | {} | {} ({}) |",
                when(id),
                r.boards,
                r.combats,
                r.blocked,
                r.blocked_alone,
                r.approximate,
                r.kept,
                r.kept_alone
            );
        }
        let _ = writeln!(out);
    }
    out
}

fn when(id: &str) -> &'static str {
    match kit::effect(id) {
        None => "not reviewed",
        Some(KitEffect::StartOfCombat) => "start of combat",
        Some(KitEffect::StartAndInCombat) => "start and during",
        Some(KitEffect::InCombat) => "during combat",
        Some(KitEffect::TwiceHeroPower) => "hero power twice",
        Some(KitEffect::InCombatBlocks) => "during combat, blocks",
        Some(KitEffect::None) => "no combat effect",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::CombatResult;
    use crate::model::{Kit, KitCard, Minion, Warband};

    fn band(powers: &[&str], trinkets: &[&str]) -> Warband {
        Warband {
            kit: Some(Kit {
                powers: powers.iter().map(|id| KitCard::new(*id)).collect(),
                trinkets: trinkets.iter().map(|id| KitCard::new(*id)).collect(),
                round: Some(5),
            }),
            ..Warband::new(vec![Minion::new("BGS_119", 2, 2)])
        }
    }

    fn combat(player: Warband, opponent: Warband) -> SocCombat {
        let team = |b: Warband| Team {
            first: b,
            second: None,
        };
        let after = [team(player), team(opponent)];
        SocCombat {
            game: 1,
            round: 5,
            duos: false,
            before: Some(after.clone()),
            after,
            result: CombatResult::Won,
            hidden: Vec::new(),
        }
    }

    #[test]
    fn a_power_nobody_reviewed_is_counted_as_blocking_and_alone() {
        let c = combat(band(&["BG99_HERO_999p"], &[]), band(&[], &[]));
        let rows = kit_rows(&[c], false);
        let r = &rows["BG99_HERO_999p"];
        assert_eq!((r.combats, r.blocked, r.blocked_alone), (1, 1, 1));
        assert_eq!((r.kept, r.kept_alone), (1, 1));
    }

    #[test]
    fn two_blockers_in_one_combat_are_not_alone() {
        let c = combat(
            band(&["BG99_HERO_999p"], &[]),
            band(&[], &["BG99_MagicItem_999"]),
        );
        let rows = kit_rows(&[c], false);
        assert_eq!(rows["BG99_HERO_999p"].blocked_alone, 0);
        assert_eq!(rows["BG99_MagicItem_999"].blocked, 1);
    }

    #[test]
    fn a_card_with_no_combat_effect_is_left_out_and_one_during_combat_is_approximate() {
        // Sharpen Blades (shop only) and Blood Golem Sticker (during combat).
        let c = combat(
            band(&["TB_BaconShop_HP_001"], &["BG30_MagicItem_442"]),
            band(&[], &[]),
        );
        let rows = kit_rows(&[c], false);
        assert!(!rows.contains_key("TB_BaconShop_HP_001"));
        let r = &rows["BG30_MagicItem_442"];
        assert_eq!((r.blocked, r.approximate, r.kept), (0, 1, 0));
    }

    #[test]
    fn a_start_of_combat_card_not_played_blocks_only_before_start_of_combat() {
        // Embrace Your Rage: its effect is already on a first-attack board.
        let c = combat(band(&["TB_BaconShop_HP_103"], &[]), band(&[], &[]));
        let r = &kit_rows(&[c], false)["TB_BaconShop_HP_103"];
        assert_eq!((r.blocked, r.kept, r.kept_alone), (0, 1, 1));
    }

    #[test]
    fn the_other_mode_is_not_counted() {
        let c = combat(band(&["BG99_HERO_999p"], &[]), band(&[], &[]));
        assert!(kit_rows(&[c], true).is_empty());
        assert!(render_kit_inventory(&[]).contains("Solo (0 combats): none."));
    }
}
