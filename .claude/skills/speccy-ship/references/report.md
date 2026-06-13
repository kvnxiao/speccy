# Worked-instance reference: `REPORT.md`

This file shows the canonical shape of a Speccy
`REPORT.md`. The example continues the SPEC-NNNN widget-render-timeout
scenario from `spec.md` / `tasks.md` in this directory.

A real `REPORT.md` lives at `.speccy/specs/NNNN-slug/REPORT.md` and is
parsed by `speccy verify` against the `RPT-*` lint family.

---

```markdown
---
spec: SPEC-NNNN
outcome: implemented
generated_at: 2026-05-22T08:15:00Z
---

# REPORT: SPEC-NNNN Widget render timeout flag — `widget render` accepts `--timeout-ms` and aborts long renders

<report spec="SPEC-NNNN">

<coverage req="REQ-NNN" result="satisfied" scenarios="CHK-NNN CHK-NNN">
T-NNN added `timeout_ms: u32` to `widget-cli/src/args.rs` with a
`clap` range value parser bounded `1..=600000` and a `default_value
= "30000"`. The parser rejects 0 and 600001 at argument-parse time
with the documented stderr message naming the rejected value and
the allowed range. Default behaviour was verified via the existing
`--print-config` debug flag asserting the effective timeout reads
back as 30000 when the flag is omitted. Retry count: 0.
</coverage>

<coverage req="REQ-NNN" result="satisfied" scenarios="CHK-NNN CHK-NNN">
T-NNN threaded the budget into `widget-core::render::Renderer` as a
`Duration` field; the render loop polls `Instant::elapsed` once per
iteration and returns `RenderError::TimedOut { budget_ms }` on
overshoot. The CLI maps that variant to `std::process::exit(124)`
after writing `widget render: aborted after <budget_ms>ms (timeout
reached)` to stderr. The integration test against
`fixtures/cycle.gv` observed wall-clock elapsed of 511ms +/- 8ms
across 20 runs, inside the SPEC's 500..600ms acceptance window.
Successful renders against `fixtures/trivial.gv` exited 0 with no
timeout-attributable stderr output. Retry count: 0.
</coverage>

</report>

## Notes

The 600000ms upper bound surfaced one design question during review:
whether the bound should be a CLI-only guardrail or also enforced in
`widget-core` library callers. DEC-NNN in SPEC-NNNN settled this as
CLI-only; library callers continue to pass any `Duration` they want.
The integration test suite carries one explicit library-caller test
passing `Duration::from_secs(3600)` to confirm the library layer
does not duplicate the bound.
```

---

## Shape invariants the lint suite enforces

- YAML frontmatter declares `spec`, `outcome`, `generated_at`.
  `outcome` is typically `implemented`, `partial`, or `abandoned`.
- A `# REPORT: SPEC-NNNN ...` heading opens the body.
- The `<report>` root element carries the required `spec="SPEC-NNNN"`
  attribute; the report-root lint flags its absence.
- One `<coverage>` element per requirement in the SPEC, in
  numerical order. Required attributes: `req="REQ-NNN"`,
  `result="<verdict>"`, `scenarios="CHK-NNN CHK-NNN ..."`
  (space-separated list of scenario ids that prove the coverage
  claim).
- `result` is one of `satisfied`, `partial`, `deferred`. A requirement
  dropped by amendment is removed from `SPEC.md` rather than carried as
  a coverage row, so there is no `not-applicable` result.
- Coverage body prose is free-form but ends with a `Retry count:
  <N>` line naming how many implementer rounds the requirement
  took to land (zero on a clean first pass).
- Unresolved placeholder substrings (the conventional unfinished-draft
  markers and angle-bracket ellipses) do not appear anywhere in a real
  REPORT.md.
