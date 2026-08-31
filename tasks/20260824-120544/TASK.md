# Release v0.12.0

- STATUS: CLOSED
- PRIORITY: 10
- TAGS: v0.12.0, release, meta

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

## Resolution

CLOSED 2026-08-31. v0.12.0 is tagged, built and published.

Against the gate:

- **Children**: no `v0.12.0`-tagged task was left OPEN but this one and the
  editor epic (`20260812-131912`, closed alongside it). The three cuts the
  epic records - the prefab loop, the readout panel, the objectives/events
  slices - are recorded on their own tasks.
- **Checks**: CI run `33381755510` is green on master across all eight jobs -
  `check / default features`, `fmt / clippy / test`, `clippy / wasm32`,
  `autopilot example`, `dependency license gate` and the three probe jobs
  (playable, systems, screenshots). `web/npm run ci` is green locally.
  Caveat: that is ONE green run of the probe jobs this session, not the
  repeated runs the gate asks for; the repetition happened across the cycle,
  not at the tag.
- **CHANGELOG**: `[Unreleased]` read whole and promoted to
  `[0.12.0] - 2026-08-31` in `dbed1dcc`. Baseline v0.11.0. Nine subsystem
  groups reordered to the header's canonical order. Seven entries were over
  the 200-character limit and three had gone in unwrapped; all fixed. 131
  entries, longest now 200.
- **Documentation**: `mdbook build` clean. `keeping-docs-in-sync.md` gained
  the news-namespace rule. The wiki and creator pages were synced by the
  cycle's own tasks, not re-audited here.
- **Tag, build, verify, publish**: tag `v0.12.0` on `5a0187ce`; `release-flow`
  run `33380389190` green on all seven jobs; four assets attached (linux
  64.2 MB, macOS 111.8 MB, windows 64.7 MB, web 34.1 MB). The web zip was
  downloaded and unpacked - `index.html`, a 62.8 MB wasm, its JS glue,
  `assets/` and `credits/`. `deploy-page` run `33383016424` published the
  site; `/news/0.12.0/`, the new drives figure and the new landing loop all
  serve 200, and the frozen `news-0110-release-lead.webm` still serves its
  v0.11.0 bytes.

One defect escaped into the tagged tree and was fixed after it:
`BENCH_BEARING` in `screenshot_section_drives.rs` was read only by a
`debug`-gated function but was not gated itself, so
`cargo check --workspace --all-targets` under default features failed on
`-D dead-code`. Fixed in `6ce601e8` on master. The release binaries build
with `cargo build --profile dist` and never compile the examples, so the
published artifacts are unaffected. The tag was deliberately left in place.
