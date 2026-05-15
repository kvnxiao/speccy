---
spec: SPEC-0014
outcome: delivered
generated_at: 2026-05-14T23:55:00Z
---

# SPEC-0014: Handoff template + friction-to-skill-update conventions

## Outcome

**delivered** — all three requirements satisfied; all 7 tasks closed
on the first pass with zero retries and zero SPEC amendments. The
shipped implementer prompt now embeds a six-field handoff template,
the implementer persona and the project's own `AGENTS.md` document
the friction-to-skill-update loop, and the report prompt instructs
agents to surface skill edits under `## Skill updates`. Six new
content-shape tests in `speccy-cli/tests/skill_packs.rs` gate the
conventions and pass cleanly under `cargo test --workspace`.

## Requirements coverage

| Requirement | Checks | Result |
|-------------|--------|--------|
| REQ-001: Six-field handoff template in implementer prompt | CHK-001, CHK-002 | proved |
| REQ-002: Friction-to-skill-update pattern documented | CHK-003, CHK-004, CHK-005 | proved |
| REQ-003: Skill-update surfacing in REPORT.md | CHK-006 | proved |

All six SPEC-0014 checks resolve to executable `cargo test`
invocations against `speccy-cli/tests/skill_packs.rs`. `speccy
verify` exercises them on the Linux CI runner; on Windows the same
checks pass via direct `cargo test` invocation.

## Task summary

- **Total tasks:** 7 (T-001 through T-007).
- **Retried tasks:** 0.
- **SPEC amendments:** 0.

Each Phase-1 / Phase-2 / Phase-3 task added one content-shape test
and the matching prompt/persona/AGENTS.md edit. T-007 ran the
four-command hygiene gate plus `speccy verify` against the release
binary and required no fixups beyond a `cargo +nightly fmt --all`
pass on two test-function wraps in `speccy-cli/tests/skill_packs.rs`.

## Out-of-scope items absorbed

Edits made during the loop that were not part of the planned task
scope but were necessary for the work to land cleanly:

- **One-line reflows in T-001 / T-002 / T-004 / T-005** to keep
  stable substrings (`handoff template`, `(none)`,
  `update the relevant skill file under skills/`,
  `Procedural compliance`) on a single source line so the
  `body.contains(...)` test assertions stayed honest after Markdown
  word-wrap kicked in. Documented inline in the matching
  Discovered-issues bullets of each task's implementer note.

## Skill updates

(none)

No `skills/**` files were edited in-flight for friction. Every
`Procedural compliance` line across T-001..T-007 reads `(none)`. The
spec edits are themselves the *planned* skill content for REQ-001 /
REQ-002 / REQ-003, not friction patches surfaced during the work.

## Deferred / known limitations

- **Windows `cargo run -p speccy-cli -- verify` self-replace
  race** — T-007 discovered that running the verify driver from
  source on Windows fails because `cargo` cannot replace the running
  `speccy.exe` while the driver shells out to `cargo test` as a
  sub-process. The workaround documented in T-007's implementer note
  is to run the pre-built release binary directly
  (`./target/release/speccy.exe verify`). This is a Windows
  contributor-experience issue, not a SPEC-0014 defect, and is
  picked up cleanly by SPEC-0016 T-008's identical workaround note.
  A follow-up doc/spec could either codify the Windows guidance in
  the contributor docs or change the verify driver to deferred-exec
  the running binary; out of scope for v1.
- **Open question 1 (split `Procedural compliance` vs `Skill
  updates`)** — left unresolved per the SPEC's own lean ("one field,
  six bullets"). Empirical evidence from SPEC-0014..0017 implementer
  notes is that `Procedural compliance` reads `(none)` in nearly
  every task, so the split is not yet warranted. Revisit if a
  multi-spec run produces enough skill edits to make the rollup
  noisy.
- **Open question 2 (promote `## Skill updates` to REPORT.md
  frontmatter)** — deferred per the SPEC's "prose section first,
  formal field only if a harness needs it" lean. No harness
  consumer surfaced during v1; leave it prose.
- **Open question 3 (corresponding planner-prompt section)** — out
  of scope here per the SPEC's own note; revisit when a planner-side
  friction pattern shows up in dogfooding.
