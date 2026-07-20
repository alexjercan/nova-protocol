# Spike: can nova_meta_gen become a Python build-time hook?

- TASK: 20260718-152255
- DATE: 2026-07-20
- DECISION: **KEEP RUST.** Do not port `nova_meta_gen` to Python.

## Question

The portal generator ported cleanly to Python (20260718-152247) because it is
engine-free. The user floated the same move for the `.meta` sidecar generator.
This spike decides port-vs-keep with evidence, rather than porting blindly.

## What the tool actually emits (evidence)

`nova_meta_gen` boots a headless, GPU-free Bevy app, registers the game's asset
loaders, and calls Bevy's own
`AssetServer::write_default_loader_meta_file_for_path` for every asset lacking a
`.meta`. So each sidecar is Bevy's EXACT default meta for that loader - not
boilerplate the tool authors. Dumped per extension (one empty file per
extension, `cargo run -p nova_meta_gen -- --assets <dir>`):

- `.png` -> `loader: "bevy_image::image_loader::ImageLoader"`, settings:
  `format: FromExtension, texture_format: None, is_srgb: true, sampler: Default,
  asset_usage: ("MAIN_WORLD | RENDER_WORLD"), array_layout: None` (6 fields).
- `.glb` / `.gltf` -> `loader: "bevy_gltf::loader::GltfLoader"`, settings:
  `load_meshes, load_materials, load_cameras, load_lights, load_animations,
  include_source, default_sampler, override_sampler, validate,
  convert_coordinates, skinned_mesh_bounds_policy` (11 fields).
- `.ogg` / `.wav` -> `loader: "bevy_audio::audio_source::AudioLoader"`,
  `settings: ()`.
- `.wgsl` -> `loader: "bevy_shader::shader::ShaderLoader"`,
  `settings: (shader_defs: [])`.
- `.content.ron` / `.bundle.ron` / `.catalog.ron` ->
  `loader: "nova_modding::ContentAssetLoader"` (and the bundle/catalog loaders),
  `settings: ()`.
- `.md`, `.txt`, `.jpg`, `.mp3`, `.flac` -> no registered loader, skipped.

Every sidecar carries `meta_format_version: "1.0"`. Three hand-authored metas
exist (the cubemaps' `array_layout` sidecars); the tool correctly skips them
(Bevy returns `MetaAlreadyExists`).

## Drift assessment

The emitted content is Bevy-VERSION-SPECIFIC in two independent ways, so a
Python hardcode would drift silently:

1. **Loader type paths.** The `loader:` string is the fully-qualified Rust type
   path (`bevy_image::image_loader::ImageLoader`, `bevy_gltf::loader::GltfLoader`,
   `bevy_shader::shader::ShaderLoader`). Bevy reorganizes its crate/module
   layout regularly (the `bevy_image` / `bevy_shader` split is itself recent), so
   these strings move between releases. A wrong loader path makes Bevy fall back
   or fail to match the loader.
2. **Settings field sets.** Each `settings:` block is the loader's
   `Settings::default()` serialization. Fields are ADDED and removed across
   versions - `convert_coordinates` and `skinned_mesh_bounds_policy` on the
   Gltf loader, and `texture_format` / `array_layout` on the image loader, are
   recent additions. A hardcoded Python emitter would produce metas missing new
   fields or carrying removed ones.

The Rust tool cannot drift: it ASKS the pinned Bevy for the current default, so
it is correct by construction for whatever Bevy the workspace pins.

## Consequences of drift (why this matters now)

v0.7.0 made `.meta` correctness load-bearing for EVERY mod cubemap, not just the
base build: `AssetMetaCheck::Always` plus the SPA-fallback 200-OK-HTML trap
(`asset-meta-always-web-cost` lesson) means a missing or malformed `.meta` on
the web makes the asset fail to load. So a Python emitter that drifts on a Bevy
bump would ship broken mod textures in production - a much worse failure than
the tool's only real cost (compiling Bevy once per fresh CI checkout, cached
thereafter, already accepted for the existing Trunk `post_build` hook).

## Decision and its shape

KEEP `nova_meta_gen` in Rust. The port would trade a self-correcting,
ask-the-engine tool for a drift-prone hardcode to save nothing - it is already a
working Trunk `post_build` hook and is off the game's runtime path. This is the
deliberate CONTRAST with the portal generator: portal gen is engine-free static
data (safe to port); meta gen is a thin shim over Bevy's own meta serialization
(unsafe to port).

No `scripts/gen-meta.py`. No new tasks seeded. The tooling map now has one
answer: portal gen -> Python (done), meta gen -> stays Rust (here).

---

# Spike round 2 (2026-07-20): the LOCATION question

- STATUS: RECOMMENDED

## Question

`nova_meta_gen` is needed for the WEB build only (native's real-filesystem 404
lets Bevy fall back to the same defaults; only the web SPA-fallback-200-HTML
trap needs the sidecars pre-written). The user does not want it sitting in
`crates/` as a member of the native game and would rather it be "a tool in the
thing that builds the game for web." Round 1 settled that it must stay Rust and
ask Bevy (a Python hardcode would drift). So: can we move it OUT of the game's
`crates/` list into web-build-owned tooling WITHOUT breaking the pin to the
game's Bevy?

