//! Combat odds in the overlay (T-605, D-152). The tracker's loop says what
//! the odds should be worked out from: the boards of the combat the game is
//! playing, and nothing in a shop turn. The simulator runs on its own thread,
//! never the window's, once the boards stop changing for a moment
//! ([`DEBOUNCE`]) and within a time budget ([`BUDGET`]). A reason for no
//! odds (a Duos board not in play yet...) is shown at once. The odds of a
//! combat go when that combat ends and never show in another one. Duos odds
//! (T-608) come the same way, once the teammate's and both opponents' boards
//! have entered play.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bg_parser::report::Status;
use tauri::{AppHandle, Emitter, Manager};
use tracker::odds::{odds_input, run_odds, Gap, LiveFacts, LiveOdds, NoOdds, OddsInput};
use tracker::Watcher;

use crate::overlay::lock;
use crate::{card_data, AppState, LIVE_CHANGED};

/// Quiet time after the last board change before the simulator runs.
pub const DEBOUNCE: Duration = Duration::from_millis(300);
/// Time the simulator may take for one answer (10 000 trials take about
/// 30 ms on the user's PC; fewer are run when the PC is busy).
pub const BUDGET: Duration = Duration::from_millis(150);

/// What the odds are worked out from now: nothing (no game on, switched off,
/// or a shop turn), a reason for no odds, or the input of a run.
pub type Wanted = Option<Result<OddsInput, NoOdds>>;

#[derive(Default)]
pub struct Odds {
    /// Everything that decides what is shown, under one lock so the
    /// simulator's thread and the tracker's loop cannot cross.
    inner: Mutex<Inner>,
    jobs: Mutex<Option<Sender<OddsInput>>>,
}

#[derive(Default)]
struct Inner {
    /// The last [`Wanted`], so only changes are acted on.
    last: Wanted,
    /// The input whose result may still be shown: the last one asked for.
    run: Option<OddsInput>,
    shown: Option<LiveOdds>,
}

/// What a new [`Wanted`] does.
#[derive(Debug, PartialEq)]
pub enum Step {
    Show(Option<LiveOdds>),
    /// Work these odds out; `clear` when what is shown is for another
    /// combat or opponent and must go at once.
    Run {
        input: Box<OddsInput>,
        clear: bool,
    },
}

/// `shown` is what the overlay has.
pub fn step(wanted: &Wanted, shown: Option<&LiveOdds>) -> Step {
    match wanted {
        None => Step::Show(None),
        Some(Err(no)) => Step::Show(Some(no.odds())),
        Some(Ok(input)) => Step::Run {
            input: Box::new(input.clone()),
            clear: shown.is_some_and(|s| {
                s.seat != Some(input.opponent_seat) || s.round != Some(input.round)
            }),
        },
    }
}

/// The odds the app wants now from the game the overlay shows.
pub fn wanted(watcher: &Watcher, on: bool) -> Wanted {
    if !on {
        return None;
    }
    let report = watcher.live_report()?;
    if report.status != Status::Incomplete {
        return None;
    }
    // A shop turn has no odds (D-152).
    let combat_round = watcher.live_combat_round()?;
    let lobby = watcher.live_lobby();
    let live = LiveFacts {
        combat_round: Some(combat_round),
        lobby: &lobby,
        damage_cap: watcher.live_damage_cap(),
    };
    Some(odds_input(report, live))
}

impl Odds {
    pub fn shown(&self) -> Option<LiveOdds> {
        lock(&self.inner).shown.clone()
    }

    /// Acts on what the tracker's loop wants now (see [`step`]).
    pub fn submit(&self, app: &AppHandle, wanted: Wanted) {
        let (changed, job) = self.apply(wanted);
        if let Some(input) = job {
            if let Some(jobs) = lock(&self.jobs).as_ref() {
                let _ = jobs.send(input);
            }
        }
        if changed {
            let _ = app.emit(LIVE_CHANGED, ());
        }
    }

