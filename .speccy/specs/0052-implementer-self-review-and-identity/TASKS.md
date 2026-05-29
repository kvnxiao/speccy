---
spec: SPEC-0052
spec_hash_at_generation: 1a131eab1435625e18f16bff40781c06f9fabdd0d71340e8bf4bc3894ca4b1a0
generated_at: 2026-05-28T23:41:47Z
---
# Tasks: SPEC-0052 Implementer pre-handoff self-review and accurate model/effort recording

<task id="T-001" state="completed" covers="REQ-003">
## Add the shared convention-checklist partial

Create a new partial under `resources/modules/` (suggested
`resources/modules/references/convention-checklist.md`) that
enumerates the five convention-drift categories the SPEC names:
reuse-over-reinvent; match-local-conventions; docs-match-code;
no-false-complexity (explicitly including splitting a function past
the file's own complexity ceiling); and re-apply-the-project's-own-hard-rules
(explicitly including vacuous / constant-copy tests, and suppressions
that must carry a justification). Write it language- and
project-agnostic so it ships safely to downstream repos via
`speccy init` — no Speccy-repo-specific identifiers, no Rust-only
phrasing. This task only creates the partial; T-002 wires it into the
two callsites.

Do not add any prose-asserting unit test for this file (per DEC-005
and the project's test-hygiene rule). The deterministic gate is the
existing `just reeject` + `cargo test --workspace` run exercised by
the wiring task.

<task-scenarios>
Given the new partial after this task,
when a reviewer reads it,
then it enumerates exactly the five named categories, each with the
parenthetical inclusions the SPEC calls out (complexity-ceiling
splitting under no-false-complexity; vacuous/constant-copy tests and
justification-carrying suppressions under re-apply-hard-rules).

Given the partial body,
when a reviewer scans it for repo-specific identifiers,
then it contains no Speccy spec ids, slugs, or repo URLs and reads as
language- and project-agnostic.

Suggested files: `resources/modules/references/convention-checklist.md`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-001 REQ-002 REQ-003">
## Add the implementer self-review step with the persona north-star map and shared-checklist include

In the implementer phase body
(`resources/modules/phases/speccy-work.md`), add a
self-review-before-handoff step positioned after the implement step
and before the `in-review` state flip / hygiene gate. The step
instructs the agent to re-read its own diff against the reviewers'
criteria and fix findings in place, framed as the cheap place to catch
drift versus a later review round. Include a north-star map naming all
four reviewer personas — business, tests, security, style — each with
a one-line statement of the outcome to achieve, withholding the
reviewers' adversarial detection tactics (no description of how the
tests reviewer detects fabrication, no mutation experiments). The
style entry defers to the shared convention checklist rather than
restating its contents, via a `{% include %}` of the T-001 partial.
Wire the same `{% include %}` of the convention-checklist partial into
the style reviewer persona (`resources/modules/personas/reviewer-style.md`)
and remove the style reviewer's prior bespoke "what to look for that's
easy to miss" enumeration, since its unique items are now subsumed by
the partial.

Run `just reeject` and `cargo test --workspace`; both must exit 0,
proving the new include resolves at render time at both callsites and
no pack-structure test breaks.

<task-scenarios>
Given the ejected `speccy-work` phase body after `just reeject`,
when a reviewer reads it,
then a self-review-before-handoff step appears after the implement
step and before the `in-review` flip, instructing the agent to fix
drift in place first, and its north-star map names business, tests,
security, and style with one-line outcomes and no reviewer detection
tactics.

Given the ejected implementer phase body and the ejected style
reviewer persona after `just reeject`,
when a reviewer reads both,
then the same convention-checklist text appears in each sourced from
the one T-001 partial, and the style reviewer's old bespoke
enumeration is gone.

Given the edited `resources/`,
when `just reeject` and `cargo test --workspace` run,
then both exit 0.

Suggested files: `resources/modules/phases/speccy-work.md`,
`resources/modules/personas/reviewer-style.md`
</task-scenarios>
</task>

<task id="T-003" state="pending" covers="REQ-004">
## Add the style reviewer grounding rule against false-positive lint-suppression blocks

In the style reviewer persona
(`resources/modules/personas/reviewer-style.md`), add a grounding rule
that the reviewer does not raise a blocking verdict demanding a
lint-driven change — above all, demanding a suppression annotation be
added — without first confirming the underlying lint fires. The rule
must state that "every sibling file carries it" is insufficient
grounds on its own, and must direct an unconfirmable demand to a
one-line aside outside the `<review>` block rather than a blocking
verdict.

Run `just reeject`; it must exit 0, propagating the rule into the
ejected style reviewer persona across hosts.

<task-scenarios>
Given the ejected style reviewer persona after `just reeject`,
when a reviewer reads it,
then it carries a rule requiring lint-fire confirmation before a
lint-driven blocking verdict, names the sibling-consistency argument
as insufficient on its own, and routes an unconfirmable demand to a
one-line aside outside the `<review>` block.

Given the edited `resources/`,
when `just reeject` runs,
then it exits 0.

Suggested files: `resources/modules/personas/reviewer-style.md`
</task-scenarios>
</task>

<task id="T-004" state="pending" covers="REQ-005 REQ-006 REQ-007">
## Add the shared identity-sourcing partial and wire it into every self-record site

Create a single shared partial under `resources/modules/` stating the
identity-sourcing rule: the recorded model id is the resolved
long-form identifier the host states in-context (e.g.
`claude-opus-4-8[1m]`), transcribed verbatim with version punctuation
preserved (hyphen form, never the dot form), falling back to the
agent definition file's `model:` value where a host states no
in-context identifier; the effort suffix is read from the agent's own
definition file (`effort:` on Claude Code, `model_reasoning_effort`
on Codex) and never from `CLAUDE_EFFORT` or other inherited
environment variables. The partial documents the
`CLAUDE_CODE_EFFORT_LEVEL` runtime-override limitation (deliberately
not read; a run that sets it records the definition-file effort).
`{% include %}` this partial at the implementer phase
(`resources/modules/phases/speccy-work.md`), the reviewer
verdict-return contract
(`resources/modules/personas/verdict_return_contract.md`), and the vet
personas (`vet-implementer.md`, `vet-reviewer.md`, `vet-simplifier.md`).

Run `just reeject` and `cargo test --workspace`; both must exit 0,
proving the identity-sourcing partial resolves at every include site.

<task-scenarios>
Given the new identity-sourcing partial after this task,
when a reviewer reads it,
then it states the model-from-host-in-context rule with verbatim
hyphen-form punctuation and a definition-file fallback, the
effort-from-definition-file rule forbidding inherited env, and the
documented `CLAUDE_CODE_EFFORT_LEVEL` override limitation.

Given the ejected implementer phase, reviewer verdict contract, and
vet personas after `just reeject`,
when a reviewer reads them,
then each carries the identity-sourcing rule from the one shared
partial rather than restating it.

Given the edited `resources/`,
when `just reeject` and `cargo test --workspace` run,
then both exit 0.

Suggested files:
`resources/modules/references/identity-sourcing.md`,
`resources/modules/phases/speccy-work.md`,
`resources/modules/personas/verdict_return_contract.md`,
`resources/modules/personas/vet-implementer.md`,
`resources/modules/personas/vet-reviewer.md`,
`resources/modules/personas/vet-simplifier.md`
</task-scenarios>
</task>

<task id="T-005" state="pending" covers="REQ-008">
## Correct the misleading env-var identity guidance and dot-form example strings

Replace the "sourced from the host harness's runtime model identifier
(env var, runtime API, ...)" phrasing in the review fan-out partial
(`resources/modules/skills/partials/review-fanout.md`) and in the
decompose phase (`resources/modules/phases/speccy-decompose.md`) with
the single-source rule (deferring to the T-004 identity-sourcing
partial where appropriate, rather than restating the env-var
phrasing). Correct any dot-form example strings (`claude-opus-4.8`) in
the touched shipped resources to the canonical hyphen form
(`claude-opus-4-8`).

Run `just reeject`; it must exit 0, propagating the corrected guidance
into the ejected packs across hosts.

<task-scenarios>
Given the ejected resources after `just reeject`,
when a reviewer greps the changed files,
then the env-var/runtime-API sourcing phrasing is gone from the review
fan-out and decompose paths, and no dot-form `claude-opus-4.8` example
remains in the touched files.

Given the edited `resources/`,
when `just reeject` runs,
then it exits 0.

Suggested files:
`resources/modules/skills/partials/review-fanout.md`,
`resources/modules/phases/speccy-decompose.md`
</task-scenarios>
</task>

<task id="T-006" state="pending" covers="REQ-009">
## Document the required `<changelog>` element in the speccy-plan SPEC template reference

In the SPEC template reference
(`resources/modules/references/spec.md`), add a `## Changelog` section
with a `<changelog>` table in its worked example, matching the
`Date | Author | Summary` shape archived specs already use. Add
`<changelog>` to the reference's "Shape invariants the lint suite
enforces" list of required elements, so an author following the
template verbatim does not trip an `SPC-001` missing-changelog parse
error.

Run `just reeject`; it must exit 0, propagating the corrected
reference into the ejected `speccy-plan` skill across hosts.

<task-scenarios>
Given the ejected `speccy-plan` SPEC template reference after
`just reeject`,
when a reviewer reads it,
then it shows a `## Changelog` / `<changelog>` worked example using the
`Date | Author | Summary` shape and lists `<changelog>` among the
enforced shape invariants.

Given the edited `resources/`,
when `just reeject` runs,
then it exits 0.

Suggested files: `resources/modules/references/spec.md`
</task-scenarios>
</task>
