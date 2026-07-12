# Infinite ammo option for the first (New Game) scenario

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0, weapons, scenario

## Goal

The first scenario the player meets (New Game loads "shakedown_run",
nova_menu/src/lib.rs:41) should not gate the player on ammunition. Give ships an
opt-in "infinite ammo" capability and turn it on for the player in that scenario,
so the intro plays without running dry (finite ammo landed in 20260525-133025;
its reload is still a future pass, so the starter must not depend on it).

Scope decision: PLAYER-scoped. The player's weapons are unlimited in the first
scenario; enemies keep their finite magazines. This mirrors the existing per-ship
knob `PlayerControllerConfig.speed_cap` and keeps the "ability" reusable by any
scenario, not hardcoded to one.

## Steps

- [x] Add `infinite_ammo: bool` to `PlayerControllerConfig`
      (nova_scenario/src/objects/spaceship.rs), defaulting false (the struct
      derives Default). Document it next to `speed_cap`.
- [x] In `insert_spaceship_sections`, derive `infinite_ammo` from the controller
      and, when true, override the weapon config's `ammo_capacity` to `None`
      before building the Turret/Torpedo section - reusing the existing
      unlimited-when-None path (no fire-system change, no new component/marker).
- [x] Fill the new field at the two explicit `PlayerControllerConfig` build
      sites: `infinite_ammo: true` in the shakedown player (scenario/shakedown.rs)
      and `false` in asteroid_field (scenario.rs), so New Game is unlimited and
      the sandbox scenario keeps finite ammo.
- [x] Test (bare-World, the module's existing pattern): a Player ship built with
      infinite_ammo=true spawns its turret section with
      `TurretSectionConfigHelper.ammo_capacity == None`; with false it stays
      `Some`. Asserting the stripped config is the right unit boundary - the
      "None => no SectionAmmo" half is already covered by the ammo tests.
- [x] Verify: cargo check + fmt; run the new test.
