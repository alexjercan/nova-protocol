# Center of mass does not shift when sections are destroyed

- STATUS: CLOSED
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
despawned; avian recomputes mass properties on collider changes; all flight
systems read live values. During the cycle the user clarified the observable
symptom: a tumbling ship visibly spins around a point OUTSIDE the surviving
structure. Steps as executed:

- [x] Ground truth physics test: despawn one of two child section colliders
      directly; `ComputedMass` halves, `ComputedCenterOfMass` shifts onto the
      survivor, angular inertia shrinks (`mass_properties_follow_a_despawned_section`,
      integrity/glue.rs). PASS - avian recomputes on collider removal.
- [x] Real-pipeline physics test: drive a leaf section to zero health through
      health -> disable -> destroy -> despawn and assert the same
      (`mass_properties_follow_a_section_destroyed_by_damage`). PASS, after two
      harness discoveries: the destroy observers need `Assets<StandardMaterial>`
      + the entropy plugin headless, and overkill damage propagates through
      `ChildOf` and kills the whole ship (follow-up task filed).
- [x] Real-app reproduce: new `examples/11_com_range.rs` - five-section line
      ship, COM / attached-centroid / spawn-COM gizmos, O = spin and K = kill
      hotkeys, plus a scripted headless run (spin, kill two front sections,
      assert live COM == attached centroid and COM moved aft). PASS with zero
      drift: the physics is correct in the real app too.
- [x] Root cause of the phantom pivot: `update_chase_camera_input`
      (camera_controller.rs) anchored the chase camera at the ship root's
      ORIGIN - the (dead) front sections' build position, i.e. empty space
      after they are destroyed - while the body correctly spins about its
      shifted COM. Fixed: anchor at the live world COM (rotation * local COM +
      translation; scale-proof), with a defensive origin fallback (every real
      ship root has a RigidBody and therefore the COM component; the editor
      preview never matches this system).
- [x] Cover the fix: unit test `chase_anchor_tracks_the_center_of_mass`
      (camera_controller.rs) plus a camera-anchor-on-COM assertion in the
      `11_com_range` headless script. Regression: `06_torpedo_range` smoke
      still green.
- [x] Document in `docs/2026-07-09-com-section-destroy.md`; CHANGELOG entries
      under Fixed and Added.
- [ ] Deferred, needs a user decision: the handling-feel fork. Even with
      correct physics, rotation cannot feel lighter - the bcs PD controller
      normalizes torque by inertia (and above its clamp both full and stripped
      ships flip faster than perception). Options (perceivable torque budget /
      fully physical PD / keep fly-by-wire) documented in the doc; belongs to
      the flight-feel retune 20260709-095043 if pursued.

## Resolution

The physics was never broken: mass, center of mass and angular inertia follow
destroyed sections at every level (avian ground truth, the real destroy
pipeline, and the running game - all covered by new tests/assertions). The
reported "spins around a non-existing point" was the chase camera: it anchored
at the ship root's origin, which after losing the front sections is empty
space, so the correctly-pivoting hull orbited the anchor on screen. The camera
now anchors on the live center of mass.

What shipped: 2 physics-level tests (integrity/glue.rs), the camera anchor fix
+ unit test (camera_controller.rs), `examples/11_com_range.rs` (interactive
gizmo range + headless smoke with COM and camera assertions), docs and
CHANGELOG. Honest test scope: the 4 new tests were run locally and pass; the
11 and 06 headless smokes are green; cargo check --workspace and fmt green;
the full suite/clippy deferred per AGENTS.md (see task 20260709-140816 on the
CI gap).

Discovered along the way, filed separately: overkill `HealthApplyDamage`
propagates its full amount through `ChildOf` to the root aggregate, so one big
hit to a single section can kill an otherwise healthy ship.

Reflection: three-level elimination (harness -> pipeline -> real app) found the
lie by exonerating everything else; and the user's one-sentence clarification
of what they actually SEE was the pivot (sic) of the whole investigation -
ask for the observable, not the theory.

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
