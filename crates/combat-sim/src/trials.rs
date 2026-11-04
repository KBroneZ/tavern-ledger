//! Many random combats between the same two warbands: the odds to win, tie or
//! lose and how much damage each result deals.

use std::collections::BTreeMap;

use crate::cards::{self, Action, Fidelity, Trigger};
use crate::combat::{play, CombatResult, Prepared};
use crate::gifts;
use crate::kit::{self, KitEffect};
use crate::kit_effects;
use crate::kit_passive;
use crate::model::{BoardError, Keywords, Kit, KitCard, Minion, Team, Tribes, Warband};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub trials: u32,
    pub seed: u64,
    /// Play start-of-combat effects. False for a board taken after them (the
    /// logged boards are taken at the first attack).
    pub start_of_combat: bool,
    /// Play cards the engine does not know as plain minions and return odds
    /// marked "not modelled" instead of refusing. For measuring the engine
    /// only, never for odds shown to a player.
    pub allow_unmodelled: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            trials: 10_000,
            seed: 0x5EED,
            start_of_combat: false,
            allow_unmodelled: false,
        }
    }
}

/// Which cards on the two boards the engine cannot play exactly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Card ids (and Dark Gift ids) with no rules, or with a start-of-combat
    /// effect the engine does not play when start of combat is on.
    pub not_modelled: Vec<String>,
    /// Card ids (and Dark Gift ids) played with a known gap, and the gap.
    pub approximate: Vec<(String, &'static str)>,
    /// The ids of [`Coverage::not_modelled`] that are hero powers or
    /// trinkets (T-628), for their reasons.
    pub kit: Vec<String>,
    /// Why an id of [`Coverage::not_modelled`] blocks when it is not what
    /// [`Coverage::reasons`] would say from the id alone: something the card
    /// or power needs beside the board is not given (T-628 round 3).
    pub why: Vec<(String, &'static str)>,
}

/// A board and its teammate's board (`None` in Solo or when not given).
type WithMate<'a> = (&'a Warband, Option<&'a Warband>);

/// Each board of a team with its teammate's.
fn with_mates(team: &Team) -> Vec<WithMate<'_>> {
    let mut out = vec![(&team.first, team.second.as_ref())];
    if let Some(second) = &team.second {
        out.push((second, Some(&team.first)));
    }
    out
}

impl Coverage {
    pub fn of(player: &Warband, opponent: &Warband, start_of_combat: bool) -> Coverage {
        let mut cov = Coverage::of_boards(&[(player, None), (opponent, None)], start_of_combat);
        if start_of_combat {
            cov.add_side_order(player, opponent);
        }
        cov
    }

    /// Every board of both teams (Duos). A board that comes in later plays
    /// only its own start of combat, so the order between the two sides is
    /// a question for the first boards only (in the rare combat where both
    /// teammates' boards come in at once it is one for them too, and is not
    /// marked).
    pub fn of_teams(player: &Team, opponent: &Team, start_of_combat: bool) -> Coverage {
        let mut boards = with_mates(player);
        boards.extend(with_mates(opponent));
        let mut cov = Coverage::of_boards(&boards, start_of_combat);
        if start_of_combat {
            cov.add_side_order(&player.first, &opponent.first);
        }
        cov
    }

