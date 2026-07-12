# Bullet-type slot + ammo-readout color-coding

Task: tasks/20260712-133349. Spikes: docs/spikes/20260712-133135 (architecture,
phase 2), docs/spikes/20260712-160505 (taxonomy). Builds on the typed-damage core
(20260712-133343).

## What shipped

The FOUNDATION of phase 2 (per user direction 2026-07-12), not the full
multi-magazine/reload system:

- `LoadedBullet { kind: DamageType, damage: f32 }` - a per-turret runtime "ammo
  slot" (turret_section.rs), seeded from the config's new `bullet_kind` +
  existing `bullet_damage`. It is the round the turret currently fires; a future
  ship-management / station / scenario action swaps the loadout by mutating this
  one small component.
- `shoot_spawn_projectile` stamps the fired bullet's `ProjectileDamage` from the
  slot (`Option<&LoadedBullet>`, falling back to the config default so bare test
  rigs and any non-`turret_section` spawn still fire). No more hardcoded Kinetic.
- `damage_type_color(DamageType) -> Color` (damage.rs): the identifying hue per
  type - Kinetic = the historical readout amber (unchanged look), ArmorPiercing =
  steel blue, EMP = cyan, Explosive = red-orange.
- The diegetic ammo readout (`drive_ammo_readouts`) now colors its pips in the
  loaded round's hue: turret readouts read their section's `LoadedBullet.kind`,
  torpedo readouts are Explosive (a torpedo always detonates an Explosive
  `NovaBlast`). Lit/dim are now alpha over that hue (`LIT_ALPHA`/`DIM_ALPHA`).

Catalog turrets stay Kinetic, so nothing about turret feel or appearance changes;
the one visible delta is torpedo-bay readouts now reading red-orange (Explosive).

## Decisions and alternatives

- **A separate `LoadedBullet` component vs baking the type in config.** The slot
  is runtime state a future menu mutates; config carries only the authoring
  default. A dedicated small component is cheaper to mutate and to query (the HUD
  reads it directly) than the whole `TurretSectionConfigHelper`, and it is the
  clean growth seam toward per-type magazines: today it is one selected round;
  later it is selected-from-a-magazine-map.
- **`Option<&LoadedBullet>` with a config fallback** in the fire path, rather than
  a required component. Production turrets (via `turret_section`) always carry the
  slot; the fallback keeps existing headless fire rigs (which spawn a turret by
  hand without the slot) working and is a sensible default.
- **Type kept separate from the `SectionAmmo` pool.** The user asked for "one mag
  of one type" now. `SectionAmmo` is unchanged (one pool); the slot is the type.
  Per-type magazines later join them. The slot also works for infinite-ammo
  weapons (no `SectionAmmo`), so the fired type is always correct.
- **HUD color from a shared `damage_type_color`** in damage.rs (next to the type)
  rather than a HUD-local table, so future damage-number / hit-effect coloring
  reuses one source. Kinetic maps to the exact prior amber so the change is
  invisible for Kinetic weapons.

## Difficulties

- Adding the required `&LoadedBullet` to the fire query first broke the headless
  fire rigs (they spawn turrets without the slot). Switched to
  `Option<&LoadedBullet>` + config fallback. Caught immediately by the fire tests'
  intent, before running - the rigs spawn `TurretSectionConfigHelper` directly.
- The `bullet_kind` config field broke the two full-literal turret configs in
  nova_assets (they do not use `..default()`); `cargo check --all-targets` caught
  both (the check-all-targets lesson).
- Removing `LIT_COLOR`'s only non-test use left it dead in the lib build (its
  remaining use was `#[cfg(test)]`). Removed the const and made the test's lit-pip
  counter alpha-based (hue-agnostic), which is more robust for per-type colors
  anyway.

## Verification

New tests, all green (`cargo check --workspace --all-targets` clean, fmt clean):

- `damage_type_color` returns four distinct colors and Kinetic == the historical
  amber.
- `turret_section` seeds `LoadedBullet` from config (EMP config -> EMP slot;
  default -> Kinetic).
- A turret fired with an EMP slot produces an EMP bullet (would fail against the
  old hardcoded-Kinetic stamp).
- The readout driver colors an EMP turret's lit pips in the EMP hue (distinct
  from Kinetic amber) and a torpedo bay's in Explosive.

## Self-reflection

- The "make the new query field optional + fall back to the authored default"
  move is worth reaching for by default when adding a required component to an
  existing system's query: it keeps every existing rig/spawn path alive and
  encodes the config-is-default / component-is-runtime relationship in one place.
- Deferred, and now trivial on top of the slot: per-type magazine pools + reload,
  and the editor/scenario/station/ship-management UI to switch the slot (mutate
  `LoadedBullet` / grow `SectionAmmo`). Authoring any catalog weapon to a
  non-Kinetic loadout is now a one-line content change.
