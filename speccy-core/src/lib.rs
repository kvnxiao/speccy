//! Speccy: a spec-driven run controller for coding agents.
//!
//! The deterministic core. Harnesses call this controller through the
//! `speccy ctl` CLI; the controller never launches an LLM (PRINCIPLES.md).
//!
//! Authority: `DESIGN.md` owns behavior and enum values, `TERMINOLOGY.md`
//! owns vocabulary, `SCHEMAS.md` owns payload shapes, `IMPLEMENTATION-PLAN.md`
//! owns build order.

pub mod config;
pub mod directive;
pub mod error;
pub mod event;
pub mod evidence;
pub mod gitx;
pub mod hash;
pub mod ids;
pub mod install;
pub mod lease;
pub mod lint;
pub mod model;
pub mod mutation;
pub mod packets;
pub mod projection;
pub mod provenance;
pub mod receipt;
pub mod render;
pub mod store;

pub use error::ErrorCode;
pub use error::Finding;
pub use error::Result;
pub use error::SpeccyError;
pub use error::envelope;
