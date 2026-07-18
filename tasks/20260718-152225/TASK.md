# CI/pre-tag guard: fail if docs/ contains anything but LESSONS.md at release (enforces the ephemeral-docs model)

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.8.0, tooling, docs, release

## Story

As the release process, I want tagging to fail loudly when docs/ still holds
scratch files, so that the ephemeral-docs model (20260718-175424) is enforced
by a machine instead of remembered by whoever cuts the release.

NARROWED under the ephemeral-docs model (20260718-175424): docs/ is a free
scratch space during a cycle and the release-time compile step folds it into
LESSONS.md and wipes it, so docs/ holds only LESSONS.md at every tag. This
task is the enforcement guard for that end-state: a cheap check that fails if
docs/ contains anything other than LESSONS.md when releasing, so a forgotten
scratch file or a stray plan can't slip into a tag.

The earlier "distribute each junk file to its correct home (task NOTES /
wiki)" design is dropped - it is superseded by "compile into LESSONS.md, then
wipe" (owned by 20260718-175424).

## Steps

- [ ] Write a small checker (scripts/ Python, matching the tooling direction)
      that lists docs/ entries not on the allowlist (just `LESSONS.md`) and
      exits non-zero, printing each offending path.
- [ ] Emit a clear remediation message ("run the docs compile step; docs/ must
      be empty except LESSONS.md before tagging").
- [ ] Wire it into the pre-tag/CI release step and document it in
      `web/src/wiki/dev/development.md` / `keeping-docs-in-sync.md`, right
      after the compile-to-LESSONS step so the two stay consistent. Decide
      whether it also runs on every CI push (informational) or only gates the
      tag path.
- [ ] Test both directions: a dirty docs/ fails with the right listing; a
      clean docs/ passes.

## Definition of Done

- The guard fails a release when docs/ holds anything but LESSONS.md, names
  the offending files, and tells the operator exactly what to run.
- The release checklist shows compile step -> guard, in that order.
- A clean docs/ passes with zero output beyond success.

## Notes

- Depends on 20260718-175424 (the compile+wipe mechanism this guards). If
  175424 slips, this guard must not land first or every release goes red.
- Keep it dumb: allowlist = LESSONS.md only. All judgment lives in the compile
  step, not here.
