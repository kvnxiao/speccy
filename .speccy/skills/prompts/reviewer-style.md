# Speccy: Review `{{task_id}}` for `{{spec_id}}` ({{persona}})

You are the **{{persona}}** reviewer for one task in one spec. Produce
exactly one inline review note appended to the task in TASKS.md. Do
not modify any other file.

## Project conventions

{{agents}}

## Persona

{{persona_content}}

## SPEC (full)

{{spec_md}}

## Task entry (verbatim from TASKS.md)

The block below is the literal `<task id="{{task_id}}">...</task>`
element copied from TASKS.md. Note two distinct validation contracts:

- The required nested `<task-scenarios>` block is the **slice-level**
  contract — the executable conditions this one slice of work must
  satisfy. Use it to judge whether the diff lives up to what the
  implementer signed up for.
- The SPEC requirements named in the `covers="..."` attribute carry
  the **user-facing-level** `<scenario>` elements. Use them to judge
  whether the slice meaningfully advances the user-visible behaviour
  the requirement names. Slice-level scenarios may be narrower than
  the user-facing ones; missing slice coverage is a different finding
  from missing user-facing coverage.

{{task_entry}}

## Diff under review

{{diff}}

## Your task

1. Read the SPEC requirements the task covers (the `covers="..."`
   attribute on the `<task>` element above lists them as
   space-separated `REQ-NNN` ids) and the `<task-scenarios>` body on
   this task. Distinguish slice-level and user-facing-level
   validation when reporting findings.
2. Apply the **{{persona}}** persona's review focus to the diff.
3. Append one bullet to the task subtree of the form:

       - Review ({{persona}}, pass | blocking): <one-line summary>.
         <optional file:line refs and details>.

Use `pass` if the diff is acceptable from this persona's perspective;
use `blocking` if a change is required before merge. The orchestrating
skill, not you, flips the task's `state="..."` attribute.
