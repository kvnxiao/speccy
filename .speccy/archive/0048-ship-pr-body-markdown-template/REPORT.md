---
spec: SPEC-0048
outcome: implemented
generated_at: 2026-05-27T07:15:00Z
---

# REPORT: SPEC-0048 Markdown PR body template — `/speccy-ship` assembles markdown from spec artifacts instead of piping raw REPORT.md XML

<report spec="SPEC-0048">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-001 created `resources/modules/references/pr-body.md` with the prescribed
four-section skeleton (`## Summary`, `## Coverage`, `## Test plan`,
`## Reference docs`) inside a fenced markdown block. The Coverage section
carries the `| Req | Result | Scenarios | Retries |` table header with a
`<coverage-rows>` placeholder line. The Test plan section lists the five
`- [x]` checklist items verbatim: `cargo test --workspace`, the clippy
command, `cargo +nightly fmt --all --check`, `cargo deny check`, and
`speccy verify`. The `## Scope: one SPEC per PR` section names the
hand-authored-body fallback for branches bundling multiple SPECs or
carrying unrelated precursor commits. The `## Filling the placeholders`
section documents three angle-bracket placeholders (`<spec-dir>`,
`<summary>`, `<coverage-rows>`) with per-column derivation rules. The
`## Anti-patterns` section prohibits raw-XML paste, fabricated rows, edits
to the fixed Test plan checklist, and dropping the Claude Code footer. Retry count: 1
(round-1 used `{{ ... }}` placeholder syntax; round-2 corrected to angle-bracket
form after business + style review).
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005">
T-001 created the two MiniJinja partials
(`resources/agents/.claude/skills/speccy-ship/references/pr-body.md.tmpl`
and `resources/agents/.agents/skills/speccy-ship/references/pr-body.md.tmpl`),
each containing exactly `{% include "modules/references/pr-body.md" %}` with
no trailing newline, matching the `report.md.tmpl` convention. Both dogfood
mirrors (`.claude/skills/speccy-ship/references/pr-body.md` and
`.agents/skills/speccy-ship/references/pr-body.md`) are byte-identical to
the canonical source. The `chk022_no_orphan_references` test in
`speccy-cli/tests/skill_body_discovery.rs` passed under
`cargo test --workspace`, enforcing source-to-host parity, cross-host parity,
and consuming-body presence under both host packs. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-006 CHK-007 CHK-008">
T-001 updated step 5 of `resources/modules/phases/speccy-ship.md` and both
dogfood mirrors (`.claude/agents/speccy-ship.md` and
`.codex/agents/speccy-ship.toml`) to name `references/pr-body.md` as the
canonical template, instruct the agent to render the template from `SPEC.md`,
`REPORT.md`, and the spec-dir path, write the result to a scratch file, and
call `gh pr create --body-file <path>`. The prohibition on piping `REPORT.md`
inline is written without the literal forbidden substring (rewrote as "Do not
pipe `REPORT.md` inline via shell command substitution into the `--body` flag").
A multi-SPEC fallback paragraph names hand-authoring as the path for branches
bundling multiple SPECs or carrying unrelated precursor commits. The
`--body "$(cat` substring is absent from all three recipe sites; it survives
only in the canonical reference's `## Anti-patterns` section, which is the
legitimate site naming the anti-pattern. Retry count: 1 (round-1 prose said
"double-brace markers" and `coverage rows`; round-2 corrected to
"angle-bracket markers" and `coverage-rows` after business + style + tests review).
</coverage>

</report>

## Notes

The angle-bracket placeholder form (`<spec-dir>`, `<summary>`, `<coverage-rows>`)
was chosen over `{{ ... }}` because the MiniJinja rendering pipeline treats
`{{ ... }}` as expression delimiters — the template body sits inside a
`{% include %}` path where double-brace tokens would require a raw-string
loader to pass through uninterpreted. Angle-bracket tokens carry the
placeholder semantics clearly without colliding with the render engine.
SPEC.md was amended (Amendment 2) to normalize the REQ-001 prose to
angle-bracket form, resolving the prose/artifact lexical mismatch surfaced
by the vet invocation 1 drift review.

This is the first PR on this repo shipped via the new `--body-file` recipe
that SPEC-0048 itself introduced, so the PR body serves as the dogfood
demonstration of the template shape.
