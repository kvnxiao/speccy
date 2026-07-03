# Open Items

Status: 0 backlog items · 16 open questions · 5 watch items
Date: 2026-07-04 (ceremony-reduction pass)

The live surface for undecided work. `DESIGN.md` and `TERMINOLOGY.md` are
authoritative for current behavior; resolved decision history is archived in
`DECISION-LOG.md`.

## Backlog

Empty. The decision index — the last backlog item — is deferred to Later
Capabilities ("Carry-Forward Decisions" and "Later Capabilities" in
`DESIGN.md`); the cheap `carry_forward` flag it needs is recorded from M6, so
the projection can be built without a data migration when multi-spec use
proves it necessary.

## Open questions

Deferred until MVP usage provides evidence. Numbering is historical (Q1–Q25);
retired numbers were resolved and their decisions live in the design docs,
with rationale in `DECISION-LOG.md`.

- **Q2 Repo artifact export:** which artifacts should be easiest to export:
  spec, acceptance ledger, review packet, lessons learned, or all of them?
- **Q3 Artifact shape:** the smallest useful spec draft and acceptance ledger
  shape; no public format compatibility promise until MVP usage proves it.
- **Q4 No-server sharing:** are review packets, compact snapshots, rerun
  commands, and optional redacted run bundles enough for team use before
  considering any shared run store?
- **Q5 Spec interop:** which external spec formats should be first-class
  import targets: OpenSpec, Spec Kit, Kiro, GSD Core, Spec Kitty, or a
  generic markdown mapper?
- **Q7 Vacuity threshold:** what minimum anti-vacuity evidence is required
  before the verifier can mark a high-priority requirement as `passed`?
  (M5 ships only the adversarial-review prose.)
- **Q8 Scenario evidence:** how much should Speccy help convert
  `given/when/then` prose into evidence requests versus delegating that to
  harness agents?
- **Q9 Custom harness integration:** are `speccy ctl ... --json` calls enough
  for custom harnesses, or should `speccy rpc`/`speccy mcp` be supported
  earlier?
- **Q10 Gate editing:** how much editing should happen inside `speccy` versus
  opening `$EDITOR`?
- **Q11 Review packet format:** markdown only, JSON plus markdown, or an HTML
  report? (MVP: markdown only.)
- **Q12 Lessons learned:** how can the system accumulate project learning
  without leaking operational state or affecting product-code/build/runtime
  footprint?
- **Q18 Security model:** how should secret redaction and deny-read rules
  work across harnesses with different sandbox systems? (MVP ships an
  env-scrubbing stub.)
- **Q19 Production validation:** how should the tool prove behavior that only
  exists in deployed environments?
- **Q21 Long-term storage:** how long should transcripts/evidence be
  retained?
- **Q22 Team mode:** when multiple humans review gates, what is the approval
  policy?
- **Q23 Distribution:** channels for the `speccy` binary (language, engine,
  and license are decided: Rust, `minijinja`, MIT).
- **Q24 Name:** is `speccy` the right name, or should the tool use a more
  explicit name around specs/evidence?

## Dogfood watch list

Behaviors to measure during M8 dogfooding. Each names its candidate change if
the friction proves real; the rejection rationale behind each candidate is in
`DECISION-LOG.md`.

- **Reviewer roster cost** — measure what the four-persona roster costs on
  ordinary `standard` specs, now that delta-scoped re-review is deferred and
  every round re-reads the full diff. Candidates: a smaller `standard` default
  with the full roster starting at `high` (a config-default change); building
  the deferred delta-scoped re-review if full-diff re-review cost proves high.
- **Accepted-risk rubber-stamping** — the ship confirmation echoes accepted
  risks before opening the PR; watch whether they get waved through anyway.
  First decision to revisit if so.
- **Approval binding** — `go` and `approve only` are bound only by the approval
  echo; the controller-side `draft_version`/`stale_draft` enforcement was
  removed as ceremony. Watch for approvals binding to the wrong spec or a stale
  card in long chats. Candidates: reinstating a `draft_version` refusal, or
  ref-bearing replies (both previously rejected as ceremony).
- **First-install ceremony** — watch first-run reactions to the preview and
  the multi-file footprint. Candidates (previously rejected): single-target
  default in dual-harness repos, a `--user` trial install.
- **Manual accept forgetting** — `/speccy-ship` prints the `speccy accept`
  command, the Awaiting-merge card carries it, PR metadata includes
  `accept_with`, and `accept` reuses the recorded `change_ref`; watch whether
  accepts still get forgotten. Candidate: a local ancestry-check prompt on
  status cards (silent on squash merges, hence not MVP).
