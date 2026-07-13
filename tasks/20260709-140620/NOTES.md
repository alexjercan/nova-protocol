# Center of mass after section destruction: physics was right, the camera lied

Task: `tasks/20260709-140620`. Play report: a ship that loses sections keeps
handling "like the full ship", and when tumbling it visibly spins around a
point outside the surviving structure - "like the COM is outside the remaining
of the spaceship".

## What the investigation found

The suspicion was stale physics. It was checked at three levels, and the
physics passed all three:

1. **avian ground truth** (`integrity/glue.rs::physics_tests::mass_properties_follow_a_despawned_section`):
   despawn one of two child section colliders directly - `ComputedMass` halves,
   `ComputedCenterOfMass` shifts onto the survivor, angular inertia shrinks.
   avian 0.7 recomputes on collider removal as promised.
2. **Real destroy pipeline** (`...::mass_properties_follow_a_section_destroyed_by_damage`):
   drive a leaf section to zero health through health -> disable -> destroy ->
   despawn (the harness needs `Assets<StandardMaterial>` + the entropy plugin
   for the debris observers) - same result.
3. **Real app** (`examples/11_com_range.rs`): a five-section ship, spun, front
   sections killed through the pipeline while tumbling - avian's COM tracked
   the attached-section centroid with zero drift the whole run.

Downstream consumers were also clean: the flight autopilot reads `ComputedMass`
and the bcs PD controller reads `ComputedAngularInertia` live, every frame.

## The actual bug

`update_chase_camera_input` (camera_controller.rs) anchored the chase camera at
`Transform.translation` - the ship root's origin. The origin is wherever the
ship's first sections were built and never moves; after those sections are
destroyed it is empty space. The body correctly spins about its shifted COM,
but the camera stays centered on the empty origin, so on screen the surviving
hull orbits a phantom point. The physics was honest; the viewpoint was not.

Fix: anchor on the live world-space center of mass (rotation * local COM +
translation - not `transform_point`, which would apply render scale that avian
ignores). The origin fallback is defensive only: every real ship root has a
`RigidBody`, which requires `ComputedCenterOfMass`; the editor preview never
matches this system at all (no `SpaceshipRootMarker`/`PlayerSpaceshipMarker`). Covered by a unit test
(`chase_anchor_tracks_the_center_of_mass`) and a headless assertion in
`11_com_range` that the camera anchor sits on the live COM after two sections
die.

## Deliberately not changed (open feel question)

Even with correct physics, a stripped ship does not *feel* lighter in rotation:
the bcs PD controller scales its torque by the principal inertia, so attitude
response is inertia-independent below the `max_torque` clamp. Above the clamp
the difference exists but is brief: for the game's 3-section ship (transverse
inertia ~2.3, controller max_torque 100) a full flip saturates at ~43 rad/s^2
vs ~120 rad/s^2 for a lone remnant section - a 180-degree flip in ~0.52s vs
~0.32s; the 11_com_range 5-section line ship is ~9 vs ~40 rad/s^2 transverse
(~0.83s vs ~0.40s). Small corrections stay fully normalized either way. Whether the game should keep this fly-by-wire
normalization or move to a hardware-torque model (heavy ships lumber, stripped
ships snap) is a design decision for the user; it belongs to the flight-feel
retune (task 20260709-095043) and, if the model changes, a bevy-common-systems
task in its own repo.

Also observed and left alone: disabled-but-attached (non-leaf) sections keep
their collider and density - dead weight still bolted on, physically sensible;
and thrust-to-mass ratios stay deceptively similar after losses because losing
sections usually loses thrusters too.

## Difficulties

- The first version of the real-pipeline test killed a section with overkill
  damage (1000 on a 100 hp section) and the whole SHIP died: `HealthApplyDamage`
  propagates through `ChildOf`, so the full overkill amount also hit the root's
  aggregate health. The test now deals exact-health damage; the overkill
  propagation itself is filed as a follow-up task - a single big hit to one
  section can currently kill a ship with plenty of healthy sections.

## Self-reflection

- The three-level verification (avian harness, real pipeline headless, real
  app scripted) localized the bug by elimination: each PASS narrowed where the
  lie could live until only the camera was left. Worth repeating for any
  "physics feels wrong" report: instrument reality before touching physics.
- The user's clarification ("spins around a non-existing point") was worth far
  more than the initial theory (stale mass caches); asking what they actually
  see, in their words, redirected the investigation from flight-model tuning
  to a one-line-of-truth camera bug.
