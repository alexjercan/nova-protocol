# WASM `.meta` deserialize errors under `AssetMetaCheck::Always`

## Symptom

On the web build the console fills with, per asset:

```
ERROR bevy_asset/server/mod.rs:593 Failed to deserialize meta for asset shaders/lock_dwell_ring.wgsl:
  Failed to deserialize minimal asset meta: SpannedError { code: ExpectedNamedStructLike("AssetMetaMinimal"),
  span: Span { start: Position { line: 1, col: 1 } } }
```

...and the affected assets never load (the game can hang waiting on `GameAssets`).

## Root cause (confirmed against the pinned bevy_asset 0.19.0 source)

`nova_core::assets_plugin()` ships `AssetMetaCheck::Always`, which reads a `<path>.meta`
sidecar for EVERY asset (`server/mod.rs:1564-1567`). We keep `Always` on purpose: it is
what makes `cubemap.png.meta`'s `array_layout` take effect, and it is the only mode that
covers dynamic `mods://`/`self://` skybox metas that a fixed `Paths` set cannot (see the
doc comment on `assets_plugin`).

The wasm asset reader (`io/wasm.rs:99-114`) maps the HTTP response like this:

- **200** -> return the body bytes.
- **403 | 404** -> `AssetReaderError::NotFound`.
- anything else -> `AssetReaderError::HttpError`.

Bevy then, in `get_meta_loader_and_reader` (`server/mod.rs:1576-1642`):

