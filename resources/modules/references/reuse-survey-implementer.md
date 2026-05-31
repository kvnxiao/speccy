## Reuse survey (implementer: survey-and-build)

Before writing any code, survey the task-relevant area and decide,
for the code you are about to add, whether to reuse, extend, or write
fresh. Reuse is a design input here, not a post-hoc cleanup: you
classify what already exists *before* you commit to a shape, so you
build on it instead of laying down a parallel implementation that a
later review round has to unwind.

**Bounded to the task's area.** Map only the area the task touches:
its covered REQs, the suggested-files hint in the task body, and the
immediate module plus its neighbouring files. This is explicitly
**not** a whole-repo scan — reusable code far outside the task's area
is out of scope by design, and hunting for it is wasted budget.

**The three tiers.** Classify the relevant existing code you find,
and for each thing you decide to add, place it in one tier:

- **Reuse-as-is.** An existing symbol already does what you need —
  call it. Name the specific existing symbol (function, type,
  constant, helper) you are reusing.
- **Extend.** An existing symbol nearly does what you need and should
  grow to cover your case rather than be duplicated. Name the
  specific existing symbol you are extending.
- **Write-fresh.** Nothing existing fits, so you write something new.
  Name the search that came up empty (what you looked for and where),
  so the absence is auditable rather than assumed.

**Round semantics.**

- The **full area-map** is round-1 only. Re-run it on a retry round
  *only* when a reuse-related blocker was raised against the prior
  round; a retry that addresses a non-reuse blocker does not re-survey
  the area.
- The **per-symbol floor** is round-agnostic. For every new top-level
  symbol the implementation introduces — in any round — name the
  existing thing it reuses or extends, or the search that found
  nothing. A retry that adds a new top-level symbol still owes this
  per-symbol accounting even when the full area-map is not re-run.