## Context (the load-bearing constraint)

The pin to "the game's Bevy" is not just the version string - it is the FEATURE
SET too. `crates/nova_meta_gen/Cargo.toml` declares `bevy = "0.19.0"` with
DEFAULT features, and its own comment records the trick: it depends on
`nova_modding` (path), which transitively via `nova_gameplay` unifies bevy's
`wav` feature into meta_gen's subgraph so `AudioLoader` claims `.wav`. WITHOUT
that unification, `cargo run -p nova_meta_gen` builds bevy without `wav` and
skips every `.wav` sidecar. That unification is a WORKSPACE-GRAPH property
(resolver 2, one feature resolution across the workspace). Leave the workspace
and you lose it - and must hand-replicate the game's exact bevy feature set,
which is the same drift class the tool exists to prevent, moved down a level.

Other facts: no `[workspace.dependencies]` (each crate pins bevy itself); no
`default-members`; NOTHING depends on `nova_meta_gen` (leaf tool, bin+lib+tests);
`web/` is an npm/TS project (no Cargo - a Rust crate there mixes toolchains);
Trunk invokes it BY PACKAGE NAME (`cargo run -p nova_meta_gen`, Trunk.toml
post_build), so a directory move does not change the invocation.

## Options considered

- **A. Relocate the crate out of `crates/`, keep it a WORKSPACE MEMBER**
  (e.g. `tools/meta-gen/`, a new top-level dir for build-time tooling). Update
  the `[workspace] members` path; optionally add `default-members` (the game
  crates) so a bare `cargo build`/`test` at the root skips it, reinforcing
  "web-build-only tool". Trunk/deploy invocation (`-p nova_meta_gen`) unchanged.
  PRO: gets it out of the game crate list and frames it as web tooling;
  PRESERVES the version+feature pin automatically (still one workspace graph);
  zero native-build impact (already never in the game's dep graph); nothing
  depends on it so the move is mechanical. CON: still technically a workspace
  member (not a fully separate project) - satisfies "not a crate in crates/"
  but not "not in the cargo workspace at all".
- **B. Pull it fully OUT of the workspace** (standalone crate under `tools/` or
  `web/`, `exclude`d from `[workspace]`, own lock/target). PRO: maximally
  separate from the game project. CON: loses feature unification -> must
  manually pin bevy version AND every feature the game uses (`wav`, ...) and add
  a drift-guard test, or it silently emits wrong/missing metas; still path-deps
  `../crates/nova_modding` for the custom RON loaders (so not truly detached);
  recompiles bevy on its own. REINTRODUCES the drift risk the tool prevents.
- **D. No dedicated crate: commit golden default metas, regen via a test.** A
  test in an existing bevy-linking crate writes the default metas to a
  web-owned golden dir; Trunk copies them; a CI `git diff` gate catches drift.
  PRO: no meta-gen crate at all. CON: changes a working mechanism (today nothing
  is committed - metas are generated fresh into the Trunk staging dir); commits
  a `.meta` for EVERY asset (repo clutter) that must be regenerated on every
  asset ADD, not just a Bevy bump - more friction than the current design.
- **E. Do nothing** (round-1 outcome). Works, but leaves the crate in `crates/`,
  which is the thing the user objects to.

## Recommendation

**Option A.** Move `crates/nova_meta_gen` to a new `tools/` directory (top-level,
for build-time tooling), keep it a workspace member, and set `default-members`
to the game crates so bare workspace builds skip it. This directly answers the
user's question: you CAN take it out of `crates/`, but it must stay a workspace
MEMBER - that membership is exactly what auto-pins both the bevy version and the
`wav`/feature set the metas depend on. Leaving the workspace (B) trades a clean
crate-list for a hand-maintained feature-sync burden and a new drift-guard test;
committing goldens (D) trades a zero-clutter build-time mechanism for per-asset
`.meta` files and a regen-on-add workflow. A is the low-risk move that honors
"a tool in the web build" while keeping the correctness the whole tool exists
for. If the user's intent is specifically "NOT in the cargo workspace at all,"
that is B and its costs are the tradeoff to accept - flagged as the open
question.

## Open questions

- "How separate is separate enough?" - A (out of `crates/`, still a workspace
  member) vs B (out of the workspace entirely, accepting feature-sync drift +
  a guard test). A user preference; A is recommended, B is the escape hatch.
- Exact home: `tools/meta-gen/` vs `build-tools/` vs keeping the package name
  `nova_meta_gen` vs renaming to `meta-gen`. Cosmetic; the planner picks.

## Next steps

Direction-level task seeded (for /plan to break into steps):

- tatr 20260720-224236: Relocate `nova_meta_gen` to `tools/` as a
  workspace-member build tool, out of the game `crates/` list, with
  `default-members` excluding it and the Trunk/deploy/docs references repointed.

Round-2 outcome (supersedes round 1's "no tasks seeded, keep as-is"): the tool
still stays Rust + asks Bevy + is a workspace member (all round-1/round-2
findings hold); only its DIRECTORY moves out of `crates/`.
