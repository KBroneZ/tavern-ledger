//! Combat odds in the overlay (T-605, D-084). The tracker's loop says what
//! the odds should be worked out from; the simulator runs on its own thread,
//! never the window's, once the boards stop changing for a moment
//! ([`DEBOUNCE`]) and within a time budget ([`BUDGET`]). A reason for no
//! odds (Duos, no board seen yet...) is shown at once. While the game shows
//! the combat, the odds worked out in the shop stay.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bg_parser::report::Status;
use tauri::{AppHandle, Emitter, Manager};
use tracker::odds::{odds_input, run_odds, Gap, LiveOdds, NoOdds, OddsInput};
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
    /// The last [`Wanted`], so only changes are acted on.
    last: Mutex<Wanted>,
    shown: Mutex<Option<LiveOdds>>,
    jobs: Mutex<Option<Sender<OddsInput>>>,
}

/// What a new [`Wanted`] does.
#[derive(Debug, PartialEq)]
pub enum Step {
    Show(Option<LiveOdds>),
    /// The game is showing the combat these odds are for: they stay.
    Keep,
    Run(OddsInput),
}

pub fn step(wanted: &Wanted, shown: Option<&LiveOdds>) -> Step {
    match wanted {
        None => Step::Show(None),
        Some(Err(no)) => {
            let fighting = no.gap == Gap::NotInShop
                && shown.is_some_and(|s| s.seat == no.seat && s.gap != Some(Gap::NotInShop));
            if fighting {
                Step::Keep
            } else {
                Step::Show(Some(no.odds()))
            }
        }
        Some(Ok(input)) => Step::Run(input.clone()),
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
    Some(odds_input(
        report,
        watcher.live_own_board(),
        watcher.live_next_opponent(),
        &watcher.live_lobby(),
        watcher.live_damage_cap(),
    ))
}

impl Odds {
    pub fn shown(&self) -> Option<LiveOdds> {
        lock(&self.shown).clone()
    }

    /// Acts on what the tracker's loop wants now (see [`step`]).
    pub fn submit(&self, app: &AppHandle, wanted: Wanted) {
        {
            let mut last = lock(&self.last);
            if *last == wanted {
                return;
            }
            *last = wanted.clone();
        }
        let next = step(&wanted, lock(&self.shown).as_ref());
        match next {
            Step::Show(odds) => self.set(app, odds),
            Step::Keep => {}
            Step::Run(input) => {
                if let Some(jobs) = lock(&self.jobs).as_ref() {
                    let _ = jobs.send(input);
                }
            }
        }
    }

    fn is_wanted(&self, input: &OddsInput) -> bool {
        matches!(&*lock(&self.last), Some(Ok(i)) if i == input)
    }

    fn set(&self, app: &AppHandle, odds: Option<LiveOdds>) {
        let changed = {
            let mut shown = lock(&self.shown);
            let changed = *shown != odds;
            *shown = odds;
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
            if state.odds.is_wanted(&input) {
                state.odds.set(&app, Some(odds));
            }
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
        }
    }

    fn input(seat: i64) -> OddsInput {
        OddsInput {
            opponent_seat: seat,
            seen_round: 2,
            own: Vec::new(),
            opponent: Vec::new(),
            own_tier: None,
            opponent_tier: None,
            damage_cap: None,
        }
    }

    #[test]
    fn a_reason_shows_at_once_and_a_run_waits_for_the_thread() {
        assert_eq!(step(&None, None), Step::Show(None));
        let duos = gap(Gap::Duos, 3);
        assert_eq!(step(&Some(Err(duos)), None), Step::Show(Some(duos.odds())));
        assert_eq!(step(&Some(Ok(input(3))), None), Step::Run(input(3)));
    }

    #[test]
    fn the_shop_odds_stay_while_the_game_shows_that_combat() {
        let mut odds = gap(Gap::Duos, 3).odds();
        odds.gap = None;
        let fighting = Some(Err(gap(Gap::NotInShop, 3)));
        assert_eq!(step(&fighting, Some(&odds)), Step::Keep);
        // Another opponent, or nothing shown yet: the reason is shown.
        let other = Some(Err(gap(Gap::NotInShop, 4)));
        assert!(matches!(step(&other, Some(&odds)), Step::Show(Some(_))));
        assert!(matches!(step(&fighting, None), Step::Show(Some(_))));
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
