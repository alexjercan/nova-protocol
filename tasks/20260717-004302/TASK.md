# Radial lock-acquisition ring HUD (UiMaterial shader)

- STATUS: OPEN
- PRIORITY: 22
- TAGS: v0.7.0,hud,torpedo,shader

Spike: tasks/20260708-165647/SPIKE.md (weapons-HUD arc); mechanic 20260708-165703.

## Goal

The "circle loading" cue for the acquisition dwell (20260708-165703): a smooth
radial arc that fills clockwise around the pending target's reticle while the
radar dwell charges, and vanishes the instant the lock snaps (LockOn cue is the
completion signal). First real UiMaterial in nova - a small WGSL fragment with
a `progress` uniform. Wasm/WebGL2-safe (simple math, no compute).

## Steps

- [ ] Add the shader `assets/shaders/lock_dwell_ring.wgsl`: import
      `bevy_ui::ui_vertex_output::UiVertexOutput`; material uniform at
      `@group(1) @binding(0)` = `{ color: vec4<f32>, progress: f32, inner: f32,
      softness: f32 }`. In `fragment(in: UiVertexOutput)`: center UV at 0.5,
      `d = length(in.uv - 0.5) * 2.0` (0..1 to edge); `ang = fract((atan2(-(uv.x
      -0.5), (uv.y-0.5)) / TAU))` for clockwise-from-top; alpha = ring band
      (`smoothstep` around `inner`/outer with `softness`) AND `ang <= progress`;
      output `color` premultiplied by alpha. Antialias both the band and the
      leading edge with smoothstep. (View+Globals are group 0 by default; not
      needed here.)
- [ ] Define the material in a new `crates/nova_gameplay/src/hud/lock_dwell_ring.rs`:
      `#[derive(Asset, AsBindGroup, TypePath, Clone)] struct LockDwellRingMaterial
      { #[uniform(0)] color: LinearRgba, #[uniform(0)] progress: f32, ... }`
      (single uniform struct via one `#[uniform(0)]`-tagged sub-struct, matching
      the WGSL layout); `impl UiMaterial { fn fragment_shader() ->
      "shaders/lock_dwell_ring.wgsl".into() }`. Register with
      `app.add_plugins(UiMaterialPlugin::<LockDwellRingMaterial>::default())`
      in this module's plugin (mirror `VelocityHudPlugin`'s `MaterialPlugin`
      registration in hud/velocity.rs).
- [ ] Spawn/despawn the ring layer with the player ship via hud/mod.rs
      `On<Add/Remove, PlayerSpaceshipMarker>` observers (the
      `setup/remove_hud_component_lock` pattern, mod.rs ~656-686). The layer is
      a `screen_indicator_layer()` full-screen click-through container holding
      one ring node: a `Node` (fixed px, e.g. 56x56, concentric with the
      reticle) + `MaterialNode(materials.add(LockDwellRingMaterial::default()))`
      + a `ScreenIndicatorAnchor`/config so the screen_indicator widget
      projects it onto the anchored entity each frame (reuse the widget, do NOT
      re-implement world_to_viewport). Store the `Handle<LockDwellRingMaterial>`
      on a marker component for the update system.
- [ ] Anchor + fill update system (in `SpaceshipTargetingSystems`-after
      ordering, like `drive_reticle_anchor` and `update_focus_meter` in
      hud/torpedo_target.rs ~248, ~375): read the player `RadarState`; the ring
      anchors to `radar.dwell_target` (the PENDING candidate, which may differ
      from the committed lock) and is Visible only while a dwell is in progress
      (hold active, `dwell_target` Some, not yet complete); set the material's
      `progress` = dwell fraction (use the `RadarState` read helper the
      mechanic task exposes, or `dwell_secs / lock_dwell_secs(..)`). Hidden and
      progress-reset otherwise. The ring vanishing at completion is the "snap"
      moment (paired with the existing LockOn SFX).
- [ ] Palette: pick an ACQUIRING accent that does not collide with the amber
      turret lead pip, nav cyan, or the red combat / white travel crosshairs
      (an acquiring hot-white or amber-with-motion reads as "working"). Record
      the final color in the arc doc.
- [ ] Tests: ring visibility follows an in-progress dwell only (hidden with no
      hold / no candidate / after completion); `progress` uniform tracks the
      dwell fraction; the ring anchors to the pending candidate, not the
      committed lock, when they differ (mid-dwell re-designation). Prefer world
      tests that drive `RadarState` directly and assert the material handle's
      `progress` and the node `Visibility` (headless - the shader itself is not
      exercised without a GPU; note that skip).
- [ ] Example: extend an existing HUD example (examples/12_hud_range.rs, the
      lock stage) - engage the radar on a distant target, advance `Time`,
      assert the ring becomes visible with a rising `progress`, then after the
      dwell completes assert the ring is hidden and `CombatLock`/`TravelLock`
      committed. Run once under Xvfb (report the skip if the GPU path can't
      init headless - fall back to asserting state without the draw).
- [ ] Verify: cargo fmt, cargo check --workspace, new + touched hud tests, one
      scripted example run under Xvfb (report skips per repo policy).
- [ ] Write the arc doc `tasks/20260708-165703/NOTES.md` (mechanic + visual
      together): the dwell end to end, the distance formula and its constants
      with values, the stealth `modifier` seam left open, the ring palette, and
      what was deferred (aspect/stealth mechanic, any per-lens polish).

## Notes

- Depends on: 20260708-165703 (the mechanic exposes `RadarState.dwell_target`,
  `dwell_secs`, and the dwell fraction this HUD renders). Cannot land first.
- Relevant files: `crates/nova_gameplay/src/hud/velocity.rs` (Material plugin
  registration template - but that is `MaterialPlugin` for 3D; here use
  `UiMaterialPlugin`), `hud/torpedo_target.rs` (`drive_reticle_anchor` ~248 and
  `update_focus_meter` ~375 - anchor + visibility idiom, and the focus BAR this
  ring sits a stage before), `hud/component_lock.rs` + `hud/mod.rs` ~656-686
  (layer spawn/despawn observer pattern), `hud/screen_indicator.rs` (~123-180
  public config: `ScreenIndicatorAnchorKind::Entity`, `ScreenIndicatorSize`,
  offscreen `Hide`), `hud/ammo_readout.rs` ~165 (trig ring layout - the fallback
  if the shader path proves troublesome on wasm).
- Verified API (bevy 0.19, bevy_ui_render 0.19.0): `UiMaterial` trait
  (`fragment_shader() -> ShaderRef`), `MaterialNode<M>(Handle<M>)`,
  `UiMaterialPlugin<M>`. Shaders load from `assets/shaders/*.wgsl` by string
  path (existing: directional_magnitude/sphere, thruster_exhaust). No UiMaterial
  exists in nova yet - this is the first; keep the fragment trivial for the
  WebGL2 build target.
- Fallback if UiMaterial misbehaves on wasm: the `ammo_readout` trig-positioned
  segment-pip idiom (BorderRadius::MAX pips lit clockwise) renders a segmented
  ring with zero shader risk. Noted as the escape hatch, not the plan.
- The ring anchors to the PENDING candidate (`dwell_target`), so during a
  mid-gesture re-designation it can differ from the still-committed lock's
  reticle - intended (you see where the new lock is charging).
