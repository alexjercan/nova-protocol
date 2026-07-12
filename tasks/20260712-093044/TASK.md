# Nav beacon and salvage crate scenario objects

- STATUS: OPEN
- PRIORITY: 45
- TAGS: v0.5.0,scenario,content,spike

## Goal

Two reusable scenario-object primitives the Shakedown Run starter scenario
needs, built as general content pieces, not tutorial hacks: a nav beacon
(the game's first player-facing waypoint) and a salvage crate (a minimal
proximity pickup). Plus the one missing scenario action their use requires:
despawning a scenario object by id.

## Steps

- [ ] Add `EventActionConfig::DespawnScenarioObject(DespawnScenarioObjectActionConfig
      { id: String })` in `crates/nova_scenario/src/actions.rs`: the action
      `push_command`s a closure that uses `commands.queue(move |world: &mut
      World| ...)` to find the entity whose `EntityId` matches and despawn
      it (recursive despawn takes children). Log a warning when no entity
      matches. Export from the prelude.
- [ ] Define `BeaconMarker` (component) and a beacon blink system in
      `crates/nova_gameplay/src/` (marker beside the other gameplay markers;
      blink = emissive intensity pulse on the beacon's material, ~1 Hz).
      The marker lives in nova_gameplay because the HUD chip system (next
      step) must query it and nova_scenario depends on nova_gameplay, not
      vice versa (same split as SpaceshipRootMarker).
- [ ] Beacon HUD chip in `crates/nova_gameplay/src/hud/` (small module,
      wired into NovaHudPlugin): on `Add<BeaconMarker>`, spawn a screen
      indicator (existing substrate, `hud/screen_indicator.rs`) anchored to
      the beacon entity - label text from a `BeaconLabel(String)` component
      + live distance readout (update system each frame), offscreen policy
      `ClampToEdge` so an off-screen beacon pins to the screen edge as a
      direction cue. Follow the flight_status/destination-marker chip
      pattern. Despawn the chip when the beacon despawns (existing
      indicator cleanup pattern).
- [ ] Add `ScenarioObjectKind::Beacon(BeaconConfig)` in
      `crates/nova_scenario/src/objects/beacon.rs`: config { label,
      radius (mesh scale), color, area_radius: Option<f32> }. Bundle:
      emissive mesh (simple sphere or octahedron), `BeaconMarker`,
      `BeaconLabel`, `Sensor` collider (no physical collisions - the
      menu_ambience penetration-impulse lesson, scenario.rs:57), and when
      `area_radius` is Some, `ScenarioAreaMarker` so the beacon fires its
      own OnEnter with its scenario id (no separate CreateScenarioArea
      needed). Wire the new kind into the match in actions.rs
      (ScenarioObjectConfig::action) and objects/mod.rs.
- [ ] Add `ScenarioObjectKind::SalvageCrate(SalvageCrateConfig)` in
      `crates/nova_scenario/src/objects/crate.rs` (module name `salvage`
      if `crate` collides with the keyword - it does; use `salvage.rs`):
      config { size, area_radius }. Bundle: small box mesh, slow tumble
      (initial AngularVelocity), `Sensor` collider + `ScenarioAreaMarker`
      (fires OnEnter with its id on player contact), EntityTypeName
      "salvage_crate". The crate does NOT despawn itself; the scenario
      script pairs its OnEnter with DespawnScenarioObject, keeping pickup
      consequences in scenario data.
- [ ] Tests (run only these + cargo check + fmt, per project rule):
      config-shape tests in the two object modules (bundle carries
      Sensor + ScenarioAreaMarker + interpolation via base object);
      an events test that a DespawnScenarioObject action removes the
      matching EntityId entity and warns on a missing id; a physics test
      that a moving player-shaped body entering a crate's sensor fires
      OnEnterEvent with the crate's id (mirror area.rs test patterns if
      present, else the avian harness pattern in actions.rs tests).
- [ ] Update `docs/scenario-system.md`: the two new object kinds, the new
      action, and the beacon's built-in-area convention.

## Notes

- Spike: docs/spikes/20260712-092926-starter-scenario.md
- Blocks: 20260711-180506 (Shakedown Run scenario uses both objects)
- Verified at plan time: targeting candidates are any RigidBody minus
  turret bullets (input/targeting.rs:324), so a Sensor beacon with
  RigidBody from base_scenario_object is aim-lockable and GOTO-able; no
  despawn action exists today (actions.rs enum); ObjectivesPlugin
  (bevy-common-systems ui/objectives.rs) rebuilds the panel in one frame
  on GameObjectives change, so complete+re-add in one action list updates
  a message without flicker.
- base_scenario_object already provides ScenarioScopedMarker, EntityId,
  RigidBody::Dynamic, TransformInterpolation - both objects build on it
  like asteroid/spaceship do.
- ScenarioAreaMarker requires Collider + Sensor and gets
  CollisionEventsEnabled from ScenarioAreaPlugin (objects/area.rs); its
  OnEnter fires with (area id, other id, other type name) - the filter
  side already supports id+other_id matching (asteroid_field:489).
