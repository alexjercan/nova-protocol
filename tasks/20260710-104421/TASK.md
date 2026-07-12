# Target inset view: render-to-texture close-up panel of the locked ship

- STATUS: CLOSED
- PRIORITY: 8
- TAGS: v0.5.0, hud, targeting, camera, spike

## Outcome (CLOSED 2026-07-12)

Shipped Option A phase 1. New `crates/nova_gameplay/src/hud/target_inset.rs`
(`TargetInsetHudPlugin`): a focus-scoped RTT camera + corner `ImageNode` panel
showing a live scope close-up of the locked ship, and an in-scene emissive
shell on the fine-locked section. Wired into `hud/mod.rs` (panel/assets born
with the player HUD; camera born with the focus dwell). Consumes existing
targeting state only.

Verification:
- `cargo test -p nova_gameplay target_inset` - 6 headless tests pass (camera +
  panel focus-gated lifecycle, no duplication, cleared on lock change;
  highlight follows the selection, no duplication, clears on section death).
- `BCS_AUTOPILOT=1 cargo run --example 12_hud_range --features debug` - the
  scripted run asserts one inset camera + visible panel while focused and zero
  + hidden after the target dies; PASS, clean exit. `10_gameplay` autopilot
  still passes (no regression). `cargo fmt --check` and `cargo check
  --workspace` (non-debug) green. Per repo policy the full test suite / clippy
  run in CI, not locally.
- Live capture (`NOVA_INSET_SHOT=1`): top-right panel shows the ship close-up
  with thruster glow; main scene intact; egui stays on the main window.

Adjacent fix (user request, this branch): the debug inspector egui was
rendering into the RTT camera because bcs's `InspectorDebugPlugin` assigns its
context to the FIRST camera added. Fixed locally in `nova_debug`
(`keep_inspector_on_window_camera`) - pins the primary egui context to a
window camera and off any Image-target camera, order-independent. Root fix
(bcs `on_add_camera`) left as an optional upstream follow-up (pinned dep).

Difficulties / notes:
- Plan mis-stated the 0.19 RTT API as `Camera { target }`; it is a standalone
  `RenderTarget` component. Caught by grounding against the engine's own
  render_to_texture example before writing code.
- The probe paid off beyond the blackout answer: it surfaced the egui bleed and
  the `BCS_SHOT` black-capture timing gotcha (force-advances before assets
  load; inject a `Screenshot` from the settled autopilot instead).
- Worktree build reused the main checkout's `target/` via `CARGO_TARGET_DIR`
  to avoid a from-scratch Bevy rebuild.
- Deferred: phase 2 (click-picking in the inset); WASM/WebGL2 perf unmeasured.

Full design record: docs/2026-07-12-target-inset-view.md.

## Goal

A minimap-style corner panel showing a live, magnified 3D view of the
currently focused/locked enemy ship, rendered with a second `Camera3d` to a
`RenderTarget::Image` and shown in a bevy_ui `ImageNode`. It lets the player
see what the component fine-lock is selecting (and watch the section take
damage / explode, scope-style) instead of squinting at sub-pixel markers at
range. Phase 1 is view-only: the inset plus an in-scene emissive highlight of
the fine-locked section. Selection stays on the existing snap/cycle mechanic;
direct click-picking in the inset is deferred (phase 2, separate decision).

Scoped from spike docs/spikes/20260710-104011-target-inset-view.md (Option A,
RECOMMENDED). Consumes existing targeting state only; no new mechanics.

## Steps

- [x] **Probe RTT first (de-risk, must pass before anything else).** Wire the
      minimal inset path in example `examples/12_hud_range.rs` (it already
      stages a player + a target ship dead ahead that auto-locks and completes
      the focus dwell): create an `Image` render target (~512x512, usages
      `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_DST`, default sRGB render
      format), spawn a second `Camera3d` with
      `Camera { target: image_handle.clone().into(), order: -1, ..default() }`
      (`RenderTarget: From<Handle<Image>>` confirmed in bevy_camera 0.19
      camera.rs:1022) posed to look at the target ship, and display the image
      in a corner `ImageNode`. Run
      `BCS_AUTOPILOT=1 cargo run --example 12_hud_range --features debug` under
      a display (Xvfb). CONFIRM: (a) the main 3D scene still renders - NO 0.19
      blackout; (b) the inset shows the scene; (c) it coexists with the main
      camera's `PostProcessingCamera` (per-camera Tonemapping+Bloom, not a
      global blit - bcs camera/post.rs) and `SkyboxConfig` (per-camera Skybox -
      bcs camera/skybox.rs). Decide and record here the working config: image
      format/usages, camera `order`, and whether the inset camera itself
      carries `PostProcessingCamera`/`SkyboxConfig` (a bloomy/skyboxed inset vs
      a plain one is a look choice). If the scene blacks out and cannot be
      resolved, STOP: fall back to schematic Option B in the spike, update the
      spike doc + this task, and re-plan.
