//! The combat simulator for the website (T-606, D-090): `crates/combat-sim`
//! built for `wasm32-unknown-unknown` behind a hand-written C ABI. The page's
//! Web Worker writes a JSON request into the module's memory and reads a
//! JSON answer back; nothing else crosses. No `wasm-bindgen`, no new crate.
//!
//! The calls, in order:
//! 1. `sim_alloc(len)` reserves `len` bytes and returns where they are;
//! 2. the worker copies the UTF-8 request there;
//! 3. `sim_run(ptr, len)` reads and frees the request, plays it and returns
//!    the answer's length in bytes;
//! 4. `sim_answer()` returns where the answer is (valid until the next run).
//!
//! The request and answer formats are in `docs/research/combat-simulator.md`
//! section 8 and in [`input`] and [`odds`].

pub mod input;
pub mod odds;

use std::cell::RefCell;

/// Plays one request (JSON text) and returns the answer (JSON text). Never
/// panics on bad input: a refusal is an answer too.
pub fn run_json(request: &str) -> String {
    let answer = match input::read(request) {
        Ok(req) => odds::run(&req),
        Err(reason) => odds::refused(&reason),
    };
    answer.to_string()
}

thread_local! {
    /// The last answer, kept until the next run so the worker can copy it.
    static ANSWER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Reserves `len` bytes for a request; the worker writes into them.
#[no_mangle]
pub extern "C" fn sim_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Reads the request at `ptr` (from `sim_alloc(len)`), frees it, plays it
/// and returns the answer's length.
///
/// # Safety
///
/// `ptr` must come from `sim_alloc(len)` with the same `len`, hold `len`
/// bytes written by the caller, and not be used again after this call.
#[no_mangle]
pub unsafe extern "C" fn sim_run(ptr: *mut u8, len: usize) -> usize {
    // SAFETY: the caller's contract above: `ptr` is the start of a live
    // allocation of capacity `len.max(1)` made by `sim_alloc`, whose first
    // `len` bytes the caller wrote. Taking it back as a Vec frees it once.
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len.max(1)) };
    let answer = match std::str::from_utf8(&bytes) {
        Ok(text) => run_json(text),
        Err(_) => odds::refused("the request is not UTF-8").to_string(),
    };
    let size = answer.len();
    ANSWER.with(|a| *a.borrow_mut() = answer.into_bytes());
    size
}

/// Where the last answer is (its length came from `sim_run`).
#[no_mangle]
pub extern "C" fn sim_answer() -> *const u8 {
    ANSWER.with(|a| a.borrow().as_ptr())
}

#[cfg(test)]
mod tests;