    fn of_boards(boards: &[WithMate<'_>], start_of_combat: bool) -> Coverage {
        let mut cov = Coverage::default();
        // A minion a played hero power will bring in (Lock and Load) is
        // checked like the board's own.
        let minions = boards.iter().flat_map(|(b, mate)| {
            b.minions
                .iter()
                .chain(held_minions(b, start_of_combat))
                .map(move |m| (m, *b, *mate))
        });
        for (m, band, mate) in minions {
            let id = &m.card_id;
            match cards::rules(id) {
                None => push_unique(&mut cov.not_modelled, id.clone()),
                Some(rules) => {
                    // What its start of combat did is needed later and a board
                    // taken after it no longer shows it (Stitched Salvager).
                    if let Some(why) = cards::after_start_refusal(id).filter(|_| !start_of_combat) {
                        cov.refuse(id, why);
                    }
                    if start_of_combat && mate.is_none() && copies_teammate(&rules) {
                        cov.refuse(id, kit::TEAMMATE_NOT_GIVEN);
                    }
                    // A buff that grows during the game needs the minion's
                    // own numbers (T-628).
                    let numbers_missing = cards::start_of_combat_scales(id) && m.counters == [0, 0];
                    let soc_gap = start_of_combat
                        && (rules.triggers.contains(&Trigger::StartOfCombatUnmodelled)
                            || numbers_missing);
                    if soc_gap {
                        push_unique(&mut cov.not_modelled, id.clone());
                    }
                    if let Fidelity::Approximate(why) = rules.fidelity {
                        let unplayed = !start_of_combat && gap_at_start_only(why, &rules);
                        if !played_with(why, band)
                            && !unplayed
                            && !cov.approximate.iter().any(|(c, _)| c == id)
                        {
                            cov.approximate.push((id.clone(), why));
                        }
                    }
                }
            }
        }
        let all = boards
            .iter()
            .flat_map(|(b, _)| b.minions.iter().chain(held_minions(b, start_of_combat)));
        for m in all {
            cov.add_gifts(m, start_of_combat);
        }
        for (band, mate) in boards {
            if let Some(kit) = &band.kit {
                cov.add_kit(kit, start_of_combat, *mate, band);
            }
        }
        for (band, _) in boards {
            let grants = reborn_grants(band);
            for m in &band.minions {
                let may_reborn = m.keywords.contains(Keywords::REBORN)
                    || grants.iter().any(|t| *t == Tribes::NONE || m.tribes.is(*t));
                if may_reborn
                    && m.base.is_none()
                    && !cov.approximate.iter().any(|(c, _)| *c == m.card_id)
                {
                    cov.approximate.push((
                        m.card_id.clone(),
                        "may come back with Reborn; card stats unknown",
                    ));
                }
            }
        }
        cov
    }

    /// A minion's Dark Gifts, by the same rules as cards: a gift the engine
    /// does not know (or whose start-of-combat effect it does not play, when
    /// start of combat is on) blocks; an approximate one is named.
    fn add_gifts(&mut self, m: &Minion, start_of_combat: bool) {
        for id in &m.gifts {
            let Some(gift) = gifts::gift(id) else {
                push_unique(&mut self.not_modelled, id.clone());
                continue;
            };
            if start_of_combat && gift.triggers.contains(&Trigger::StartOfCombatUnmodelled) {
                push_unique(&mut self.not_modelled, id.clone());
            }
            if let Fidelity::Approximate(why) = gifts::fidelity(gift, &m.card_id) {
                if !self.approximate.iter().any(|(c, _)| c == id) {
                    self.approximate.push((id.clone(), why));
                }
            }
        }
    }

    /// `id` blocks the odds for `why`.
    fn refuse(&mut self, id: &str, why: &'static str) {
        push_unique(&mut self.not_modelled, id.to_string());
        if !self.why.iter().any(|(c, _)| c == id) {
            self.why.push((id.to_string(), why));
        }
    }

    /// A board's hero powers and trinkets (T-628): one not reviewed blocks;
    /// one that acts at the start of combat blocks when start of combat is
    /// played unless the engine plays it ([`kit_effects`]; with start of
    /// combat off the board was taken after it), or when something it needs
    /// beside the board is not given; one that acts later in the combat is
    /// approximate unless the engine plays it ([`kit_passive`], with its own
    /// gap if any; one played only before start of combat stays approximate
    /// on a board taken after it).
    fn add_kit(
        &mut self,
        kit: &Kit,
        start_of_combat: bool,
        mate: Option<&Warband>,
        band: &Warband,
    ) {
        let power_in_combat = kit.powers.iter().any(|p| kit::acts_in_combat(&p.card_id));
        for card in kit.cards() {
            let id = &card.card_id;
            let effect = kit::effect(id);
            let play = kit_effects::played(id);
            if let Some(mark) = play
                .and_then(kit_effects::mark)
                .filter(|_| !start_of_combat)
            {
                self.add_marked(id, mark, band);
                continue;
            }
            if let Some(play) = play.filter(|_| start_of_combat) {
                self.add_played_at_start(play, card, kit.round, mate);
                continue;
            }
            if effect == Some(KitEffect::InCombatBlocks) {
                self.refuse(id, kit::IN_COMBAT_BLOCKED);
                push_unique(&mut self.kit, id.clone());
                continue;
            }
            let refuse = match effect {
                None => true,
                Some(KitEffect::StartOfCombat | KitEffect::StartAndInCombat) => {
                    start_of_combat && play.is_none()
                }
                Some(_) => false,
            };
            if refuse {
                push_unique(&mut self.not_modelled, id.clone());
                push_unique(&mut self.kit, id.clone());
                continue;
            }
            // Played during combat (T-628 round 4): only its own gap, if any.
            let passive =
                kit_passive::passive(id).filter(|p| start_of_combat || !p.before_start_only);
            if let Some(p) = passive {
                if let Fidelity::Approximate(why) = p.fidelity {
                    if !played_with(why, band) {
                        self.approximate_once(id, why);
                    }
                }
                continue;
            }
            let gap = match effect {
                Some(KitEffect::InCombat | KitEffect::StartAndInCombat) => {
                    Some(kit::IN_COMBAT_NOT_PLAYED)
                }
                Some(KitEffect::TwiceHeroPower) if power_in_combat => Some(kit::TWICE_NOT_PLAYED),
                _ => None,
            };
            if let Some(why) = gap {
                self.approximate_once(id, why);
            }
        }
    }

    /// A card whose start of combat marks a minion, on a board taken after
    /// it (T-631): the mark shows which minion it chose; without the mark
    /// the card's effect is unknown and the odds are blocked.
    fn add_marked(&mut self, id: &str, mark: &str, band: &Warband) {
        let shown = band
            .minions
            .iter()
            .any(|m| m.gifts.iter().any(|g| g == mark));
        if !shown {
            self.refuse(id, kit::MARK_NOT_GIVEN);
            push_unique(&mut self.kit, id.to_string());
        }
    }

    /// A card the engine plays at the start of combat, with start of combat
    /// played: blocked when what it needs is not given, else its own gap.
    fn add_played_at_start(
        &mut self,
        play: &kit_effects::KitPlay,
        card: &KitCard,
        round: Option<u32>,
        mate: Option<&Warband>,
    ) {
        let id = &card.card_id;
        if let Some(why) = unmet(play, card, round, mate) {
            self.refuse(id, why);
            push_unique(&mut self.kit, id.clone());
            return;
        }
        if kit_effects::acts(play, &card.counters, round) == Some(false) {
            return;
        }
        let why = match play.fidelity {
            Fidelity::Approximate(why) => Some(why),
            Fidelity::Exact => None,
        };
        let why = why.or_else(|| {
            kit_effects::counter_missing(play, &card.counters).then_some(kit::COUNTER_NOT_GIVEN)
        });
        if let Some(why) = why {
            self.approximate_once(id, why);
        }
    }

    /// Names `id` approximate for `why`, once.
    fn approximate_once(&mut self, id: &str, why: &'static str) {
        if !self.approximate.iter().any(|(c, _)| c == id) {
            self.approximate.push((id.to_string(), why));
        }
    }

    /// Both sides act at the start of combat: the order between them is not
    /// proven on the user's logs (T-628), so each side's start-of-combat
    /// cards are named approximate.
    fn add_side_order(&mut self, player: &Warband, opponent: &Warband) {
        let (mine, theirs) = (start_of_combat_ids(player), start_of_combat_ids(opponent));
        if mine.is_empty() || theirs.is_empty() {
            return;
        }
        for id in mine.into_iter().chain(theirs) {
            if !self.approximate.iter().any(|(c, _)| *c == id) {
                self.approximate.push((id, SIDE_ORDER));
            }
        }
    }

    pub fn is_exact(&self) -> bool {
        self.not_modelled.is_empty() && self.approximate.is_empty()
    }

    /// Why each card of [`Coverage::not_modelled`] blocks the odds, in the
    /// same order, for the page to show (T-616).
    pub fn reasons(&self) -> Vec<(String, &'static str)> {
        self.not_modelled
            .iter()
            .map(|id| {
                if let Some((_, why)) = self.why.iter().find(|(c, _)| c == id) {
                    return (id.clone(), *why);
                }
                let why = if !self.kit.contains(id) {
                    refusal_reason(id)
                } else if kit::effect(id).is_some() {
                    kit::START_OF_COMBAT_NOT_PLAYED
                } else {
                    kit::NOT_REVIEWED
                };
                (id.clone(), why)
            })
            .collect()
    }
}

/// Why start-of-combat cards are approximate when both sides have some.
pub const SIDE_ORDER: &str =
    "the order of start-of-combat effects between the two sides is not proven";

/// The cards and Dark Gifts of a board that act at the start of combat and
/// that the engine plays.
fn start_of_combat_ids(band: &Warband) -> Vec<String> {
    let mut out = Vec::new();
    for m in &band.minions {
        let acts = cards::rules(&m.card_id).is_some_and(|r| {
            r.triggers
                .iter()
                .any(|t| matches!(t, Trigger::StartOfCombat(_) | Trigger::AttackImmediately(_)))
        });
        if acts {
            push_unique(&mut out, m.card_id.clone());
        }
        for g in m.gifts.iter().filter(|g| gifts::has_start_of_combat(g)) {
            push_unique(&mut out, g.clone());
        }
    }
    let round = band.kit.as_ref().and_then(|k| k.round);
    for card in band.kit.iter().flat_map(Kit::cards) {
        let acts = kit_effects::played(&card.card_id)
            .is_some_and(|p| kit_effects::acts(p, &card.counters, round) != Some(false));
        if acts {
            push_unique(&mut out, card.card_id.clone());
        }
    }
    out
}

/// The minions the board's played hero powers hold and will bring in this
/// combat (Lock and Load), when start of combat is played.
fn held_minions(band: &Warband, start_of_combat: bool) -> Vec<&Minion> {
    let Some(kit) = band.kit.as_ref().filter(|_| start_of_combat) else {
        return Vec::new();
    };
    kit.cards()
        .filter(|c| {
            kit_effects::played(&c.card_id).is_some_and(|p| {
                p.space == Some(kit_effects::SpaceAction::FireHeld)
                    && kit_effects::acts(p, &c.counters, kit.round) == Some(true)
            })
        })
        .filter_map(|c| c.held.as_ref())
        .collect()
}

/// What a played power needs beside the board and is not given, if any
/// (nothing when it is known not to act this combat).
fn unmet(
    play: &kit_effects::KitPlay,
    card: &KitCard,
    round: Option<u32>,
    mate: Option<&Warband>,
) -> Option<&'static str> {
    let needs = kit_effects::needs(play);
    match kit_effects::acts(play, &card.counters, round) {
        None => return Some(kit::ROUND_NOT_GIVEN),
        Some(false) => return None,
        Some(true) => {}
    }
    if needs.target && card.target.is_none() {
        Some(kit::TARGET_NOT_GIVEN)
    } else if needs.held && card.held.is_none() {
        Some(kit::HELD_NOT_GIVEN)
    } else if needs.teammate && mate.is_none() {
        Some(kit::TEAMMATE_NOT_GIVEN)
    } else {
        None
    }
}

