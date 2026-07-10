# Review: Replace SOI shell with a velocity-sphere-style gravity indicator

- TASK: 20260710-201514
- BRANCH: gravity-indicator

## Round 1

- VERDICT: APPROVE

Verified sound: Visibility toggle semantics (only the gravity feeder writes
the root Visibility; no other writer in the hud modules), query aliasing in
both new/changed systems (all reads plus disjoint mutables), GravitySettings
init coverage (VelocityHudPlugin::build init_resource covers 05_directional,
which adds the plugin without the gravity plugin), the dead-well path (the
On<Remove, GravityWell> observer strips DominantWell, so the dangling window
is one flush and source_vector hides the widget), well_accel argument order
at the shader call site, shell removal completeness in code (zero grep hits
for SoiShellRing/sync_soi_shell/SHELL_), remove_hud_velocity despawning both
widget variants by target, and test honesty (assertions match the feeder
byte for byte; the shader arm is covered only via the pure magnitude helper,
matching the task's stated scope).

- [x] R1.1 (MINOR) crates/nova_gameplay/src/hud/velocity.rs (velocity_hud) -
  the gravity widget spawns `Visibility::Visible` and only hides on the
  feeder's first run, so a ship spawned in flat space flashes the yellow
  sphere for at least one frame (longer if bcs inserts
  DirectionalSphereOrbitInput a frame late, since the feeder query requires
  it). Pick the initial root visibility from the source: Gravity spawns
  Hidden, making "hidden until proven in a well" explicit.
  - Response: fixed - velocity_hud now matches on the source for the initial
    Visibility (Gravity -> Hidden); the gravity test asserts the spawn state.
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/holo_instruments.rs:52,60 -
  the HoloAssets doc comment still lists "the shell" among the holo
  elements, and the gate_mesh doc says "unlike the shell's"; both reference
  the feature this branch removed.
  - Response: fixed - both comments updated.
- [x] R1.3 (NIT) crates/nova_gameplay/src/hud/velocity.rs
  (update_velocity_hud_input) - the missing-velocity error message says
  "not found in q_target" but the parameter was renamed q_velocity.
  - Response: fixed - message says q_velocity.

Notes without action: the sphere observer errors on a bare marker while the
indicator observer falls back to the default palette - behaviorally fine
(the bundle always carries the palette), unify if the module is touched
again. Whether the 5.6/5.0 nesting of the two spheres reads well is playtest
territory.
