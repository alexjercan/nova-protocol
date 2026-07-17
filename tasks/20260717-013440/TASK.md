# cubemap_alt.png.meta not in meta_check Paths (broadside skybox may miss its cube layout)

- STATUS: CLOSED
- PRIORITY: 25
- TAGS: v0.7.0,assets,bug

## Context (surfaced during the base-art migration review, 2026-07-17)

`crates/nova_core/src/lib.rs` `assets_plugin()` uses `AssetMetaCheck::Paths(...)`
listing ONLY `base/textures/cubemap.png` (was `textures/cubemap.png` before the
Option A move, task 20260717-002105). But `base/textures/cubemap_alt.png` ALSO
has a `.meta` sidecar with `array_layout: Some(RowCount(rows: 6))` - the 6-face
cube-skybox reinterpret - and cubemap_alt IS a live skybox (broadside scenario,
and the gauntlet/ledger mods via `dep://base/textures/cubemap_alt.png`). Under
`AssetMetaCheck::Paths`, a path NOT in the set has its `.meta` ignored, so
cubemap_alt would load as a flat 2D image, not a cube.

This is PRE-EXISTING (master listed only `cubemap.png` before the move too), not a
migration regression - but the migration is when it surfaced, and cubemap_alt now
has more consumers (the mods that dep://base it).

## Goal

Decide whether cubemap_alt needs its `.meta` honored (does the broadside/gauntlet
skybox currently render as a cube or a flat image?), and if so add
`base/textures/cubemap_alt.png` to the `meta_check` Paths set. Verify visually
(the skybox actually reads as a cube) - no test currently covers skybox visual
correctness, so a manual run or a screenshot check is needed.

## Notes

- File: crates/nova_core/src/lib.rs (`assets_plugin`, the `AssetMetaCheck::Paths`
  set).
- If cubemap_alt has been rendering flat all along, this is a latent visual bug;
  if the skybox system applies the cube layout another way, this may be a no-op -
  investigate before changing (changing meta_check alters rendering).

## Investigation (2026-07-17, this branch)

The "renders flat" premise is FALSE in the normal path, but the meta still must
be honored. Mechanism, from source:

- The bundle manifest's `resources` list is validation data only
  (`nova_modding::BundleAsset.resources` - carried strings, never loaded), so
  cubemap_alt loads LAZILY: `on_load_scenario` (loader.rs) spawns the scenario
  camera with `PendingSkyboxSwap` and `resolve()` kicks off the load.
- Loaded assets land in `Assets<Image>` in `PreUpdate`
  (bevy_asset 0.19 `handle_internal_asset_events`); `apply_pending_skybox_swaps`
  (Update, same frame) inserts `SkyboxConfig`; the bcs `SkyboxPlugin` observer
  (bevy-common-systems v0.19.0, src/camera/skybox.rs) sees 1 layer and
  reinterprets stacked->6-layer + sets the Cube view during the command flush -
  BEFORE `PostUpdate` emits the AssetEvents that drive render extraction. The
  GPU only ever sees the 6-layer form; broadside's sky reads as a cube today.
- Residual bug (why the meta matters anyway): if the scenario tears down while
  the 4096x24576 PNG is still decoding (quit to menu, NextScenario), the image
  lands with no applier alive, and extraction uploads the RAW STACKED image -
  24576 px exceeds the 16384 max_texture_dimension_2d of llvmpipe/WebGL2-class
  GPUs: a fatal wgpu validation error. Same class as the original cubemap.png
  upload race (see `prepare_cubemap_view`'s doc in nova_assets).
- TRAP the naive fix would hit: with the meta applied, the image arrives
  already 6-layer, so the bcs observer SKIPS its fallback branch - which is
  also where the Cube texture view is set. Nothing else sets it for
  cubemap_alt (`prepare_cubemap_view` only handles `GameAssets.cubemap`), and
  bevy's skybox sanity check (`sanity_check_skybox_image_and_warn`,
  bevy_core_pipeline 0.19 skybox/mod.rs:261) refuses a non-Cube view with a
  warn_once and withholds the skybox bind group - the sky silently disappears
  (corrected in review R1.1; originally misdescribed as a fatal wgpu error).
  The fix must pair the meta_check entry with view-setting on the swap path.
- Consumers swept: broadside (base) + gauntlet/ledger webmods
  (`dep://base/...` rewrites to the same default-source path - covered);
  menu details thumbnail uses `banner.png` and has a non-2D guard (unaffected);
  editor uses `GameAssets.cubemap` (unaffected); example mod's `nebula.png`
  stays on the fallback path (same hazard class - follow-up task, not this
  diff).

## Steps

- [x] Regression test (fail-first): extend
  `crates/nova_core/tests/cubemap_meta_app_config.rs` to load
  `base/textures/cubemap_alt.png` through `nova_core::assets_plugin()` and
  assert 6 array layers; record the failing numbers pre-fix.
  FAIL-FIRST RECORDED (pre-fix tree, 2026-07-17):
  `app_asset_config_loads_cubemap_alt_as_six_layer_array` failed with
  `assertion left == right failed ... left: 1, right: 6` - the shipped config
  loads cubemap_alt as a single-layer stacked image. The sibling cubemap.png
  test stayed green (filtered out in the evidence run; green in the final
  suite run below).
- [x] Fix 1: add `base/textures/cubemap_alt.png` to the `meta_check` Paths set
  in `assets_plugin()` (crates/nova_core/src/lib.rs); update its doc comment.
  Both app-config tests green post-fix (2 passed, 0.52s).
- [x] Fix 2 + unit test (fail-first): `apply_pending_skybox_swaps`
  (crates/nova_scenario/src/actions.rs) sets the Cube texture view before
  installing `SkyboxConfig` when the image is already multi-layer and has no
  view descriptor; mutate only when needed (no gratuitous `AssetEvent::Modified`
  re-upload of a ~400MB texture - pin with a no-Modified assertion for the
  already-cubed case).
  A/B RECORDED (fix committed as d8860a82, then sabotaged with
  `if false && needs_cube_view`): `skybox_swap_sets_cube_view_on_a_preinterpreted_cubemap`
  failed `left: None, right: Some(Cube)`; restored via
  `git checkout HEAD -- crates/nova_scenario/src/actions.rs`, `git diff
  --exit-code` clean. Post-fix: 3/3 lib skybox tests green. The no-churn pin
  (`skybox_swap_does_not_remodify_an_already_cubed_image`) pins that consuming
  a swap for an already-cubed image emits no `AssetEvent::Modified` (i.e. no
  unconditional descriptor overwrite; review R1.2 corrected the earlier
  "get_mut emits Modified" wording - the AssetMut guard queues Modified only
  on an actual write).
- [x] Strengthen `crates/nova_scenario/tests/skybox_swap_e2e.rs`: assert the
  swapped image has 6 layers and a Cube view (fails pre-fix: view is None).
  Same sabotage run: e2e failed `the swapped cubemap must carry a Cube texture
  view: left: None, right: Some(Cube)`; green post-fix (1 passed, 0.80s).
- [x] Docs: CHANGELOG line (player-facing: broadside sky robust on
  16384-limit GPUs / web) added under Unreleased/Fixes; docs-sync map checked -
  no wiki page documents the meta_check set (the mod-authoring guide's `.meta`
  wording belongs to follow-up 20260717-111558).
- [x] Visual verify: run broadside and eyeball the skybox as a cube.
  RIG: `Xvfb :99` + `DISPLAY=:99 NOVA_SHOT_DIR=<tmp> BCS_AUTOPILOT=1 RUST_LOG=info
  cargo run --example 19_broadside --features debug` on the post-fix branch;
  rendered on the machine's real RTX 3060 Ti (Vulkan), NOT llvmpipe. Full
  slice green: menu backdrop swap, Broadside load (cubemap_alt via the applier,
  meta applied), defeat -> Retry (the teardown+reload path), victory; probe
  lines `defeat overlay up` / `victory overlay up` / `script complete` all
  present, zero wgpu validation errors, no single-layer canary. Review R1.1
  corrected the evidence reasoning here: a missing Cube view would NOT crash -
  bevy warn_once's ("must be TextureViewDimension::Cube") and silently skips
  rendering the skybox - so "no crash" alone proves nothing. The actual view
  evidence: that warn_once line is ABSENT from the run log (grep: 0 matches
  for "TextureViewDimension::Cube"/"texture view dimension"), and the
  brightened `broadside_victory.png` shows the sky RENDERING (directional star
  field, violet deep-field glow top-right, no stacked-strip smear) - a wrong
  view would have removed the skybox bind group and left pure black. Plus the
  e2e's explicit Cube-view assertion. The 16384-limit case needs no GPU
  witness post-fix: the image leaves the loader as 6 square 4096 layers
  (pinned by the app-config test), so no 24576-tall upload can exist.
- [x] File the follow-up tatr task: mod-shipped cubemaps (mods:// and
  `mods/example/textures/nebula.png`) still ride the fallback reinterpret
  (teardown upload race) and the bcs observer's unconditional `get_mut`
  re-uploads the cubemap on every SkyboxConfig insert - candidate upstream
  bevy-common-systems fix. FILED: tasks/20260717-111558.
- [x] fmt + check + the newly written tests (full suite stays on CI per user
  instruction), close the task. `cargo fmt` clean; `cargo check --workspace
  --all-targets` result recorded below; tests: 2/2 app-config, 3/3 lib skybox,
  1/1 e2e - all green post-fix.

## Close-out (2026-07-17, branch meta-check-cubemap-alt)

### What changed and why

- `crates/nova_core/src/lib.rs` `assets_plugin()`: `base/textures/cubemap_alt.png`
  joined the `AssetMetaCheck::Paths` set, so its `.meta` `array_layout:
  RowCount(6)` applies at LOAD time - the image never exists as a 24576-tall
  single-layer texture. Doc comment now explains the pairing with the applier.
- `crates/nova_scenario/src/actions.rs` `apply_pending_skybox_swaps`: sets the
  Cube `texture_view_descriptor` before installing `SkyboxConfig` when the
  image is already multi-layer and has no view. Without this, Fix 1 alone
  BREAKS the swap: a meta-applied cubemap skips the bcs observer's
  single-layer fallback (the only place the view was set), and bevy's skybox
  sanity check refuses the non-Cube view (warn_once) and withholds the skybox
  bind group - the sky silently disappears (R1.1 corrected the earlier "fatal
  wgpu validation error" description). The view write happens only when the
  view is missing, so no `AssetEvent::Modified` (= full texture re-upload) for
  already-cubed images.
- Tests: cubemap_alt joined the app-config meta pin (refactored to a shared
  helper, two tests); two new applier unit tests (Cube view on pre-arrayed
  images; no-Modified churn guard); the skybox e2e now asserts 6 layers +
  Cube view on the swapped image.
- CHANGELOG Fixes line; follow-up task 20260717-111558 for the mod-shipped
  cubemap class (dynamic `mods://` paths cannot join the static set; candidate
  upstream bcs fixes: view-set for pre-arrayed images, conditional get_mut).

### The answer to the task's question

The broadside sky was NOT rendering flat: the bcs SkyboxPlugin fallback
reinterprets the stacked image in the same frame it lands in `Assets<Image>`
(PreUpdate insert -> Update applier -> command-flush observer -> PostUpdate
asset events -> extraction), so the GPU normally only ever saw the 6-layer
form. The real defect was the TEARDOWN window: a scenario torn down while the
PNG decodes leaves the image with no applier alive, and the raw 24576-tall
upload is a fatal wgpu validation error on 16384-limit GPUs (llvmpipe,
WebGL2-class). Honoring the meta closes that by construction.

### Alternatives considered

- Preload cubemap_alt via `GameAssets` + `prepare_cubemap_view`: rejected -
  permanently pins a ~400MB texture that only broadside uses.
- Fix the view upstream in bcs `setup_skybox_camera`: the cleaner long-term
  home, but needs a bevy-common-systems release + workspace-wide tag bump;
  routed to 20260717-111558. The applier is the right nova-side seam - it
  already owns "make the SkyboxConfig insert safe" (the deferred-load design),
  and `prepare_cubemap_view` is the established app-side precedent.
- `AssetMetaCheck::Always`: rejected long ago for wasm 404-per-asset cost
  (documented on `assets_plugin`).

### Difficulties / bugs along the way

- The naive one-line fix (just add the path) would have shipped an INVISIBLE
  SKY: the bcs observer couples the reinterpret and the view-set in one
  single-layer-only branch, so a meta-applied image got neither, and bevy's
  skybox sanity check silently skips a non-Cube view (warn_once, no crash -
  R1.1). Caught by reading the dependency source
  (verify-engine-guarantees-in-source), not by any existing test - the e2e
  was headless (no render app), so nothing validated view dimensions.
- `Assets::get_mut` returns a change-detecting guard: the binding needs
  `let Some(mut image)`; the first evidence run failed compile on E0596.
- Hit `piped-cargo-masks-exit-code` AGAIN (ledger x2 -> x3): the first
  app-config run piped through `tail` + `echo EXIT`, so the harness reported
  exit 0 for a failed compile; the truth was only in the output text.

### Self-reflection

- The task's own premise ("may render flat") was falsified by tracing the
  schedule; the investigation should always precede the fix - the meta_check
  one-liner LOOKED trivially safe and was not. Reading the consumer's source
  (bcs skybox.rs, 40 lines) was the single highest-value step of the task.
- The fail-first discipline paid twice: the app-config test recorded the
  1-vs-6 evidence, and the sabotage A/B (`if false &&`) proved both new view
  assertions can fail - the no-churn pin passing under sabotage confirmed it
  pins the guard, not the fix.
- Should have run the first evidence command unpiped from the start; the
  masked exit code cost one round-trip (and it is a x3 ledger lesson now).
