---
spec: SPEC-0044
outcome: implemented
generated_at: 2026-05-24T23:51:33Z
---

# Report: SPEC-0044

<report spec="SPEC-0044">

<coverage req="REQ-001" result="satisfied" scenarios="CHK-001 CHK-002 CHK-003">
The reviewer-persona carve-out is fully removed. `is_host_native_reviewer_file` and its
caller branch in `append_host_pack_items` are deleted from `speccy-cli/src/init.rs`.
The `use speccy_core::personas::ALL as PERSONAS_ALL` import is gone. Every rendered
host-pack file -- including `.claude/agents/reviewer-<persona>.md` and
`.codex/agents/reviewer-<persona>.toml` -- is now classified by `classify_content`
uniformly (Create / Unchanged / Conflict).

CHK-001 is satisfied by `t002_claude_reviewer_agent_files_overwrite_user_edits_under_force`
in `speccy-cli/tests/init.rs`: the test appends a sentinel to
`.claude/agents/reviewer-business.md`, runs `--force`, asserts the sentinel is absent,
byte-equality with the shipped bundle holds, and `(!) overwritten` appears in stdout.

CHK-002 is satisfied by `t002_codex_reviewer_agent_files_overwrite_user_edits_under_force`:
same sentinel-overwrite pattern against `.codex/agents/reviewer-security.toml` (fixed
in VET invocation 2 after invocation 1 flagged the path discrepancy).

CHK-003 is satisfied by the hygiene gate: no hits for `is_host_native_reviewer_file`,
`Skip-on-exists`, or `PERSONAS_ALL` in `speccy-cli/src/init.rs`; `cargo clippy
--workspace --all-targets --all-features -- -D warnings` exits 0 with no `dead_code`
or `unused_imports` warnings.
</coverage>

<coverage req="REQ-002" result="satisfied" scenarios="CHK-004 CHK-005 CHK-006">
All documentation, docstrings, and tests describe one uniform `--force` rule with no
per-file exception.

CHK-004 is satisfied: `docs/ARCHITECTURE.md` contains zero occurrences of
`Skip-on-exists`; all three named regions (lines ~323, ~1684, ~1889) are rewritten
to describe the uniform Create / Unchanged / Conflict rule. No surviving prose claims
that reviewer-file edits survive `--force`.

CHK-005 is satisfied (judgment): `InitArgs::force`, the `Action` enum docstring, the
`Action::Unchanged` variant docstring, the `build_plan` comment, and the
`execute_plan` comment in `speccy-cli/src/init.rs` all describe the uniform rule
without carve-out language; zero `Skip-on-exists` occurrences remain in that file.

CHK-006 is satisfied: the two tests renamed to `...overwrite_user_edits_under_force`
assert overwrite-not-preservation. The plan-summary inversion test was deleted per
the SPEC's "pick whichever keeps the file smaller" allowance -- CHK-006 explicitly
permits deletion. `cargo test --workspace` exits 0.
</coverage>

## Retry counts

- T-001: 1 implementation round, 1 review round, 2 vet invocations.
  The second vet invocation addressed a minor CHK-002 path discrepancy
  (reviewer-business.toml vs reviewer-security.toml); no implementation retry
  was needed.

## Out-of-scope items absorbed

None. The carve-out removal was self-contained in `speccy-cli/src/init.rs`; no
consumers outside that file referenced `is_host_native_reviewer_file`. SPEC-0027
REQ-001, REQ-003, and REQ-004 remain in effect and were not touched.

</report>