- [x] Create `crates/nova_gameplay/src/hud/target_inset.rs`. Define the render
      target resource (`TargetInsetRenderTarget(Handle<Image>)`, created in the
      plugin's `build` via `Assets<Image>`), the panel marker
      (`TargetInsetHudMarker`), the inset camera marker
      (`TargetInsetCameraMarker`), a `target_inset_hud()` bundle (a corner-
      anchored `Node` holding an `ImageNode` of the render target + a thin
      border; `HudTier::Chrome` + `HudSelfDrivenVisibility` since its
      visibility is focus-driven), and the `TargetInsetHudPlugin`. Pick the
      corner as a `const` (default top-left to avoid the right-side objectives
      column and the centered instruments); tune in verify.
- [x] Register the module in `crates/nova_gameplay/src/hud/mod.rs`: add
      `pub mod target_inset;`, re-export its prelude, `add_plugins(
      target_inset::TargetInsetHudPlugin)`, and add
      `setup_hud_target_inset` / `remove_hud_target_inset` observers on
      `Add`/`Remove` `PlayerSpaceshipMarker` that spawn/despawn the panel
      (mirror `setup_hud_component_lock`). Panel spawns Hidden.
- [x] Reconcile the inset camera with focus (Update system in `NovaHudSystems`,
      `target_inset.rs`): compute `focused = lock.is_some() &&
      focus.focused_on(lock)` from `SpaceshipPlayerTargetLock` +
      `SpaceshipPlayerLockFocus`. Spawn the RTT camera (config from the probe,
      targeting `TargetInsetRenderTarget`) when focused and none exists;
      despawn it when unfocused; drive the panel `Visibility` to `Visible`
      while focused and `Hidden` otherwise (the `HudSelfDrivenVisibility`
      pattern, so `apply_hud_visibility` still tier-hides it). Idempotent
      reconcile, like `sync_component_markers`.
- [x] Pose + frame the inset camera each frame while focused (system in
      `target_inset.rs`; run late enough to see the ship's moved pose - align
      with the screen-indicator projection slot, PostUpdate after the chase
      camera move, or Update if adequate - confirm against
      `hud/mod.rs` ordering notes). Aim at `live_structure_anchor(target)`
      (sections/mod.rs) from a player-relative bearing: place the camera on the
      line from the target anchor toward the player ship's anchor, pulled back
      by a distance derived from the target's section extent so the whole hull
      frames (scope-like: shows the face the player is shooting). Confirm the
      framing distance source: union of the locked ship's section
      `GlobalTransform` translations padded by the uniform section half-extent
      (every section is `Collider::cuboid(1,1,1)`, base_section.rs:65). Keep the
      pull-back factor and any vertical offset as named `const`s (feel knobs).
- [x] In-scene emissive highlight of the fine-locked section (system in
      `target_inset.rs` or a small sibling, reconciled on
      `SpaceshipPlayerComponentLock.section`): add a highlight child to the
      selected section - an unlit emissive cuboid slightly larger than the
      section's uniform `Collider::cuboid(1,1,1)` box (a shell/outline that
      reads in BOTH the main view and the inset with no projection code),
      removed when the selection moves or the section dies/detaches. Prefer a
      dedicated overlay child over mutating the section's material: section
      render children (`SectionRenderOf`) use heterogeneous materials
      (`StandardMaterial`, `ExtendedMaterial` for thrusters), so a uniform
      overlay is simpler and reverts cleanly. Tint = the hot-metal lock red
      already used by the markers (`hud/component_lock.rs` `MARKER_SELECTED_*`).
- [x] Headless unit tests in `target_inset.rs` (RunSystemOnce pattern, like
      component_lock.rs tests): the inset camera + visible panel exist ONLY
      while focused on the current lock; losing focus (or changing lock before
      dwell) despawns the camera and hides the panel; the highlight overlay
      follows `SpaceshipPlayerComponentLock.section` and reverts when it moves;
      a section death/detach clears its highlight. Each assertion must be able
      to fail (delivery-guarded: assert the positive state before removing the
      condition).
- [x] Live verify in `examples/12_hud_range.rs`: keep the probe wiring as the
      real inset (or replace it with the shipped `TargetInsetHudPlugin` path),
      and extend the scripted autopilot assertions so that after the focus
      dwell completes (~+2s) the inset camera exists, the panel is visible, and
      the main scene is NOT blacked out. Run under Xvfb; capture a screenshot
      for the docs note if cheap.
- [x] Document in `docs/`: a dated decision note (RTT config that worked on
      0.19 + this post/skybox stack, camera-pose choice, highlight approach,
      perf notes), update the spike's "Open questions" with the RTT-coexistence
      answer, and add a CHANGELOG.md entry.

## Probe result (step 1) - RTT coexistence CONFIRMED

Ran an env-gated probe in `examples/12_hud_range.rs` (`NOVA_INSET_PROBE`): a
second `Camera3d` with the standalone `RenderTarget::Image` component
(`order: -1`), carrying its OWN `PostProcessingCamera` + `SkyboxConfig` (the
hardest coexistence case), rendering the live scene into a 512x512 `Image`
(`Image::new_target_texture(512, 512, Rgba8Unorm, Some(Rgba8UnormSrgb))`),
shown in a corner `ImageNode`.

