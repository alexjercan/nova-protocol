# Target inset view: render-to-texture close-up panel of the locked ship

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.5.0,hud,targeting,camera,spike

Spike: docs/spikes/20260710-104011-target-inset-view.md

Goal: a minimap-style inset panel (corner of the screen) showing a live
magnified 3D view of the currently locked enemy ship, so the player can see
what the component fine-lock is selecting instead of cycling sub-pixel
markers at range. Phase 1 only: view-only inset + in-scene highlight of the
fine-locked section; direct click-picking in the inset is a later, separate
decision.

Direction (see spike for the full reasoning):

- Second `Camera3d` with `RenderTarget::Image` into a bevy_ui `ImageNode`
  panel. FIRST step must be a small probe that RTT coexists with
  `PostProcessingCamera`, the skybox and the WASM build on Bevy 0.19 (a
  second window-targeting camera is known to black out the scene; RTT is a
  different path but unverified here). If the probe fails, fall back to the
  schematic-panel option B in the spike and update it.
- Inset camera spawns/despawns with the focused lock (hud/mod.rs observer
  pattern), framed on `live_structure_anchor` with distance from the
  section-AABB union.
- Highlight the `SpaceshipPlayerComponentLock` section in-scene (emissive
  tint/outline) so it reads in both the main view and the inset with no
  projection code.
- Camera pose (fixed local-frame offset vs player-relative vs slow orbit)
  is an open feel knob; pick one at plan time, keep it a constant.

Notes: backlog item, explicitly not v0.4.0 (parked at p0 per the roadmap
convention, spike 20260708-203517). Consumes existing targeting state only;
no new mechanics.
