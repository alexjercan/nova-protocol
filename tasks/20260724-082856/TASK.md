# Refresh frontend app images: fill missing + re-capture stale screenshots across web/

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: v0.10.0, web, assets, screenshot

## Story

Replace every missing, placeholder, or stale website capture with current
v0.10.0 output, produced by the rebuilt `screenshots/` examples
(`20260804-093910`, which reduces them to capture-only) and packaged by
`scripts/gen-web-screenshots.py`. This task owns the shipped asset refresh AND
the script's producer manifest - probe never enters this path
(`20260802-120045` WONTDO).

The coverage flag is `--report` and there is only one name for it. Owner call
2026-08-04: `20260804-093910` had cited a `--check` flag; both `--check` and
`--report` are absent from the script today
(`scripts/gen-web-screenshots.py:568-574` has only `--stage-dir`, `--no-icons`,
`--self-test`), and this task is the one that builds it.

## Steps

- [x] Add an ADVISORY coverage report to `scripts/gen-web-screenshots.py`
      (`--report`): scan `web/src/**` for referenced `assets/<name>`, diff
      against the manifest and the shipped assets, and print each gap with its
      OWNER class - `capturable` (names a producer example), `manual`
      (authored art: post cards, diagrams, icons), or `historical` (a figure
      for an older shipped version). Always exits 0. Wrong dimensions and undeclared staging files are reported the same
      way, not as errors.
- [ ] Wire the report into CI as a warning-only job: it prints the outstanding
      list on every run and never fails the build. It is a worklist, for the
      owner (what art to draw) and for automation (what a producer could
      capture), not a gate.
- [ ] Inventory all image references under `web/src/` from that report and
      assign each one its owner class, incl. the ~45 unresolved names today
      (36 `news-0X0-*`, 7 post-card thumbnails, `wiki-settings.png`).
- [ ] Reconcile thumbnail naming between the website and the script's manifest;
      retain one version-based scheme and remove obsolete devlog aliases.
      Post-card thumbnails are `manual` - they get a manifest slot and a
      destination, never a producer.
- [ ] Run the `screenshots/` producers into staging and package. Refresh HUD,
      radar, flight, combat, editor, NOVA OS, gravity, sections, tutorial,
      and feature images the report marks stale or missing and `capturable`.
- [ ] Fill what old-version news figures the current build can plausibly
      stand in for, and leave the rest outstanding. Owner call 2026-08-04: an
      approximate historical figure beats an empty placeholder box; exactness
      is not required, and no post is blocked on one.
- [ ] Review every generated image at its actual page crop. Adjust the producer
      example's scene/framing step, then recapture; do not hand-fix generated
      screenshots.
- [ ] Re-run the report and the website build. Open the rendered landing,
      tutorial, news, and affected wiki pages.

## Definition of Done

- The coverage report lists every unresolved image with an owner class and
  exits 0, so it can run in CI as a warning.
  (cmd: `nix develop --command python3 scripts/gen-web-screenshots.py --report`)
- Every `capturable` reference resolves to current generated output with one
  declared producer; what remains outstanding is `manual` or `historical`.
  (test: `web_asset_report_classifies_every_reference`)
- HUD/radar captures show the current instruments and no pre-v0.10.0 version
  chrome. (manual: inspect rendered HUD and radar wiki figures)
- Landing, tutorial, and affected wiki pages use intentional crops; a remaining
  placeholder is one the report classes `manual` or `historical`.
  (manual: inspect the locally rendered website pages)

## Notes

- Historical figures: capture what the current build can plausibly show, skip
  the rest. They will not be 100% accurate to the version they illustrate and
  that is accepted.
- The report NEVER hard-fails. Owner directive 2026-08-04: a missing image is a
  worklist item, not a broken build - some images (post-card thumbnails,
  diagrams) are hand-made art that no automation can produce.
- The script stays stdlib-only python, run from the repo root, with the capture
  producers invoked by hand or by a small wrapper - not by `nova_probe`.
- Known drift: site version thumbnails and `thumb-devlog-*` generator names do
  not currently agree; `wiki-hud.png` and `wiki-radar.png` reuse older captures.
- Authored diagrams/icons may remain authored. Only game-rendered imagery needs
  an automation producer.
- SUPERSEDED 2026-08-05 by `20260805-105154`. Step 1 landed on master
  (`0ff077ff`): `scripts/gen-web-screenshots.py --report` is the advisory
  worklist, README + `development.md` document it. The rest did not: the owner
  chose to delete every shipped game-rendered capture and redo the
  `screenshots/` examples for better-looking shots, one image per step, rather
  than patch the old set in place. The replacement carries the `capturable`
  worklist as its input.
