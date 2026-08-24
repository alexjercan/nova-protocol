# Release v0.12.0

- STATUS: OPEN
- PRIORITY: 10
- TAGS: v0.12.0,release,meta

Meta task: ship v0.12.0, the editor release. Runs last. The v0.11.0 flow
(`20260823-192403`) is the precedent.

## Gate

- Every `v0.12.0`-tagged task is CLOSED or explicitly cut with the cut
  recorded on the task. The epic's cut order: readout panel first, then the
  objectives/events slices of `20260714-081703`; foundations and save/load
  do not get cut.
- Full correctness probe green repeatedly (not once), content lint, Rust
  checks, and web CI all pass on master.
- CHANGELOG.md `[Unreleased]` reviewed whole against the changelog rules:
  baseline is v0.11.0, one entry per released change, no intra-release fix
  notes, grouped by subsystem.
- Documentation (/wiki, /create, /dev) matches shipped behavior - the editor,
  settings/rebinding, scenario language additions, and process channel all
  changed player- and creator-facing contracts this release.
- Tag, build, verify the web artifact, publish.
