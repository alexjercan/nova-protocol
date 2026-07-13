# Multi-type magazines, reload and bullet-type switching

- STATUS: CLOSED
- PRIORITY: 44
- TAGS: v0.5.0,weapons,spike

## Goal

Phase 2 of the combat-depth pass, scoped to the FOUNDATION (user direction
2026-07-12): give each weapon an ammo-type "slot" that decides the fired
projectile's `DamageType`, stamp the projectile from that slot, and color-code
the diegetic ammo readout by type. Structure it so per-type magazines, reload,
and in-game/editor/station switching layer on later (ship-management menu) by
mutating one small component - NOT built now.

For now: one magazine (the existing single `SectionAmmo` pool) of one authored
type per weapon. Depends on the typed-damage core (20260712-133343, CLOSED) for
`DamageType`/`ProjectileDamage`.

"Done" = a turret fires bullets whose `ProjectileDamage.kind` comes from its
loaded-ammo slot (not a hardcoded Kinetic), the slot is a runtime-mutable
component authored from config, and each weapon's ammo readout pips are drawn in
that type's color; proven in headless tests.

## Steps

- [x] Add `LoadedBullet { kind: DamageType, damage: f32 }` component (the ammo
  slot) in turret_section.rs (prelude-exported). It is the runtime round the
  turret fires; a future scenario/station/ship-management action mutates it. Its
  authoring default comes from config (next step).
- [x] Add `bullet_kind: DamageType` to `TurretSectionConfig` (turret_section.rs:33
  struct, :86 Default = `DamageType::Kinetic`). Keep `bullet_damage`. In the
  `turret_section` bundle fn (turret_section.rs:115) insert `LoadedBullet { kind:
  config.bullet_kind, damage: config.bullet_damage }` (read both before `config`
  is moved into `TurretSectionConfigHelper`; both are `Copy`). Arity is fine
  (bundle grows 6 -> 7).
- [x] Fire from the slot: in `shoot_spawn_projectile` (turret_section.rs:864) add
  `&LoadedBullet` to `q_turret` (line 878) and stamp `ProjectileDamage { amount:
  loaded.damage, kind: loaded.kind }` at the spawn (turret_section.rs:1027),
  replacing `config.bullet_damage` + hardcoded `DamageType::Kinetic`. Catalog
  turrets keep Kinetic (default), so feel is unchanged.
- [x] Add `pub fn damage_type_color(kind: DamageType) -> Color` in damage.rs
  (prelude-exported): an opaque per-type hue. Kinetic = the current readout amber
  `(1.0, 0.75, 0.2)` (so Kinetic weapons look identical); ArmorPiercing = steel
  blue; Emp = cyan; Explosive = red-orange. Distinct hues, readable on the dark
  HUD behind the existing pip outline.
- [x] Color the readout by type: in ammo_readout.rs `drive_ammo_readouts`
  (ammo_readout.rs:344, the single ammo-source point the file flagged for exactly
  this) resolve each readout's `DamageType` - Turret readouts read the section's
  `LoadedBullet.kind`; Torpedo readouts are Explosive (torpedoes always detonate
  an Explosive `NovaBlast`) - and set lit pips to `damage_type_color(kind)` at the
  current lit alpha (0.95) and dim pips to the same hue at the dim alpha (0.16).
  Replace the `LIT_COLOR`/`DIM_COLOR` consts' role with per-type derivation; keep
  the black `PIP_OUTLINE_COLOR`.
- [x] Tests (headless): (1) `turret_section(config)` inserts a `LoadedBullet`
  matching `config.bullet_kind`/`bullet_damage`; default config is Kinetic.
  (2) `shoot_spawn_projectile` stamps the spawned bullet's `ProjectileDamage` from
  the slot - set a turret's `LoadedBullet.kind = Emp` and assert the fired bullet
  carries `Emp` (not Kinetic); this proves the slot drives the round and would
  fail if the fire path still read the hardcoded kind. (3) `damage_type_color`
  returns four DISTINCT colors and Kinetic == the historical amber. (4) the readout
  color path: a helper that maps a readout's resolved kind -> lit color equals
  `damage_type_color(kind)`; assert a non-Kinetic turret's lit pip color differs
  from Kinetic's. Use production spawn helpers; every assertion able to fail.
