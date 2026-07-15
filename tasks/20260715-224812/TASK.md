# Make Demo Mod Arena a playable target-destruction challenge

- STATUS: OPEN
- PRIORITY: 58
- TAGS: modding, scenario, gameplay

## Goal

Turn the `demo_mod_arena` scenario (`assets/mods/demo/mod.content.ron`) from a
lone static beacon into a simple, playable shooting gallery: the player spawns
in a ship with a working turret and must destroy a small cluster of derelict
target rocks; destroying them all completes the objective and shows a victory
line. Keep the section-overlay demo (the up-armored `reinforced_hull_section`)
intact. Pure data-driven RON, no engine changes.

## Steps

- [ ] Keep the existing `Section((... id: "reinforced_hull_section" ...))`
      overlay in `mod.content.ron` unchanged - `demo_scenario.rs` asserts that
      overlay, and it is the point of the "demo" mod.
- [ ] In `demo_mod_arena`'s `OnStart`, spawn a player ship (id
      `player_spaceship`) at `(0,0,0)` with a turret, copying the player
      Spaceship block (controller / hull_front / thruster / turret) from
      `assets/base/scenarios/shakedown_run.content.ron`, `infinite_ammo: true`,
      no `DisableVerb` mods. The turret input mapping is what makes it shootable.
- [ ] Keep the `demo_mod_beacon` (recolor/reposition if useful) as an arena
      centerpiece / visual anchor.
- [ ] Spawn 3 destructible target asteroids around the beacon via
      `SpawnScenarioObject` with `kind: Asteroid((radius: ~2.0, texture:
      "textures/asteroid.png", health: 100.0, surface_gravity: None,
      invulnerable: false, lock_signature: None))` at three spread positions
      (ids `arena_target_1..3`). Copy the Asteroid config shape from
      `assets/base/scenarios/demo.content.ron`.
- [ ] On `OnStart`: `VariableSet(key:"targets_left", Number(3.0))`, an
      `Objective((id:"clear_arena", message:"Destroy the 3 derelict rocks."))`,
      and `ObjectiveMarkerAttach` on each target.
- [ ] Add an `OnDestroyed` event per target (or one filtered by the shared
      target type) that decrements `targets_left` by 1 and detaches that
      target's marker. Use the `VariableSet` + arithmetic expression grammar
      from shakedown's counting handlers (`crates/nova_scenario/src/actions.rs`
      variable expressions). Confirm the `OnDestroyed` filter shape (`id` /
      `other_id`) against `crates/nova_scenario/src/events.rs` +
      `filters.rs` before wiring - verify-first.
- [ ] Add an `OnUpdate` (or the last `OnDestroyed`) gated by
      `Expression((Equal(targets_left, 0.0)))` that
      `ObjectiveComplete((id:"clear_arena"))` and adds a terminal
      `Objective((id:"arena_clear", message:"Arena cleared - all targets
      down."))`. Guard it so it fires once (e.g. a `done` flag variable), the
      way shakedown gates one-shot beats with a `beat` variable.
- [ ] Add a production-faithful behavior test for the destroy -> counter ->
      objective bridge (lessons `scripted-walks-skip-the-bridges`,
      `production-faithful-rigs`). Prefer modelling on the existing destruction
      tests in `crates/nova_scenario/src/objects/asteroid.rs` /
      `salvage.rs`; if the full destruction pipeline is too heavy for a unit
      rig, at minimum assert (a) the scenario loads and (b) the `OnDestroyed`
      handler decrements `targets_left` when the real destroy signal fires.
      Do not hand-fire the event as the only proof.
- [ ] Keep `cargo test -p nova_assets --test demo_scenario` green - it asserts
      `GameScenarios` contains `demo_mod_arena` when the demo mod is enabled and
      the section overlay applies. Add/extend an assertion for any new invariant
      worth pinning (e.g. the scenario now has the destroy handlers).

## Notes

- Relevant files: `assets/mods/demo/mod.content.ron` (edit),
  `assets/mods/demo/demo.bundle.ron` (meta),
  `assets/base/scenarios/shakedown_run.content.ron` (player-ship + counting
  reference), `assets/base/scenarios/demo.content.ron` (Asteroid config
  reference), `crates/nova_scenario/src/events.rs` + `filters.rs` (OnDestroyed
  filter shape), `crates/nova_assets/tests/demo_scenario.rs` (load gate).
- The demo mod is an INSTALLED mod (in `assets/mods.catalog.ron`), hand-authored
  RON, NOT generator-guarded by `content_ron_parity.rs`. Edit the RON directly.
- Design choice: 3 destructible rocks (a shooting gallery) over an AI-brain
  duel - it exercises the turret + OnDestroyed cleanly and is far easier to
  test deterministically. An AI scavenger duel is a possible follow-up.
- Verify-first: confirm `OnDestroyed` fires with the target's `id` as the
  subject (`filters.rs`) so the per-target counter works, and that turret
  fire actually damages/destroys an Asteroid with `invulnerable: false`.
- Depends on: nothing (independent of the gauntlet task).
