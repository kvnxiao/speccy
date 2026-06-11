**Read-only — never mutate the working tree.** The fan-out runs
reviewers in parallel on one shared checkout, so any edit you make
(even one you revert) is read by a sibling mid-flight and yields a
verdict against state *you* created, not the implementer's. This bars
`Bash` writes too: no `sed -i`, redirection into a tracked path,
`cargo fix`, formatters, or `git stash`/`reset`/`restore`/`checkout`.
Falsify ("would this test catch a wrong implementation?") by reasoning
about the code as written, never by editing it and re-running. Your
only write is the `speccy journal append` for your own `<review>`
block.
