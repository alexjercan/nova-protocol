# Review: Catalog-driven mod loading (base as a default-enabled mod)

- TASK: 20260714-174120
- BRANCH: modding/mod-catalog

## Round 1

- VERDICT: APPROVE

Reviewed the refactor (removed `ModList`/`enabled.mods.ron`; added
`InstalledCatalog`/`CatalogLoader`/`ModEntry` in nova_modding; `GameAssets.catalog` +
`EnabledMods` + `seed_enabled_mods` + re-merge in nova_assets). Self pass plus an
independent out-of-context adversarial pass. Independently re-derived the load-bearing
timing rather than trusting the summary:

- The empty-catalog wipe hazard is closed: `register_bundles` unconditionally inserts
  `GameSections`/`GameScenarios`, so running with the catalog absent would wipe them -
  but it cannot. The `OnEnter(Processing)` copy runs only after bevy_asset_loader gates
  the collection on the catalog's RECURSIVE load (catalog present by construction); the
  `Update` copy is gated `resource_exists::<GameAssets>` (only true at/after Processing,
  when the catalog is loaded) + `not(in_state(Loading))`. No frame has GameAssets present
  with the catalog absent.
- Ordering: `OnEnter(Processing)` (StateTransition schedule) runs `seed_enabled_mods`
  then `register_bundles` BEFORE `Update` in the same frame, so the seed always precedes
  any re-merge; the initial `EnabledMods` change during Loading is consumed while the
  Update system is gated off. At most one idempotent extra re-merge on the Processing
  frame. Benign.
- Behavior preservation: with only `base` enabled by default, base's
  `reinforced_hull_section` stays 200 (un-overridden), base `demo` + four built-ins
  present, the demo mod's `demo_mod_arena`/400-override absent - startup identical. The
  demo bundle still LOADS (recursive gate) but is not merged.

Verification: nova_assets (20 unit + demo_scenario 5 + parity) and nova_modding (3)
tests pass; `cargo test --workspace --no-run` green; fmt clean; both `12_menu_newgame`
and `09_editor` run clean headless (0 loader errors, 0 conflicts, no panic) - the
catalog loads via the real untyped `GameAssets` path.

The out-of-context reviewer concurred (no defects) and confirmed the tests are not
false-passes: `toggling_enabled_mods_remerges_live` drives real frames with the
production `resource_changed::<EnabledMods>` run condition (not `run_system_once`);
`catalog_loads_and_base_only_merges_by_default` asserts the negative
(`demo_mod_arena` absent, hull=200) so it fails if demo leaks in;
`catalog_untyped_load_resolves_the_loader` pins the in-game untyped path.

No BLOCKER/MAJOR/MINOR/NIT. Ships.
