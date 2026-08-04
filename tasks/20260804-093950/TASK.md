# Deepen sections/ to multi-scene multi-round runs, merging com_range and torpedo_guidance

- PRIORITY: 81
- TAGS: v0.10.0,content,examples,testing
- KIND: STORY
- ACTIVITY: -
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

- [ ] Deepen `controller_section`: multiple layouts, repeated rounds,
      predicate-gated assertions on the attitude command it tracks.
- [ ] Deepen `thruster_section`: throttle -> impulse and plume across rounds.
- [ ] Merge `com_range` into `hull_section`: port
      `assert_com_follows_sections` (com_range.rs:374) in as a round after the
      destroy round, then delete `com_range` and its catalog entry.
- [ ] Deepen `turret_section`: tracking and firing across scenes; keep the
      slider submodule for human tuning.
- [ ] Merge `torpedo_guidance` into `torpedo_section`: its PN closest-approach
      assertion becomes the lead-a-crosser round; delete `torpedo_guidance`
      and its catalog entry.
- [ ] Extract the ship builders as shared `fn`s with a count knob so
      `stress/many_sections` can reuse them.

## Definition of Done

- Every `sections/` run covers at least two scenes and repeated rounds, and
  asserts through predicates rather than elapsed time.
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
- Ship builders should be shared `fn`s with a count knob so `stress/many_sections`
  can reuse them.
- `sections/` carries no fps window.
- Examples must be RUN under Xvfb :99, not only checked.
