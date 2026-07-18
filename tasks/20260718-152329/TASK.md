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

## Notes

- This is the one-time cleanup that sets the end-state the v0.8.0 release-flow
  checker (20260718-152225) will enforce automatically.

