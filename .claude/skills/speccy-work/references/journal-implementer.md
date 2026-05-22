# Worked-instance reference: per-task journal `<implementer>` block

This file shows the canonical post-SPEC-0034 + post-SPEC-0037 shape of
an `<implementer>` block inside a per-task journal file. The example
continues the SPEC-0042 widget-render-timeout scenario from `spec.md` /
`tasks.md` in this directory.

A real journal file lives at
`.speccy/specs/NNNN-slug/journal/T-NNN.md` (sibling of `SPEC.md` and
`TASKS.md`) and is parsed by `speccy verify` against the `JNL-*` lint
family.

---

## Full journal file shape

The first journal entry on a task creates the file with YAML
frontmatter declaring exactly three fields, then the `<implementer>`
block beneath.

```markdown
---
spec: SPEC-0042
task: T-001
generated_at: 2026-05-21T19:45:00Z
---

<implementer date="2026-05-21T19:45:00Z" model="claude-opus-4.7[1m]/low" round="1">
- Completed: Landed `--timeout-ms <N>` on `widget render` end-to-end.
  Added `timeout_ms: u32` to the `clap`-derived `RenderArgs` struct
  in `widget-cli/src/args.rs` with a range value parser bounded
  `1..=600000` and `default_value = "30000"`. Threaded the value
  into `widget-core::render::Renderer` as a `Duration` budget field;
  the render loop polls `Instant::elapsed` once per iteration and
  returns the new `RenderError::TimedOut { budget_ms }` variant on
  overshoot. CLI maps that variant to `std::process::exit(124)`
  after writing the deterministic stderr line
  `widget render: aborted after <budget_ms>ms (timeout reached)`.
  Unit tests cover the range parser (rejects 0 / 600001, accepts 1
  / 30000 / 600000); a new integration test
  `widget-cli/tests/timeout.rs` drives the binary against
  `fixtures/cycle.gv` with `--timeout-ms 500` and asserts exit 124,
  the stderr line, and wall-clock elapsed in `500..=600ms`.

- Undone: T-002 (help-text and README documentation) is left for a
  separate implementer turn per its `<task>` element in TASKS.md.
  Nothing from T-001's scope is deferred.

- Hygiene checks: `cargo test --workspace` exited 0 with 142 tests
  passing including the new `range_parser_rejects_zero`,
  `range_parser_accepts_boundaries`, and
  `cycle_fixture_times_out_at_500ms` cases. `cargo clippy
  --workspace --all-targets --all-features -- -D warnings` exited 0
  with zero new warnings introduced. `cargo +nightly fmt --all
  --check` exited 0. `cargo deny check` exited 0 (no new
  dependencies added — `Duration` and `Instant` are `std`).

- Evidence: red-then-green paper trail in
  `.speccy/specs/0042-widget-render-timeout/evidence/T-001.md`
  records two scenarios: scenario 1 captures the pre-edit run of
  `cargo run -- render --timeout-ms 500 fixtures/cycle.gv` hanging
  past 60s (killed manually, no exit code) inside `<red>`, and the
  post-edit run exiting 124 in 511ms with the expected stderr line
  inside `<green>`. Scenario 2 captures the range-parser rejection
  paths via `cargo run -- render --timeout-ms 0` exiting 2 with the
  documented stderr message.

- Discovered issues: The pre-existing
  `widget-core::render::cycle_detector::CycleDetector` carries an
  inline comment marking cycle handling as deferred work dating to
  before this SPEC. The comment is unchanged by this task (the
  timeout flag is the user-facing workaround until cycle detection
  lands as its own SPEC); flagged here so the next implementer
  touching the cycle detector sees the context.

- Procedural compliance: This implementer entry lands directly in
  `journal/T-001.md` per the post-SPEC-0037 schema. No
  `<implementer-note>` block was written into TASKS.md (that
  element was retired in SPEC-0037 and the parser rejects it). The
  TASKS.md `state="..."` attribute for T-001 flips from
  `in-progress` to `in-review` as the final step of this turn. No
  shipped skill bodies under `skills/` required edits during this
  task — the implementer prompt at HEAD already documents the
  post-SPEC-0034 six-field handoff template, so no
  friction-to-skill-update was triggered.
</implementer>
```

---

## Required attributes on `<implementer>`

All three are required; there are no optional attributes:

- `date` — full ISO8601 date-time with seconds and timezone
  designator (e.g. `2026-05-21T19:45:00Z` or
  `2026-05-21T19:45:00+00:00`).
- `model` — the model identity that ran the implementer turn. A
  slash-suffix encodes effort or reasoning-intensity when the host
  harness exposes that knob (e.g. `claude-opus-4.7[1m]/low`,
  `claude-opus-4.7[1m]/medium`). Hosts without an effort knob omit
  the suffix (e.g. `model="claude-opus-4.7"`). The parser validates
  `model` is non-empty but does not enforce suffix membership.
- `round` — monotonic positive integer starting at 1. Increment by
  exactly 1 on each post-blocker retry attempt. The first
  implementer turn on a task is `round="1"`; if a review round
  blocks and the task flips back to `pending`, the next implementer
  attempt writes `round="2"`, and so on. Do not skip values; do not
  reset.

## Six-field handoff template

The body of every `<implementer>` block uses these six fields in
this order, each as a bullet line prefixed by `- <Field>:`. The
field labels are the post-SPEC-0034 canonical set; the pre-SPEC-0034
labels `Commands run` and `Exit codes` are retired as bullet-line
prefixes and folded into the `Hygiene checks` field as narrative
prose.

- **Completed**: what landed in this turn, named concretely
  (files touched, behaviours observed). Past tense.
- **Undone**: what is deliberately deferred and why; what is left
  for the next turn or a follow-up task.
- **Hygiene checks**: the project's four standard hygiene gates
  (or the project-equivalent set) and their observed exit codes;
  any other commands the implementer ran for verification.
- **Evidence**: pointer to the per-task `evidence/T-NNN.md` paper
  trail; one-sentence summary of which scenarios it covers and the
  red-then-green pattern recorded.
- **Discovered issues**: pre-existing problems noticed but not
  fixed (out-of-scope); context for the next implementer.
- **Procedural compliance**: confirms the state transition in
  TASKS.md, confirms no retired XML elements were written, and
  notes any shipped-skill edits made per the
  "friction-to-skill-update" convention in AGENTS.md.

## Subsequent rounds

On retry after a blocking review, append a new `<implementer>`
block after the existing journal contents — do not modify earlier
blocks. The new block carries the next monotonic `round`. The
`generated_at` in frontmatter stays at its original file-creation
timestamp; do not rewrite it.
