**Retry shape.** A task is in retry shape iff its journal contains
both an `<implementer>` element and a `<blockers>` element whose
`round` attribute matches the highest implementer round. Otherwise
it's first-attempt shape — the strict clean-tree gate applies. See
`{{ speccy_references_path }}/retry-shape.md` for the full rule
statement, read-only scope, worked examples, and the
"implementer awaiting review" edge case.