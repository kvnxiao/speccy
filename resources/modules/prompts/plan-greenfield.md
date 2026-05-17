# Speccy: Plan (greenfield)

You are drafting a new SPEC for an existing speccy workspace.

## Your task

Author the next slice as `SPEC-{{next_spec_id}}`.

1. Propose a slug (lowercase kebab-case).
2. Decide placement:
   - If the spec belongs in an existing mission folder (a focus area
     that already has `.speccy/specs/[focus]/MISSION.md`), write to
     `.speccy/specs/[focus]/{{next_spec_id}}-<slug>/SPEC.md`.
   - Otherwise write flat to
     `.speccy/specs/{{next_spec_id}}-<slug>/SPEC.md`. Do not invent
     a new mission folder for a single spec; grouping is worthwhile
     only when 2+ related specs share enough context that loading
     them together at plan time is cheaper than rediscovering it.
3. Create the SPEC.md using the PRD-shaped template in
   `.speccy/ARCHITECTURE.md`. Each requirement is wrapped in a
   `<requirement id="REQ-NNN">` element block; each validation
   scenario lives in a nested `<scenario id="CHK-NNN">` element
   block under the requirement it proves. The scenario body is
   English Given/When/Then prose describing the behavior the
   requirement must satisfy. Speccy renders these scenarios;
   project tests and reviewers judge whether they're satisfied.

   ```markdown
   <requirement id="REQ-001">
   ### REQ-001: <one-line behavior>

   <prose describing the requirement>

   <scenario id="CHK-001">
   Given <preconditions>, when <action>, then <observable result>.
   </scenario>
   </requirement>
   ```

   Wrap each `## Changelog` table in a `<changelog>` element
   block. Per-spec `spec.toml` is no longer used (SPEC-0019
   migration) and HTML-comment markers (SPEC-0019) were
   superseded by raw element tags in SPEC-0020; the element tree
   is the machine contract.
4. Surface any material questions inline in `## Open questions`.

Do not write TASKS.md; the next phase will decompose it.