/// True when the card copies a minion of its teammate's board (Sandy).
fn copies_teammate(rules: &cards::CardRules) -> bool {
    rules
        .triggers
        .iter()
        .any(|t| t.actions().contains(&Action::BecomeTeammateCopy))
}

/// Why the engine refuses a card or Dark Gift: a start-of-combat effect it
/// does not play on a board taken before it, or the reason written for the
/// card (T-616).
pub fn refusal_reason(id: &str) -> &'static str {
    // Played whenever start of combat is: refused only on a board taken
    // after it (T-628 round 3).
    if let Some(why) = cards::after_start_refusal(id) {
        return why;
    }
    if gifts::gift(id).is_some() || cards::rules(id).is_some() {
        return START_OF_COMBAT_GAP;
    }
    if id.starts_with("BG36_MidGameEffect") {
        return GIFT_NOT_REVIEWED;
    }
    cards::refusal(id)
}

pub(crate) const START_OF_COMBAT_GAP: &str =
    "its Start of Combat effect is not played, and this board was taken before it";
const GIFT_NOT_REVIEWED: &str = "the simulator has not reviewed this Dark Gift yet";

/// True when the card's only gap is a game-long value its board brings
/// (T-610): Blood Gem or Tavern spell upgrades above zero. Zero is unknown
/// (a board without values), so the card stays approximate then.
fn played_with(why: &str, band: &Warband) -> bool {
    let values = &band.values;
    match why {
        w if w == crate::cards_round1::BLOOD_GEM => values.blood_gem != (0, 0),
        w if w == crate::cards_round2::SPELL => values.spell != (0, 0),
        w if w == crate::cards_round1::DEITY => values.deity.known(),
        w if w == crate::cards_round1::LOBSTER => lobster_known(band),
        _ => false,
    }
}

