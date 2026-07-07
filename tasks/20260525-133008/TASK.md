# Unit tests for health + destruction pipeline

- STATUS: CLOSED
- PRIORITY: 85
- TAGS: v0.4.0, test

Sequence: damage → disable → explode. [new]

## Resolution

Added 13 co-located integration tests (the pipeline was untested), driving the real
observer/system cascade in a minimal headless `App` (no avian/assets needed - the core
integrity observers are avian-free and bcs `HealthPlugin` is observer-only):

- `integrity/plugin.rs` (8): `calculate_blast_damage` falloff; leaf derivation from
  connection count; disabled leaf -> destroy; disabled non-leaf -> not destroyed;
  becoming-a-leaf-while-disabled -> destroy (chain trigger); disabled root -> whole-body
  destroy; the headline **damage -> health zero -> disabled -> destroyed** sequence via a
  real `HealthApplyDamage`; and a **chain reaction** through a connected A-B-C line
  (pruning neighbours cascades the destruction).
- `integrity/glue.rs` (4): ship health is the sum of its sections; ship health reaches
  zero when the sections are gone; disabled non-leaf section -> deactivated
  (`SectionInactiveMarker`); disabled leaf section -> not deactivated (destroyed instead).
- `integrity/explode.rs` (1): a meshless destroyed entity is despawned.

42 nova_gameplay tests pass; the integrity additions are clippy-clean. (An unrelated
pre-existing `needless_update` warning in `hull_section.rs` is surfaced by `clippy
--tests` but left untouched - out of scope for this test task.)
