//! Battlegrounds combat simulator (F6, T-602): the odds that a combat between
//! two known warbands is won, tied or lost, and the damage, by playing it
//! many times with the game's random choices.
//!
//! One engine for the desktop app and, later, the website (see
//! design notes). It has no dependencies, so it can be
//! built for WebAssembly. A card the engine does not know is never guessed:
//! [`simulate`] returns [`Verdict::NotModelled`] instead of a number.
//!
//! ```
//! use combat_sim::{simulate, Minion, Options, Verdict, Warband};
//!
//! let player = Warband::new(vec![Minion::new("BGS_119", 2, 2)]);
//! let opponent = Warband::new(vec![Minion::new("BG_ICC_026t", 1, 1)]);
//! let Verdict::Odds(odds) = simulate(&player, &opponent, &Options::default()).unwrap() else {
//!     panic!("both cards are known");
//! };
//! assert_eq!(odds.won, odds.trials);
//! ```

pub mod cards;
mod cards_round1;
mod cards_round2;
mod cards_round3;
mod cards_round4;
mod cards_round5;
mod cards_round7;
mod cards_round8;
mod cards_round8_pool;
pub mod combat;
pub mod gifts;
pub mod kit;
pub mod kit_effects;
mod kit_lists;
pub mod model;
pub mod rng;
pub mod trials;

#[cfg(feature = "harness")]
pub mod harness;
#[cfg(feature = "harness")]
pub mod harness_duos;
#[cfg(feature = "harness")]
pub mod harness_export;
#[cfg(feature = "harness")]
pub mod harness_games;
#[cfg(feature = "harness")]
pub mod harness_logs;
#[cfg(feature = "harness")]
pub mod harness_misses;
#[cfg(feature = "harness")]
pub mod harness_saved;
#[cfg(feature = "harness")]
pub mod harness_soc;
#[cfg(feature = "harness")]
pub mod harness_summary;
#[cfg(feature = "harness")]
pub mod harness_why;
#[cfg(feature = "saved")]
pub mod saved;
pub mod score;
pub mod why;
#[cfg(feature = "saved")]
pub mod why_json;

pub use combat::{CombatResult, Outcome};
pub use model::{
    BaseCard, BoardError, GameValues, Keywords, Kit, KitCard, Minion, Team, Tribes, Warband,
    MAX_BOARD,
};
pub use trials::{simulate, simulate_teams, Coverage, Odds, Options, Verdict};

/// The engine's version: its minor number is the card round (D-088). The
/// accuracy export and the website show it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