- on **NotFound** -> falls back to `loader.default_meta()`. Asset loads fine. (This is the
  path the `assets_plugin` doc comment assumes: "come back HTTP 404 ... bevy handles by
  falling back to the loader's default meta". It is correct FOR A SERVER THAT 404s.)
- on **200** -> parses the body as RON `AssetMetaMinimal`. If the body is an HTML page
  (`<!DOCTYPE html>`), RON fails at line 1 col 1 -> `AssetLoadError::DeserializeMeta` ->
  the whole asset load fails (logged at `server/mod.rs:593` / `:684`).

So a "Failed to deserialize meta" is ONLY possible when the server answers a missing
`.meta` with **HTTP 200 and an HTML body** instead of a 404. That is the classic SPA
history-fallback antipattern.

## Which server does that here

- `trunk serve` returns **200 + index.html** for non-existent files. Upstream, still open:
  trunk-rs/trunk#192 "trunk serve returns 200 OK on not-existing files". This is the
  reproduction path for local single-game dev (`trunk serve` at the repo root, and the
  webpack dev server's `/play/` proxy forwards to it).
- GitHub Pages (the deploy in `.github/workflows/deploy-page.yaml`) returns a real **404**
  for missing files -> NotFound -> graceful fallback. **Production is NOT broken by this.**
- `npx http-server` (used by `scripts/preview-web.sh`) returns real 404s by default -> also
  fine.
- itch.io returns 403 for missing files -> bevy already treats 403 as NotFound -> fine.

Net: this is a `trunk serve` dev-loop problem, not a production outage. The deployed
`/play/` site loads.

## Why the usual fix does not apply

The internet's stock answer is `AssetMetaCheck::Never` on wasm (bevy#18002, #10157). We
cannot: `Never` silently defeats `cubemap.png.meta`'s `array_layout` and resurrects the
skybox upload race (tasks/20260710-143138). `Paths` cannot enumerate dynamic mod skybox
paths (task 20260717-111558). `Always` is the deliberate, correct choice.

## Recommended fix: ship a real `.meta` for every asset

If every asset has a real sidecar, the server always returns 200 + valid RON, so there is
never a missing-file request to mishandle. This is server-independent (fixes `trunk serve`,
and also removes the extra per-asset 404 request + console noise on GH Pages/itch.io) and
keeps `Always` untouched.

Bevy exposes exactly the right API:

```rust
AssetServer::write_default_loader_meta_file_for_path(path)  // server/mod.rs:1827
```

It picks the correct loader by extension, serializes `loader.default_meta()`, and refuses
to clobber an existing meta (`MetaAlreadyExists`) - so our hand-authored cubemap metas are
preserved. A small native headless tool (an example/xtask that boots the game's exact
plugin/loader set) can walk `assets/**` and call it for every file lacking a `.meta`.

Sub-decision (for the user): commit the generated metas alongside the assets, or generate
them at build time via a Trunk `[[hooks]]` post-build stage (repo stays clean, must run in
`preview-web.sh` and the deploy too). Committing is consistent with the existing checked-in
cubemap metas; a hook avoids churn when assets change.

### Alternatives considered

- `trunk build` + a static server (http-server/python) instead of `trunk serve`: zero code,
  but loses the hot-reload dev loop.
- Patch/proxy trunk to 404 missing `.meta`: fights an upstream bug; not portable.

## Implementation

New crate `crates/nova_meta_gen` (a native, GPU-free binary) plus a Trunk `post_build`
hook.

- `nova_meta_gen` boots a headless Bevy `App` (`MinimalPlugins` + `AssetPlugin` rooted at
  the target asset dir) and registers exactly the loaders the game's assets use, WITHOUT
  `RenderPlugin` and without ever calling `App::run`:
  - `ImageLoader::new(CompressedImageFormats::NONE)` (`png`), same pattern as the headless
    `cubemap_meta` test.
  - `ShaderLoader` (`wgsl`) and `AudioLoader` (`wav`) - both unit structs, registered
    directly.
  - `GltfPlugin::default()` (`glb`/`gltf`): its `finish()` registers the loader and falls
    back to `CompressedImageFormats::NONE` when no render device is present.
  - `NovaModdingPlugin` for the three custom RON loaders (`content.ron`, `bundle.ron`,
    `catalog.ron`). These compound extensions map 1:1 to their loaders via Bevy's
    longest-suffix matching, so the extension-only lookup in
    `write_default_loader_meta_file_for_path` cannot pick the wrong loader. There are no
    plain-`.ron` assets, so no ambiguity exists.

  It then walks the asset tree and calls
  `block_on(AssetServer::write_default_loader_meta_file_for_path(rel))` for each file,
  treating `MetaAlreadyExists` (hand-authored cubemap metas) and `MissingAssetLoader`
  (`.md` wiki files) as normal skips. `default_meta()` reads only each loader's
  `Settings::default()`, so the throwaway loader instances' field values never reach the
  output - which is why NONE/placeholder construction is correct.

- The dependency on `nova_modding` also does load-bearing feature work: transitively
  (`nova_modding -> nova_gameplay`) it unifies Bevy's `wav` feature into this crate's
  subgraph, so `AudioLoader` claims `.wav`. Without it `cargo run -p nova_meta_gen` would
  build Bevy without `wav` and silently skip all 16 `.wav` sidecars.

- `Trunk.toml` gains a `post_build` hook (`cargo run -p nova_meta_gen`). Trunk runs it
  against `$TRUNK_STAGING_DIR` (which already contains the copied `assets/`) BEFORE moving
  staging into `dist/`, and blocks on it, so the sidecars land in `dist/`. `preview-web.sh`
  and `.github/workflows/deploy-page.yaml` both call `trunk build`, so they inherit the hook
  with no change.

### Scope note

Production (GitHub Pages) already loads without this - it returns real 404s. The fix is
still applied there because it also removes the extra per-asset `.meta` 404 request and
console noise on every web load, and hardens against any future SPA-fallback host. The one
cost is that the deploy compiles the native tool (and thus Bevy) once per fresh checkout;
`nova_portal_gen` is deliberately engine-free to keep deploys fast, and this tool is not, so
if deploy time becomes a concern the hook can be gated to debug builds only (production does
not need it).

## Reflection

- The `assets_plugin` doc comment already anticipated web `.meta` 404s and claimed they were
  handled gracefully - which is TRUE for a 404, and misled the initial reading of the bug.
  The tell was the error kind: a `DeserializeMeta` at `line 1, col 1` is a parse of real
  bytes, not a not-found. Reading `io/wasm.rs` (status -> `NotFound` vs bytes) and
  `server/mod.rs:1576-1642` (the 200 vs `NotFound` fork) was what turned "another 404 noise"
  into "the server returns 200 + HTML". Lesson: match the exact error variant to the code
  path before trusting a prose explanation of the symptom.
- The headless/GPU-free constraint (CI deploy) drove the design more than the fix itself.
  The `cubemap_meta` test was the key precedent proving loaders register without a device.
- The `wav` feature-unification subtlety is easy to miss and would have failed silently (no
  error, just no `.wav` sidecars). The test asserts a `.wav` meta is written specifically to
  catch a future dependency change that drops the feature.
