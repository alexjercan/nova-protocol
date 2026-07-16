# Optional: news-fragment changelog authoring pipeline (changelog_entries/ + release compile + CI nag)

- STATUS: OPEN
- PRIORITY: 30
- TAGS: spike,docs,tooling

## Goal

Adopt the Bevy/Wesnoth news-fragment authoring pattern so changelog entries are
written per-change while context is fresh, instead of hand-edited into a single
file at release time. Each change drops a small markdown fragment (with
frontmatter: title, PR/commit refs, subsystem section, optional
breaking-changes body) into a `changelog_entries/` directory; a release-time
step compiles the fragments into both `CHANGELOG.md` and the web changelog
page; a CI check nags when a change lands without a fragment.

Deferrable: only worth doing once the two artifacts (20260716-102950 and
20260716-102954) exist and start drifting. Lowest priority; may be dropped if
the two-artifact maintenance stays cheap by hand.

Done = a fragment format + compile step + CI nag exist, or a documented
decision not to build it.

## Notes

Spike: tasks/20260716-102940/SPIKE.md
Depends on 20260716-102950 and 20260716-102954.