- [x] Docs: `docs/<date>-bullet-type-slot.md` (the slot design, why a separate
  runtime component vs baking in config, the deferred per-type-magazine/reload
  growth seam, the HUD color choices). Append a Fix record entry to spike
  20260712-133135. Add a CHANGELOG Unreleased line (note torpedo readouts now read
  red-orange = Explosive).

## Notes

- Spike: tasks/20260712-133135/SPIKE.md (architecture), 20260712-160505 (taxonomy /
  the four types + their intent). This is that spike's phase-2, scoped to the
  foundation.
- SectionAmmo (sections/ammo.rs) is UNCHANGED - still one pool. The "type" (slot)
  and the "magazine" (pool) are separate concepts now; per-type magazines join
  them later. The ammo readout already isolated its single ammo-source read
  (ammo_readout.rs:342-343 comment) for exactly this.
- The slot works for infinite-ammo weapons too (LoadedBullet is independent of
  SectionAmmo), so the fired type is correct whether or not ammo is finite.
- DEFERRED (future ship-management, not this task): per-type magazine pools,
  in-game reload, editor/scenario/station UI to switch the slot, and authoring
  any catalog weapon to a non-Kinetic type. Each is a small change on top of the
  slot (mutate `LoadedBullet` / grow `SectionAmmo` to per-type).
- Relevant files: crates/nova_gameplay/src/sections/turret_section.rs
  (config :33/:86, bundle :115, `shoot_spawn_projectile` :864, stamp :1027),
  crates/nova_gameplay/src/damage.rs (DamageType, add color fn),
  crates/nova_gameplay/src/hud/ammo_readout.rs (`drive_ammo_readouts` :344,
  LIT/DIM :66-69).
- Run `cargo check --workspace --all-targets` after adding the config field
  (check-all-targets-for-struct-field): `TurretSectionConfig` literals in
  nova_assets/sections.rs use `..default()`, so only the two authoritative
  configs and the Default need the new field, but verify examples/tests.
- Blocks nothing further in the family; alt-fire (20260712-133356) is independent.

## Implementation record

Landed: `LoadedBullet` slot + `bullet_kind` config in turret_section.rs, fire
stamps from the slot, `damage_type_color` in damage.rs, per-type coloring in
ammo_readout.rs `drive_ammo_readouts`. Write-up + reflection:
tasks/20260712-133349/NOTES.md. Fix record appended to spike 20260712-133135.

Deviations from the plan as written:

- The fire query uses `Option<&LoadedBullet>` with a fallback to the config
  default (plan said required `&LoadedBullet`). A required component would have
  stopped every headless fire rig (they spawn turrets by hand without the slot)
  from firing. Optional + config fallback keeps them working and encodes
  config-is-default / component-is-runtime.
- Removed the now-dead `LIT_COLOR` const (its only remaining use was in a test);
  the lit hue is derived from `damage_type_color`, and the test's lit-pip counter
  is now alpha-based (hue-agnostic). `DIM_COLOR` stays as the neutral pre-drive
  spawn fill.

Verification: `cargo check --workspace --all-targets` clean, `cargo fmt` clean,
new + affected tests green (damage 8/8, turret_section 20/20 incl. the two new
slot tests, ammo_readout 9/9 incl. the color-by-type test). Full suite in CI.

Playtest note (2026-07-12, filed separately): PDC one-shots asteroids/objects -
authored Kinetic per-hit x 1.0 resistance vs low-HP unclassed targets. Routed to
its own tuning task, not this branch.
