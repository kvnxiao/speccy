//! Library surface for the speccy CLI binary.
//!
//! The library hosts command implementations and helpers so integration
//! tests can exercise them without going through the binary's argument
//! parser. The binary in `main.rs` is a thin dispatcher over this
//! library.

#![deny(unsafe_code)]

pub mod archive;
pub mod check;
pub mod check_selector;
pub mod cwd;
pub mod embedded;
pub mod git;
pub mod host;
pub mod init;
pub mod lock;
pub mod next;
pub mod next_output;
pub(crate) mod paths;
pub mod render;
pub mod status;
pub mod status_output;
pub mod vacancy;
pub mod verify;
pub mod verify_output;
