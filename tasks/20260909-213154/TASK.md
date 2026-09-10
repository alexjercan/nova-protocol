# Release v0.14.0

- STATUS: OPEN
- PRIORITY: 10
- TAGS: v0.14.0, release, meta

Meta task: ship v0.14.0, the stabilization and store release. Runs last. The
v0.13.0 flow (`20260831-145920`) is the precedent.

## Gate

- Every `v0.14.0`-tagged task is CLOSED or explicitly cut with the cut
  recorded on the task.
- Full correctness probe green three times in a row on master, content
  lint, Rust checks, and web CI all pass. A flaky range is a defect, not a
  re-run.
- The gameplay feedback ledger (`20260909-213118`) has a disposition on
  every row.
- CHANGELOG.md `[Unreleased]` reviewed whole against the changelog rules:
  baseline is v0.13.0, one entry per released change, no intra-release fix
  notes, grouped by subsystem. Bugs found and fixed inside this cycle that
  never shipped get no entry; bugs that shipped in v0.13.0 or earlier go
  under Fixes.
- Documentation (/wiki, /create, /dev) matches shipped behavior; the wiki
  stills re-shot where a fix changed what they show.
- Anything deferred out of v0.14.0 lands in the backlog, recorded on its
  task.
- Tag, build, verify the web artifact, publish the site, then hand the
  builds to the distribution task (`20260824-130004`) for itch.io.
