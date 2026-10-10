//! Card names and card art for the app (T-304, D-038), from HearthstoneJSON.
//! The data and the images are © Blizzard Entertainment: they are fetched at
//! run time and kept only on the player's PC, never in the repo. Nothing is
//! read from the game's own files (D-004). Images are for free features
//! only (Blizzard's fan licence is non-commercial, D-003).

pub mod art;
pub mod catalog;
pub mod net;
pub mod store;

#[cfg(test)]
mod store_tests;

pub use catalog::valid_id;
pub use store::{CardStatus, CardStore};
