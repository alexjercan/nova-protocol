# v0.7.0 pre-release: bring web wiki + news/changelog current with everything shipped in v0.7.0 before tagging

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.7.0,docs,web,release

## Goal

Before tagging v0.7.0, the web surface must reflect what shipped. There is no
0.7.0 news post yet and the CHANGELOG [Unreleased] section + wiki need to be
brought current. This is the v0.7.0 release-time doc pass (execution), distinct
from the ongoing wiki-accuracy audit that is a v0.8.0 task (20260718-152214).

## Steps

- Finalize `CHANGELOG.md`: promote [Unreleased] to [0.7.0] with the date, in the
  subsystem groups, covering Broadside + outcome frame, settings menu (audio /
  graphics presets / keybind ref), RCS, render-scale lever, asset variety +
  mod-relative resources, per-scenario thumbnails, and all v0.7.0 fixes. Fix the
  scatter-field claim (see task 20260718-130911).
- Write `web/src/news/0.7.0.md` following the existing news post format
  (frontmatter + narrative), cross-linking devlogs, not duplicating them.
- Sweep the player wiki (`web/src/wiki/`) for anything a player would now see
  that is undocumented (settings panel, RCS controls, new scenarios/keybinds).
- Follow `web/src/wiki/dev/keeping-docs-in-sync.md`: CHANGELOG + News + wikis +
  tutorial, so nothing drifts at the tag.

## Notes

- News posts: `web/src/news/` (latest is 0.6.0). CHANGELOG groups by subsystem.
- Deep reference-doc accuracy (making every dev wiki page follow the code) is
  the broader v0.8.0 task 20260718-152214; here, cover the release surface.