/// Whether a Lobster on the board shows the side's Lobster buff (T-645).
pub(crate) fn lobster_known(band: &Warband) -> bool {
    band.minions
        .iter()
        .any(|m| cards::lobster_shown(&m.card_id, m.counters).is_some())
}

/// True when the card's gap is about how it plays its start of combat and
/// that is its only effect (Runic Arcanist's spell values, Sandy's copy;
/// T-628): on a board taken after start of combat it has acted already.
fn gap_at_start_only(why: &str, rules: &cards::CardRules) -> bool {
    (why == crate::cards_round2::SPELL || why == cards::SANDY_COPY)
        && !rules.triggers.is_empty()
        && rules
            .triggers
            .iter()
            .all(|t| matches!(t, Trigger::StartOfCombat(_)))
}

/// Tribes a card on this warband can give Reborn to in combat.
fn reborn_grants(band: &Warband) -> Vec<Tribes> {
    let mut out = Vec::new();
    for m in &band.minions {
        let Some(rules) = cards::rules(&m.card_id) else {
            continue;
        };
        for t in rules.triggers {
            for a in t.actions() {
                if let Action::GiveKeyword { keyword, tribe, .. } = a {
                    if keyword.contains(Keywords::REBORN) {
                        out.push(*tribe);
                    }
                }
            }
        }
    }
    out
}

