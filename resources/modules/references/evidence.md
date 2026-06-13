# Evidence for SPEC-NNNN T-NNN

Canonical shape of an evidence paper-trail file. The real file lives at
`.speccy/specs/NNNN-slug/evidence/T-NNN.md` (sibling of `SPEC.md` and
`TASKS.md`) and captures the red-then-green pairs that prove the task's
behaviour changed in the expected direction. This example continues the
widget-render-timeout scenario from the sibling `spec.md`, `tasks.md`, and
`journal-implementer.md`.

## Coverage rule

The journal `<implementer>` block's `Evidence:` field carries a CHK-by-CHK roll
call; this file backs the entries it labels `demonstrated`.

- Every `demonstrated` label ↔ exactly one `### Scenario N` block here. Cite the
  scenario heading verbatim in the roll call so the reviewer navigates without
  grepping.
- An orphan scenario (no `demonstrated` label cites it) signals a stale evidence
  file or a missing journal entry.
- CHKs labeled `hygiene` or `judgment-only` get no scenario block: the
  hygiene-gate block below is the catch-all for `hygiene`, and `judgment-only`
  CHKs have no execution artifact — they live in the persona-review surface.

## Session 2026-05-21-attempt-1

Red-then-green trail for the `--timeout-ms` flag added to `widget render` with a
bounded range parser and an exit-124 abort path.

### Scenario 1 — Cycle fixture aborts at the configured budget (CHK-NNN)

<red>
Pre-edit the flag does not exist, so the run leaned on the external wrapper
`timeout 60 widget render fixtures/cycle.gv`, which hangs to the wrapper's own
exit 124 — the abort is external, not from `widget`:

```
$ timeout 60 ./target/debug/widget render fixtures/cycle.gv
(60s of no output)
$ echo $?
124
```
</red>

<green>
Post-edit the binary aborts itself at ~511ms and writes the documented stderr
line:

```
$ ./target/debug/widget render --timeout-ms 500 fixtures/cycle.gv
widget render: aborted after 500ms (timeout reached)
$ echo $?
124
```

Wall-clock 0.51s sits inside the SPEC's 500..600ms window; exit 124 matches the
requirement; the stderr line is byte-identical to the SPEC-mandated form.
Repeated 20×: elapsed 503..519ms (mean 511, stddev 5).
</green>

### Scenario 2 — Range parser rejects out-of-bounds values (CHK-NNN)

<red>
Pre-edit the flag is unknown, so `clap` emits the generic
`error: unexpected argument '--timeout-ms' found` (exit 2) — not the
SPEC-mandated message naming the rejected value and the allowed range.
</red>

<green>
Post-edit a range value parser rejects out-of-bounds values with the mandated
message:

```
$ ./target/debug/widget render --timeout-ms 0 fixtures/cycle.gv
error: invalid value '0' for '--timeout-ms <MS>': --timeout-ms must be between 1 and 600000 (got 0)
$ echo $?
2
```
</green>

## Hygiene-gate evidence

All four standard gates exited 0 after the diff landed:

```
$ cargo test --workspace
test result: ok. 142 passed; 0 failed; 0 ignored
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(no warnings)
$ cargo +nightly fmt --all --check
$ cargo deny check
```
