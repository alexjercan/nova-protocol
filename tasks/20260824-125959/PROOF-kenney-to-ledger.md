# Proof: the Kenney fleet moved into The Ledger

Branch `kenney-to-ledger`, commit `0d9329dc`, off `568149b2`.

## What moved

- `webmods/the-ledger/ledger_sections.content.ron` (the racer, cargoa and
  cargob part prototypes) and `ledger_ships.content.ron` (racer, cargob,
  cargob_lance, cargoa, cargoa_raider), sliced out of the generated base RON.
  Sounds now resolve through `dep://base/`; the meshes are `self://`.
- 21 GLBs `assets/base/gltf/parts/**` -> `webmods/the-ledger/gltf/parts/**`,
  declared in the mod bundle (`version: 1.28.0`).
- Deleted `base_content/ships/{racer,cargo_a,cargo_b,shared}.rs`; base's
  `section_catalog` is `standard_section_prototypes` alone.
- The Gauntlet's inline racer hull -> `Prototype("block_cutter")`
  (`version: 1.12.0`), so the time trial needs no campaign mod.

## Checks

- `cargo fmt`; `cargo check --workspace --all-targets` clean.
- `content gen` reproduces the trimmed base RON with no drift.
- `content lint`: 0 errors, 0 warnings, 0 findings, 10 scenarios
  balance-audited, 15 creative maps - base, example, gauntlet and the-ledger
  merged together, so every ledger chapter's `cargoa_*`/`cargob_*` reference
  resolves against the mod's own prototypes.
- The Ledger's 21 declared part resources all exist; none undeclared; no
  ledger RON references `dep://base/gltf/parts/`.
- `web`: `npm run format:check`, `lint` and `test` pass (site, theme, widgets,
  ron, assets-namespace).
- Live runs under Xvfb (`NOVA_AUTOPILOT=1`, no capture), all "cycle complete,
  no panic": `loop_damage_sequence`, `loop_spine_cut`, `loop_torpedo_blast`,
  `screenshot_hero_ship`, `screenshot_combat_wide`, `screenshot_radar_lock`,
  `loop_goto_arrival`.

## Greps

`crates/`, `assets/`, `examples/screenshots/` and `docs/` name no Kenney craft.
What remains is deliberate:

- `art/kenney-space-kit/`, `art/part-candidates/{racer,cargoa,cargob}`,
  `scripts/part-recipes/craft_*.json` and `scripts/cut-obj-into-*.py` - the raw
  art pipeline, never shipped in `assets/`.
- `examples/playable/parts_viewer.rs` and
  `examples/screenshots/screenshot_thruster_gallery.rs` - art-review tools that
  browse `part-candidates://`, not game content.
- `web/src/**` and `webmods/the-ledger/**` name the craft as THE LEDGER's, and
  the wiki catalog rows say so under each table.
- `web/src/news/**` and old `CHANGELOG.md` releases - history, left frozen.

## Known follow-up

A Ledger installed under `~/.local/share/nova-protocol/mods/` at 1.26.0 lints
with unknown `cargoa_*` prototypes against the new base, as expected: the fleet
now ships with the mod, so that copy needs the 1.28.0 update.
