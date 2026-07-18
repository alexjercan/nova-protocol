# Craft ships into the base game

Moved the `craft_racer` and `craft_cargob` example mods into the base game and
re-skinned the mainline campaign ships with them: the racer is the player ship
AND the scavenger enemy; the cargob is the Rust Tally boss.

## Important: base content is generated from Rust

The base campaign `*.content.ron` files are NOT hand-authored - they are
generated from Rust builders by `cargo run -p nova_assets --bin content -- gen`
and pinned by the `content_ron_parity` test (`crates/nova_assets/tests/`). So the
ship definitions had to change in Rust; editing the `.ron` directly would be
reverted on the next `gen` and fail parity. (The first attempt at this task
edited the `.ron` by hand - wrong; this is the redo in Rust.)

Regeneration flow: edit the Rust builders -> `content gen` -> commit the changed
`.ron`. The section catalog comes from `nova_assets::sections::build_sections`;
the scenarios from `nova_assets::scenario::*`.

## What changed

- **Meshes moved out of the mods** into the base game:
  - `assets/mods/craft_cargob/gltf/*.glb` -> `assets/base/gltf/cargob/`
  - `assets/mods/craft_racer/gltf/*.glb`  -> `assets/base/gltf/racer/`
  - Both example mods (bundle + content + demo scenario) were deleted and their
    entries removed from `assets/mods.catalog.ron`. `base.bundle.ron` lists all
    60 cube meshes under `resources`; base content references them with
    `self://gltf/{racer,cargob}/...`.
- **New Rust module `crates/nova_assets/src/scenario/craft.rs`** builds both
  ships inline from their cube tables (`RACER_CUBES`, `CARGOB_CUBES`, one entry
  per cut `.glb`). `racer_sections(grade, controller_mods)` and
  `cargob_sections()` return `Vec<SpaceshipSectionConfig>` with `Inline` section
  configs. The turret joint tree is reused from `sections::turret_joint_tree`
  (now `pub(crate)`), with the cut cube mounted on its fixed base and
  counter-rolled to render upright.
- **Campaign ship swaps** (in `scenario/shakedown.rs` and `scenario/broadside.rs`):
  - `player_ship()` in both scenarios -> racer; the controller block is kept and
    the fire input binds the two racer turret cubes (`RACER_TURRET_IDS`).
    Shakedown keeps its tutorial `DisableVerb(Goto/Lock/Orbit)` on the controller
    cube via `controller_mods`.
  - `pirate_ship()` / `corvette()` -> racer at `ShipGrade::Enemy`, keeping each
    ship's AI tuning (patrol / leash / engage_delay).
  - `gunship()` ("Rust Tally") -> `cargob_sections()`, keeping its AI controller.
  - Neutral hauler, menu-backdrop ships and the hidden `asteroid_field` demo were
    left on the legacy prototype sections (out of scope).

## Balance: grade drives HP and turret power (no modification overlays)

Because the ships are built in Rust, `ShipGrade` sets health and turret stats
directly rather than via `SetHealth` / light-turret-prototype overlays:

- Racer hull cube HP: 60 (player) / 35 (enemy). Thruster 70/25, controller
  100/45, turret 130/60. Cargob hull cube HP: 70 (boss, sturdier).
- Player racer turrets are full-power PDCs (fire 100, dmg 4.0, ammo 500). Enemy
  racer turrets are scavenger-grade (fire 25, muzzle 60, dmg ~ light turret, ammo
  150) - the "make enemies weaker" lever. The cargob boss keeps full-power
  turrets and two torpedo tubes.

A racer is ~18 sections and the cargob ~43, so per-section HP is kept low to keep
total ship HP sane. These are first-pass numbers - tune in `craft.rs`.

## Sounds

The moved sections were given the base-game audio conventions rather than copied
verbatim: every section base carries `impact_sound` / `destroy_sound`, and the
racer/cargob controllers carry the lock / radar / safety feedback cues the base
controller has (so campaign targeting audio works). Turret fire/dry-fire, thruster
loop and torpedo launch/detonation sounds reuse the shared base refs.

## Tests

- `content_ron_parity` (regenerate + diff) - the gate that makes Rust the source
  of truth; run `content gen` after any builder change.
- `nova_assets` shakedown/broadside unit + integration tests were updated to
  check ship shape by section KIND (racer/cargob sections are inline) instead of
  by catalog prototype id, and to assert the scavenger is weaker (lower turret
  damage + hull HP) than the player. Full `cargo test -p nova_assets` is green.

## Follow-ups / things to playtest

- Racer controller `max_torque` is 800 (from the mod); with ~18 sections it may
  feel twitchy or sluggish - tune after a flight test.
- Enemy HP / turret numbers are first-pass; adjust after a combat pass.
- `asteroid_field` (hidden demo) still uses the legacy ship.
