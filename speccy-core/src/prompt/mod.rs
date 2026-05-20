//! Prompt-adjacent helpers retained for the surviving CLI surface.
//!
//! After SPEC-0033 T-001 the natural-text prompt-rendering pipeline
//! (template loader, single-pass `{{NAME}}` renderer, budget trimmer,
//! task-scoped spec slicer) was removed: the CLI no longer carries
//! phase prompt bodies. The one helper that outlives that deletion is
//! [`allocate_next_spec_id`], which the workspace ID allocator and the
//! forthcoming `speccy vacancy` verb (SPEC-0033 T-003) both build on.
//!
//! SPEC-0033 leaves an open question about whether `id_alloc` should
//! relocate out of `prompt::` into a more general
//! `speccy_core::specs::` module now that nothing else lives here. That
//! decision is deferred to T-003 (which adds the `vacancy` command);
//! until then the existing import path is preserved.

pub mod id_alloc;

pub use id_alloc::allocate_next_spec_id;
