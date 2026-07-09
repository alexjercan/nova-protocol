# Center of mass does not shift when sections are destroyed

- STATUS: OPEN
- PRIORITY: 85
- TAGS: v0.4.0,bug,physics,handling

Reported in play (2026-07-09): when a ship loses sections, its center of mass
feels like it stays where it was with the full ship. Losing everything except a
hull + thruster should handle like a small hull+thruster, but it still flies as
if all the sections were attached.

Likely mechanics to investigate: avian computes `ComputedCenterOfMass` /
`ComputedMass` from the body's colliders, so if destroyed sections' colliders
(and `ColliderDensity`) are removed or the entities despawned, avian should
recompute. Suspects:

- Sections may be disabled (`SectionInactiveMarker` / `IntegrityDisabledMarker`)
  but keep their collider + density, so mass properties never change.
- Something may cache mass/inertia at spawn (PD controller tuning, thruster
  force application, flight/autopilot planner constants) and never refresh when
  the body's `ComputedMass`/`ComputedCenterOfMass`/inertia change.
- The multi-thruster planner and PD wobble tuning (see flight overhaul, #55) may
  assume launch-time handling stats.

## Steps

Investigation (plan phase, 2026-07-09) found: leaf-destroyed sections ARE fully
despawned (explode.rs `despawn_destroyed_without_mesh`); avian 0.7 recomputes
mass properties on collider add/change and claims removal handling (verify!);
the autopilot reads `ComputedMass` and the PD controller reads
`ComputedAngularInertia` live every frame - no caching anywhere. Two suspects
remain:

(A) the avian recompute-on-despawn trigger not actually firing for child
    collider removal, leaving ComputedMass/COM/inertia stale; and
(B) even with correct physics, rotation feel cannot change: bcs
    `pd_controller.rs` scales torque by `inertia_principal` (angular response =
    gain * error, inertia-independent below the `max_torque` clamp), so a
    stripped ship turns exactly like the full build by design.

- [ ] Ground truth for (A), physics-level test: dynamic body + two child
      colliders with density; despawn one; assert `ComputedMass` halves,
      `ComputedCenterOfMass` shifts toward the survivor, and angular inertia
      shrinks. Model on `integrity/glue.rs::physics_tests` (despawn the entity
      directly; do NOT route through the explode observer - it needs render
      resources headless).
- [ ] Instrumented reproduce for (A) through the REAL path: drive a ship's
      sections to zero health via the integrity pipeline in a headless-capable
      example/test, log mass/COM/inertia before and after the destroy cascade,
      confirm live values match the survivors.
- [ ] If (A) is broken: fix in nova's destroy path (force avian's
      `RecomputeMassProperties` or explicitly remove collider components on
      destroy); re-run both checks above.
- [ ] With (A) evidence in hand, put the (B) feel fork to the user
      (AskUserQuestion): keep inertia-normalized attitude control (identical
      turn feel regardless of size, current design) vs hardware-torque model
      (drop the inertia scaling so max_torque is the real actuator limit -
      light ships snap, heavy ships lumber). A hardware-torque change lives in
      bevy-common-systems -> its own task + flow there per AGENTS.md, then a
      rev bump here.
- [ ] Apply what the user picks (nova-side wiring/tuning; bcs work tracked in
      its own repo if chosen).
- [ ] Feel check in an example range (kill sections, confirm handling reflects
      the survivors) and document the outcome in docs/.

## Notes

- Source: user play report. Wording: "I lose all sections except a hull with a
  thruster and it doesn't feel like a small hull+thruster, but somehow like it's
  still having all the sections attached."
- Key files: bcs `src/physics/pd_controller.rs` (torque scaled by
  `inertia_principal`, then clamped by max_torque), avian 0.7
  `dynamics/rigid_body/mass_properties/mod.rs` (recompute triggers),
  nova `integrity/explode.rs` (despawn on destroy), `integrity/glue.rs`
  (disable keeps collider+density), `flight.rs:430-482` (live ComputedMass).
- Disabled-but-attached (non-leaf) wreckage keeps its collider + density and
  so keeps contributing mass until it becomes a leaf and explodes
  (20260706-174738 fixed the cascade). Treated as intended dead-weight unless
  the user says otherwise.
- Thrust is applied as fixed-magnitude impulses; linear accel scales with 1/m
  automatically once mass is right. Losing thrusters also lowers thrust, so
  thrust-to-mass can stay deceptively similar on stripped ships.
