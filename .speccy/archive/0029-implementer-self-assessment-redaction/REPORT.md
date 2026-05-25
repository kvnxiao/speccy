---
spec: SPEC-0029
outcome: delivered
generated_at: 2026-05-18T00:00:00Z
---

# Report: SPEC-0029 Implementer self-assessment redaction in reviewer prompts

<report spec="SPEC-0029">

## Outcome

delivered

## Requirements coverage

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001">
`TASKS_ELEMENT_NAMES` in `speccy-core/src/parse/task_xml/mod.rs`
now lists `["tasks", "task", "task-scenarios", "implementer-note",
"review", "retry"]`. `validate_tag_shape` rejects unknown
attributes for each new element; dedicated `ParseError` variants
fire for missing `session`, empty `<implementer-note>` body,
invalid `verdict`, and invalid `persona`. Exercised by the
round-trip and malformed-fixture tests in
`speccy-core/tests/task_xml_body_items.rs` plus the existing
`speccy-core/src/parse/task_xml/mod.rs` unit tests.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-002">
`BodyItem` enum (variants `ImplementerNote`, `Review`, `Retry`,
each carrying attributes plus verbatim body and `ElementSpan`) is
declared in `speccy-core/src/parse/task_xml/mod.rs`. `Task.body_items:
Vec<BodyItem>` preserves source order across mixed kinds.
`ReviewVerdict::{Pass, Blocking}` lands in `speccy_core` with
`as_str`/`from_str` mirroring `TaskState`. Pinned by source-order,
round-trip, and `ReviewVerdict` unit tests in
`speccy-core/tests/task_xml_body_items.rs`.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-003">
`speccy-cli/src/review.rs:137` substitutes the redacted projection
emitted by `speccy_core::parse::redact_implementer_notes` into
`{{task_entry}}`. Helper takes no `Persona` parameter; inserts no
placeholder. Verified by the five integration tests in
`speccy-cli/tests/review_redaction.rs`, including the
six-sub-bullet forbidden-substring scan and the byte-identical
`## Task entry` cross-persona assertion over
`speccy_core::personas::ALL`.
</coverage>

<coverage req="REQ-004" result="satisfied" scenarios="CHK-004">
`speccy-cli/src/implement.rs:107` is byte-identical pre/post
(still substitutes `task_entry_raw`); `speccy-cli/src/report.rs`
carries no `redact_implementer_notes` call and consumes typed
`body_items` for retry counting; `resources/modules/prompts/report.md`
still derives `## Skill updates` from `Procedural compliance`
lines. The REQ-004 carve-out test in
`speccy-cli/tests/review_redaction.rs` asserts the implement
prompt carries `<implementer-note` verbatim. After migration every
in-tree TASKS.md still carries `<implementer-note>` element bodies
in full (the conversion is syntactic, not a content strip).
</coverage>

<coverage req="REQ-005" result="satisfied" scenarios="CHK-005">
`resources/modules/prompts/implementer.md`,
`resources/modules/prompts/reviewer-{business,tests,security,style,architecture,docs}.md`,
and `.claude/skills/speccy-review/SKILL.md` (plus its `.agents/`
mirror and the shared `resources/modules/skills/speccy-review.md`
include) emit the new XML elements. md5sum is identical across
the six reviewer prompts (`5beb66f5…`), satisfying the
lockstep-wording contract. Legacy `- Implementer note (session-`,
`- Review (`, and `- Retry:` markdown forms are absent from
`resources/` and `.claude/`.
</coverage>

<coverage req="REQ-006" result="satisfied" scenarios="CHK-006">
`speccy-core/tools/migrate_tasks_schema/` (private one-shot
binary, not a CLI subcommand) converted every in-tree TASKS.md
under `.speccy/specs/`. `spec_hash_at_generation` values are
unchanged (hash is over SPEC.md, not TASKS.md). All TASKS.md
parses cleanly under the new whitelist, including round-trip
through `task_xml::render`. The done-when greps
`^- Implementer note (session-`, `^- Review (\w+, `, and
`^- Retry: ` return zero matches; this required a small in-PR
post-fix to T-004's own body (see Out-of-scope items absorbed).
The migration script is idempotent — a second run is a no-op.
</coverage>

