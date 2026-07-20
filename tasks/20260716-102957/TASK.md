# Optional: news-fragment changelog authoring pipeline (changelog_entries/ + release compile + CI nag)

- STATUS: OPEN
- PRIORITY: 22
- TAGS: spike,docs,tooling,v0.8.0

## Story

As the maintainer of two changelog artifacts (CHANGELOG.md and the /news/
posts), I want changelog entries authored per-change while context is fresh -
IF the hand-maintained flow ever starts drifting - so that release-time
writing is compilation, not archaeology.

Adopt the Bevy/Wesnoth news-fragment authoring pattern: each change drops a
small markdown fragment (with frontmatter: title, PR/commit refs, subsystem
section, optional breaking-changes body) into a `changelog_entries/`
directory; a release-time step compiles the fragments into both
`CHANGELOG.md` and the web news/changelog page; a CI check nags when a change
lands without a fragment.

## Steps

- [ ] Re-check the trigger condition first: the v0.7.0 docs review
      (2026-07-18) found CHANGELOG.md fully consistent with git history - the
      drift was one-directional editorial compression in the NEWS page (~20
      shipped items uncovered), which fragments would NOT have fixed, since
      news prose is deliberately curated. If the hand flow is still holding,
      write the documented decision not to build it and close.
- [ ] If building: define the fragment format (frontmatter keys, section
      mapping to the CHANGELOG's subsystem headers) and the directory
      convention.
- [ ] Build the release compile step (fragments -> CHANGELOG [version]
      section; the news post stays hand-written prose per the exemplar-first
      lesson) and the CI nag for changes without a fragment.
- [ ] Update keeping-docs-in-sync.md: fragment-on-change replaces
      line-in-CHANGELOG-on-change.

## Definition of Done

- Either: a fragment format + compile step + CI nag exist and one release has
  been cut through them, OR a documented decision not to build it (with the
  evidence) is recorded here and the task closed.

## Notes

- Spike: tasks/20260716-102940/SPIKE.md
- Depends on 20260716-102950 and 20260716-102954 (the two artifacts).
- Evidence for "not yet": the v0.7.0 cycle kept CHANGELOG.md complete by hand
  under the AGENTS.md same-task rule; the ephemeral-docs compile step
  (20260718-175424) is adopting the compile-then-clear pattern for lessons
  anyway, so revisit only if a release ships with a CHANGELOG hole.

## Grooming (2026-07-20): reprioritized 25 -> 22; recommend CLOSE-with-decision

The task's own step 1 says: "If the hand flow is still holding, write the
documented decision not to build it and close." It is holding - v0.7.0 shipped
with CHANGELOG.md consistent with git history; the only drift was deliberate
editorial compression in the news page, which fragments would not fix. The
honest outcome is to close this with that decision recorded, not to build the
pipeline. Left OPEN at the bottom rather than closed unilaterally in a
reprioritization pass; flagged for closure at the next docs cycle.
