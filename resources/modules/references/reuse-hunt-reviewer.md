## Reuse hunt (reviewer: adversarially verify-and-hunt)

The implementer is supposed to have surveyed the task area for
reusable or extendable code before writing. Your job is to verify that
survey adversarially and to hunt the reinvention it missed — reuse
drift is cheap to catch here and expensive once it lands.

- **Verify the survey is present.** Confirm the implementer's
  `<implementer>` journal block carries a `Reuse survey` field. A
  missing field is itself a finding: the survey is unenforced by any
  lint, so its absence surfaces only here.
- **Confirm the named symbols exist.** For each symbol the survey
  claims to reuse or extend, confirm that symbol actually exists in
  the tree and is the thing the diff calls. A survey that names a
  reused helper which does not exist, or which the code does not
  actually call, is not honest.
- **Hunt reinvention.** Independently of the survey's claims, look for
  new helpers, types, or constants in the diff that duplicate or
  near-duplicate something already in the tree — including a few
  directories away, where it is easy to miss. A byte-for-byte copied
  helper or constant, or a fresh implementation of similar code that
  should have extended an existing one, is reuse drift the survey
  should have caught. Flag it even when the survey looks complete.
