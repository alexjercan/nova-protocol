# CI/pre-tag guard: fail if docs/ contains anything but LESSONS.md at release (enforces the ephemeral-docs model)

- STATUS: OPEN
- PRIORITY: 30
- TAGS: v0.8.0, tooling, docs, release

## Goal

NARROWED under the ephemeral-docs model (20260718-175424): docs/ is a free
scratch space during a cycle and the release-time compile step folds it into
LESSONS.md and wipes it, so docs/ holds only LESSONS.md at every tag. This task
is the enforcement guard for that end-state: a cheap check that fails if docs/
contains anything other than LESSONS.md when releasing, so a forgotten scratch
file or a stray plan can't slip into a tag.

The earlier "distribute each junk file to its correct home (task NOTES / wiki)"
design is dropped - it is superseded by "compile into LESSONS.md, then wipe"
(owned by 20260718-175424).

## Steps

- Write a small checker (Rust bin or scripts/ Python) that lists docs/ entries
  not on the allowlist (just `LESSONS.md`) and exits non-zero.
- Wire it into the pre-tag/CI release step in
  `web/src/wiki/dev/development.md` / `keeping-docs-in-sync.md`, right after the
  compile-to-LESSONS step so the two stay consistent.
- Emit a clear message ("run the docs compile step; docs/ must be empty except
  LESSONS.md before tagging").

## Notes

- Depends on 20260718-175424 (the compile+wipe mechanism this guards).
- Keep it dumb: allowlist = LESSONS.md only. All judgment lives in the compile
  step, not here.

