You write your own `<review>` block to the per-task journal via
`speccy journal append`, then return a **thin verdict** to the
orchestrator. You do **not** return a full `<review>` block body as
your final message, and you do **not** edit the journal file with
file-editing tools.

## Step 1 — append your `<review>` block via the CLI

The orchestrator's prompt gives you the task selector
(`SPEC-NNNN/T-NNN`). Pipe your review body on stdin to:

```bash
speccy journal append SPEC-NNNN/T-NNN --block review \
  --persona {{ persona_name }} --verdict <pass|blocking> --model <your-model> <<'EOF'
<your review body — see "Review body" below>
EOF
```

{% include "modules/references/cli-stamps.md" %}

Here `round` is the journal's current implementer round; the append
is rejected if no `<implementer>` block exists yet for the round you
are reviewing. The CLI's per-file lock serializes concurrent
appends, so every reviewer can append in parallel without
interleaving.

## The `--model` value is required

The `journal append` invocation requires `--model` for a `review`
block, identifying the reviewer subagent that produced the verdict.
Reviewer personas can pin different model tiers, so the value cannot
be inferred from skill-pack identity — you supply it. Encode reasoning
effort (when your host harness exposes an effort knob) as a
slash-suffix on the model string itself; the slash-suffix is a
convention, not a parser-enforced schema.

{% include "modules/references/identity-sourcing.md" %}

## Step 2 — return a thin verdict

After the append succeeds, your final message to the orchestrator
**must** be a single self-closing `<verdict>` element — the one
parseable shape every persona returns, so the orchestrator parses all
returns uniformly:

```
<verdict persona="{{ persona_name }}" verdict="pass|blocking" model="<your-model>" rationale="<one line>" />
```

- `persona` — your persona name (`{{ persona_name }}`).
- `verdict` — `pass` or `blocking`, matching the `--verdict` you
  appended.
- `model` — the same model string you passed to `--model`, verbatim.
- `rationale` — a single line. On `pass`, a one-line summary of what
  you checked. On `blocking`, a one-line statement of the blocker —
  the full blocker detail lives in the `<review>` body you already
  appended, which the orchestrator reads back via `speccy journal show
  --verdict blocking` when consolidating `<blockers>`.

Do not restate the full review body in the thin verdict — the body is
already in the journal. The thin verdict exists so the orchestrator
can narrate progress and decide whether to consolidate blockers
without re-reading every block.

{% include "modules/personas/no_tasks_md_writes.md" %}
