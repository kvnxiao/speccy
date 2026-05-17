# Speccy: Review `{{task_id}}` for `{{spec_id}}` ({{persona}})

You are the **{{persona}}** reviewer for one task in one spec. Produce
exactly one inline review note appended to the task in TASKS.md. Do
not modify any other file.

## Persona

{{persona_content}}

## SPEC (pointer)

Before reviewing, read SPEC.md at `{{spec_md_path}}`. The CLI no
longer inlines the SPEC body into this prompt; load it via your Read
primitive when you need to inspect the requirement and decision
context.

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

Speccy does not inline the branch diff. Fetch it yourself before
reviewing:

1. Resolve the merge-base against the host's main branch. A
   defensive form that works whether the upstream tracks `origin/main`
   or `origin/master`:

       base=$(git merge-base HEAD origin/main 2>/dev/null \
           || git merge-base HEAD origin/master 2>/dev/null \
           || git merge-base HEAD main 2>/dev/null \
           || git merge-base HEAD master)

2. Extract the `Suggested files:` list from the `<task>` element
   above and pass it as pathspecs:

       git diff "$base"...HEAD -- <suggested-files>

   When the task entry does not name suggested files, run
   `git diff "$base"...HEAD` over the full slice. Read just the
   files the diff touches; do not re-read the whole repository.

## Your task

1. Read the SPEC requirements the task covers (the `covers="..."`
   attribute on the `<task>` element above lists them as
   space-separated `REQ-NNN` ids) and the `<task-scenarios>` body on
   this task. Distinguish slice-level and user-facing-level
   validation when reporting findings.
2. Apply the **{{persona}}** persona's review focus to the diff you
   fetched in the section above.
3. Append one bullet to the task subtree of the form:

       - Review ({{persona}}, pass | blocking): <one-line summary>.
         <optional file:line refs and details>.

Use `pass` if the diff is acceptable from this persona's perspective;
use `blocking` if a change is required before merge. The orchestrating
skill, not you, flips the task's `state="..."` attribute.
