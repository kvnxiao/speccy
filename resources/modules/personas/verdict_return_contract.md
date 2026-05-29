Your final message to the orchestrator **must** be a single
`<review persona="{{ persona_name }}" verdict="..." model="...">…</review>`
element block — structured enough for the orchestrator to parse without
ambiguity. On a `verdict="pass"` result, a one-line summary
suffices. On a `verdict="blocking"` result, include the blocker
body text you want recorded against the task so the orchestrator
can aggregate it into the consolidated `<blockers>` element it
appends to `.speccy/specs/NNNN-slug/journal/T-NNN.md`.

## The `model` attribute is required

Every returned `<review>` element **must** carry a `model`
attribute identifying the reviewer subagent that produced the
verdict. This is non-optional. Reviewer personas can pin different
model tiers, so the orchestrator cannot infer per-reviewer model
identity from skill-pack identity alone — it has to read the value
off your reply.

Encode reasoning effort (when your host harness exposes an effort
knob) as a slash-suffix on the model string itself rather than as a
separate attribute. The slash-suffix is a convention, not a
parser-enforced schema; the orchestrator copies whatever string you
put in `model` verbatim into the per-task journal entry.

{% include "modules/references/identity-sourcing.md" %}

## Orchestrator-side transcription rule

When the orchestrator transcribes your returned `<review>` block
into `.speccy/specs/NNNN-slug/journal/T-NNN.md`, it copies the
`model` attribute **verbatim** from your reply into the journal
entry. The orchestrator does not infer a model value from the
skill-pack identity, the persona name, or any other source.

## No-substitute clause

If a reviewer subagent returns a `<review>` element without a
`model` attribute, the orchestrator surfaces the contract
violation (e.g. by halting the review fan-out and reporting the
non-conforming persona) rather than inventing a model value to
transcribe into the journal. Missing `model` is a hard error on
the return contract — the orchestrator will not paper over it.

{% include "modules/personas/no_tasks_md_writes.md" %}
