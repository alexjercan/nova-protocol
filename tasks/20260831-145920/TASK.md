# Release v0.13.0

- STATUS: OPEN
- PRIORITY: 10
- TAGS: v0.13.0,release,meta

Meta task: ship v0.13.0, the content and feel release. Runs last. The
v0.12.0 flow (`20260824-120544`) is the precedent.

## Gate

- Every `v0.13.0`-tagged task is CLOSED or explicitly cut with the cut
  recorded on the task.
- Full correctness probe green repeatedly (not once), content lint, Rust
  checks, and web CI all pass on master.
- CHANGELOG.md `[Unreleased]` reviewed whole against the changelog rules:
  baseline is v0.12.0, one entry per released change, no intra-release fix
  notes, grouped by subsystem.
- Documentation (/wiki, /create, /dev) matches shipped behavior.
- Anything deferred out of v0.13.0 lands on the v0.14.0 board or in the
  backlog, recorded on its task. v0.14.0 is the stabilization and store
  release, so this release's leftovers are its raw material.
- Tag, build, verify the web artifact, publish.
