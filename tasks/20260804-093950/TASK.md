# Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

- PRIORITY: 80
- TAGS: v0.10.0, content, examples, testing
- KIND: STORY
- ACTIVITY: PLANNING
- GATES: -
- RESOLUTION: -
- PARENT: 20260802-115955
- DEPENDS ON: 20260804-003244, 20260804-093855

## Story

Deepen the `sections/` runs and collapse seven examples into five, one per
section family.

Today's section runs are mostly one scene, one section, one beat, with
wall-clock runways where an assertion belongs. With predicate steps a run can
walk several rounds across at least two scenes or section layouts - spawn,
drive, damage, destroy, reload, re-enter, assert the invariant again - and gate
each beat on the value it depends on rather than sleeping past it.

## Steps

Owner call 2026-08-04: "deepen" is bounded by a NAMED INVARIANT LIST per run,
not by a scene or round count. Done means every listed invariant is asserted.
Scenes and rounds are means to that end - if an invariant needs two scenes, the
run gets two scenes; if it does not, padding one in proves nothing.

The lists below are drafted from what each run asserts TODAY (cited), plus the
merged-in assertions, plus the gaps. Confirm or amend them at planning; they
are the task's stopping rule, so they are worth arguing about before work
starts rather than after.

- [ ] `controller_section` - 4 invariants (2 exist, 2 new):
      1. the attitude command sweeps away from identity (:200, delivery guard)
      2. the hull tracks the command within `TRACK_TOLERANCE_RAD` (:213)
      3. NEW tracking holds on a second section layout
      4. NEW tracking re-converges after a reload
- [ ] `thruster_section` - 5 invariants (3 exist, 2 new):
      1. nose speed grows under a full burn (:189)
      2. the drive spawns its exhaust plume material (:204)
      3. the plume shader input follows the held throttle (:216)
      4. NEW the throttle -> impulse relation holds at a partial setting
      5. NEW plume input returns to zero when the throttle is released
- [ ] `hull_section` - 8 invariants (3 exist, 4 merged from `com_range`, 1 new):
      1. a partial hit subtracts EXACTLY (:193 - the damage path does
         arithmetic, not vibes)
      2. an overkilled section is destroyed and despawned (:220)
      3. the ship root and its controller survive a leaf loss (:230,:234)
      4. MERGED COM sits on the attached-section centroid, drift < 0.3
         (com_range.rs:381)
      5. MERGED local COM moved aft after losing the front sections (:386)
      6. MERGED the root keeps `TransformInterpolation` (:394)
      7. MERGED the chase camera anchor tracks the live COM (:408)
      8. NEW 1-7 hold again after a reload and a second destroy round
      Then delete `com_range` and its catalog entry.
- [ ] `turret_section` - 4 invariants (2 exist, 2 new):
      1. a turret round is fired in the window (:478)
      2. a gate takes turret hits (:479)
      3. NEW aim error converges on a MOVING target
      4. NEW the sequence repeats after a reload
      Keep the slider submodule for human tuning.
- [ ] `torpedo_section` - 6 invariants (4 exist, 1 merged, 1 new):
      1. fired (:517)  2. armed (:518)  3. detonated (:519)
      4. a gate takes blast damage (:523)
      5. MERGED PN guidance closest-approach against a CROSSING target
         (the lead-a-crosser round, from `torpedo_guidance`)
      6. NEW the sequence repeats across a second scene
      Then delete `torpedo_guidance` and its catalog entry.
- [ ] Build the ship fixture LOCALLY here. Do NOT extract a shared builder:
      owner call 2026-08-04, `20260804-094006` is the third caller and does the
      extraction, having seen all three shapes. One caller is not an
      abstraction.

## Definition of Done

- Every invariant in the Steps list is asserted by its run, and every run
  asserts through predicates rather than elapsed time. 27 invariants across
  five runs: 14 exist today, 5 come in with the merges, 8 are new.
  (cmd: `nix develop --command cargo run -p nova_probe -- run sections`)
- The two merged runs are gone and their assertions live in the absorbing run.
  (cmd: `! rg -n 'com_range|torpedo_guidance' Cargo.toml examples tests`)
- The catalog, disk and smoke lists agree after the merges.
  (test: `catalog_matches_disk`)

## Notes

Roster per the spike (`20260804-003244`) - each run gets harder, not thinner:

| Run | Change |
| --- | --- |
| `controller_section` | Deepen. PD attitude control across multiple layouts and repeated rounds. |
| `thruster_section` | Deepen. Throttle -> impulse + plume, same shape. |
| `hull_section` | Deepen, ABSORBS `com_range`. |
| `turret_section` | Deepen. PDC tracking + firing. |
| `torpedo_section` | Deepen, ABSORBS `torpedo_guidance`. |

The two merges:

- `com_range` -> `hull_section`. `hull_section` owns the damage -> destroy
  pipeline, and COM-follows-destruction is that pipeline's consequence, not a
  separate subject. `com_range.rs:374` (`assert_com_follows_sections`) becomes
  a round after the destroy round. `com_range` is already predicate-driven, so
  the beats port directly.
- `torpedo_guidance` -> `torpedo_section`. Both are the torpedo bay family and
  one example per family is the contract. The PN closest-approach assertion
  becomes the lead-a-crosser round of the merged run.

- Assert through predicates on the values the section family owns (mass/COM,
  thrust, integrity, guidance, lock, range), not through elapsed time.
- `turret_section` carries a 203-line interactive slider submodule for human
  tuning. It stays; if the probe path never touches it, a later task may
  extract it to a shared dev-widget module. Not blocking here.
- Ship builders stay LOCAL here. `20260804-094006` extracts the shared `fn`
  with its count knob as the third caller (owner call 2026-08-04), designing
  the signature from three visible shapes instead of from this one.
- `sections/` carries no fps window.
- Examples must be RUN under Xvfb :99, not only checked.
