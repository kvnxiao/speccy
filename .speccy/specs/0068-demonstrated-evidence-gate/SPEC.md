---
id: SPEC-0068
slug: demonstrated-evidence-gate
title: Evidence-backed demonstrated gate — `speccy journal append` refuses an implementer block claiming `demonstrated` coverage with no backing evidence scenario
status: in-progress
created: 2026-06-27
supersedes: []
---

# SPEC-0068: Evidence-backed demonstrated gate — `speccy journal append` refuses an implementer block claiming `demonstrated` coverage with no backing evidence scenario

## Summary

The per-task journal `<implementer>` block carries an `Evidence:` field with a
CHK-by-CHK roll call. A CHK labelled `demonstrated` is meant to be backed by a
red-then-green `### Scenario` block in the standalone evidence file at
`.speccy/specs/NNNN-slug/evidence/T-NNN.md`; a CHK backed only by a passing
suite test is meant to be labelled `hygiene` (citing the test). Today
`speccy journal append` treats the block body as opaque text and validates only
its XML shape via a round-trip parse — it never looks at the roll call or the
evidence file. So an implementer can label a CHK `demonstrated`, write no
evidence file, and have the append succeed. The hygiene gates stay green
(the tests themselves are sound), and the gap surfaces only a full review round
later when `reviewer-tests` bounces the task — and, being an
under-specification, recurs identically on the next task. Each instance costs a
full bounce: five review sub-agents plus a retry implementer pass, all
avoidable.

Grounding in the current code: `run_task_append` in `speccy-cli/src/journal.rs`
derives the journal path from the resolved task location and hands off to
`append_under_lock`, which calls `validate_and_render_block` (pure XML
validation, no body inspection) and then round-trips the would-be file through
the journal parser as a byte-identical-on-failure gate before
`fs_err::write`. The implementer body is an opaque `String`; the `Evidence:`
roll call is never parsed, and no module reads evidence files at all.

This SPEC adds an append-time gate. A pure helper in `speccy-core` extracts the
CHK ids an implementer body labels `demonstrated`, and `append_under_lock`
refuses the write — before the existing round-trip and write, preserving the
byte-identical contract — when any such CHK lacks a backing evidence scenario
on disk. It also closes the happy-path gap that let the omission through: the
`speccy-work` recipe gains an explicit evidence-file-creation step, and the
references disambiguate `demonstrated` from `hygiene`. The gate is the
high-leverage fix because it catches the omission in the same implementer turn,
local to the only sanctioned journal writer.

## Goals

<goals>
- `speccy journal append --block implementer` exits non-zero and writes nothing
  when the block's roll call labels a CHK `demonstrated` while the canonical
  evidence file `evidence/T-NNN.md` is absent or carries no `### Scenario`
  heading; the error names the offending CHK id(s), the expected evidence path,
  and whether the file was missing or present-without-a-scenario.
- The detection recognizes a `demonstrated` claim whether the roll call is
  written as a bullet or as prose, and is line-scoped so the token alone on a
  CHK-less line does not over-trigger.
- The same append succeeds and writes the block when every `demonstrated` CHK
  is backed by an evidence file containing at least one `### Scenario`, and when
  the roll call labels no CHK `demonstrated`.
- A refused append leaves the journal file byte-identical to its pre-append
  state (or absent, for a refused first append).
- The `speccy-work` recipe makes evidence-file creation an unambiguous step
  before the append, and the references disambiguate `demonstrated` (needs a
  scenario) from `hygiene` (cites a test).
</goals>

## Non-goals

<non-goals>
- No `speccy verify` lint for `demonstrated`-to-scenario consistency. Existing
  completed specs predate evidence files, so an error-level lint would
  retroactively fail the in-tree lint test and CI; a warn-level lint adds a
  family and snapshot churn for little gain once the append gate covers the
  workflow. Out of scope here; revisitable once specs carry evidence files.
- No structured grammar for the `Evidence:` roll call. Detection stays a
  line-scoped heuristic over the free-form body.
- No validation of a scenario's internal red-then-green structure. The gate
  checks for the presence of a `### Scenario` heading, nothing deeper.
