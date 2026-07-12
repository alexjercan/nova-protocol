# Target inset view (render-to-texture scope of the locked ship)

Task: 20260710-104421. Spike: docs/spikes/20260710-104011-target-inset-view.md
(Option A, RECOMMENDED). Phase 1 (view-only inset + in-scene section
highlight).

## What shipped

A corner HUD panel showing a live, magnified 3D close-up of the currently
focused/locked enemy ship, rendered by a second `Camera3d` into an `Image`
render target and displayed in a bevy_ui `ImageNode`. It lets the player see
which section the component fine-lock is selecting - and watch that section
take damage / explode, scope-style - instead of squinting at sub-pixel markers
at range.

New module `crates/nova_gameplay/src/hud/target_inset.rs`:

- `TargetInsetHudPlugin` runs two reconciling systems in `NovaHudSystems`:
  - `drive_inset_camera`: spawns/despawns the RTT camera and shows/hides the
    panel with the focus dwell (`SpaceshipPlayerLockFocus::focused_on`), and
    poses the camera each frame on the locked ship's `live_structure_anchor`
    from a scope-like player-relative bearing (camera on the line from the
    target toward the player, pulled back by the ship's framing radius).
  - `sync_section_highlight`: keeps one emissive shell overlay on the
    fine-locked section (`SpaceshipPlayerComponentLock.section`), so the
    selection reads in BOTH the main view and the inset with no projection
    code. A dedicated overlay child, not a material mutation, because section
    render children use heterogeneous materials (`StandardMaterial`,
    `ExtendedMaterial` for thrusters).
- The panel + the shared render-target image + the shared highlight mesh/material
  are created with the player HUD (`hud/mod.rs` `setup_hud_target_inset` /
  `remove_hud_target_inset` observers, mirroring the other overlays). The
  camera itself is focus-scoped, not player-scoped, so the scene is only
  rendered a second time while the player is actually scoping a lock.

It is a pure consumer of existing targeting state (input/targeting.rs); no new
targeting mechanics.

## The RTT de-risk probe (why it was step 1)

The spike flagged one real unknown: a second WINDOW-targeting camera blacks out
the 3D scene on Bevy 0.19 (see hud/screen_indicator.rs), which is why the whole
HUD is a UI pass. Render-to-texture is a different path but was unverified in
this codebase against its post-processing + skybox stack.

The probe (an env-gated second RTT camera wired into `12_hud_range`) confirmed:

- NO blackout: the main scene renders fully with the RTT camera present
  (verified from a real captured frame).
- NO crash: a second `SkyboxConfig`/`PostProcessingCamera` is safe. Both are
  PER-CAMERA components (bcs camera/post.rs adds Tonemapping+Bloom on Insert;
  camera/skybox.rs attaches a `Skybox` and guards its one-shot cubemap
  reinterpret), not a global blit, so a second camera is independent.
- Main camera UNAFFECTED: `12_hud_range`'s projection assertions (reticle,
  turret pip) still measure 0.0 px drift through the `ScreenIndicatorCamera`.
- Every camera query in the codebase is marker-filtered
  (`SpaceshipCameraController`, `ScenarioCameraMarker`, `WASDCameraController`,
  `ScreenIndicatorCamera`), so an unmarked second camera trips no
  `Single<Camera>`. The inset camera therefore carries ONLY its own
  `TargetInsetCameraMarker` (+ `PostProcessingCamera`).

Bevy 0.19 API note: `RenderTarget` is a STANDALONE component
(`RenderTarget::Image(handle.into())`), not a `Camera { target }` field - the
3d/render_to_texture example pattern. `Image::new_target_texture(w, h,
Rgba8Unorm, Some(Rgba8UnormSrgb))` sets the RTT usages.

## Decisions (feel knobs, kept as `const`s)

- Camera pose: player-relative bearing (scope-like), not a fixed local offset
  or a slow orbit. Frames the face the player is shooting.
- No skybox on the inset: a dark clear (`INSET_CLEAR_COLOR`) makes the ship
  stand out and avoids plumbing the scenario cubemap into gameplay. The inset
  DOES carry `PostProcessingCamera` so thruster glow / explosions bloom.
