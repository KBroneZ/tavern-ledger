//! Combat odds in the overlay (T-605, D-084). The tracker's loop says what
//! the odds should be worked out from; the simulator runs on its own thread,
//! never the window's, once the boards stop changing for a moment
//! ([`DEBOUNCE`]) and within a time budget ([`BUDGET`]). A reason for no
//! odds (no board seen yet...) is shown at once. While the game shows the
//! combat, the odds worked out in the shop stay. Duos odds (T-608) come the
//! same way, with the teammate's and both opponents' boards.

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

/// What the odds are worked out from now: nothing (no game on, or switched
/// off), a reason for no odds, or the input of a run.
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
    /// The input whose result may still be shown: the last one asked for in
    /// the shop. Kept while the game shows that combat.
    run: Option<OddsInput>,
    shown: Option<LiveOdds>,
}

/// What a new [`Wanted`] does.
#[derive(Debug, PartialEq)]
pub enum Step {
    Show(Option<LiveOdds>),
    /// The game is showing the combat the last shop input is for: its odds
    /// stay, or come when its run ends.
    Keep,
    /// Work these odds out; `clear` when what is shown is for another
    /// opponent and must go at once.
    Run {
        input: Box<OddsInput>,
        clear: bool,
    },
}

/// `run` is the last shop input asked for; `shown` what the overlay has.
pub fn step(wanted: &Wanted, run: Option<&OddsInput>, shown: Option<&LiveOdds>) -> Step {
    match wanted {
        None => Step::Show(None),
        Some(Err(no)) => {
            let fighting =
                no.gap == Gap::NotInShop && run.is_some_and(|r| Some(r.opponent_seat) == no.seat);
            if fighting {
                Step::Keep
            } else {
                Step::Show(Some(no.odds()))
            }
        }
        Some(Ok(input)) => Step::Run {
            input: Box::new(input.clone()),
            clear: shown.is_some_and(|s| s.seat != Some(input.opponent_seat)),
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
    let lobby = watcher.live_lobby();
    let live = LiveFacts {
        own_now: watcher.live_own_board(),
        own_values: watcher.live_own_values(),
        next: watcher.live_next_opponent(),
        lobby: &lobby,
        damage_cap: watcher.live_damage_cap(),
        fights_first: watcher.live_fights_first(),
    };
    Some(odds_input(report, live))
}

impl Odds {
    pub fn shown(&self) -> Option<LiveOdds> {
        lock(&self.inner).shown.clone()
    }

    /// Acts on what the tracker's loop wants now (see [`step`]).
    pub fn submit(&self, app: &AppHandle, wanted: Wanted) {
        let (changed, job) = {
            let mut inner = lock(&self.inner);
            if inner.last == wanted {
                return;
            }
            inner.last = wanted.clone();
            let next = step(&wanted, inner.run.as_ref(), inner.shown.as_ref());
            let before = inner.shown.clone();
            let job = match next {
                Step::Show(odds) => {
                    inner.run = None;
                    inner.shown = odds;
                    None
                }
                Step::Keep => None,
                Step::Run { input, clear } => {
                    inner.run = Some((*input).clone());
                    if clear {
                        inner.shown = None;
                    }
                    Some(*input)
                }
            };
            (inner.shown != before, job)
        };
        if let Some(input) = job {
            if let Some(jobs) = lock(&self.jobs).as_ref() {
                let _ = jobs.send(input);
            }
        }
        if changed {
            let _ = app.emit(LIVE_CHANGED, ());
        }
    }

    fn is_wanted(&self, input: &OddsInput) -> bool {
        lock(&self.inner).run.as_ref() == Some(input)
    }

    /// Shows a run's result if its input is still the one wanted, checked
    /// and set under the same lock.
    fn finish(&self, app: &AppHandle, input: &OddsInput, odds: LiveOdds) {
        let changed = {
            let mut inner = lock(&self.inner);
            if inner.run.as_ref() != Some(input) {
                return;
            }
            let changed = inner.shown.as_ref() != Some(&odds);
            inner.shown = Some(odds);
            changed
        };
        if changed {
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
                    LiveOdds::gap(
                        Gap::BadBoard,
                        Some(input.opponent_seat),
                        Some(input.seen_round),
                    )
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

    fn input(seat: i64) -> OddsInput {
        OddsInput {
            opponent_seat: seat,
            seen_round: 2,
            seen_after_start_of_combat: false,
            own: Vec::new(),
            opponent: Vec::new(),
            own_values: Default::default(),
            opponent_values: Default::default(),
            own_tier: None,
            opponent_tier: None,
            damage_cap: None,
            duos: None,
            build: None,
        }
    }

    #[test]
    fn a_reason_shows_at_once_and_a_run_waits_for_the_thread() {
        assert_eq!(step(&None, None, None), Step::Show(None));
        let unseen = gap(Gap::NoBoardSeen, 3);
        assert_eq!(
            step(&Some(Err(unseen.clone())), None, None),
            Step::Show(Some(unseen.odds()))
        );
        assert_eq!(
            step(&Some(Ok(input(3))), None, None),
            Step::Run {
                input: Box::new(input(3)),
                clear: false
            }
        );
    }

    #[test]
    fn odds_for_another_opponent_go_at_once_when_a_new_run_starts() {
        let mut shown = gap(Gap::NoBoardSeen, 3).odds();
        shown.gap = None;
        assert_eq!(
            step(&Some(Ok(input(4))), None, Some(&shown)),
            Step::Run {
                input: Box::new(input(4)),
                clear: true
            }
        );
        assert_eq!(
            step(&Some(Ok(input(3))), None, Some(&shown)),
            Step::Run {
                input: Box::new(input(3)),
                clear: false
            }
        );
    }

    #[test]
    fn the_last_shop_input_stays_wanted_while_the_game_shows_that_combat() {
        let fighting = Some(Err(gap(Gap::NotInShop, 3)));
        // Its run may not have ended yet: it is kept, and still lands.
        assert_eq!(step(&fighting, Some(&input(3)), None), Step::Keep);
        // No shop input for that opponent (another one, or none yet): the reason.
        assert!(matches!(
            step(&fighting, Some(&input(4)), None),
            Step::Show(Some(_))
        ));
        assert!(matches!(step(&fighting, None, None), Step::Show(Some(_))));
    }

    #[test]
    fn a_run_lands_only_while_its_input_is_wanted() {
        let odds = Odds::default();
        lock(&odds.inner).run = Some(input(3));
        assert!(odds.is_wanted(&input(3)));
        assert!(!odds.is_wanted(&input(4)));
        lock(&odds.inner).run = None;
        assert!(
            !odds.is_wanted(&input(3)),
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
