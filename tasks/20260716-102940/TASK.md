# Spike: rework the changelog (concise CHANGELOG.md + in-web-app release notes)

- STATUS: CLOSED
- PRIORITY: 70
- TAGS: spike, docs, web

## Outcome

Research complete. See `SPIKE.md` in this folder for the full write-up.

Recommendation (RECOMMENDED): ship two artifacts from a clear division of
labor - a terse subsystem-grouped `CHANGELOG.md` (Wesnoth-style), and a richer
image-capable in-web-app `/changelog/` section grouped by feature with a
modder-facing breaking-changes callout (the Bevy-migration-guide analogue),
reusing the existing blog markdown pipeline. Devlogs stay the narrative and get
cross-linked, not duplicated. An optional per-PR news-fragment authoring
pipeline is seeded but deferrable.

Seeded tasks:
- 20260716-102950 (P80) - Tighten and re-section CHANGELOG.md. Do first.
- 20260716-102954 (P60) - Build the in-web-app changelog section.
- 20260716-102957 (P30) - Optional news-fragment authoring pipeline.