- Panel: top-right, 256 px on-screen over a 512 px texture. Top-right is clear
  of the objectives column (mid-right), the keybind hints (bottom-left) and
  the dev inspector overlay (top-left).
- Highlight: an unlit, translucent, emissive red shell scaled 1.14x the
  section's unit box; reverts cleanly when the selection moves and dies with
  its section.
- The on-ship component markers coexist with the inset (both Chrome tier).

## Adjacent fix: inspector egui bleeding into the RTT camera

bcs's `InspectorDebugPlugin` assigns `PrimaryEguiContext` to the FIRST camera
added ("first camera wins" observer). With a second camera that renders to an
Image, the inspector egui can land inside the inset's texture if the inset
camera's `Add` fires first. Fixed locally in `nova_debug`
(`keep_inspector_on_window_camera`): a per-frame reconcile that keeps the
primary egui context on a window-targeting camera and off any Image-target
camera, order-independent. The root fix would be in bcs's `on_add_camera`
(only assign to window cameras); left as an optional upstream follow-up since
bcs is a pinned dependency and the local reconcile fully resolves the symptom.

## Verification

- `cargo test -p nova_gameplay target_inset`: 6 headless unit tests (camera +
  panel exist only while focused; no duplication; cleared on lock change;
  highlight follows the selection, does not duplicate, and clears on section
  death).
- `BCS_AUTOPILOT=1 cargo run --example 12_hud_range --features debug`: scripted
  assertions confirm exactly one inset camera + a visible panel while focused
  (~+2 s), and zero cameras + a hidden panel after the target dies (~+4.5 s).
- Live capture: `NOVA_INSET_SHOT=1` alongside `BCS_AUTOPILOT` injects a
  `Screenshot` from the settled autopilot to `inset_shot.png` (top-right panel
  shows the ship close-up; egui stays on the main window).

### Headless screenshot gotcha

`BCS_SHOT` captures a BLACK frame in a headless run here: it force-advances to
`Playing` and captures ~30 frames later, before async asset loading has a scene
to render (the GPU is real - NVIDIA/Vulkan). Injecting a
`Screenshot::primary_window` from the autopilot script at a settled moment
(~+2.3 s) captures a real frame instead. Useful technique for any headless
visual check of a loaded scene.

## What could have gone better / notes for next time

- The plan wrote the 0.19 RTT API as `Camera { target }`; it is actually a
  standalone `RenderTarget` component. Grounding against the engine's own
  `render_to_texture` example (not memory) caught it before any code - the
  "verify-first against the system, not a model of it" lesson in action.
- The probe was worth it beyond the blackout answer: it surfaced the egui
  bleed and the `BCS_SHOT` timing gotcha, both of which would otherwise have
  shown up as confusing verification noise later.
- Perf: the inset renders the scene a second time. Mitigated by a small texture
  and focus-scoped spawning. Not yet profiled on the WASM/WebGL2 build - a
  follow-up if the inset ever feels heavy in-browser (RTT is standard on
  WebGL2, so expected to work, but unmeasured here).

## Update: ship-only scope + AABB framing (task 20260712-203345)

Playtest follow-up: the inset was scoping beacons (nav waypoints), which is not
worth a close-up. Added an opt-in `InsetZoomable` flag; the inset only scopes
flagged bodies. Authored by observers (`On<Add, SpaceshipRootMarker>`,
`On<Add, TorpedoTargetChosen>`) plus a bundle line on asteroids in
nova_scenario - so ships, committed torpedoes and asteroids are zoomable, and
beacons (which never get the flag) are skipped.

Framing was section-based (`ship_framing_radius` over `SectionMarker` children),
which does not fit section-less torpedoes/asteroids. Generalized to
`zoomable_framing_radius`: union the body's non-sensor collider AABBs
(`screen_indicator::target_world_aabb`, now `pub(crate)`) and take the
anchor-to-farthest-corner distance. Uniform across sectioned and section-less
bodies; falls back to the section half-extent when a body has no collider AABB.

The inset is `ALL`-HUD-mode only (hidden at Minimal/None) - already the case via
the Chrome tier + the `shows(HudTier::Chrome)` camera gate; pinned by a test.
