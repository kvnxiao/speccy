//! Library surface for the speccy CLI binary.
//!
//! The library hosts command implementations and helpers so integration
//! tests can exercise them without going through the binary's argument
//! parser. The binary in `main.rs` is a thin dispatcher over this
//! library.

#![deny(unsafe_code)]

pub mod check;
pub mod git;
pub mod status;
pub mod status_output;
pub mod verify;
pub mod verify_output;
