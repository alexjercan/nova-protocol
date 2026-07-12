# Nav beacon and salvage crate scenario objects

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.5.0,scenario,content,spike

## Goal

Two reusable scenario-object primitives the Shakedown Run starter scenario
needs, built as general content pieces, not tutorial hacks: a nav beacon
(the game's first player-facing waypoint) and a salvage crate (a minimal
proximity pickup). Plus the one missing scenario action their use requires:
despawning a scenario object by id.

## Steps

- [x] Add `EventActionConfig::DespawnScenarioObject(DespawnScenarioObjectActionConfig
      { id: String })` in `crates/nova_scenario/src/actions.rs`: the action
      `push_command`s a closure that uses `commands.queue(move |world: &mut
      World| ...)` to find the entity whose `EntityId` matches and despawn
      it (recursive despawn takes children). Log a warning when no entity
      matches. Export from the prelude. AS EXECUTED: the lookup is
      restricted to `With<ScenarioScopedMarker>` - spaceship SECTIONS also
      carry `EntityId` (per-ship ids like "controller"), and an unscoped
      match would despawn that section from every ship in the scene.
- [x] Define `BeaconMarker` + `BeaconLabel` in
      `crates/nova_gameplay/src/beacon.rs` (marker beside the other
      gameplay markers). AS EXECUTED: the blink did NOT go here - it lives
      with the render observer in nova_scenario (below), because the blink
      drives the material the render side creates; nova_gameplay only owns
      what the HUD queries.
- [x] Beacon HUD chip in `crates/nova_gameplay/src/hud/beacon_chips.rs`
      (BeaconChipsHudPlugin, wired into NovaHudPlugin): on
      `Add<BeaconMarker>` spawn a chip layer (label + live distance,
      `ClampToEdge` with a chip-scale chevron in the edge-indicator arrow
      language), `HudTier::Chrome`; per-frame label update from the player
      ship distance (label-only without a player); chip despawns on
      `Remove<BeaconMarker>`.
- [x] `ScenarioObjectKind::Beacon(BeaconConfig)` in
      `crates/nova_scenario/src/objects/beacon.rs`: { label, radius,
      color, area_radius: Option<f32> }. Emissive blinking orb child
      (render observer + blink system, gated on `render`),
      `RigidBody::Static` override (nav points hold position), authored
      `LockSignature(20.0)`, and with `area_radius` set the beacon doubles
      as its own trigger (ScenarioAreaMarker + sensor sphere). AS
      EXECUTED, ADDITIONAL: the targeting gate
      (`input/targeting.rs`) rejected ALL non-Dynamic bodies except wells,
      which made a Static beacon unlockable and would have broken the
      GOTO beat - the gate now also admits Static bodies with an AUTHORED
      LockSignature. Trigger areas never carry a signature, so the
      invisible-statics rule holds; regression test
      `static_beacons_lock_but_static_areas_never_do`.
- [x] `ScenarioObjectKind::SalvageCrate(SalvageCrateConfig)` in
      `crates/nova_scenario/src/objects/salvage.rs` (named salvage.rs -
      `crate` is a keyword): { size, area_radius }. Bright tumbling box
      child (tumble is a render-child animation, NOT physics: a
      sensor-only collider has no mass, a Dynamic crate would be an avian
      zero-mass warning - the root is Static), sensor trigger via
      ScenarioAreaMarker; the crate does not despawn itself, the script
      pairs its OnEnter with DespawnScenarioObject.
- [x] Tests (8 new, all passing; cargo check + fmt clean; full suite is
      CI's job per project rule): despawn action removes exactly the
      scoped match / spares an unscoped id-collision stand-in / missing id
      warns harmlessly (actions.rs); beacon contract + own-trigger shape
      (beacon.rs); crate contract + tumble-child-only (salvage.rs);
      pickup end to end through REAL physics and the REAL event pipeline
      (moving body -> sensor -> OnEnter -> filtered EventHandler ->
      VariableSet lands in NovaEventWorld) with a delivery guard
      (salvage.rs); static-with-signature lockable vs static-unsigned
      never (targeting.rs).
- [x] Update `docs/scenario-system.md`: the two new object kinds, the new
      action, and the beacon's built-in-area convention.

## Notes

- Spike: docs/spikes/20260712-092926-starter-scenario.md
- Blocks: 20260711-180506 (Shakedown Run scenario uses both objects)
- Verified at plan time: no despawn action existed (actions.rs enum);
  ObjectivesPlugin (bevy-common-systems ui/objectives.rs) rebuilds the
  panel in one frame on GameObjectives change, so complete+re-add in one
  action list updates a message without flicker.
- CORRECTED during work: the plan-time "any RigidBody is a targeting
  candidate" read was incomplete - the candidates QUERY takes any
  RigidBody, but a gate inside the collection lambda rejected non-Dynamic
  non-well bodies (targeting.rs:383). Static beacons needed the
  authored-signature exemption described in the beacon step.
- base_scenario_object already provides ScenarioScopedMarker, EntityId,
  RigidBody::Dynamic, TransformInterpolation - both objects build on it
  like asteroid/spaceship do; both override the RigidBody to Static.
- ScenarioAreaMarker requires Collider + Sensor and gets
  CollisionEventsEnabled from ScenarioAreaPlugin (objects/area.rs); its
  OnEnter fires with (area id, other id, other type name) - the filter
  side already supports id+other_id matching (asteroid_field).
- The avian CollisionStart body1/body2 ordering worry (area.rs only
  treats body1 as the area) resolved empirically: the pipeline test's
  OnEnter fired with the static sensor as body1. If ordering ever
  regresses it will show up as this test going quiet.

## Close record

What changed: one new scenario action (DespawnScenarioObject, scoped-only
lookup), two new scenario object kinds (Beacon in objects/beacon.rs,
SalvageCrate in objects/salvage.rs) wired into ScenarioObjectsPlugin and
the SpawnScenarioObject match; BeaconMarker/BeaconLabel components in
nova_gameplay/beacon.rs; a beacon HUD chip module (hud/beacon_chips.rs)
on the screen-indicator substrate with the edge-clamp direction chevron;
one targeting-gate amendment (Static + authored LockSignature is
lockable) with a regression test; docs/scenario-system.md update.

Alternatives considered: crate self-despawn on contact (rejected - pickup
consequences belong in scenario data, and the despawn action is
reusable); Dynamic beacon with explicit mass instead of the
targeting-gate change (rejected - a nav point on rails is the honest
model, and the gate change is principled: authored signature = designed
to be seen).

Difficulties: the targeting gate discovery (plan-time read missed the
in-lambda gate - the fix and its test are in this branch); Assets::get_mut
returning a change-tracking guard needing a `mut` binding
(compiler-guided one-liner).

Self-reflection: the plan step said the beacon "is aim-lockable" based on
reading the candidates query signature and stopping before the collection
lambda that gates it - reading the CONSUMER to the end would have caught
this before implementation. Feeds the existing verify-first-plan-steps
ledger entry.
