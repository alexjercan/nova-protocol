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
