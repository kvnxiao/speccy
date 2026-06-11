---
id: SPEC-0057
slug: xml-foreign-tag-balance
title: Unbalanced foreign-tag lint — `speccy verify` flags leaked orphan XML tags in parsed artifacts
status: in-progress
created: 2026-06-10
supersedes: []
---

# SPEC-0057: Unbalanced foreign-tag lint — `speccy verify` flags leaked orphan XML tags in parsed artifacts

## Summary

Speccy artifacts are authored by skills that write files through a raw
file-write tool. When a host model mis-serialises a write, its own
tool-call wrapper tags can bleed into the file body — a real incident
left two orphan close tags (`</content>` and `</invoke>`) at the end of
a `TASKS.md`, and a lone `</content>` at the end of an evidence file.

These survived silently. The shared XML scanner
(`speccy-core/src/parse/xml_scanner/mod.rs`) treats any tag-shaped line
whose name is outside the artifact's Speccy whitelist as ordinary
Markdown prose — the deliberate SPEC-0020 DEC-002 "foreign HTML flows
through verbatim" contract that lets task prose contain `<details>` and
`<br>` without breaking the parser. Because the scanner discards such
lines before the block-assembly step, an unmatched foreign close never
reaches the code that would flag it, parsing succeeds, and the
consistency check reports `ok`. No lint covers the gap.

