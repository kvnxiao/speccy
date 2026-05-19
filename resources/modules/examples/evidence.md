# Evidence: T-NNN worked example

This file demonstrates the evidence-shape contract referenced by the
implementer and reviewer-tests prompts. Each task captures its own
red-green paper trail under
`.speccy/specs/<SPEC-folder>/evidence/<TASK>.md`; the shape below is
what reviewers grep when they audit the proof.

<evidence task="T-042" spec="SPEC-0099">

## Session 2026-05-18-T042-rev1 (attempt 1)

Command: `cargo test -p speccy-core parse::red_block_exit_code`

<red exit="101">
running 1 test
test parse::red_block_exit_code ... FAILED

failures:

---- parse::red_block_exit_code stdout ----
thread 'parse::red_block_exit_code' panicked at speccy-core/src/parse.rs:84:9:
assertion `left == right` failed
  left: None
 right: Some(101)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    parse::red_block_exit_code

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 412 filtered out
</red>

<green exit="0">
running 1 test
test parse::red_block_exit_code ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 412 filtered out; finished in 0.04s
</green>

## Session 2026-05-18-T042-rev2 (attempt 2, no test delta)

Tightened the doc comment on `RedBlock::exit_code` to name the parse
fallback after review feedback; no production code or test changed,
so this attempt carries no red/green pair.

</evidence>
