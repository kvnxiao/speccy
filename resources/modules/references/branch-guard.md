## Branch-guard prelude

The prelude guarantees that HEAD is on a feature branch before any
artifact is committed, so an authored `SPEC.md` / `TASKS.md` never lands
on the repository's default branch. It creates a branch only when it
must, reuses an existing feature branch otherwise, and never prompts for
confirmation.

The caller supplies one parameter:

- **Spec directory.** The path to the spec's directory, used to derive
  the branch name. For a flat spec this is `.speccy/specs/NNNN-slug/`;
  for a mission-foldered spec it is `.speccy/specs/[focus]/NNNN-slug/`.

### No-git short-circuit

Before doing anything, check whether the working directory is inside a
git repository:

```bash
git rev-parse --is-inside-work-tree
```

If this exits non-zero (the project is not a git repository), **skip the
entire branch-guard without erroring**. The authoring skill still writes
its artifact and continues; no branch is created and no git failure is
surfaced. This preserves Speccy's "works identically in any project
state" property for non-git projects.

### Default-branch detection

When a git repository is present, identify the repository's default
branch via an ordered three-tier chain. Each tier is consulted only when
the prior tier **does not resolve**:

1. **Remote symbolic ref `origin/HEAD`.** When a remote exists, read the
   branch it points at:

   ```bash
   git symbolic-ref --quiet refs/remotes/origin/HEAD
   ```

   The trailing path component (e.g. `origin/main` → `main`) is the
   default-branch name. If there is no remote, or `origin/HEAD` is not
   set, this tier does not resolve — fall through to tier 2.

2. **`git config init.defaultBranch`.** Otherwise, read the configured
   default-branch name:

   ```bash
   git config init.defaultBranch
   ```

   If this is set (e.g. `trunk`), it is the default-branch name. If it is
   unset, this tier does not resolve — fall through to tier 3.

3. **`{main, master}` name match.** Otherwise, when neither remote nor
   config resolved, treat HEAD as the default branch only when its own
   branch name is `main` or `master`. HEAD on any other name is treated
   as a feature branch.

### Branch-creation condition

Read the current HEAD branch name:

```bash
git symbolic-ref --quiet --short HEAD
```

A detached HEAD makes this exit non-zero (there is no current branch
name).

- **Create path** — when HEAD is the detected default branch **or** HEAD
  is detached, derive the branch name and create it. The branch name is
  the literal `spec-` prefix followed by the **basename** of the spec
  directory (its final `NNNN-slug` path component). For a mission-
  foldered spec at `.speccy/specs/[focus]/NNNN-slug/`, the `[focus]`
  segment is dropped — only the basename is used, so the name stays flat
  `spec-NNNN-slug`. Then create and switch to it:

  ```bash
  git switch -c spec-NNNN-slug
  ```

  Emit a one-line notice naming the created branch, for example:

  ```
  Created and switched to branch spec-NNNN-slug.
  ```

- **Reuse path** — when HEAD is on any other branch (an existing
  `spec-NNNN-slug`, or an unrelated feature branch), reuse it: create
  nothing, switch nothing, leave HEAD unchanged.

The one-line creation notice is emitted **only on the create path**,
never on the reuse path.

> Illustrative example — substitute your own values. For a spec at
> `.speccy/specs/0042-example-slug/` (or
> `.speccy/specs/acme/0042-example-slug/` under a mission folder), the
> derived branch is `spec-0042-example-slug` in both cases.
