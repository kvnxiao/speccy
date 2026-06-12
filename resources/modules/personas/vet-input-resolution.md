**Use `git diff <base-ref>`** (no `...HEAD`). That command compares
the **working tree** against the ref, capturing both committed and
uncommitted changes. The vet-implementer leaves its changes
uncommitted between rounds, so the `...HEAD` form would silently miss
them.

If the caller did not pass resolved paths (a human invoked you
directly, the prompt got mangled, etc.), fall back to resolving them
yourself:

```bash
# Spec dir: pick the directory matching the SPEC ID
ls -d .speccy/specs/NNNN-*/  # NNNN from SPEC-NNNN

# Base ref: default branch name
git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@'
# Fall back to "main" if empty.
```