    /// Takes in what is wanted: whether what the overlay shows changed, and
    /// the run to start, if any.
    fn apply(&self, wanted: Wanted) -> (bool, Option<OddsInput>) {
        let mut inner = lock(&self.inner);
        if inner.last == wanted {
            return (false, None);
        }
        inner.last = wanted.clone();
        let next = step(&wanted, inner.shown.as_ref());
        let before = inner.shown.clone();
        let job = match next {
            Step::Show(odds) => {
                inner.run = None;
                inner.shown = odds;
                None
            }
            Step::Run { input, clear } => {
                inner.run = Some((*input).clone());
                if clear {
                    inner.shown = None;
                }
                Some(*input)
            }
        };
        (inner.shown != before, job)
    }

    fn is_wanted(&self, input: &OddsInput) -> bool {
        lock(&self.inner).run.as_ref() == Some(input)
    }

    /// Shows a run's result if its input is still the one wanted, checked
    /// and set under the same lock. True when what the overlay shows changed.
    fn land(&self, input: &OddsInput, odds: LiveOdds) -> bool {
        let mut inner = lock(&self.inner);
        if inner.run.as_ref() != Some(input) {
            return false;
        }
        let changed = inner.shown.as_ref() != Some(&odds);
        inner.shown = Some(odds);
        changed
    }

    fn finish(&self, app: &AppHandle, input: &OddsInput, odds: LiveOdds) {
        if self.land(input, odds) {
            let _ = app.emit(LIVE_CHANGED, ());
        }
    }
}

/// The newest input once none came for `quiet`; None when the app is
/// closing.
pub fn settled<T>(jobs: &Receiver<T>, first: T, quiet: Duration) -> Option<T> {
    let mut job = first;
    loop {
        match jobs.recv_timeout(quiet) {
            Ok(newer) => job = newer,
            Err(RecvTimeoutError::Timeout) => return Some(job),
            Err(RecvTimeoutError::Disconnected) => return None,
        }
    }
}

