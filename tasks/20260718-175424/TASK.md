# Ephemeral docs/ model: release-time compiles docs/ into LESSONS.md entries then wipes it; retire docs/plans in favor of tatr tasks

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.8.0,docs,tooling,release,refactor

## Goal

Redefine the docs/ model on the changelog_entries pattern: docs/ becomes a
free scratch space during development (agents write whatever documentation they
feel like, no structure required), and at RELEASE time a step compiles
everything in docs/ into LESSONS.md entries and then removes the folder's
contents. LESSONS.md is the only durable output; docs/ is empty at every tag.
Plans stop living in docs/plans and become their own tatr tasks. This replaces
the "distribute junk to its correct home" approach (20260718-152225, which is
folded into this) with "fold into the lessons ledger, then wipe."

## Steps

- Define the model precisely and rewrite docs/README.md to describe it: docs/
  is transient scratch during a cycle; LESSONS.md is the compiled durable
  record; nothing in docs/ survives a release except LESSONS.md itself.
- Build the release-time compile step (Rust bin or scripts/ Python): read every
  file left in docs/ (root notes, design/, any dated investigations), distill
  each into one or more LESSONS.md ledger entries (same format /compound
  appends), append them, then clear docs/ back to just LESSONS.md. Mirror how
  the news-fragment/changelog_entries flow compiles-then-clears.
- Decide the fate of docs/design/*: same treatment (distilled into LESSONS or a
  wiki dev page, then removed) - the point is docs/ holds nothing durable.
- Retire docs/plans/: migrate existing release plans into tatr tasks (a plan is
  a task with the strand breakdown in its body, or a parent task linking the
  per-strand tasks), update any references, and stop writing new plans to
  docs/. Wire "plan lives in a tatr task" into the plan/flow workflow docs.
- Add the enforcement guard (a pre-tag/CI check that docs/ contains only
  LESSONS.md) - the piece 20260718-152225 was narrowed to; keep them consistent.
- Update the release checklist (web/src/wiki/dev/development.md +
  keeping-docs-in-sync.md) with the compile-and-wipe step.

## Notes

- Supersedes the design of 20260718-152225 (distribute-to-correct-home); that
  task is reduced to the CI guard under this model.
- The v0.7.0 pre-release cleanup (20260718-152329) is the interim manual pass;
  once this lands, that manual step is replaced by the automated compile+wipe.
- LESSONS.md format + append behavior: see docs/LESSONS.md and the /compound
  skill.