<coverage req="REQ-007" result="satisfied" scenarios="CHK-007">
`Task.notes()` is removed from `speccy-core/src/parse/task_xml/mod.rs`.
`speccy-cli/src/report.rs`'s retry-counting flow consumes typed
`BodyItem::Retry` variants via `body_items`. Tests previously
exercising `notes()` are rewritten against `body_items` in
`speccy-core/tests/task_xml_body_items.rs` and
`speccy-cli/tests/report.rs`; retry-count semantics are preserved
(one increment per `BodyItem::Retry`).
</coverage>

## Task summary

- Total tasks: 5 (T-001 through T-005), all `state="completed"`.
- Tasks with retries: 1 (T-001, one retry triggered by a
  reviewer-style blocking verdict on the first pass; resolved by
  tightening test placement and clippy hygiene around the new
  parser surface). Retry summary derived from inline `<retry>`
  elements via `speccy report`'s typed `body_items` flow.
- SPEC amendments during the loop: none. The Changelog row is
  the original draft; Open Questions resolved before any task
  ran.

## Out-of-scope items absorbed

- Post-T-005 cleanup of T-004's own review notes. T-004 was
  reviewed before T-005 retired the legacy markdown-bullet form
  from shipped skill prompts, so the four passing reviews on
  T-004 had been written as `- Review (persona, pass): …`
  markdown bullets. The migration script ran during T-002/T-003
  and could not retroactively re-migrate later writes. As part
  of the ship pass these four review bodies were converted to
  the canonical `<review persona="..." verdict="pass">…</review>`
  XML form in `.speccy/specs/0029-…/TASKS.md` so REQ-006's
  done-when grep (`^- Review (\w+, `) returns clean post-ship.
  No behaviour change; the parser already accepts both layouts
  inside `<task>` bodies, but the SPEC's own contract demanded
  zero remaining bullets.

## Skill updates

(none) — no `skills/**` files were edited in-flight for
friction reasons during this SPEC. Procedural-compliance lines
across all five implementer notes report "(none)" or scope edits
to private migration tooling (`speccy-core/tools/migrate_tasks_schema/`),
which is not a shipped skill surface.

## Deferred / known limitations

- Line-based redactor in `speccy-core/src/parse/task_xml/mod.rs`
  keys on `trim_start().starts_with("<implementer-note ")` /
  `"</implementer-note>"`, which assumes the canonical multi-line
  rendering. The parser also accepts an inline single-line form;
  a hand-edited TASKS.md or future writer-skill regression could
  defeat the redactor's state machine. Acceptable for v1 because
  the migration script and the T-005 writer-side skills both emit
  canonical form. Flagged by reviewer-security on T-004 as a
  robustness gap (not a confidentiality bypass — an adversarial
  reviewer agent can always read TASKS.md directly via its Read
  primitive). Worth filing as F-N to re-render through
  `task_xml::render` or anchor on `BodyItem::ImplementerNote`
  spans.
- `skill_packs.rs` has no compile-time guardrail locking the new
  XML-element authoring instructions into the shipped reviewer
  prompts (e.g. no `read_prompt("implementer.md").contains("<implementer-note session=")`
  assertion, no lockstep-byte-identity property across the six
  reviewer prompts). Consistent with CHK-005's grep-driven
  validation philosophy, but the next slice touching reviewer
  prompts has no compile-time backstop. Out of scope for T-005;
  candidate for a follow-on hardening SPEC.
- Carried-forward `clippy::result_large_err` warning against
  `speccy_core::error::ParseError` (from SPEC-0026 T-003)
  inherits the existing `#[expect]` suppression; this SPEC adds
  four new variants and increases the surface but does not address
  the underlying cleanup. F-7 in the backlog tracks it.

</report>