Findings (NVIDIA RTX 3060 Ti, Vulkan; `BCS_AUTOPILOT`):
- NO blackout. The main 3D scene renders fully with the RTT camera present
  (confirmed by a real captured frame - inset_probe.png during the run).
- NO crash / no panic. The skybox observer's one-shot cubemap reinterpret
  guard means a second `SkyboxConfig` insert does not double-reinterpret; the
  post observer just adds Tonemapping+Bloom per-camera.
- Main camera UNAFFECTED: every scripted assertion still passes, reticle and
  turret-pip projection drift stays 0.0 px through the `ScreenIndicatorCamera`.
- API note (0.19): `RenderTarget` is a STANDALONE component, not
  `Camera { target }`. This is the bevy 3d/render_to_texture pattern.
- Why it is safe here: post-processing (bcs camera/post.rs) and skybox (bcs
  camera/skybox.rs) are per-camera components, not a global blit; and every
  camera query in the codebase is marker-filtered (SpaceshipCameraController,
  ScenarioCameraMarker, WASDCameraController, ScreenIndicatorCamera), so an
  unmarked second camera trips no `Single<Camera>`. The known 0.19 blackout is
  specific to a second WINDOW-targeting camera; RTT is a different target.
- Decision: the inset camera WILL carry a matching look treatment; keep the
  probe's Rgba8Unorm/Rgba8UnormSrgb target, `order: -1`. Skybox on the inset
  is optional (a scope of a ship rarely needs a full skybox); default to
  giving it the skybox so background reads, revisit in verify.
- Verification-harness finding: `BCS_SHOT` captures a black frame here because
  it force-advances to Playing and captures 30 frames later, BEFORE async
  asset loading has a scene to render. Injecting a `Screenshot::primary_window`
  into the world from the autopilot script AT A SETTLED MOMENT (~+2.3 s)
  captures a real frame - this is the reliable headless visual-verify path.
- Option B (schematic) NOT needed; proceeding with Option A.

## Notes

- Relevant files:
  - Targeting state (consume, do not modify): `input/targeting.rs` -
    `SpaceshipPlayerTargetLock`, `SpaceshipPlayerLockFocus` (`.focused_on`,
    `.fraction`), `SpaceshipPlayerComponentLock` (`.section`).
  - HUD lifecycle + tiers: `hud/mod.rs` (observer setup/remove on
    `PlayerSpaceshipMarker`, `HudTier`, `HudSelfDrivenVisibility`,
    `apply_hud_visibility`, `NovaHudSystems`).
  - Reconcile/highlight prior art: `hud/component_lock.rs`
    (`sync_component_markers`, `highlight_selected_marker`).
  - Anchor + sections: `sections/mod.rs::live_structure_anchor`,
    `sections/base_section.rs` (`SectionMarker`, `SectionRenderOf`,
    `Collider::cuboid(1,1,1)`).
  - Main camera: `nova_scenario/src/loader.rs` on_load_scenario spawns the one
    `Camera3d` with `PostProcessingCamera` + `SkyboxConfig` (order 0). Inset
    camera renders the same scene from a second pose; NOT scenario-scoped -
    gameplay owns its lifecycle (spawn/despawn with focus, despawn with
    player).
  - RTT API: bevy_camera 0.19 `camera.rs` - `RenderTarget::Image(
    ImageRenderTarget { handle, scale_factor: 1.0 })`, `From<Handle<Image>>`.
  - Verify harness: `examples/12_hud_range.rs` + `BCS_AUTOPILOT` autopilot;
    `tests/examples_smoke.rs` runs harnessed examples headless.

- Decisions baked in (feel knobs, keep as `const`s, tune in verify):
  - Camera pose: player-relative bearing (scope-like), NOT fixed local offset
    or orbit. Spike left this open; picked at plan time per the task note.
  - Panel: square corner inset, default top-left, ~256-320 px on screen with a
    render image ~512 px.
  - Highlight: dedicated emissive overlay child, not material mutation.
  - The on-ship component markers (`hud/component_lock.rs`) COEXIST with the
    inset for now (both are Chrome tier); revisit only if they fight visually.

- Open / verify-first (do not bake a guess):
  - RTT + this post-processing/skybox stack on 0.19 and WASM: the probe step
    answers it. Blackout risk is the whole reason the probe is step 1.
  - Whether the inset camera carries `PostProcessingCamera`/`SkyboxConfig`
    (bloom needs an HDR-capable target to glow; decide in the probe).
  - Exact framing distance formula (section-extent union): confirm against the
    real section transforms in the probe/verify.
  - Perf: renders the scene a second time. Mitigated by spawning only while
    focused and a small texture; note WASM cost in the verify.

- Depends on: none (all consumed state already shipped).
- Phase 2 (direct click-picking via cursor-release + UV->ray) is explicitly
  out of scope; do not start it until phase 1 proves the inset earns its
  screen space.
