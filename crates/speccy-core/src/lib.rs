#![doc = include_str!("../README.md")]
#![deny(unsafe_code)]

pub mod error;
pub mod parse;

pub use error::ParseError;
