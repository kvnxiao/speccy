#![doc = include_str!("../README.md")]
#![deny(unsafe_code)]

pub mod error;
pub mod lint;
pub mod next;
pub mod parse;
pub mod personas;
pub mod prompt;
pub mod task_lookup;
pub mod tasks;
pub mod workspace;

pub use error::ParseError;
