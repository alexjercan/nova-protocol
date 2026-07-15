# Static mod portal: webmods/ + generator bin (catalog.json, hashed files) + deploy step; demo mod moves online

- STATUS: CLOSED
- PRIORITY: 17
- TAGS: modding,web

Spike: tasks/20260714-202515/SPIKE.md (options D, H, J)
Depends on: 20260715-142849 (bundle meta - the generator reads it).

Goal: remote mods served as generated static files on the existing GitHub Pages
site. Remote mod sources live in a repo-root `webmods/` folder (same bundle
shape, outside `assets/` so they don't ship in the game). A small workspace bin
scans `webmods/`, validates, computes per-file size + sha256, and emits
`site/mods/catalog.json` (versioned `schema_version`; entries: id, version,
meta, `files: [{path, size, sha256}]`) plus copies files to
`site/mods/<id>/<version>/...`. Wire format is JSON (serde types shared between
game and generator). Add the deploy-workflow step. Document the portal layout +
publish flow in docs/.

SCOPE CORRECTION (20260715, at plan time): the spike said "demo moves online".
Since then demo became the load-bearing REAL-FILE test subject (merge/metadata
tests AND the synthetic hidden rig reuse its bundle, 151551) and it remains the
in-repo modding example. Moving it would churn everything three landed tasks
just stabilized, for no extra dogfood value. Instead: demo STAYS shipped; the
portal's FIRST mod is a NEW small webmods/ mod, which dogfoods the install path
end-to-end in 142906/142916. Flagged to the user in the flow report.

## Plan (20260715)

Design decided from the code:

- The generator must NOT depend on nova_modding: that crate pulls bevy (Asset
  traits) + nova_gameplay/nova_scenario, which would compile the whole engine
  natively inside the deploy job. EXTRACT the pure serde/ron types -
  `ModMeta`, `BundleManifest`, `ModEntry`, `CatalogManifest` - into a new tiny
  `crates/nova_mod_format` (deps: serde, ron only); nova_modding re-exports
  them (`pub use nova_mod_format::...` in its prelude), so downstream imports
  do not change. The PORTAL wire types live there too: `PortalCatalog {
  schema_version: u32, entries: Vec<PortalEntry> }`, `PortalEntry { id,
  version, bundle (manifest path within the mod dir), meta: ModMeta, files:
  Vec<PortalFile>, total_size }`, `PortalFile { path, size, sha256 }`. The
  game (142906) parses the same types.
- DEEP content validation (do the content files actually load?) does NOT run
  in the generator: it becomes a PR-CI integration test that recursive-loads
  every webmods bundle through the REAL loaders (native tests can list dirs).
  The generator validates what a manifest gate can: bundle parses, meta has
  non-empty name+version, listed content files exist on disk, mod ids unique
  and not colliding with the shipped catalog.
- Deploy workflow (.github/workflows/deploy-page.yaml, assembly at ~lines
  72-81): one new step after site assembly - `cargo run --release -p
  nova_portal_gen -- --source webmods --shipped assets/mods.catalog.ron --out
  site/mods`. nova_portal_gen builds in seconds (no engine deps).

Steps:
- [x] 1. New crate `crates/nova_mod_format` (workspace member; serde + ron):
  MOVE `ModMeta`/`BundleManifest`/`ModEntry`/`CatalogManifest` (+ their unit
  tests) from nova_modding, which re-exports them so no other crate changes.
  Add the portal wire types (above) with serde derives + defaults and a JSON
  round-trip unit test (serde_json as a dev-dep).
- [x] 2. New bin crate `crates/nova_portal_gen` (deps: nova_mod_format, clap,
  ron, serde_json, sha2): scan `--source` (each subdir = one mod, id = dir
  name; exactly one `*.bundle.ron`), validate (manifest parses; meta name +
  version non-empty; every listed content file exists; unique ids; no id
  collision with `--shipped` catalog), hash+size every file in the mod dir,
  and write `--out`: a deterministic (sorted) `catalog.json` +
  `<id>/<version>/<files>`. Clear one-line errors; non-zero exit on any
  validation failure.
- [x] 3. Author the first portal mod in `webmods/gauntlet/`: one scenario
  (adapted from the demo mod's arena content - a small combat gauntlet, no
  section overrides), `gauntlet.bundle.ron` with full meta (name/description/
  author/version 1.0.0). Keep content modest; the task's goal is infra.
- [x] 4. Generator tests (crates/nova_portal_gen, integration): run the
  generator against the REAL `webmods/` into a temp dir - catalog.json parses
  as `PortalCatalog`, every listed file exists with matching size + sha256
  (recomputed), entries sorted. Failure cases with synthetic temp sources:
  duplicate id, id colliding with the shipped catalog, missing content file,
  empty meta version - each must fail with the right error.
- [x] 5. PR-CI deep gate (crates/nova_assets/tests/webmods_validation.rs, or
  nova_modding): a headless-app test with `AssetPlugin.file_path` at
  `../../webmods` that lists the mod dirs (native), recursive-loads each
  `*.bundle.ron` through the real loaders, and asserts Loaded - the publish
  content gate, running where CI already runs tests.
- [x] 6. Deploy workflow: add the generator step after site assembly (before
  the deploy action); site layout becomes /mods/catalog.json +
  /mods/<id>/<version>/... next to /play/.
- [x] 7. Docs: new docs/mod-portal.md (portal layout, publish-a-mod flow,
  catalog.json schema + schema_version policy, local serving for dev
  `python -m http.server` / trunk serve note, future real-server path per the
  spike); cross-ref from modding-ron-format.md; CHANGELOG entry.
- [x] 8. Verify: `cargo fmt --check`; `cargo check --workspace --all-targets`;
  `cargo test -p nova_mod_format -p nova_portal_gen` (one -p per run),
  `-p nova_modding`, `-p nova_assets` test targets; run the generator locally
  against webmods/ into the scratchpad and eyeball catalog.json; sanity-check
  the workflow YAML (actionlint if present, else careful diff).

## Notes

- Relevant files: crates/nova_modding/src/lib.rs (types to move: ModMeta:~84,
  BundleManifest:~120, ModEntry:~295, CatalogManifest:~320 + prelude + unit
  tests), .github/workflows/deploy-page.yaml:72-81, assets/mods/demo/* (content
  reference for authoring), Cargo.toml workspace members.
- Ids/versions: id = webmods subdir name (validated against `[a-z0-9-]+`?
  decide in-code; demo/base use kebab-case); version from meta (non-empty for
  published mods).
- The game-side consumption (fetch/parse/install) is 142906; this task only
  serves + validates. No game behavior changes at all.
- serde_json + sha2 are new workspace deps (generator-side only; serde_json
  also dev-dep of nova_mod_format for the round-trip test).

## Close-out (20260715)

What changed: (1) `crates/nova_mod_format` - the pure serde format types
(`ModMeta`, `BundleManifest`, `ModEntry`, `CatalogManifest` MOVED from
nova_modding, which re-exports them so no downstream import changed) plus the
NEW portal wire schema (`PortalCatalog`/`PortalEntry`/`PortalFile`,
`PORTAL_SCHEMA_VERSION = 1`) with a JSON round-trip pin. (2)
`crates/nova_portal_gen` - lib (testable `generate`) + thin clap CLI: scans a
webmods source tree, manifest-gate validates (parse, publishable meta, content
files exist, id charset, no shipped-id shadowing, deps resolve), hashes
(sha256) + sizes every file, copies to `<out>/<id>/<version>/` and writes a
deterministic catalog.json (files before catalog, so a partially-written
portal never lists missing files). Engine-free: no bevy anywhere in its tree.
(3) `webmods/gauntlet` - the first portal mod (one scenario: a beacon slalom,
constructs proven by the demo mod's content). (4) Deep publish gate:
`webmods_validation` test recursive-loads every webmods bundle through the
real loaders on PR CI. (5) Deploy workflow publishes `/mods/` next to `/play/`.
(6) docs/mod-portal.md + cross-refs; architecture.md crate table updated (also
backfilled the pre-existing missing nova_modding/nova_ui rows); CHANGELOG.

Deviation from the seed: demo does NOT move online (scope correction at plan
time - it is the real-file test subject and in-repo example; a new mod
dogfoods the portal instead). Flagged to the user in the flow report.

Evidence: fmt clean; `cargo check --workspace --all-targets` clean;
nova_mod_format 3 passed (incl. moved RON decode tests), nova_portal_gen 9
passed (real-tree publish with hash recomputation, byte-identical determinism
per the verify-generator-stability lesson, and six distinct failure-mode
cases), nova_modding 1 passed, demo_scenario 11 passed, webmods_validation 1
passed (gauntlet loads recursively through the real loaders). Local generator
run published gauntlet 1.0.0 (2 files, 4654 bytes); catalog.json eyeballed -
exact designed shape. Full suite on CI; the workflow step itself runs first on
the next real deploy (workflow_dispatch), noted as residual risk - its command
is byte-identical to the locally-run one.

Review: round 1 REQUEST_CHANGES - one MAJOR (R1.1: an escaping content path
`../x.content.ron` validated by existence but was never copied, publishing a
broken mod with exit 0; fixed by validating MEMBERSHIP in the walked file set,
with the reviewer's exact repro as a regression test) + three MINORs and three
NITs, all addressed (empty-source guard, sorted-output assertions, doc fixes;
the plan's "duplicate id" failure case turned out impossible by construction -
ids are unique subdir names - substituted accordingly). Round 2 APPROVE.
Generator tests now 12 passed.

Reflection: the engine-free split (nova_mod_format) was decided at plan time by
asking what the deploy job would have to COMPILE - catching that before writing
the generator avoided a heavy rework; the re-export trick made the extraction
invisible to every consumer. The R1.1 lesson: validate against the SET you
will actually serve, not against the filesystem - existence checks answer a
different question than membership checks.

