---
spec: SPEC-0068
outcome: implemented
generated_at: 2026-06-28T03:00:00Z
---

# REPORT: SPEC-0068 Evidence-backed demonstrated gate — `speccy journal append` refuses an implementer block claiming `demonstrated` coverage with no backing evidence scenario

<report spec="SPEC-0068">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
T-001 added `demonstrated_chk_ids(implementer_body: &str) -> Vec<String>` in the
new pure module `speccy-core/src/parse/journal_xml/evidence.rs`. The function is
line-scoped: a CHK id is treated as demonstrated only when its own line also
carries the token `demonstrated`, giving the DEC-001 heuristic. In
`append_under_lock` (`speccy-cli/src/journal.rs`) a gate block runs after
`validate_and_render_block` and strictly before the round-trip parse and
`fs_err::write`, preserving the byte-identical-on-failure contract. When the
result is non-empty, the gate requires `spec_dir/evidence/{task_id}.md` to exist
and to contain at least one `### Scenario` heading; otherwise it returns the new
`JournalError::MissingDemonstratedEvidence` variant, carrying the offending CHK
id(s), the expected evidence path, and a reason string distinguishing
`"evidence file missing"` from `"present but carries no ### Scenario heading"`.
Integration tests `bullet_demonstrated_with_no_evidence_is_refused` (CHK-001)
and `prose_demonstrated_with_no_evidence_is_refused` (CHK-002) each assert
non-zero exit, stderr contains both the CHK id and `T-001.md`, and no journal
file exists afterward. `evidence_present_without_scenario_is_refused` (CHK-003)
asserts the specific phrase distinguishing the present-but-no-scenario branch
from the missing-file branch. A refused append leaves the journal byte-identical
in all three scenarios. Retry count: 0.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
The same T-001 gate covers the accept paths. `demonstrated_backed_by_scenario_succeeds`
(CHK-004) writes `evidence/T-001.md` with a `### Scenario` heading first, then
appends a demonstrated-claiming block, and asserts exit zero and exactly one
parsed implementer block in the journal. `no_demonstrated_label_succeeds_without_evidence`
(CHK-005) labels every CHK `hygiene` or `judgment-only` with no evidence file
present and asserts exit zero. `demonstrated_token_on_chk_less_line_succeeds_without_evidence`
(CHK-006) places the token `demonstrated` on a CHK-less line with a hygiene CHK
line, no evidence file, and asserts exit zero — directly falsifying a naive
whole-body substring check. Core unit tests `token_alone_on_chk_less_line_is_not_a_claim`
and `hygiene_only_roll_call_is_empty` back up the line-scoped invariant. Retry count: 0.
</coverage>

<coverage req="REQ-003" result="satisfied" scenarios="CHK-007 CHK-008 CHK-009">
T-002 added an explicit ordered step 9 to `resources/modules/phases/speccy-work.md`
between implement (step 8) and the exit/append step, directing the implementer to
write the canonical `evidence/T-NNN.md` with one red-then-green `### Scenario`
per `demonstrated` CHK before appending, and noting the hard-fail. Trailing steps
renumbered to 10-12; intra-body cross-references updated consistently (first-attempt
span 4-12, retry-branch step refs 9/11). Both `resources/modules/references/evidence.md`
and `resources/modules/references/journal-implementer.md` received a one-line
demonstrated-versus-hygiene disambiguation naming the append refusal. `docs/CLI.md`
names the refusal condition under the `journal append` entry. `docs/SCHEMA.md`
documents the append-time evidence check in the per-task journal section and notes
the absent `speccy verify` lint per DEC-003. `just reeject` regenerated all ejected
copies; the dogfood byte-identity test confirmed eject == source. CHK-009: the
`resource_prose_hygiene` suite ran 65 tests green over the edited phases body.
CHK-007 and CHK-008 are reviewer-docs judgments confirmed by the review round
(all five personas passed on round 1). Retry count: 0.
</coverage>

</report>

## Notes

The line-scoped heuristic (DEC-001) produced one recoverable false positive
during dogfooding: T-002's own first append attempt was refused because the
trigger token co-occurred with a CHK id in the implementer's roll-call prose.
The implementer reworded the roll call so no CHK-bearing line shared a line with
the token. This is the exact recoverable behavior DEC-001 documents.

The `speccy verify` lint for demonstrated-to-scenario consistency was cut as a
non-goal (DEC-003) because existing completed specs predate evidence files and
an error-level lint would retroactively fail CI. That lint remains a clean
follow-up once in-flight specs carry evidence files.
