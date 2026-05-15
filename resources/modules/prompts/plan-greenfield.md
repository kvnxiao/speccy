# Speccy: Plan (greenfield)

You are drafting a new SPEC for an existing speccy workspace. The
project-wide product north star is carried in `AGENTS.md` below —
read it for what we're building, who for, the v1 outcome, the
quality bar, and known unknowns.

## Project conventions and product north star

{{agents}}

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
   `.speccy/ARCHITECTURE.md`.
4. Create `spec.toml` alongside, mapping each requirement to at
   least one check. Each `[[checks]]` row is exactly `id` and
   `scenario` — an English Given/When/Then describing the behavior
   the requirement must satisfy. Speccy renders these scenarios;
   project tests and reviewers judge whether they're satisfied.

   ```toml
   [[checks]]
   id = "CHK-001"
   scenario = """
   Given <preconditions>, when <action>, then <observable result>.
   """
   ```

   Do not author `kind`, `command`, `prompt`, or `proves` fields;
   they were removed in SPEC-0018. The CLI does not run project
   tests — that is project CI's job.
5. Surface any material questions inline in `## Open questions`.

Do not write TASKS.md; the next phase will decompose it.