fn push_unique(list: &mut Vec<String>, id: String) {
    if !list.contains(&id) {
        list.push(id);
    }
}

/// The spread of results over all trials, for the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Odds {
    pub trials: u32,
    pub won: u32,
    pub tied: u32,
    pub lost: u32,
    /// Damage dealt to the opponent's hero -> trials, over the won trials
    /// where every tier is known.
    pub damage_dealt: BTreeMap<i32, u32>,
    /// Damage taken -> trials, over the lost trials where every tier is known.
    pub damage_taken: BTreeMap<i32, u32>,
    /// Won or lost trials whose damage is unknown (a tier was missing).
    pub damage_unknown: u32,
    /// Trials that hit the turn limit (counted as ties).
    pub stalled: u32,
    pub coverage: Coverage,
}

impl Odds {
    /// Adds another run over the same boards (another seed): the counts and
    /// spreads add up.
    pub fn merge(&mut self, other: &Odds) {
        self.trials += other.trials;
        self.won += other.won;
        self.tied += other.tied;
        self.lost += other.lost;
        for (mine, theirs) in [
            (&mut self.damage_dealt, &other.damage_dealt),
            (&mut self.damage_taken, &other.damage_taken),
        ] {
            for (d, n) in theirs {
                *mine.entry(*d).or_insert(0) += n;
            }
        }
        self.damage_unknown += other.damage_unknown;
        self.stalled += other.stalled;
    }

