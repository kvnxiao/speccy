---
paths: **/*.{rs,toml}
---

# Unsafe Rust

## Core Principle: Avoid `unsafe` at All Costs

**Unsafe Rust should be treated as a last resort.** The vast majority of Rust code should never need `unsafe` blocks.

## Why Avoid `unsafe`

1. **Defeats Rust's safety guarantees** - You lose memory safety, thread safety, and type safety
2. **Hard to audit** - Requires deep understanding of invariants and undefined behavior
3. **Maintenance burden** - Future changes can introduce subtle bugs
4. **Propagates risk** - Unsafety in one place can affect the entire codebase
5. **Better alternatives exist** - Safe wrappers and crates are usually available

## The `unsafe` Hierarchy (Prefer Earlier Options)

```
1. Pure safe Rust              - BEST
2. Safe wrapper crates         - GOOD
3. Well-audited unsafe in deps - ACCEPTABLE
4. Your own unsafe code        - AVOID
5. Extensive unsafe code       - NEVER
```

## Use Safe Wrapper Crates Instead

Before writing `unsafe` code, **always search for a safe wrapper crate first**.

### Common Safe Wrappers

**Windows API:**
```toml
# Bad: Using windows-sys (unsafe bindings)
[dependencies]
windows-sys = "0.52"

# Good: Using winsafe (safe wrapper)
[dependencies]
winsafe = "0.0.27"
```

**POSIX/Unix APIs:**
```toml
# Instead of libc (unsafe)
[dependencies]
nix = "0.27"  # Safe POSIX wrapper
rustix = "0.38"  # Safe Unix system calls
```

**FFI Bindings:**
```toml
libsqlite3-sys = "0.27"  # Unsafe bindings
rusqlite = "0.30"        # Safe wrapper

openssl-sys = "0.9"      # Unsafe bindings
openssl = "0.10"         # Safe wrapper
```

**Memory Manipulation:**
```toml
[dependencies]
bytemuck = "1.14"  # Safe transmutation
zerocopy = "0.7"   # Safe zero-copy parsing
```

## When You Think You Need `unsafe`

**Ask these questions first:**

1. Is there a safe wrapper crate?
2. Can I restructure to avoid it?
3. Can I use standard library types?
4. Is this a premature optimization?

## If You Absolutely Must Use `unsafe`

### 1. Minimize the Surface Area

```rust
pub fn process_data(data: &[u8]) -> Vec<u8> {
    // 95 lines of safe code

    // Only the truly unsafe operation
    let value = unsafe {
        *ptr  // 1 line
    };

    // More safe code
}
```

### 2. Document EVERY Invariant

```rust
/// # Safety
///
/// The caller must ensure:
/// - `ptr` is non-null
/// - `ptr` is properly aligned for type `T`
/// - `ptr` points to a valid, initialized instance of `T`
/// - `ptr` is dereferenceable (points to allocated memory)
unsafe fn read_from_ptr<T>(ptr: *const T) -> &T {
    &*ptr
}
```

### 3. Wrap in Safe Abstractions

```rust
// Bad: Exposing unsafe interface
pub unsafe fn raw_operation(ptr: *mut u8) { }

// Good: Safe public API, unsafe internals
pub fn safe_operation(data: &mut [u8]) -> Result<()> {
    if data.is_empty() {
        return Err(Error::EmptyData);
    }

    unsafe {
        // SAFETY: We validated data is non-empty and properly aligned
        raw_operation_internal(data.as_mut_ptr())
    }

    Ok(())
}
```

### 4. Run Miri

```bash
# Install Miri
rustup +nightly component add miri

# Run tests under Miri to detect undefined behavior
cargo +nightly miri test
```

## Red Flags That Should Make You Reconsider

- You're not 100% sure the unsafe code is correct
- You can't articulate all safety invariants
- The unsafe code interacts with external state
- You're using unsafe for "performance" without profiling
- The unsafe block is more than 5 lines
- You're using transmute without understanding it fully
- You're doing pointer arithmetic

## Safe Alternatives Checklist

Before writing `unsafe`, verify you've checked:

- [ ] Searched crates.io for safe wrappers
- [ ] Checked if standard library types can solve it
- [ ] Asked in Rust community (Reddit, Discord, forums)
- [ ] Profiled to confirm performance is actually a problem
- [ ] Reviewed similar crates to see their approach
- [ ] Considered restructuring the API to avoid the need
