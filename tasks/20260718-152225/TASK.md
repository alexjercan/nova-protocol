# Release-flow docs distribution: script + checklist step that clears docs/ root junk into task folders / wiki so docs/ is clean at release

- STATUS: OPEN
- PRIORITY: 52
- TAGS: v0.8.0,tooling,docs,release

## Goal

docs/README.md is explicit: "Do not create per-task record files under docs/.
The only record kept here is the LESSONS.md ledger." Yet dated investigation
notes accumulate in the docs/ root during a cycle (e.g. at time of writing:
`2026-07-17-frametime-baseline-harness.md`, `2026-07-18-render-scale-lever.md`,
`wasm-asset-meta-always-investigation.md`, `craft-ships-into-base.md`). The
release flow should take that junk and distribute it to its correct home so
docs/ is clean at every tag. This task builds the automation; the v0.7.0
one-time cleanup is 20260718-152329.

## Steps

- Define the policy precisely: what may live in docs/ root (README, LESSONS,
  design/, plans/) vs what must move. A dated/investigation note belongs in the
  owning `tasks/<id>/NOTES.md` (or a wiki dev page if it is durable reference).
- Write a checker script (Rust bin or Python under scripts/) that lists docs/
  root entries not on the allowlist and fails, so CI/pre-tag catches stray
  files; make it suggest a destination where it can infer one.
- Decide distribution: fully automated move is risky (destination needs
  judgment), so the tool should flag + propose and the release step confirms.
  Wire it into the release checklist in
  `web/src/wiki/dev/development.md` / `keeping-docs-in-sync.md`.
- Also cover the `docs/design/*.md` question: are these durable design docs
  (keep) or should they graduate into the wiki? Document the rule.

## Notes

- Depends conceptually on the v0.7.0 manual cleanup (20260718-152329) which
  establishes the correct end-state the automation should maintain.
- No release automation exists today; the manual process is
  `web/src/wiki/dev/keeping-docs-in-sync.md` + the development.md checklist.

