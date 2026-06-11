The CLI is the sole authority for the appended block's `date` and
`round` attributes and for the journal's structural scaffolding
(creating the file with frontmatter, sectioning where the journal
has it). **Do not compute, supply, or hand-author `date`, `round`,
or the block's open/close tags** — there is no flag to override
them; the body you pipe on stdin is the inner text only, and the
CLI emits the paired element. Validation runs before any write; a
malformed body leaves the journal byte-identical.
