---
paths: **/*.{rs,toml}
---

# Linter and Formatter Configuration

## Clippy

Always use this STRICT workspace level clippy lint configuration in `Cargo.toml`:

```toml
# Groups (broad coverage)
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
cargo = { level = "deny", priority = -1 }

# Cargo group escape hatch
multiple_crate_versions = "allow"

# Restriction lints — not in any group, must opt in individually
# Don't panic
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
string_slice = "deny"
indexing_slicing = "deny"
get_unwrap = "deny"
unwrap_in_result = "deny"
panic_in_result_fn = "deny"
unchecked_time_subtraction = "deny"
todo = "warn"
unimplemented = "warn"

# Don't fail silently
let_underscore_future = "deny"
let_underscore_must_use = "deny"
unused_result_ok = "deny"
map_err_ignore = "deny"
assertions_on_result_states = "deny"

# Async correctness
await_holding_lock = "deny"
await_holding_refcell_ref = "deny"
if_let_mutex = "deny"
large_futures = "deny"

# Unsafe hygiene
mem_forget = "deny"
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"
unnecessary_safety_doc = "deny"
unnecessary_safety_comment = "deny"

# Float correctness
float_cmp = "deny"
float_cmp_const = "deny"
lossy_float_literal = "deny"
invalid_upcast_comparisons = "deny"

# Misc footguns
rc_mutex = "deny"
debug_assert_with_mut_call = "deny"
iter_not_returning_iterator = "deny"
expl_impl_clone_on_copy = "deny"
infallible_try_from = "deny"
dbg_macro = "deny"

# Force #[expect(..., reason = "...")] over silent #[allow]
allow_attributes = "deny"
allow_attributes_without_reason = "deny"

# Performance
large_enum_variant = "warn"
large_stack_arrays = "deny"
needless_pass_by_value = "warn"

# Style
missing_errors_doc = "warn"
missing_panics_doc = "warn"
```

And ensure a `clippy.toml` is present in the workspace root with the following configuration:

```toml
# Allow expect in test code
allow-expect-in-tests = true
```

Always run Clippy with `cargo clippy --workspace --all-targets --all-features` to catch potential issues early, followed by a `cargo fmt --check` to ensure code is formatted correctly.