/// Starts the simulator's thread.
pub fn start(app: AppHandle, state: Arc<AppState>) {
    let (send, jobs) = channel();
    *lock(&state.odds.jobs) = Some(send);
    thread::spawn(move || {
        while let Ok(first) = jobs.recv() {
            let Some(input) = settled(&jobs, first, DEBOUNCE) else {
                return;
            };
            if !state.odds.is_wanted(&input) {
                continue;
            }
            let cards = app.state::<card_data::Cards>();
            let facts = |id: &str| card_data::combat_facts(&cards, id);
            // A bug in the simulator must not take the app down: no odds then.
            let odds = catch_unwind(AssertUnwindSafe(|| run_odds(&input, &facts, BUDGET)))
                .unwrap_or_else(|_| {
                    LiveOdds::gap(Gap::BadBoard, Some(input.opponent_seat), Some(input.round))
                });
            // A newer input may have come while it ran; that one is queued.
            state.odds.finish(&app, &input, odds);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(gap: Gap, seat: i64) -> NoOdds {
        NoOdds {
            gap,
            seat: Some(seat),
            boards: Vec::new(),
        }
    }

    fn input(seat: i64, round: i64) -> OddsInput {
        OddsInput {
            opponent_seat: seat,
            round,
            seen_after_start_of_combat: false,
            own: Vec::new(),
            opponent: Vec::new(),
            own_values: Default::default(),
            opponent_values: Default::default(),
            own_kit: None,
            opponent_kit: None,
            estimate: false,
            own_tier: None,
            opponent_tier: None,
            damage_cap: None,
            duos: None,
            build: None,
        }
    }

    /// Odds as a finished run gives them.
    fn done(i: &OddsInput) -> LiveOdds {
        LiveOdds {
            won: 1.0,
            ..LiveOdds::gap(Gap::BadBoard, Some(i.opponent_seat), Some(i.round))
        }
    }

    #[test]
    fn a_reason_shows_at_once_and_a_run_waits_for_the_thread() {
        assert_eq!(step(&None, None), Step::Show(None));
        let unseen = gap(Gap::NoBoardSeen, 3);
        assert_eq!(
            step(&Some(Err(unseen.clone())), None),
            Step::Show(Some(unseen.odds()))
        );
        assert_eq!(
            step(&Some(Ok(input(3, 2))), None),
            Step::Run {
                input: Box::new(input(3, 2)),
                clear: false
            }
        );
    }

    #[test]
    fn odds_for_another_opponent_or_combat_go_at_once_when_a_new_run_starts() {
        let shown = done(&input(3, 2));
        let clears = |i: OddsInput| match step(&Some(Ok(i)), Some(&shown)) {
            Step::Run { clear, .. } => clear,
            other => panic!("a run was expected, got {other:?}"),
        };
        assert!(clears(input(4, 2)), "another opponent");
        assert!(
            clears(input(3, 3)),
            "the next combat, even the same opponent"
        );
        assert!(!clears(input(3, 2)), "the same combat: it only refines");
    }

    #[test]
    fn the_estimate_stays_until_the_first_attacks_odds_replace_it() {
        let odds = Odds::default();
        let early = OddsInput {
            estimate: true,
            ..input(3, 2)
        };
        let (_, job) = odds.apply(Some(Ok(early)));
        let early = job.expect("the estimate's run");
        let estimate = LiveOdds {
            estimate: true,
            ..done(&early)
        };
        assert!(odds.land(&early, estimate));

        // The first attack: the same combat, so the estimate stays meanwhile.
        let (changed, job) = odds.apply(Some(Ok(input(3, 2))));
        let last = job.expect("the first attack's run");
        assert!(!changed);
        assert_eq!(odds.shown().map(|o| o.estimate), Some(true));
        assert!(!odds.land(&early, done(&early)), "a late estimate");
        assert!(odds.land(&last, done(&last)));
        assert_eq!(odds.shown().map(|o| o.estimate), Some(false));
    }

    #[test]
    fn the_odds_go_when_the_combat_ends_and_never_show_in_the_next_one() {
        let odds = Odds::default();
        let combat = |round| Some(Ok(input(3, round)));

        // Combat 4 starts: its run is asked for, and its result lands.
        let (_, job) = odds.apply(combat(4));
        let four = job.expect("a run for combat 4");
        assert!(odds.land(&four, done(&four)));
        assert_eq!(odds.shown().and_then(|o| o.round), Some(4));

        // The shop turn: no odds in the recruit phase.
        let (changed, job) = odds.apply(None);
        assert!(changed && job.is_none());
        assert_eq!(odds.shown(), None);

        // Combat 5 starts against the same opponent: combat 4's odds are not
        // shown, and a late result of that run is refused.
        let (_, job) = odds.apply(combat(5));
        let five = job.expect("a run for combat 5");
        assert_eq!(odds.shown(), None);
        assert!(!odds.land(&four, done(&four)), "a late result of combat 4");
        assert_eq!(odds.shown(), None);
        assert!(odds.land(&five, done(&five)));
        assert_eq!(odds.shown().and_then(|o| o.round), Some(5));
    }

    #[test]
    fn a_result_that_lands_after_the_shop_began_is_refused() {
        let odds = Odds::default();
        let (_, job) = odds.apply(Some(Ok(input(3, 4))));
        let four = job.expect("a run");
        odds.apply(None);
        assert!(!odds.land(&four, done(&four)));
        assert_eq!(odds.shown(), None);
    }

    #[test]
    fn a_run_lands_only_while_its_input_is_wanted() {
        let odds = Odds::default();
        lock(&odds.inner).run = Some(input(3, 2));
        assert!(odds.is_wanted(&input(3, 2)));
        assert!(!odds.is_wanted(&input(4, 2)));
        assert!(!odds.is_wanted(&input(3, 3)));
        lock(&odds.inner).run = None;
        assert!(
            !odds.is_wanted(&input(3, 2)),
            "switched off or a reason shown since"
        );
    }

    #[test]
    fn the_newest_input_runs_once_the_changes_stop() {
        let (send, jobs) = channel();
        send.send(2).unwrap();
        send.send(3).unwrap();
        assert_eq!(settled(&jobs, 1, Duration::from_millis(20)), Some(3));
        drop(send);
        assert_eq!(
            settled(&jobs, 4, Duration::from_millis(20)),
            None,
            "closing"
        );
    }
}
