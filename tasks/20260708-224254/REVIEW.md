# Review: Robust SFX/juice listener: dedicated camera marker, not first Camera3d

- TASK: 20260708-224254
- BRANCH: feature/sfx-listener-marker (local branch by user request)

## Round 1

- VERDICT: APPROVE

Verified against the spec with fresh eyes:

- All four audio observer `q_camera` params plus `listener_position` in
  `audio.rs` are marker-scoped; no `With<Camera3d>` listener query remains in
  either module (grep confirms only doc/test mentions are left).
- All juice call sites from the F2 scope are converted: `listener_position`,
  `ensure_camera_shake` (now cannot attach a shake to the editor camera),
  the facing query in `draw_juice_flashes`, and `emit_juice`'s
  `CameraShakeInput` sink plus both observers' params.
- The graceful fallback is intact: `listener.map_or(1.0, ...)` in both
  `play_positional` and `emit_juice`, pinned by the audio test's
  "no marked listener -> None" assertion.
- The marker is spawned exactly where it should be: scenario camera
  (scenario-scoped, so teardown removes it) and the self-spawned cameras of
  examples 02/04 which run the full game plugins. The editor camera is
  deliberately unmarked and the reasoning is documented on the marker.
- Step 1 (latent vs live) was actually checked, not assumed: the editor
  camera is `DespawnOnExit(Editor)` and the claim in TASK.md matches the
  code paths in `nova_editor/src/lib.rs` and `loader.rs`.
- Tests are behavioral, not smoke: trauma-only-to-marked-sink,
  attenuation-from-marked-camera-with-a-nearer-unmarked-Camera3d (the exact
  failure mode from the finding), shake-attach scoping, and the audio
  listener test. Existing juice tests were updated to the marked shape,
  which now mirrors production (the shake input only exists on the marked
  camera), not weakened.
- Checks: workspace check green, nova_gameplay lib tests 276/276, headless
  10_gameplay autopilot reaches Playing and completes with no panic. Clippy
  and the full workspace suite are deferred to CI per standing instruction.

Findings:

- [x] R1.1 (NIT) crates/nova_gameplay/src/audio.rs:227 - `SfxListenerMarker`
  derives `Reflect` and is registered, but without `#[reflect(Component)]`
  the type registration is of little use to reflection-based tooling (the
  inspector will not list it as a component). Add `#[reflect(Component)]`.
  - Response: added `#[reflect(Component)]` to the marker; tests re-run green.
- [x] R1.2 (NIT) crates/nova_gameplay/src/audio.rs:227 - the `Default` derive
  on the marker is unused and inconsistent with sibling markers
  (`ScenarioCameraMarker` derives `Component, Debug, Clone`). Drop `Default`.
  - Response: dropped the `Default` derive in the same change.

Note (not a code finding): commit dfd4b38 ("docs: defer bug to later",
re-prioritizing task 20260709-125640) is a user-authored change riding on
this branch; it should be landed on master as its own commit at merge time
rather than folded into this task's squash commit.
