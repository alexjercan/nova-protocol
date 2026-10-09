# T1 plan: restore point, lifetime, rounds, blast wait, cue events

Source of truth: TRANSIENT-GATE.md rev 2 (owner approved, bound 240 s,
saved refs in nova_gameplay). This file only splits the work.

## Order

1. Main worker, first: the foundations that the others name.
   - `nova_gameplay/src/saved_refs.rs` (new): `SavedOwner { Ship(EntityId), Gone }`,
     `SavedBodyRef(EntityId)`, `SavedSectionRef { ship, section }`. They
     have serde behind the crate's `serde` feature and go in the prelude.
     The doc says why `Gone` is not an absent owner: an absent
     `ProjectileOwner` means an authorless round that its first collider
     consumes (`rounds.rs:1185-1189`).
   - The same file: `TransientFreezeFault { Unsettled(UnsettledBody), NoDurableId { label } }`
     and `SavedOwner::of`. `nova_world` gets `SectorSnapshotError::NoDurableId { label }`
     on the Failed path.
   - `nova_gameplay/src/lifetime.rs`: `SavedLifetime { total, remaining }`,
     `SavedLifetime::of`, `resumed_lifetime`. `on_insert_temp_entity` keeps
     an existing `TempEntityState`.
2. In parallel, after step 1 compiles:
   - SW-CUES (file owner: `nova_ship/src/sections/turret_section/*`,
     `nova_ship/src/sections/torpedo_section/bay.rs`,
     `nova_ship/src/sections/torpedo_section/render.rs`,
     `nova_ship/src/ship_audio/combat.rs`, plus the tests of those files).
   - SW-ROUNDS (file owner: `nova_gameplay/src/rounds.rs` and its tests,
     `nova_ship/src/sections/frozen_rounds.rs` (new), and the one `mod`
     line in `nova_ship/src/sections/mod.rs`).
   - Main worker: `nova_gameplay/src/freeze.rs` (`FreezeOwner::WorldResume`),
     the `nova_world_base/src/save/transients.rs`
     collector and restore, `WorldSaveState.transients`, the bound, progress
     and refusal resources.
3. After those: SW-REFUSE (file owner: `nova_menu/src/load_screen.rs`,
   `nova_menu/src/leave.rs` if the teardown is shared, a new
   `refuse_resumed_world`, `nova_core/src/loading_screen.rs` progress line).
4. Main worker: integration, P-T1, P-T6, P-T8 (after SW-CUES and
   SW-ROUNDS), P-T11, the mutations, and the review.

## SW-CUES spec (D-T2)

- New events in nova_ship:
  - `RoundFired { round: Entity }` in turret_section;
  - `TorpedoLaunched { torpedo: Entity }` in torpedo_section.
  
  Both use `#[derive(EntityEvent)]` with `#[event_target]` on the field, and
  both are documented.
- The turret fire path (`turret_section/firing.rs:335-395`) triggers
  `RoundFired` after its spawn. The torpedo fire path (`torpedo_section/bay.rs`,
  launch) triggers `TorpedoLaunched` after its spawn.
- Move the four observers from `On<Add, marker>` to the events:
  - `on_projectile_marker_effect` (`turret_section/render.rs`);
  - `on_turret_fire_play_sfx` (`ship_audio/combat.rs`);
  - `on_torpedo_launch_effect` (`torpedo_section/render.rs`);
  - `on_torpedo_launch_play_sfx` (`ship_audio/combat.rs`).
  
  Keep their bodies; change only how they find the entity.
- Search every test and example that spawns a bare
  `TurretBulletProjectileMarker` or `TorpedoProjectileMarker` and expects a
  cue. Update it to trigger the event, or report it.
- No new tests. P-T8 comes later, after the thaw exists.

## SW-ROUNDS spec (gate 4.1)

- `nova_gameplay/src/rounds.rs`:
  - `FrozenRoundFlight { translation, rotation, velocity, damage: ProjectileDamage, allegiance: Option<Allegiance>, owner: SavedOwner, rake_radius: Option<f32> }`,
    with serde behind the feature;
  - `freeze_round_flight(world: &World, entity: Entity) -> Result<FrozenRoundFlight, TransientFreezeFault>`.
    The pose is the easing `end` when set, otherwise `Transform`. A raking
    slug with a non-empty `armed` set is
    `Err(TransientFreezeFault::Unsettled(UnsettledBody { reason: "a raking slug is mid-body" }))`.
    The owner is `SavedOwner::of(world, projectile_owner)?`. A round with no
    `ProjectileOwner` is authorless; report how one can exist and stop
    before choosing a record for it;
  - `thaw_round_flight(flight: &FrozenRoundFlight, owner: Entity) -> impl Bundle`
    seeds both easing `start` values with the pose and adds `ResumedRound`;
  - `ResumedRound` (Component): on its first sweep step, every collider
    that the cast meets at distance zero counts as bitten, with no damage,
    and then the marker is removed. Check whether the cast reports origin
    overlaps. If not, use `shape_intersections` at the tip on that first
    step.
- `nova_ship/src/sections/frozen_rounds.rs` (new):
  - `FrozenRound { flight, source: RoundSourceType }`;
  - `RoundSourceType { Turret { render_mesh: Option<AssetRef<WorldAsset>> }, Railgun }`;
  - freeze and thaw functions that call the gameplay pair and add the
    turret or railgun art bundle without `TurretSectionPartOf`,
    `TurretSectionMuzzleEntity` or the wake (D-T2, D-T6).
  
  A render mesh that is a handle with no asset path panics with its name.
  Read the railgun slug spawn (`railgun_section/firing.rs:189-220`) and list
  what a thawed slug needs.
- Check `nova_probe` `snapshot.rs:1388`: does it read the turret refs as
  required? Report it; do not edit it.
- Tests (approved names): P-T2 `a_resumed_round_flies_on_and_expires_on_its_saved_lifetime`,
  P-T3 `a_round_resumed_inside_a_plate_does_not_bite_it_again`,
  P-T4 `a_resumed_round_never_hits_the_ship_that_fired_it`. They run in
  nova_gameplay with the resolved owner passed in. Each test names its
  mutation and the result of running it.