This SPEC adds one lint code, `XML-001` (Error), that detects an
**unbalanced foreign tag** — a foreign close with no matching open, or a
foreign non-void open with no matching close — in any structurally
parsed artifact (SPEC.md, TASKS.md, REPORT.md, journal/*.md). The
detection lives in the lint engine, leaves the permissive scanner
contract untouched, and preserves the legality of balanced inline
foreign HTML. The leak class becomes loud at `speccy verify` time
instead of rotting in a committed file.

## Goals

<goals>
- A parsed artifact containing a foreign close-tag-shaped line with no
  matching preceding foreign open produces exactly one `XML-001` Error
  diagnostic naming the artifact path and the offending line.
- A parsed artifact containing a foreign non-void open-tag-shaped line
  with no matching following foreign close produces exactly one
  `XML-001` Error diagnostic.
- Balanced foreign tags (every open paired with a later same-name
  close) anywhere in an artifact produce no `XML-001` diagnostic, so the
  SPEC-0020 DEC-002 inline-HTML passthrough stays intact.
- `XML-001` is registered at Error severity, so a single occurrence
  makes `speccy verify` exit non-zero.
</goals>

## Non-goals

<non-goals>
- No change to the scanner's foreign-tag passthrough. The scanner keeps
  treating non-whitelisted tags as prose; detection is added in the
  lint engine only, not by turning foreign tags into parse errors.
- No strict cross-name nesting enforcement. Improper nesting of
  distinct foreign names (a close that interleaves with a different
  open) is out of scope; only per-name balance is checked.
- No coverage of unparsed prose files. `evidence/*.md` and other files
  with no structured parser are not linted by this SPEC, even though
  the original leak also touched one.
- No autofix. The lint reports the orphan tag; it does not rewrite the
  file.
</non-goals>

## User Stories

<user-stories>
- As a Speccy maintainer running CI, I want a leaked tool-call wrapper
  tag in any committed spec artifact to fail `speccy verify` loudly, so
  malformed files cannot rot undetected the way the original incident
  did.
- As a skill author whose write leaked a stray tag, I want the
  diagnostic to name the file and line of the orphan tag, so I can find
  and delete it without hand-scanning the artifact.
</user-stories>

## Assumptions

<assumptions>
- A "foreign tag" is a line matching the scanner's open- or close-tag
  shape regex whose element name is outside that artifact's Speccy
  whitelist. HTML5-ness is irrelevant except for the void-element
  exemption (REQ-002).
- Self-closing `<foo/>` syntax does not match the scanner's strict open
  regex, so it is neither an open nor a close for balance purposes — it
  never fires and never needs a partner.
- Journal files are reached on demand by deriving their paths from the
  parsed `TASKS.md` tasks and reading each with `fs-err`, the same way
  the `JNL-*` rules do; there is no journal field on `ParsedSpec`.
- A new `XML-*` lint family prefix is non-breaking because the stability
  registry is append-only.
</assumptions>

## Requirements

<requirement id="REQ-001">
### REQ-001: Unbalanced foreign tag detected as a single XML-001 diagnostic

The lint engine emits one `XML-001` diagnostic for any unbalanced
foreign (non-whitelisted) tag in a parsed artifact. "Unbalanced" means
either a foreign close-tag-shaped line with no matching preceding
foreign open, or a foreign non-void open-tag-shaped line with no
matching following foreign close. Both failure shapes use the same code
(`XML-001`) and the same message template; there is no separate code or
message for the open case versus the close case.

<done-when>
- An artifact ending in a bare `` `</invoke>` `` line with no `` `<invoke>` `` open emits exactly one `XML-001` diagnostic.
- An artifact containing a foreign non-void open with no later matching close emits exactly one `XML-001` diagnostic.
- An artifact whose foreign tags are all balanced emits zero `XML-001` diagnostics.
- The open-orphan and close-orphan cases both carry code `XML-001` with the same message template, differing only in the substituted tag name and line.
</done-when>

<behavior>
- Given a `TASKS.md` whose body ends with a bare `` `</content>` `` line followed by a bare `` `</invoke>` `` line and no matching opens, when `speccy verify` lints it, then two `XML-001` diagnostics fire, one per orphan close line.
- Given a `SPEC.md` whose requirement body contains a balanced `` `<details>` ``…`` `</details>` `` pair, when the artifact is linted, then no `XML-001` diagnostic fires.
</behavior>

<scenario id="CHK-001">
Given a fixture `TASKS.md` that parses successfully but ends with a
bare foreign close tag with no matching open,
when the lint engine runs over the workspace,
then exactly one `XML-001` Error diagnostic is produced whose file is
that `TASKS.md` and whose line is the orphan close tag's line.
</scenario>

<scenario id="CHK-002">
Given a fixture artifact containing a foreign non-void open tag on its
own line with no matching close anywhere after it,
when the lint engine runs,
then exactly one `XML-001` Error diagnostic is produced naming that
open tag's line.
</scenario>

<scenario id="CHK-003">
Given a fixture artifact whose body contains a balanced foreign pair
(a same-name open followed later by its close),
when the lint engine runs,
then no `XML-001` diagnostic is produced for that artifact.
</scenario>
</requirement>

<requirement id="REQ-002">
### REQ-002: HTML5 void elements never count as unbalanced opens

HTML5 void elements (`area`, `base`, `br`, `col`, `embed`, `hr`, `img`,
`input`, `link`, `meta`, `param`, `source`, `track`, `wbr`) have no
close tag by definition. A foreign open whose name is a void element is
never reported as a dangling open.

<done-when>
- A lone `` `<br>` `` line (a void element with no close) produces no `XML-001` diagnostic.
- A lone foreign non-void open with no close still produces an `XML-001` diagnostic, confirming the exemption is scoped to the void set.
</done-when>

<behavior>
- Given an artifact body containing a single `` `<br>` `` line, when the lint engine runs, then no `XML-001` diagnostic is produced.
- Given an artifact body containing a single foreign open whose name is not in the void set, when the lint engine runs, then one `XML-001` diagnostic is produced.
</behavior>

<scenario id="CHK-004">
Given one fixture artifact containing a lone void-element open
(`` `<br>` ``) and a second fixture containing a lone non-void foreign
open,
when the lint engine runs over both,
then the void-element fixture produces no `XML-001` diagnostic and the
non-void fixture produces exactly one.
</scenario>
</requirement>

<requirement id="REQ-003">
### REQ-003: Foreign tags inside fenced code blocks are exempt

A foreign tag-shaped line that lies inside a fenced code block is not
counted for balance and never fires `XML-001`. Detection reuses the
scanner's existing fence-awareness
(`collect_code_fence_byte_ranges`) so illustrative tags in code
examples — including this SPEC's own examples — are inert.

<done-when>
- A foreign close tag that appears only inside a fenced code block produces no `XML-001` diagnostic.
- A foreign open inside a fence and its matching close outside the fence do not pair: the outside close is still reported as an orphan, confirming fenced lines are excluded from balance rather than merely skipped symmetrically.
</done-when>

<behavior>
- Given an artifact whose only foreign close tag sits inside a triple-backtick fenced block, when the lint engine runs, then no `XML-001` diagnostic is produced.
- Given an artifact with a foreign close outside any fence, when the lint engine runs, then `XML-001` fires regardless of any fenced occurrences of the same name.
</behavior>

<scenario id="CHK-005">
Given a fixture artifact whose only occurrence of an otherwise-orphan
foreign close tag is inside a fenced code block,
when the lint engine runs,
then no `XML-001` diagnostic is produced for that artifact.
</scenario>
</requirement>

<requirement id="REQ-004">
### REQ-004: Detection covers every structurally parsed artifact

`XML-001` detection runs over `SPEC.md`, `TASKS.md`, and `REPORT.md`
using the raw source already retained on their parsed documents, and
over existing `journal/*.md` files read on demand by deriving their
paths from the parsed tasks (the `JNL-*` access pattern). Evidence and
other unparsed prose files are excluded (see Non-goals).

<done-when>
- A dangling foreign tag in a `SPEC.md` produces an `XML-001` diagnostic whose file is that `SPEC.md`.
- A dangling foreign tag in a `REPORT.md` produces an `XML-001` diagnostic whose file is that `REPORT.md`.
- A dangling foreign tag in an existing `journal/T-NNN.md` produces an `XML-001` diagnostic whose file is that journal file.
</done-when>

<behavior>
- Given a workspace where each of `SPEC.md`, `TASKS.md`, and `REPORT.md` contains one dangling foreign tag, when the lint engine runs, then one `XML-001` diagnostic fires per artifact, each naming its own file.
- Given a `journal/T-001.md` containing a dangling foreign tag, when the lint engine runs, then one `XML-001` diagnostic fires naming that journal file.
</behavior>

<scenario id="CHK-006">
Given a fixture spec whose `SPEC.md`, `TASKS.md`, and `REPORT.md` each
contain exactly one dangling foreign tag,
when the lint engine runs over the workspace,
then exactly three `XML-001` diagnostics are produced, one per
artifact, each with the correct file path.
</scenario>

<scenario id="CHK-007">
Given a fixture spec with a task whose `journal/T-NNN.md` exists and
contains a dangling foreign tag,
when the lint engine runs,
then exactly one `XML-001` diagnostic is produced whose file is that
journal file.
</scenario>
</requirement>

<requirement id="REQ-005">
### REQ-005: XML-001 is an Error in the append-only registry and drives a non-zero verify exit

`XML-001` is appended to the stability registry
(`speccy-core/src/lint/registry.rs`) at Error severity and is pinned by
the registry snapshot test. Because it is an Error, a single occurrence
makes `speccy verify` exit non-zero. The diagnostic message names the
artifact path and the orphan tag's 1-indexed source line.

<done-when>
- The registry snapshot includes `XML-001` at Error severity.
- `speccy verify` over a workspace containing one dangling foreign tag exits non-zero.
- The rendered diagnostic message contains the artifact path and the 1-indexed line of the orphan tag.
</done-when>

<behavior>
- Given a workspace whose only lint finding is one dangling foreign tag, when `speccy verify` runs, then the process exits non-zero.
- Given that diagnostic, when it is rendered, then its text names the offending file and the orphan tag's line number.
</behavior>

<scenario id="CHK-008">
Given a fixture workspace whose sole lint finding is one dangling
foreign tag in a parsed artifact,
when `speccy verify` runs against it,
then the process exits non-zero and the rendered output names the
artifact path and the orphan tag's 1-indexed line.
</scenario>
</requirement>

<requirement id="REQ-006">
### REQ-006: Raw source retention on parsed docs is locked by a regression test

`SpecDoc`, `TasksDoc`, and `ReportDoc` each retain their full raw
source bytes (they carry `pub raw: String` today). A regression test
pins this property so a future refactor cannot silently drop the field
the `XML-001` lint depends on for its input.

<done-when>
- For a fixture source string, the parsed `SpecDoc.raw` equals the source bytes exactly.
- For a fixture source string, the parsed `TasksDoc.raw` equals the source bytes exactly.
- For a fixture source string, the parsed `ReportDoc.raw` equals the source bytes exactly.
</done-when>

<behavior>
- Given a valid SPEC.md / TASKS.md / REPORT.md source string, when it is parsed, then the resulting document's `raw` field is byte-identical to the input source.
</behavior>

<scenario id="CHK-009">
Given valid fixture sources for `SPEC.md`, `TASKS.md`, and `REPORT.md`,
when each is parsed,
then each parsed document's `raw` field is byte-identical to its source
string.
</scenario>
</requirement>

## Decisions

<decision id="DEC-001">
Detection lives in the lint engine, not the scanner. The scanner stays
mechanical and keeps the SPEC-0020 DEC-002 foreign-HTML passthrough, and
diagnostics belong in the lint layer where `speccy verify`'s exit-code
policy already lives. A scanner-level hard parse error was rejected: it
is the wrong altitude (it would mutate the permissive passthrough
contract) and is coarser than a lint — a parse failure rejects the whole
artifact, whereas a lint points at the specific orphan tag.
</decision>

<decision id="DEC-002">
Balance is computed name-scoped with a per-name stack, fence-aware, and
does not enforce cross-name nesting. Each foreign non-void open pushes
its location onto its name's stack; a foreign close pops its name's
stack — an empty stack means that close is a dangling orphan and its
location is flagged; any locations left on a stack at end of input are
dangling-open orphans and are flagged. This yields the precise
offending-tag location REQ-005 requires and handles repeated same-name
tags, while deliberately not enforcing cross-name nesting order (an
interleaving like a close of one name across an open of another does not
fire), which keeps false positives off loosely-nested legitimate inline
HTML. Flat per-name counts were rejected: they cannot point at the
offending tag and they mask interleaved mismatches.
</decision>

<decision id="DEC-003">
Journal coverage is defense-in-depth rather than the primary target.
Journal files are written exclusively through `speccy journal append`,
which validates XML before writing, so they are the lowest-risk
artifact; `SPEC.md` and `TASKS.md`, written by skills through a raw
file-write tool during plan and decompose, are where the leak actually
landed. Journals are covered anyway for robustness, read on demand via
the `JNL-*` path-derivation pattern instead of adding a journal field to
`ParsedSpec`.
</decision>

<decision id="DEC-004">
The lint consumes a new mechanical scanner helper rather than
re-implementing the tag-shape grammar. `scan_foreign_tags` is added to
`speccy-core/src/parse/xml_scanner` as the inverse of the existing
`scan_tags`: it walks lines with the same fence-awareness
(`collect_code_fence_byte_ranges` / `range_inside_any_fence`) and the same
open/close tag-shape regexes, but yields the **non-whitelisted** (foreign)
tag occurrences — each with `is_close` and a 1-indexed source line —
instead of the whitelisted structural tags `scan_tags` keeps. This keeps
DEC-001 intact: the scanner gains no balance logic and emits no
diagnostics; it only exposes the foreign-tag view, and the SPEC-0020
DEC-002 passthrough used by the parsers is untouched. All balance
computation and `XML-001` emission live in the lint engine. Forking the
open/close regexes and fence-walk into the lint was rejected: it would
duplicate the tag grammar away from its single source in the scanner, so a
future grammar tweak could silently desync the lint from the parsers.
</decision>

## Notes

A strict-whitelist framing (disallow every non-whitelisted tag) was
considered and rejected: it would override SPEC-0020 DEC-002 and break
legitimate inline HTML such as `<details>` and `<br>` already used in
existing specs — far broader than the leak it would catch.

Merge-ordering interaction: a separate cleanup removing the leaked
`</content>` / `</invoke>` tags from the existing `SPEC-0056` `TASKS.md`
(and a `SPEC-0055` evidence file) is in flight on another branch. That
cleanup must land before or together with this lint; otherwise, once
`XML-001` ships, `speccy verify` would fire on the un-cleaned `TASKS.md`
still present on the trunk. The evidence-file leak is out of scope here
(unparsed prose), so it is cleanup-only.

## Changelog

<changelog>
| Date | Author | Summary |
| --- | --- | --- |
| 2026-06-10 | kevin | Initial SPEC: XML-001 unbalanced-foreign-tag lint over parsed artifacts; lint-layer detection, void exemption, fence exemption, raw-retention lock. |
| 2026-06-10 | claude-opus-4-8[1m] | Decompose: added DEC-004 pinning the `scan_foreign_tags` reuse seam (lint consumes a mechanical scanner helper; tag-shape regexes stay single-sourced in the scanner). |
</changelog>
