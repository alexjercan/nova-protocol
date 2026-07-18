# v0.7.0 pre-release: clear docs/ root junk into its correct homes (task folders / wiki) per docs/README rules before tagging

- STATUS: OPEN
- PRIORITY: 55
- TAGS: v0.7.0,docs,release

## Goal

Before tagging v0.7.0, docs/ must be clean. docs/README.md forbids per-task
record files in docs/ root (only README, LESSONS, design/, plans/ belong), but
the current cycle left dated investigation notes at the root. Manually
distribute them to their correct homes now; the automation to keep it clean is
the v0.8.0 task 20260718-152225.

## Steps

- Triage each stray docs/ root file and move it to its correct home:
  - `2026-07-17-frametime-baseline-harness.md` -> the perf-baseline task's
    NOTES (tasks/20260716-123551) or the perf wiki dev page.
  - `2026-07-18-render-scale-lever.md` -> the owning task's NOTES + fold the
    user-facing lever into the settings/graphics wiki.
  - `wasm-asset-meta-always-investigation.md` -> the meta-gen task NOTES / a
    wiki dev page (referenced by nova_meta_gen; keep it findable).
  - `craft-ships-into-base.md` -> the owning task NOTES or the sections/modding
    wiki, whichever is durable.
- Decide `docs/design/*.md`: keep as durable design docs, or graduate into
  `web/src/wiki/dev/`. Apply the decision (don't leave it ambiguous for v0.7.0).
- Leave README.md, LESSONS.md, plans/ in place. Confirm docs/ root matches the
  allowlist and `ls docs/` is clean.

## Notes + Maybe things to add to the review in the closing task

- This is the one-time cleanup that sets the end-state the v0.8.0 release-flow
  checker (20260718-152225) will enforce automatically.
- We should also update AGENTS.md to match the new codebase and add lessons in
  there. Also updating LESSONS.md; and making sure everything we added in
  version is properly documented and we get all the possible lessons from these
  sessions. Maybe going over the commits and reviews and task and spike files
  would be a good idea and getting a summary of what happened; Making kind of a
  review of the entire sprint (in this folder).
- Some things I find important: harness tests I think we should add more focus
  on those, they are really useful; for example when we find a bug we should
  create a test with a harness that reproduces the bug only when we know what
  happens we attempt to fix it.
- Obviously there are more learnings we should take; We should go through all
  the task files and see what we can see.
- We will also need to find a better way to run tests and cargo things because
  on worktrees we need to build from scratch and it takes ages;
