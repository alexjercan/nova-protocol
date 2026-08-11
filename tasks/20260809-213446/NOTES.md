# Notes

## What landed

One commit on master, web/ plus this record. Both halves of the task.

### Half 1 - the wiki sweep

Every dev page that names a crate, module path, or cargo target was re-derived
against the current tree. Method: four parallel verification agents (one per
page group) checked every structural claim against source; findings were
applied by hand. The two player pages that carry structure (`scenarios.md`,
`modding.md`) were swept too; the rest name no code and were left alone.

Load-bearing fixes:

- `dev/architecture.md`: crate map gained `nova_autopilot`, `nova_probe_cli`,
  `nova_perf_web`, `nova_authoring`, `nova_events_macros`; the `nova_probe`
  row now describes the in-game half only; the `nova_ui` row lists the real
  module set and consumers. The mermaid graph gained the edges the benchmark
  failed on (`nova_hud -> nova_ui`, `nova_gameplay -> nova_ui`,
  `nova_os_ui -> nova_ui`, `nova_scenario -> {gameplay,ship,hud}`,
  `nova_ship -> nova_events`, `editor -> ship`, `assets -> scenario`) plus a
  curation caveat and a dev-tools paragraph. App assembly updated to the real
  `build()` order (ship/hud/os_ui/loading-screen). The "built-ins are Rust,
  data files are a TODO" tail replaced with the generated-RON pipeline.
- `dev/development.md`: content bin path (`nova_authoring`), wasm CI command
  (`--exclude nova_probe_cli`), `catalog_matches_disk` needs `--workspace`,
  qualified `nova_probe::nova_timeline()` forms, `trace` feature, `/mods/` in
  the Pages deploy, news enumeration to v0.9.0, CI autopilot-example step,
  dead `playable` example name.
- `dev/sections.md` + `dev/guide-add-section.md`: every `bcs` attribution
  replaced with the real owners (`integrity/health.rs`, `integrity/core.rs`);
  the five-piece `NovaIntegrityPlugin` roster; builders/`content -- gen`
  pipeline; the `nova_os_ui` exhaustive matches a new section kind must edit;
  the three `placement.rs` matches; turret joint-tree config.
- `dev/scenario-system.md` + `dev/guide-extend-scenarios.md`: `loader/` and
  `actions/` directory split, the two reserved variables, the no-base-body
  spawn contract, the `Light` kind (scenes with none render black),
  `ScatterObjects`, builders moved to `nova_authoring`, data-files TODO
  deleted.
- `dev/automation-harness.md`: `nova_probe` vs `nova_probe_cli` split, real
  example deadlines, `NOVA_SHOT_DIR=target/shots`.
- `dev/guide-make-a-mod.md` / `dev/modding-ron.md` / `dev/mod-portal.md`:
  lint command crate, real manifest snapshots, update-detection and
  stall-timeout claims, `RemoteCatalogState`, format-vs-loader ownership.
- Two broken `./modding-ron` relative links -> `../modding-ron/`.

Gate: `cd web && npm run ci` green (format, lint, tests, build).

## Half 2 - why the routing map did not catch a four-crate split

Three causes, none of them "nobody thought about docs":

1. **The sweeps were name-level, not claim-level.** Several epic lanes DID
   update wiki pages (L8.x, L9, L10.1 all touched `web/src/wiki/`), and the
   L10 proof even says "architecture.md (crate table, mermaid graph) all name
   the new crate". They did name it. What survived was every claim BETWEEN
   the names: the graph kept drawing `nova_ui` as menu+editor-only while new
   crate rows were being added around it. t1-005 failed on exactly that edge,
   on a page that names all the new crates.
2. **The routing map is keyed by crate/dir names, and a structural refactor
   invalidates the keys themselves.** No row's trigger was "the crates/
   layout changed", so the map had nothing to say about the one change class
   that rewrites its own index (and the map itself went stale: content CLI
   still keyed to `nova_assets`).
