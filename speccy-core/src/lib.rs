#![doc = include_str!("../README.md")]
#![deny(unsafe_code)]

pub mod error;
pub mod lint;
pub mod parse;
pub mod workspace;

pub use error::ParseError;
