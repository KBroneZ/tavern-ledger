//! Desktop sign-in and game upload (T-104d, D-025). Upload is off until the
//! user signs in and turns it on. Only the local report goes up (card ids,
//! places, boards); never player names, BattleTags, ratings or MMR, and the
//! server checks that again (supabase/functions/upload-game).

pub mod auth;
pub mod config;
pub mod engine;
pub mod http;
pub mod loopback;
pub mod pkce;
pub mod queue;
pub mod secrets;
pub mod settings;
pub mod signin;

pub use engine::{Engine, Status, Wake};
