//! Prompt-adjacent helpers retained for the surviving CLI surface.
//!
//! The CLI does not render phase prompt bodies; phase prompts live in
//! the shipped skill packs. The one helper that lives here is
//! [`allocate_next_spec_id`], shared by the workspace ID allocator and
//! the `speccy vacancy` verb.

pub mod id_alloc;

pub use id_alloc::allocate_next_spec_id;
pub use id_alloc::allocate_next_spec_id_across_dirs;
