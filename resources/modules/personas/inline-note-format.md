The review body you pipe on stdin to `speccy journal append`:

    <one-line verdict>.
    <optional file:line refs and details>.

The CLI wraps this body in the `<review persona="{{ persona_name }}"
verdict="..." model="..." date="..." round="...">` element and stamps
the `date` and `round` attributes itself — your body is the inner text
only, not the wrapping element. On a `blocking` verdict, make the body
concrete (what was expected, what was observed, the file:line
evidence) so the orchestrator can aggregate it into the consolidated
`<blockers>` directive.
