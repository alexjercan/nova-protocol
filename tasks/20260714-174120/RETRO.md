# Retro: Catalog-driven mod loading (base as a default-enabled mod)

- TASK: 20260714-174120
- BRANCH: modding/mod-catalog
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- The spike's data model (catalog asset whose deps are all installed bundles +
  runtime `EnabledMods` selecting the merge subset) dropped in cleanly because the
  merge was already a pure `merge_bundles` core (from 134119). The refactor was
  "change which bundles feed the merge", not "rewrite the merge" - the earlier
  investment in a pure core paid off a third time.
- Applied the family lessons up front: stemmed `mods.catalog.ron` (-> `catalog.ron`
  ext) so the untyped in-game load resolves (163342); a `catalog_untyped_load` guard
  and a live-frames `toggling_enabled_mods_remerges_live` test (not `run_system_once`)
  per `test-the-production-load-path` / `registered-system-for-change-detection`.
- Behavior preservation held: "base is a mod" is now a data fact (a `base:true`
  catalog entry seeded into `EnabledMods`), yet startup is byte-identical (base only)
  and both examples run clean. The demo bundle loads-but-doesn't-merge - exactly the
  target state.
- The live re-merge run-condition wiring
  (`resource_changed::<EnabledMods>` + `resource_exists::<GameAssets>` +
  `not(in_state(Loading))`) was the one subtle part; I traced the state-transition
  timing by hand and an out-of-context reviewer independently confirmed no
  empty-catalog wipe window and no clobber-before-seed. No defects.

## What went wrong

- Nothing broke. The one thing to watch was the unconditional
  `insert_resource(GameSections/GameScenarios)` in `register_bundles`: if it ever ran
  with the catalog absent it would WIPE both to empty. It cannot today (gated), but it
  is a latent foot-gun for future callers - noted for the persistence task, which adds
  another pre-merge input.

## What to improve next time

- When a system unconditionally overwrites a resource, make its run conditions (or an
  early-return on missing input) explicit enough that a future edit cannot accidentally
  open a wipe window. Here the safety lives in the plugin's run_if wiring, one layer
  away from the system - a comment at the insert points to it, but co-locating the
  guard would be more robust.

## Action items

- [x] Live re-merge proven across frames; untyped catalog guard added.
- [ ] 174126 (Mods menu) consumes `EnabledMods` + the catalog metadata; toggling drives
      the live re-merge. THE GOAL (see demo, enable it) is met there.
- [ ] 174131 (persistence) pre-populates `EnabledMods` before `seed_enabled_mods` (which
      only seeds if empty) - the seam is already in place.