- No change to the ship-time retro or `.speccy/MEMORY.md` capture rules (the
  feedback's secondary observation).
- No change to `review`, `blockers`, or any VET.md block validation. Only the
  `implementer` block carries a `demonstrated` roll call and only it is gated.
</non-goals>

## User Stories

<user-stories>
- As a `speccy-work` implementer, I want the append to refuse a `demonstrated`
  claim that has no backing evidence file in the same turn, so I create the
  file immediately instead of discovering the gap after a full review bounce.
- As a reviewer, I want the evidence-file contract enforced before review, so a
  review round is spent on substance rather than catching a missing file the
  CLI could have caught mechanically.
</user-stories>

## Requirements

<requirement id="REQ-001">
### REQ-001: Append refuses a `demonstrated` claim with no backing evidence scenario

When an `<implementer>` block is appended via
`speccy journal append --block implementer`, if its roll call labels any CHK
`demonstrated` while the canonical evidence file
`.speccy/specs/NNNN-slug/evidence/T-NNN.md` is absent, or exists but contains no
`### Scenario` heading, the append is refused with a non-zero exit. The error
names the offending CHK id(s), the expected evidence path, and whether the file
was missing or present-without-a-scenario. A `demonstrated` claim is recognized
in both the bullet roll-call form and the prose roll-call form.

<done-when>
- Appending a block whose roll call marks a CHK `demonstrated` with no evidence
  file exits non-zero; stderr names the CHK id and the expected
  `evidence/T-NNN.md` path; no journal file is created.
- The same refusal fires whether the `demonstrated` claim is written in bullet
  form or in prose form.
- Appending such a block when the evidence file exists but has no `### Scenario`
  heading is refused; the error states the file is present but carries no
  scenario.
- A refused append performs no write: the journal file remains byte-identical to
  its pre-append state, or absent for a refused first append.
</done-when>

<behavior>
- Given a first-attempt task with no evidence file, when an implementer block
  claiming a `demonstrated` CHK is appended, then the command exits non-zero and
  no journal file is created.
- Given an evidence file present but containing no `### Scenario` heading, when
  the `demonstrated`-claiming block is appended, then the append is refused
  naming the present-but-no-scenario condition.
</behavior>

<scenario id="CHK-001">
Given a task whose `evidence/T-NNN.md` does not exist,
when `speccy journal append --block implementer` receives a body whose roll-call
bullet reads `- CHK-NNN (...): demonstrated`,
then the command exits non-zero, stderr contains the CHK id and the
`evidence/T-NNN.md` path, and no journal file exists afterward.
</scenario>

<scenario id="CHK-002">
Given the same task with no evidence file,
when the appended body writes the claim in prose form
`CHK-NNN demonstrated by some_passing_test`,
then the command is refused identically — the prose roll-call form is detected.
</scenario>

<scenario id="CHK-003">
Given an `evidence/T-NNN.md` that exists but contains no `### Scenario` heading,
when a `demonstrated`-claiming block is appended,
then the command exits non-zero and stderr distinguishes the
present-but-no-scenario case from the missing-file case.
</scenario>

</requirement>

<requirement id="REQ-002">
### REQ-002: Append accepts a backed or non-demonstrated block unchanged

The gate refuses only unbacked `demonstrated` claims. An implementer block is
written unchanged when every `demonstrated` CHK is backed by an evidence file
carrying at least one `### Scenario` heading, when the roll call labels no CHK
`demonstrated` (only `hygiene` or `judgment-only`), or when the token
`demonstrated` appears only on a line carrying no CHK id. Detection is
line-scoped: a CHK is treated as `demonstrated` only when its own line also
carries the `demonstrated` token.

<done-when>
- Appending a `demonstrated`-claiming block after writing `evidence/T-NNN.md`
  with a `### Scenario` heading exits zero and the journal contains the block.
- Appending a block whose roll call uses only `hygiene` or `judgment-only`
  labels exits zero with no evidence file present.
- A body containing the token `demonstrated` only on a CHK-less line, whose CHK
  lines are all `hygiene` or `judgment-only`, is accepted with no evidence file
  required.
</done-when>

<behavior>
- Given an evidence file with one `### Scenario`, when the
  `demonstrated`-claiming block is appended, then the block is written and the
  command exits zero.
- Given a roll call with no `demonstrated` label, when the block is appended
  with no evidence file, then the command exits zero.
</behavior>

<scenario id="CHK-004">
Given `evidence/T-NNN.md` written first with a `### Scenario` heading,
when a block whose roll call marks `CHK-NNN ... demonstrated` is appended,
then the command exits zero and the journal contains the implementer block.
</scenario>

<scenario id="CHK-005">
Given a body whose roll call labels every CHK `hygiene` or `judgment-only` and
no evidence file exists,
when the block is appended,
then the command exits zero and the journal contains the block.
</scenario>

<scenario id="CHK-006">
Given a body with the token `demonstrated` on a line carrying no CHK id, whose
CHK lines are all labelled `hygiene`, and no evidence file exists,
when the block is appended,
then the command exits zero — the incidental token does not over-trigger the
gate.
</scenario>

</requirement>

<requirement id="REQ-003">
### REQ-003: Recipe and docs make evidence-file creation explicit and disambiguate the labels

The implementation recipe and the schema docs make the standalone evidence file
an explicit deliverable and remove the `demonstrated`-versus-`hygiene`
ambiguity. The `speccy-work` recipe carries an ordered step, between
implementing and appending the `<implementer>` block, to write the evidence file
with one red-then-green `### Scenario` per CHK to be labelled `demonstrated`. The
evidence and journal-implementer references state that a passing suite test is
`hygiene` (cite the test) while `demonstrated` requires a red-then-green
`### Scenario`, and that the CLI now refuses the append otherwise. The
`journal append` contract in the CLI docs documents the refusal condition.

<done-when>
- The regenerated `speccy-work` recipe contains an explicit ordered step to
  create the evidence file before appending the implementer block.
- The evidence and journal-implementer references carry a one-line
  `demonstrated`-versus-`hygiene` disambiguation that names the new append
  refusal.
- The `journal append` entry in the CLI docs names the refusal condition.
- The resource-prose hygiene suite passes over the edited `phases/` body —
  generic placeholders only, no lint code cited by number.
</done-when>

<behavior>
- Given the regenerated recipe, when a reader follows it literally, then writing
  the evidence file is an unambiguous step that precedes the append.
- Given the references, when a reader checks how to label a CHK backed by a
  passing test, then the guidance directs them to `hygiene`, not `demonstrated`.
</behavior>

<scenario id="CHK-007">
Given the regenerated `speccy-work` recipe at HEAD,
when a reviewer reads the steps between implement and append,
then an explicit evidence-file-creation step is present and unambiguous.
Presence-and-clarity is a reviewer-docs judgment, not a scriptable assertion.
</scenario>

<scenario id="CHK-008">
Given the evidence and journal-implementer references at HEAD,
when a reviewer reads the label definitions,
then the `demonstrated`-versus-`hygiene` boundary is stated in one line and
names the append refusal. Persona-review judgment.
</scenario>

<scenario id="CHK-009">
Given the resource-prose hygiene suite,
when it runs over the edited `phases/` recipe body,
then it passes — only generic placeholders appear and no lint code is cited by
number.
</scenario>

</requirement>

## Decisions

<decision id="DEC-001">
Detection is a line-scoped heuristic, not a parsed grammar. A CHK id is treated
as `demonstrated` only when its own line also carries the `demonstrated` token.
This matches both documented roll-call forms — the bullet form and the prose
form observed in real dogfooding — without committing to a brittle grammar over
free-form prose. A false positive is recoverable: the refusal is
byte-identical, so the author fixes the block and re-appends. The gate therefore
biases toward catching the omission over silently accepting it.
</decision>

<decision id="DEC-002">
The gate checks the canonical path `evidence/T-NNN.md` on disk, not a path
parsed out of the roll-call prose. The canonical location is authoritative, so
checking it removes the fragility of parsing a path from free text and collapses
the failure modes (no path named, file absent, no scenario) into one on-disk
check.
</decision>

<decision id="DEC-003">
Enforcement is append-time only; no `speccy verify` lint ships in this SPEC. The
append path is the sole sanctioned journal writer, so the in-turn gate covers
the real workflow. A verify lint could only ship at warn severity given
pre-existing completed specs that carry `demonstrated` labels with no evidence
directory, and was deliberately cut to stay small.
</decision>

<decision id="DEC-004">
Only the `implementer` block is gated. The `demonstrated` roll call lives only
in the implementer Evidence field; `review`, `blockers`, and VET.md blocks carry
no such claim, so the gate is scoped to the implementer block kind alone.
</decision>

## Notes

The line-scoped heuristic is a deliberate trade-off (DEC-001): it cannot fully
parse arbitrary prose, but it matches the two roll-call forms Speccy's own
references teach and the prose form seen in dogfooding. Promotion to a
`speccy verify` lint (DEC-003) is a clean follow-up once existing specs carry
evidence files and the retroactive-failure risk is gone.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-27 | Kevin Xiao | Initial SPEC: append-time gate refusing an implementer block that claims `demonstrated` coverage with no backing evidence scenario (REQ-001), accepting backed or non-demonstrated blocks unchanged (REQ-002), and recipe/reference/doc tightening that makes the evidence file an explicit deliverable and disambiguates `demonstrated` from `hygiene` (REQ-003). Append-time enforcement only; no verify lint. |
</changelog>