    pub fn share(&self, result: CombatResult) -> f64 {
        let n = match result {
            CombatResult::Won => self.won,
            CombatResult::Tie => self.tied,
            CombatResult::Lost => self.lost,
        };
        f64::from(n) / f64::from(self.trials.max(1))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Odds the engine can give (check `coverage` for approximations).
    Odds(Odds),
    /// A card on the boards is not modelled: no number, by design.
    NotModelled(Coverage),
}

/// Plays `options.trials` combats between the player's warband and the
/// opponent's. Deterministic for a given seed.
pub fn simulate(
    player: &Warband,
    opponent: &Warband,
    options: &Options,
) -> Result<Verdict, BoardError> {
    player.validate()?;
    opponent.validate()?;
    if options.trials == 0 {
        return Err(BoardError::NoTrials);
    }
    let coverage = Coverage::of(player, opponent, options.start_of_combat);
    if !coverage.not_modelled.is_empty() && !options.allow_unmodelled {
        return Ok(Verdict::NotModelled(coverage));
    }
    let prepared = Prepared::new(player, opponent, options.start_of_combat);
    Ok(Verdict::Odds(run_trials(&prepared, coverage, options)))
}

/// A Duos combat (T-608): each team's first board fights; once one is gone
/// its teammate's board comes in, and the combat ends when a team has no
/// board left. The damage is the tier of the hero whose board is on the
/// field at the end plus the tiers of every surviving minion of the team,
/// a teammate's board that never came in included. A team without a second
/// board plays as in Solo. Deterministic for a given seed.
pub fn simulate_teams(
    player: &Team,
    opponent: &Team,
    options: &Options,
) -> Result<Verdict, BoardError> {
    player.validate()?;
    opponent.validate()?;
    if options.trials == 0 {
        return Err(BoardError::NoTrials);
    }
    let coverage = Coverage::of_teams(player, opponent, options.start_of_combat);
    if !coverage.not_modelled.is_empty() && !options.allow_unmodelled {
        return Ok(Verdict::NotModelled(coverage));
    }
    let prepared = Prepared::teams(player, opponent, options.start_of_combat);
    Ok(Verdict::Odds(run_trials(&prepared, coverage, options)))
}

fn run_trials(prepared: &Prepared, coverage: Coverage, options: &Options) -> Odds {
    let mut rng = Rng::new(options.seed);
    let mut odds = Odds {
        trials: options.trials,
        won: 0,
        tied: 0,
        lost: 0,
        damage_dealt: BTreeMap::new(),
        damage_taken: BTreeMap::new(),
        damage_unknown: 0,
        stalled: 0,
        coverage,
    };
    for _ in 0..options.trials {
        let outcome = play(prepared, &mut rng);
        if outcome.stalled {
            odds.stalled += 1;
        }
        let spread = match outcome.result {
            CombatResult::Won => {
                odds.won += 1;
                &mut odds.damage_dealt
            }
            CombatResult::Lost => {
                odds.lost += 1;
                &mut odds.damage_taken
            }
            CombatResult::Tie => {
                odds.tied += 1;
                continue;
            }
        };
        match outcome.damage {
            Some(d) => *spread.entry(d).or_insert(0) += 1,
            None => odds.damage_unknown += 1,
        }
    }
    odds
}
