# Robust SFX/juice listener: dedicated camera marker, not first Camera3d

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.4.0,audio,juice,refactor

## Goal

Finding F1 from the PR #53 review (`docs/reviews/2026-07-08-pr53-audio-sfx.md`):
the distance-attenuation listener is `q_camera.iter().next()` over
`Query<&GlobalTransform, With<Camera3d>>`, which assumes exactly one `Camera3d`.
It holds today, but ECS query order is not guaranteed, so if a second `Camera3d`
ever coexists (a minimap / render-to-texture / picture-in-picture camera, or an
editor camera left alive when the scenario camera spawns) the listener would flip
between cameras frame to frame and the SFX attenuation would jitter. Make the
listener explicit and stable.

Scope extended by the PR #54 review (F2,
`docs/reviews/2026-07-09-pr54-combat-juice.md`): `juice.rs` added more call
sites with the same "first/any Camera3d" assumption - its own
`listener_position`, `ensure_camera_shake` (attaches `CameraShake` to *any*
`Camera3d`, including the editor camera; not state-gated), the ring-facing
camera in `draw_juice_flashes`, and `emit_juice` broadcasting trauma to every
`CameraShakeInput`. The same marker should scope all of them, so shake and
flash attenuation cannot diverge from the audio listener.

## Steps

- [ ] First confirm the concrete risk: check whether the editor -> scenario
      transition (`nova_editor/src/lib.rs`, `nova_scenario/src/loader.rs:167`)
      ever leaves two `Camera3d` alive at once. If it does, this is a live bug,
      not just latent - note that in the fix.
- [ ] Introduce a dedicated audio-listener signal rather than "first Camera3d".
      Prefer reusing/marking the gameplay camera: add a small marker (e.g.
      `SfxListener`, or reuse an existing main-camera marker if one exists) on the
      camera spawned for gameplay, and query
      `Single<&GlobalTransform, With<SfxListener>>` (or a `.iter().next()` over the
      marked set).
- [ ] Update `listener_position` and the four observer `q_camera` params in
      `crates/nova_gameplay/src/audio.rs` to use the marked listener.
- [ ] Update the juice call sites in `crates/nova_gameplay/src/juice.rs` to the
      same marker: `listener_position`, `ensure_camera_shake` (attach shake only
      to the marked gameplay camera, not any `Camera3d`), the `q_camera` facing
      query in `draw_juice_flashes`, and scope `emit_juice`'s
      `Query<&mut CameraShakeInput>` to the marked camera.
- [ ] Keep the graceful fallback: no listener yet -> full base volume / full
      juice strength (not silence), as today.
- [ ] Verify: fmt, clippy --all-targets, cargo test --workspace, headless
      `10_gameplay` autopilot (Playing, no panic). Shared CARGO_TARGET_DIR.

## Notes

- Source: PR #53 review F1; scope extended by PR #54 review F2. Depends on:
  20260708-162011 / -213155 / -162013 (CLOSED).
- Latent today (one gameplay camera; `PostProcessingDefaultPlugin` adds a
  *component*, not a second camera). Low priority unless step 1 finds the editor
  transition already spawns a second `Camera3d`.
- The listener is deliberately the camera (what the player hears from), not the
  player ship; keep that.
