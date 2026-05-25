---
spec: SPEC-0038
generated_at: 2026-05-22T02:47:32Z
---

## Invocation 1 — 2026-05-22T02:47:32Z

<drift-review verdict="pass" round="1" date="2026-05-22T02:55:00Z" model="claude-opus-4-7[1m]/high">
Holistic drift review for SPEC-0038 against the working tree (`git diff main` includes the uncommitted SPEC-0038 changes).

**REQ-001 (two location classes ejected by `speccy init`):** PASS. Both host packs ship `references/` subfolders under every owning skill and a `speccy-references/` directory at host root. Source under `resources/modules/references/` exists; `resources/modules/examples/` is gone. Byte-identical source==Claude==Codex verified for all seven reference files (`diff` exit 0 across every triple).

**REQ-002 (one canonical reference per artifact, classified by path):** PASS. All seven artifact→path rows match the SPEC's table verbatim — `speccy-plan/references/spec.md`, `speccy-tasks/references/tasks.md`, `speccy-ship/references/report.md`, `speccy-work/references/journal-implementer.md`, `speccy-review/references/journal-review.md` (skill-local); `speccy-references/evidence.md` and `speccy-references/journal-blockers.md` (host-shared). `.claude/speccy-references/` contains exactly two files; no spurious extra references.

**REQ-003 (post-SPEC-0034 canonical shape):** PASS. `journal-implementer.md` carries the six fields in canonical order at lines 29/45/49/58/68/76 (Completed/Undone/Hygiene checks/Evidence/Discovered issues/Procedural compliance) with zero `Commands run:` / `Exit codes:` bullet labels. `evidence.md` opens with `# Evidence for SPEC-0042 T-001` matching the canonical regex; zero `<evidence task=` substrings present. Zero `TBD` / `TODO` / `<...>` placeholders across all seven references.

**REQ-004 (one-line pointer per consuming body; no inline ≥8-line example blocks):** PASS. Grep counts confirm exactly one pointer per body for each reference: speccy-plan→spec.md (1 each host), speccy-tasks→tasks.md (1 each host), speccy-ship→report.md (1 each host), speccy-work→journal-implementer.md+evidence.md (1 each), speccy-review→journal-review.md+journal-blockers.md (1 each host), speccy-amend→journal-blockers.md (1 each host). Rendered Codex bodies carry the `.agents/` form; zero `.claude/` leakage into `.codex/agents/reviewer-tests.toml`. Phase body lengths trimmed (speccy-tasks 74 lines, speccy-work 131 lines) — the inline TASKS.md fragment and the `<implementer>` example block are gone.

**REQ-005 (orphan files removed; salvageable prose moved):** PASS. `resources/modules/personas/implementer.md`, `resources/modules/personas/planner.md`, and `resources/modules/examples/evidence.md` plus the now-empty `examples/` directory all absent. `PERSONA_FILES` in `skill_body_discovery.rs` no longer lists them. Hygiene suite green confirms no test depended on those files.

**REQ-006 (reviewer-tests gains pointer; other personas audited):** PASS. `.claude/agents/reviewer-tests.md` carries `.claude/speccy-references/evidence.md` (1 hit); `.codex/agents/reviewer-tests.toml` carries `.agents/speccy-references/evidence.md` (1 hit) with zero `.claude/` form leakage. DEC-001 in SPEC.md documents the audit outcome — the other five reviewer personas gain no pointers, matching the SPEC.

**REQ-007 (`chk022_no_orphan_references` test):** PASS. Test exists at `speccy-cli/tests/skill_body_discovery.rs:601` (chk022, the next free `chk0NN` slot). It globs `.claude/skills/*/references/*.md` ∪ `.claude/speccy-references/*.md` (and Codex siblings), reads consuming bodies under `.claude/skills/*/`, `.claude/agents/*.md`, `.agents/skills/*/`, `.codex/agents/*.toml`, and asserts: orphan-pointer presence (Claude + Codex separately), cross-host byte parity, and source-to-host byte parity. `cargo test --workspace` exits 0; failure-mode message strings match the SPEC's three fault-injection scenarios (CHK-023..CHK-026).

**Cross-cutting checks:**

- All four standard-hygiene gates exit 0: `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo +nightly fmt --all --check`, `cargo deny check`.
- ARCHITECTURE.md (T-007) carries the seven-row mapping at lines 1777-1783 and links to SPEC-0038 REQ-002 as the source of truth at line 1792. No `ARCHITECTURE.md` mentions inside `resources/modules/` or `resources/agents/` (T-002/T-007 portability guard observed).
- The deletion of `.speccy/examples/evidence.md` is a natural consequence of T-003 retiring `render_speccy_examples_pack`; the file was that retired render pass's ejected output, not in-scope SPEC state. Not drift.
- TASKS.md frontmatter `spec_hash_at_generation: 94397441e6a46fda409777b7cc4b77a802dffe92afacb6d34cfd6c236a865675` — both no SPC-005 lint indicator from `speccy next` and tasks all `state="completed"`, consistent with a clean ship boundary.

No requirement satisfied by no task; no behavior introduced that the SPEC never authorized; per-task code adds up to the SPEC as a unit. Clean for ship.

Note on subagent dispatch: the `Task` tool / `subagent_type` primitive the skill prescribes for the `speccy-holistic-reviewer` fan-out is not exposed in this session's tool surface. This review was performed inline by the skill's orchestrator session reading SPEC.md, TASKS.md, the working-tree diff, and the rendered host-pack output directly. Same scrutiny, single context.
</drift-review>

<simplifier-scan verdict="clean">
No simplification candidates that would tighten the SPEC-0038 diff without weakening invariants or violating project conventions.

Surface scanned: the SPEC-0038 working-tree diff against `main` excluding unrelated `.claude/skills/speccy-holistic-review`, `speccy-orchestrate`, and `.claude/agents/speccy-holistic-*` paths (those are out-of-scope additions).

The diff is dominated by content surface — seven canonical reference markdown files, six host-pack consuming bodies trimmed of inline shape blocks, ARCHITECTURE.md mapping table, orphan file deletions — none of which carry simplification headroom. The Rust additions are narrow and idiomatic:

- `HostChoice::template_context_raw` adds one new string field (`speccy_references_path`); shape matches the four existing fields and the doc comment names every key.
- `chk022_no_orphan_references` and its helpers (`enumerate_reference_files`, `collect_consuming_bodies`, `walk_collect`, `shallow_collect`, `first_diff`) follow project conventions: `fs_err` for IO, `camino`/`Utf8PathBuf` not in scope here because the test operates on `std::path::PathBuf` (mirrors the sibling helper in `tests/init.rs`), descriptive `panic_with_message` strings, `.get(i)` over `[i]` in `first_diff`, no `unwrap()`, no `unsafe`, no `#[allow(...)]` suppressions.
- The `bodies.retain` filter at `skill_body_discovery.rs:533-536` keeps both `references/` and `references\\` substring checks deliberately for Windows cross-platform path handling — collapsing to one form would regress correctness.

Nothing to apply.
</simplifier-scan>
