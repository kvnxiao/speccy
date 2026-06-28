---
spec: SPEC-0068
spec_hash_at_generation: 89453d2bd6a9df47402d1962780d527d44b5adc3732ab5a8e52b3b5f53eb3797
generated_at: 2026-06-27T21:05:38Z
---
# Tasks: SPEC-0068 Evidence-backed demonstrated gate — `speccy journal append` refuses an implementer block claiming `demonstrated` coverage with no backing evidence scenario

<task id="T-001" state="completed" covers="REQ-001 REQ-002">
## Add the demonstrated-evidence append-time gate

Add a pure detection module `speccy-core/src/parse/journal_xml/evidence.rs`
exposing two functions: `demonstrated_chk_ids(implementer_body: &str) ->
Vec<String>` — line-scoped, a `CHK-NNN` id is treated as demonstrated only when
its own line also carries the token `demonstrated`; the result is deduplicated
and sorted — and `scenario_heading_count(evidence_body: &str) -> usize`, the
count of `### Scenario` headings in an evidence-file body. Declare `pub mod
evidence;` in `speccy-core/src/parse/journal_xml/mod.rs` and re-export both
functions from `speccy-core/src/parse/mod.rs`. Add unit tests in the new module
for: a bullet-form roll call, a prose-form roll call, a body whose CHK lines are
all `hygiene`/`judgment-only` (empty result), a mixed multi-line body, a
same-line guard (the token alone on a CHK-less line yields nothing), and
`scenario_heading_count`.

Add a `JournalError::MissingDemonstratedEvidence` variant in
`speccy-cli/src/journal.rs` carrying the offending CHK id(s), the expected
`evidence/T-NNN.md` path, and whether the file was missing or
present-without-a-scenario; its message ends "journal left unchanged". Thread
`spec_dir` into `AppendInputs`, sourced from `location.spec_dir` in
`run_task_append`. In `append_under_lock`, for the implementer block kind only,
after `validate_and_render_block` and before the round-trip parse and
`fs_err::write`, run `demonstrated_chk_ids` over the body; when it is non-empty,
require the canonical evidence file at `spec_dir`/`evidence`/`{task_id}.md` to
exist and to contain at least one `### Scenario` heading, otherwise return the
new error. Running the check before the write preserves the existing
byte-identical-on-failure contract.

Add integration tests in `speccy-cli/tests/journal_append.rs` following the
byte-identical pattern: a bullet-form demonstrated claim with no evidence file
is refused (stderr names the CHK id and the expected path; no journal file
exists afterward); the prose form is refused identically; an evidence file
present but carrying no scenario heading is refused with the
present-but-no-scenario message; an append after the evidence file with a
scenario heading is written first succeeds; a roll call using only
`hygiene`/`judgment-only` labels succeeds with no evidence file; and a body
whose only `demonstrated` token sits on a CHK-less line succeeds with no
evidence file.

<task-scenarios>
Given a task whose evidence file does not exist,
when an implementer block whose roll call bullet reads `- CHK-NNN (...):
demonstrated` is appended,
then the command exits non-zero, stderr names the CHK id and the
`evidence/T-NNN.md` path, and no journal file is created.

Given the same task with no evidence file,
when the appended body writes the claim in prose form `CHK-NNN demonstrated by
some_passing_test`,
then the command is refused identically.

Given an evidence file that exists but contains no `### Scenario` heading,
when a demonstrated-claiming block is appended,
then the command is refused and stderr distinguishes the present-but-no-scenario
case.

Given an evidence file written first with a `### Scenario` heading,
when a demonstrated-claiming block is appended,
then the command exits zero and the journal contains the implementer block.

Given a body labelling every CHK `hygiene` or `judgment-only`, or carrying the
token `demonstrated` only on a CHK-less line, and no evidence file,
when the block is appended,
then the command exits zero.

Suggested files: `speccy-core/src/parse/journal_xml/evidence.rs`,
`speccy-core/src/parse/journal_xml/mod.rs`, `speccy-core/src/parse/mod.rs`,
`speccy-cli/src/journal.rs`, `speccy-cli/tests/journal_append.rs`
</task-scenarios>
</task>

<task id="T-002" state="completed" covers="REQ-003">
## Make evidence-file creation an explicit recipe step and disambiguate the labels

Edit the source modules under `resources/` (never the ejected copies under
`.claude/`, `.agents/`, `.codex/`). In
`resources/modules/phases/speccy-work.md`, add an explicit ordered step,
between implementing the task and appending the implementer journal block, to
write the evidence file at `.speccy/specs/NNNN-slug/evidence/T-NNN.md` with one
red-then-green `### Scenario` per CHK to be labelled `demonstrated`, before
appending — and note that the append now hard-fails when a `demonstrated` claim
has no backing scenario. In `resources/modules/references/evidence.md` and
`resources/modules/references/journal-implementer.md`, add a one-line
`demonstrated`-versus-`hygiene` disambiguation: a passing suite test is
`hygiene` (cite the test); `demonstrated` requires a red-then-green
`### Scenario` and the CLI now refuses the append otherwise. In `docs/CLI.md`,
note the refusal condition under the `journal append` entry. In
`docs/SCHEMA.md`, note the append-time evidence check in the per-task journal
section. Then run `just reeject` so the ejected skills and agents regenerate
from source.

Honour the resource-prose hygiene suite: the `phases/` body uses only generic
placeholders and cites no lint code by number; concrete SPEC-0042-family ids
stay confined to the `references/` files.

<task-scenarios>
Given the regenerated `speccy-work` recipe at HEAD,
when a reviewer reads the steps between implement and append,
then an explicit evidence-file-creation step is present and unambiguous
(reviewer-docs judgment).

Given the evidence and journal-implementer references at HEAD,
when a reviewer reads the label definitions,
then the `demonstrated`-versus-`hygiene` boundary is stated in one line and
names the append refusal (reviewer-docs judgment).

Given the resource-prose hygiene test suite,
when it runs over the edited `phases/` recipe body,
then it passes — only generic placeholders appear and no lint code is cited by
number.

Suggested files: `resources/modules/phases/speccy-work.md`,
`resources/modules/references/evidence.md`,
`resources/modules/references/journal-implementer.md`, `docs/CLI.md`,
`docs/SCHEMA.md`
</task-scenarios>
</task>
