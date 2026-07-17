# Review: Mod-shipped skybox cubemaps bypass load-time meta (Always switch)

- TASK: 20260717-111558
- BRANCH: fix-mod-skybox-meta

## Round 1

- VERDICT: APPROVE

Small, focused diff that delivers the Goal: `assets_plugin()` switches from the
per-path `AssetMetaCheck::Paths` set to `AssetMetaCheck::Always`, so a mod's own
skybox sidecar is honored and its cube arrives 6-layer from the loader, closing
the teardown upload race for mod cubemaps the same way the base fix closed it for
base cubemaps.

Independently re-verified the load-bearing claims (shared implementer/reviewer
session, so this is required):

1. **The `Always` web cost is real but non-fatal** - confirmed at the pinned
   bevy rev: `server/mod.rs:1564` (`Always` => `read_meta = true` per asset),
   `io/wasm.rs:122` (fetches `<path>.meta`), `io/wasm.rs:100` (404 -> `NotFound`),
   `server/mod.rs:1616` (falls back to `default_meta()`). Extra web requests +
   console noise, not breakage. Native pays a filesystem stat.
2. **The downloaded-mod path is covered, not just the shipped one.** The new
   test loads the SHIPPED `mods/example/...nebula.png` via the default file
   source. The DOWNLOADED path (`mods://`) was the likely hole - it works only
   if the `.meta` is in the cache. Verified it is: `nova_portal_gen` packages a
   mod via `walk_files` ("every file of the mod, verbatim copy", no extension
   filter), so the sidecar ships in the download and `store_mod_files` caches it;
   `mod_binary_resources.rs:145` asserts the sidecar ships beside the png.
3. **`Always` introduces no surprise behavior.** Only 3 `.meta` files exist
   repo-wide (both base cubemaps + the mod cubemap), so `Always` newly honors
   exactly `nebula.png.meta` - the intended target - and nothing else changes.
4. **The new test is a real regression pin.** Under the old `Paths` config,
   `nebula.png` is unlisted -> loads single-layer -> `array_layer_count() == 1`,
   failing the `== 6` assert. It fails with the fix reverted.

Checks: `trunk build` (web) exit 0; `cargo check` + `cargo fmt` clean;
`cubemap_meta_app_config` 3/3 pass incl. the new mod case. Full suite deferred
to CI per the project's local-test policy (only new tests run locally).

Docs swept correctly (CHANGELOG, make-a-mod guide, stale `meta_check` comments in
`cubemap_meta.rs` and `actions.rs`, design note). TASK.md decision matches the
code.

- [x] R1.1 (NIT) crates/nova_core/tests/cubemap_meta_app_config.rs:99 - the mod
  case exercises the shipped-mod path (default file source). The downloaded-mod
  path (`mods://` source reading a cached sidecar) is verified by reading
  `nova_portal_gen`'s verbatim copy, not by a test. Optional: a one-line note in
  the test doc that the downloaded path's coverage lives in the packaging
  guarantee, so a future reader does not assume `mods://` is exercised here.
  - Response: Addressed - added a doc paragraph to the test noting it covers the
    shipped path and pointing the downloaded-path coverage at
    `mod_binary_resources.rs`'s packaging assertion. Verified.

### Out of scope (filed separately, not blocking)

- `nova_editor/src/ui/mod.rs:110` inserts `SkyboxConfig` directly (not via
  `apply_pending_skybox_swaps`) with the base cubemap, which already arrives
  6-layer today under `Paths` - so the bcs observer skips setting the `Cube`
  view and the editor skybox may already be missing its view. Pre-existing (not
  regressed by this change, since that cubemap was already meta-applied). Filed
  as a follow-up investigation task rather than a finding on this branch.
