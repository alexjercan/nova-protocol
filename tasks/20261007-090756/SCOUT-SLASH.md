# SCOUT-SLASH: reserving `/` in EntityId (D-T7 second amendment)

Scout report (scout-slash, read-only, given inline and saved here by the
main worker). The main worker added the "Main worker check" section, which
corrects two scout claims.

## 1. The type

`EntityId(pub String)`, no character rule (`nova_events/src/lib.rs:67-78`).
`EntityId::new` and the tuple constructor take any `String`.

## 2. Where an id value comes from

Authored:
- Scenario object id: `BaseScenarioObjectConfig.id`, `@Names::NewObject`
  (`nova_scenario/src/actions/spawn.rs:76-79`, used at `:110`). A scatter
  base id has the same shape (`spawn.rs:665-668`, used at `:694`).
- Ship section id: `section.id` of each `ShipDesign.sections` entry, used
  at `nova_scenario/src/objects/spaceship.rs:914`.

`walk_names` gives `AuthoredName { names, field, text }` for every authored
name (`nova_scenario/src/names.rs:84-97`, walk at `:107-174`).

Generated, none can contain `/` (read by scout and main worker):
- sector body ids `sector_id` = `{coord.slug()}_{name}_{index}`
  (`nova_world/src/generation.rs:509-510`); names are the literals
  `{cluster_slug}_parent|rock|hull`, `hull_{i}_escort`, `escort_{i}`,
  `background_rock` (`nova_world_base/src/clusters.rs:1554,1568,1578,1593,1617,1653,2138`);
  slugs are `sector_{x}_{y}_{z}` and `cluster_{x}_{y}_{z}` with `n<abs>`
  for negatives (`nova_world/src/lib.rs:347-355`, `clusters.rs:506-515`);
- generated ship section ids `hull_z{z}_x{x}_y{y}`, `drive_x{x}_y{y}`,
  `controller_{i}`, `{slot}_starboard|port`, `link_{n}`
  (`nova_world_base/src/ship_layout.rs:1544,1584,1606,1628,2064,2075,2442`).

Not minted yet: `sever_disconnected_structures` spawns the wreck root with
no `EntityId` (`nova_ship/src/sections/integrity.rs:628-646`).

## 3. Readers

Every reader compares whole strings: `scoped_entities` (`spawn.rs:40`),
`object_reference_resolves` (`names.rs:209-221`), `live_by_id`
(`nova_world_base/src/save/transients.rs:273-288`). Nothing splits on `/`.

## 4. Existing `/` use

No authored or test `EntityId` contains `/`. UI theme ids such as
`base/phosphor` contain `/`, but they are a catalog key
(`nova_modding/src/lib.rs:110-117`), never an `EntityId`. The rule must not
touch them. Unverified: mod folders outside `crates/` and `assets/base`.

## 5. Lint and load

- Authoring lint: `nova_authoring/src/lint_walk.rs` calls `lint_scenario`
  (`nova_scenario/src/lint/scenario.rs:114`) and `lint_ship_design_config`
  (`nova_scenario/src/lint/ship.rs:137`), which calls
  `check_design_sections` (`ship.rs:100`) and through it
  `resolve_ship_design`.
- `LintIssue` has no structured field member
  (`nova_scenario/src/lint/mod.rs:120-130`); checks put the field in the
  message text.
- Field-naming refusal precedent: `SectorFault::Manifest { id, field, value }`
  (`nova_world/src/generation.rs:548-591`).

## 6. Mod load

`ContentAssetLoader` (`nova_modding/src/lib.rs:26-34`) does not validate.
`register_bundles` (`nova_assets/src/merge.rs:78`) is the gate.

## 7. UI

The editor shows `Names::NewObject` and `Names::Section` as free text
with no validation (`nova_editor` `inspect.rs:2832,2842`). The console
mints no authored id at runtime.

## Main worker check

- Wrong (scout section 5 and 8): "`register_bundles` does not re-validate
  object or section ids". `register_bundles` runs `lint_ship_design_config`
  on every merged ship and `lint_scenario` on every merged scenario, base
  and mod (`nova_assets/src/merge.rs:469-535`). `on_load_scenario` refuses
  a scenario with an Error finding (`nova_scenario/src/loader/lifecycle.rs:171`,
  `ContentIssues` keyed by scenario id, `loader/mod.rs:124`). So a lint
  Error on an object id is also the load refusal, for mods too.
- Gap that stays: a finding on a CATALOG ship design is filed under the
  ship id, and nothing refuses a scenario that references that design.
  The spawn then calls `resolve_ship_design`, which skips a bad section and
  logs it (`nova_scenario/src/objects/ship_design.rs:460-470`).
- Wrong (scout section 5, 8 and the D-T7 amendment text): `resolve_and_mate`
  (`nova_world_base/src/ship_layout.rs:2249-2262`) checks GENERATED ship
  designs only (callers `ship_layout.rs:2104,2201,2204`). Authored section
  ids go through `resolve_ship_design`, the one resolver of spawn, lint,
  preload, audit and editor (`ship_design.rs:460-473`). It has no
  duplicate-section check.
- Gap (main worker): no authored duplicate section-id check exists.
  `resolve_ship_design` and `check_design_sections` do not compare section
  ids (searched `nova_scenario/src/lint/{ship,scenario}.rs`,
  `objects/{ship_design,spaceship}.rs`). The D-T7 premise "section ids are
  unique per ship" holds for generated ships only. The wreck mint still
  fails closed on a duplicate: the second fragment's id is already live,
  so it stays id-less with an `error!`.
