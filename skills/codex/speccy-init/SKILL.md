---
name: speccy-init
description: Bootstrap a new Speccy workspace by scaffolding `.speccy/` and installing the host-native skill pack. Use when the user says "set up speccy", "init speccy", "add speccy to this repo", or wants to start a spec-driven workflow somewhere that has no `.speccy/` yet. Run once per project before any other speccy-* skill.
---

# speccy-init

Bootstraps a Speccy workspace by scaffolding `.speccy/` and copying the
Codex skill pack into `.codex/skills/`.

## When to use

Run once per project, before any other Speccy skill. Re-run with
`--force` after upgrading `speccy` to refresh shipped recipes while
preserving user-authored files.

## Steps

1. Check whether `.speccy/` exists. If yes, ask the user whether to
   `--force` (refresh shipped files in place) before continuing.
2. Run the CLI:

   ```bash
   speccy init
   ```

3. Read the plan summary the CLI prints. The plan lists every file
   under `create`, `overwrite`, or `preserve`. `VISION.md` is always
   preserved if it already exists.
4. Report the final counts (`N created, N overwritten, N preserved`)
   to the user.
5. Suggest the next step: `speccy-plan` if `VISION.md` was just
   scaffolded, otherwise `speccy-plan SPEC-NNNN` to amend an existing
   spec.

This recipe does not loop.