3. **The repo rule arrived late.** "Ship code and invalidated docs together.
   Follow the docs routing map." landed in `AGENTS.md` only in c2dde47d
   (Aug 8), after the epic's lanes ran. The lanes' DoD carried code proofs;
   no lane checklist referenced the routing map.

### The fix (all in web/, per owner scope)

`dev/keeping-docs-in-sync.md`:

- New FIRST row in the dependency map: crate split/merge/rename/move ->
  `dev/architecture.md`, `dev/project-tour.md`, and this page's own keys.
- Stale keys corrected (content CLI -> `nova_authoring`; harness row gains
  `nova_probe_cli` + `dev/development.md`).
- A new section, "Check means re-derive, not grep", recording this failure
  mode so the next epic reads it: name-level sweeps pass while claim-level
  rot accumulates, and a lane-per-change epic needs an explicit cross-cutting
  sweep step when `crates/*` changes shape.

The `AGENTS.md` rule (cause 3) was already fixed by the owner in c2dde47d;
nothing outside web/ needed changing for this task.

## Out-of-lane observations (fixed in the follow-up commit, owner-approved)

A workspace-wide scan for declared-but-unused deps (every nova_* dep in every
crate, plus all root dev-deps) found eleven leftovers from the refactor,
removed after `cargo check --workspace --all-targets` (default and
`--features debug`) stayed green:

- `nova_gameplay`: `nova_info`, `nova_os` (zero source references).
- `nova_hud`: `nova_events`, `nova_info`.
- `nova_os_ui`: `nova_info`.
- Root dev-deps: `nova_autopilot` (examples reach `AutopilotPlugin` through
  the preludes now), `nova_modding` + `ron` + `serde` + `bevy_rand` + `rand`
  (the screenshot-reel example that justified them was deleted; `ron` only
  survives in string literals).

Kept deliberately: `nova_modding -> nova_gameplay` - no code use, but the
lib.rs intra-doc links name `nova_gameplay::prelude::AssetRef` and the
explicit `features = ["serde"]` states intent; its stale SectionConfig
comment was rewritten instead.

Also fixed: `scripts/serve-mods.sh`'s stale comment pointing at a
`Trunk.toml` `[[proxy]]` that no longer exists, and the architecture.md
pruned-edge example retargeted (`nova_hud -> nova_events` is no longer an
edge at all).

### Third-party pass (cargo-udeps, owner-requested)

`cargo udeps --workspace --all-targets` (nixpkgs cargo-udeps 0.1.61,
RUSTC_WRAPPER unset - sccache breaks its rustc interception), run twice:
default features and `--features debug`, so feature-gated usage is not
misread as dead.

- One genuine hit: `nova_gameplay -> bevy_enhanced_input` (input moved to
  `nova_ship`, which declares its own). Removed.
- One false positive, expected: the root `nova_debug` dev-dep is flagged on
  the default run because every use is `cfg(feature = "debug")`-gated; the
  debug run clears it. Its Cargo.toml comment already documents this. No
  udeps ignore metadata added - udeps is not part of the CI loop.
- The serde-optional deps in `nova_scenario`/`nova_ship`/`nova_gameplay` are
  exercised in the workspace build (feature unification via `nova_modding`),
  so the runs cover them.

Docs cross-check after the removal: the architecture.md graph draws only
`nova_*` edges (unchanged), `EnhancedInputPlugin` is still added by
`nova_core` (which keeps its own dep), and modding-ron.md's
`bevy_enhanced_input::Binding` serde note names the type, not a crate. No
wiki change needed.

Still open (accepted): `dev/guide-author-scenario.md`'s snippets were
verified against source, but the page is 1143 lines and quotes numeric
balance values from the catalog, which will drift again - same class of risk
`guide-author-section.md` had, which now carries an "illustrative values"
disclaimer.
