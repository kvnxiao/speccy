//! Speccy: a spec-driven run controller for coding agents.
//!
//! The deterministic core. Harnesses call this controller through the
//! `speccy ctl` CLI; the controller never launches an LLM (PRINCIPLES.md).
//!
//! Authority: `DESIGN.md` owns behavior and enum values, `TERMINOLOGY.md`
//! owns vocabulary, `SCHEMAS.md` owns payload shapes, `IMPLEMENTATION-PLAN.md`
//! owns build order.

pub mod cli;
pub mod config;
pub mod directive;
pub mod error;
pub mod event;
pub mod gitx;
pub mod humancli;
pub mod ids;
pub mod lint;
pub mod model;
pub mod ops;
pub mod packets;
pub mod projection;
pub mod store;

pub use error::{envelope, ErrorCode, Finding, Result, SpeccyError};
