# SW-ROUNDS report

Scope: T1-PLAN.md "SW-ROUNDS spec (gate 4.1)" and TRANSIENT-GATE.md sections
4.1, 5, 10 (P-T2, P-T3, P-T4). Owned files only: crates/nova_gameplay/src/rounds.rs
(+ its test module), crates/nova_ship/src/sections/frozen_rounds.rs (new),
crates/nova_ship/src/sections/mod.rs (one `mod` line + prelude line).
Mid-turn integration note from the main worker (pub fields on FrozenRoundFlight
and FrozenRound, signatures `freeze_round`/`thaw_round`) is followed as given.

## Diffs

- crates/nova_gameplay/src/rounds.rs:193-227 New `FrozenRoundFlight { translation,
  rotation, velocity, damage: ProjectileDamage, allegiance: Option<Allegiance>,
  owner: SavedOwner, rake_radius: Option<f32> }`, all fields `pub` with a doc
  line each (main worker's note), `#[cfg_attr(feature = "serde", derive(...))]`.
- rounds.rs:233-283 New `freeze_round_flight(world: &World, entity: Entity) ->
  Result<FrozenRoundFlight, TransientFreezeFault>`. Refuses
  `Unsettled(UnsettledBody)` while a raking round's `RoundRake.armed` is
  non-empty. Panics naming the entity if `Transform`, `RoundVelocity`,
  `ProjectileDamage` or `ProjectileOwner` is missing (see "authorless round"
  below). Pose: easing `end` when `TranslationEasingState`/`RotationEasingState`
  is present, else the raw `Transform`.
- rounds.rs:296-324 New `thaw_round_flight(flight: &FrozenRoundFlight, owner:
  Entity) -> impl Bundle`. Returns transform, `Visibility::Visible`,
  `RoundVelocity`, `ProjectileDamage`, `ProjectileOwner(owner)`, `ResumedRound`,
  `TransformInterpolation`, and both easing states with `start` seeded from the
  thawed pose and `end: None`. Does NOT carry `Allegiance` or a fresh
  `RoundRake` - no `Bundle` impl exists for `Option<Component>` (confirmed by
  grep across `bevy_ecs::bundle`), so the caller (`thaw_round` /
  `thaw_round_flight`'s own caller) inserts those conditionally from
  `flight.allegiance` / `flight.rake_radius`, same as the live spawn paths do.
- rounds.rs:328-333 New `ResumedRound` (empty marker `Component`, `Reflect`,
  registered in `NovaRoundPlugin::build`).
- rounds.rs:931,945-951 `sweep_narrow` gained a `resumed: bool` parameter. At
  the top of its hit loop: if `resumed` and `hit.impact == 0.0`, remember the
  collider as bitten without charging damage and continue the walk - a wound
  already in the bite ring would otherwise be billed again.
- rounds.rs:1012,1035-1042 Same change in `sweep_raking`'s PASS ONE loop.
- rounds.rs:465,479,510 `advance_rounds`'s query gained `Has<ResumedRound>`;
  the loop removes `ResumedRound` via `try_remove` (not `remove`, which warns
  on a missing entity) on the first step a resumed round is swept, then
  threads `resumed` into both sweep calls.
  Module prelude (rounds.rs:25-29) exports `freeze_round_flight,
  thaw_round_flight, FrozenRoundFlight, ResumedRound` alongside the pre-existing
  names.

- crates/nova_ship/src/sections/frozen_rounds.rs (new, 124 lines). `FrozenRound
  { flight: FrozenRoundFlight, source: RoundSourceType }`, both fields `pub`
  with a doc line (main worker's note). `RoundSourceType { Turret { render_mesh:
  Option<AssetRef<WorldAsset>> }, Railgun }`. `freeze_round(world: &World,
  entity: Entity) -> Result<FrozenRound, TransientFreezeFault>`: calls
  `freeze_round_flight`, then reads `BulletProjectileRenderMesh` for a turret
  marker or picks `Railgun`; panics naming the round if neither marker is
  present (every `GunRoundMarker` spawn is one or the other - a bug in the
  caller, not a save-time condition), and panics naming the round if a turret
  mesh is a code-built `AssetRef::Handle` with no path (unsaveable).
  `thaw_round(commands: &mut Commands, round: &FrozenRound, owner: Entity) ->
  Entity`: spawns `thaw_round_flight`'s bundle, conditionally inserts
  `Allegiance`/`RoundRake`, then inserts the turret marker + render mesh or the
  railgun marker - without `TurretSectionPartOf`, `TurretSectionMuzzleEntity` or
  a wake (D-T2, D-T6). Both signatures match the main worker's integration
  note exactly.
- crates/nova_ship/src/sections/mod.rs:21 `pub mod frozen_rounds;` added.
  mod.rs:46-61 (prelude) `frozen_rounds::prelude::*,` inserted after
  `frozen::prelude::*,`, rewrapped by hand to match the file's existing
  wrap/order (rustfmt.toml pins edition 2021; running the bare `rustfmt`
  binary at edition 2024 by mistake first reordered the whole unrelated list -
  caught by diff review and manually reverted to the original order before
  committing to the owned one-line insertion).

- crates/nova_gameplay/src/damage.rs:59 `#[cfg_attr(feature = "serde",
  derive(serde::Serialize, serde::Deserialize))]` added on `ProjectileDamage`.
  Granted by the owner via subagent_ask, scoped to this one line, needed so
  `FrozenRoundFlight.damage` round-trips through RON. `DamageType` (the only
  field type) already carried the same derive (damage.rs:59 region, confirmed
  by read) - no further type needed serde.

## Origin-overlap evidence

avian3d 0.7.0 `SpatialQuery::cast_shape_predicate` (system_param.rs) sets
`stop_at_penetration: !config.ignore_origin_penetration`, and both
`ShapeCastConfig::DEFAULT` and `::from_max_distance` set
`ignore_origin_penetration: false` (shape_caster.rs). So a cast whose origin
already overlaps a collider reports it at `time_of_impact = 0.0` - no
`shape_intersections` fallback is needed. `ResumedRound`'s zero-distance check
in `sweep_narrow`/`sweep_raking` (rounds.rs:945,1035) relies on exactly this.
Confirmed in the live tests too: `a_round_resumed_inside_a_plate_does_not_bite_it_again`
spawns a round genuinely embedded mid-plate and freezes/thaws it there; the
resumed round's first step correctly reports the embedded collider at
distance zero and is exempted.

## Authorless-round finding

Both production round spawns always insert `ProjectileOwner` in the same
bundle as the round marker: `turret_section/firing.rs:338`
(`ProjectileOwner(*spaceship)`) and `railgun_section/firing.rs:192`
(`ProjectileOwner(spaceship)`). A live round with no `ProjectileOwner` is only
reachable from a bare test fixture (e.g. `rounds.rs` test helpers
`spawn_round`/`spawn_round_at`, which deliberately omit it to isolate what
they test). `freeze_round_flight` therefore panics naming the entity rather
than inventing a record - per TRANSIENT-GATE.md section 5's rejection of
proposal (a) (placeholder owner) and option (b) (optional owner changes
"authorless round" semantics).

## Railgun slug: what a thawed slug needs

Read `railgun_section/firing.rs:189-206` (the task asked for 189-220; the
bundle ends at 206). The live spawn bundle is: `Name::new("Railgun Slug")`,
`RailgunSlugProjectileMarker`, `ProjectileOwner(spaceship)`, `Transform`
(translation + rotation, no easing components), `RoundVelocity(slug_velocity)`,
`ProjectileDamage { amount, power: config.slug_power, kind: DamageType::Pierce
}`, `TempEntity(config.slug_lifetime)`, `Visibility::Visible`, then
conditionally `RoundRake::new(radius)` if `config.rake()` is `Some`, then
conditionally `Allegiance`. A thawed slug needs exactly: the gameplay flight
bundle (pose, velocity, damage, owner, `ResumedRound`, easing), conditionally
`Allegiance`/`RoundRake` from the frozen flight's own optional fields, plus
`Name::new("Railgun Slug")` and `RailgunSlugProjectileMarker` - no render mesh,
since a slug carries none. `thaw_round`'s `RoundSourceType::Railgun` arm does
exactly this (frozen_rounds.rs:118-120). Notably the live slug bundle carries
no `TransformInterpolation`/easing states at all (unlike a turret bullet), so
a thawed slug exercises `freeze_round_flight`'s raw-`Transform` fallback path,
not the easing-`end` path.

## Probe finding

`crates/nova_probe/src/capabilities/snapshot.rs` (not
`nova_probe/src/snapshot.rs` as stated in the task - confirmed the actual
path by reading it). Line 1388's `muzzle_aim_error_deg` reads
`TurretSectionMuzzleEntity` off the weapon SECTION entity, not off a round -
unrelated to a thawed round's component set. The per-round snapshot path is
`ordnance_record` (snapshot.rs:1577-1631), which reads `TorpedoProjectileMarker`,
`Transform`, `ProjectileOwner`, damage, `Allegiance`, `LinearVelocity`,
`TempEntity`/`TempEntityState`, and torpedo-only fields - it never reads
`TurretSectionPartOf` or `TurretSectionMuzzleEntity`. Conclusion: no probe
edit needed; a thawed round is readable by both probe paths as-is. Not
edited, per instruction.

## Tests (approved names only)

All three in `crates/nova_gameplay/src/rounds.rs` test module, under
`// ---- Freeze/thaw: a round a save keeps and a later load resumes ----`:

- `a_resumed_round_flies_on_and_expires_on_its_saved_lifetime` (rounds.rs:2978,
  P-T2, `#[cfg(feature = "serde")]`). Builds its own app (not `round_app()`,
  since `TempEntityPlugin` must be added before `App::finish()`). Burns the
  round's fuse to 0.4s of 1.0s remaining, freezes `FrozenRoundFlight` and
  `SavedLifetime`, round-trips both through RON (`ron` was already a
  dev-dependency of nova_gameplay - no addition needed), asserts pose,
  velocity, damage and owner survive the round-trip, thaws from the
  deserialized values via `thaw_round_flight` + `resumed_lifetime`, and
  asserts it is still alive after 20 frames (short of the 0.4s remaining) but
  gone after 20 more (past 0.4s, short of a fresh 1.0s total).
- `a_round_resumed_inside_a_plate_does_not_bite_it_again` (rounds.rs:3099,
  P-T3). A Kinetic round could not model this: one whose authored damage does
  not exceed the target's health is fully absorbed and despawns on its first
  hit (`pierce_remainder`'s Kinetic branch), so it can never be "mid-plate,
  still flying." Built on a Pierce round instead (`power: 1.0e6`, modeled on
  the existing `the_bite_ring_holds_every_layer_a_round_can_rest_inside`
  precedent): crawls into a plate at 50 u/s, registers exactly one bite while
  still geometrically embedded (asserted before the freeze), freezes, thaws,
  flies clear, and asserts the TOTAL damage dealt is still exactly the one
  authored bite.
- `a_resumed_round_never_hits_the_ship_that_fired_it` (rounds.rs:3177, P-T4).
  Mirrors the existing `a_round_flies_out_of_the_hull_that_fired_it`: a round
  owned by and spawned inside its own shooter's collider. Freezes and thaws it,
  then asserts it survives and the shooter takes no damage - proving the
  owner filter still holds once `ProjectileOwner` has gone through
  `SavedOwner::of` and back through `thaw_round_flight`'s `owner: Entity`
  parameter.

Run: `cargo test -j 8 -p nova_gameplay --lib rounds --features serde` -
37 passed, 0 failed, 1 ignored (pre-existing, unrelated wall-clock test).

## Mutations (gate section 10)

Both mutations are in my own owned code (`thaw_round_flight` / my own test),
not in the pre-existing `lifetime.rs`. Backup taken at
`/tmp/rounds.rs.pre-mutation-backup` before either mutation; `diff` against it
after each restore reported no differences.

- Drop the `ResumedRound` seeding: removed `ResumedRound` from
  `thaw_round_flight`'s bundle. `a_round_resumed_inside_a_plate_does_not_bite_it_again`
  failed: "a round resumed mid-plate must not bite it a second time: authored
  20, dealt 40" - exactly the doubled-bite failure the gate names. Restored;
  diff against the pre-mutation backup: identical.
- Thaw `TempEntity(total)` without state: in the P-T2 test, replaced
  `resumed_lifetime(lifetime)` with a bare `TempEntity(lifetime.total)` (so
  `on_insert_temp_entity` initializes a fresh full-duration timer instead of
  the saved remaining one). `a_resumed_round_flies_on_and_expires_on_its_saved_lifetime`
  failed: "a resumed round must expire on its saved remaining fuse, not a
  fresh total." Restored; diff against the pre-mutation backup: identical.

## Checks

- `cargo check -j 8 -p nova_gameplay -p nova_ship --tests --features
  nova_gameplay/serde`: clean (one pre-existing unrelated warning,
  `frozen.rs:318` unused import, not touched by this work).
- `cargo test -j 8 -p nova_gameplay --lib rounds --features serde`: 37 passed,
  0 failed, 1 ignored.
- `rustfmt` run on exactly the four owned files (rounds.rs, frozen_rounds.rs,
  sections/mod.rs, damage.rs); re-ran both checks above afterward, still
  clean/passing. First pass used the wrong edition flag and reflowed two
  import lists in damage.rs and one in sections/mod.rs beyond the owned
  lines - reverted those three hunks back to their original order by hand
  (diffed against HEAD to confirm only the granted/owned lines remained).

## Resolved via subagent_ask (owner decisions, not guessed)

- `BulletProjectileRenderMesh` visibility: was module-private in
  `turret_section/mod.rs`, which I do not own, blocking `frozen_rounds.rs`.
  Owner's answer: widen to `pub(crate)` (struct and field), landed by the
  SW-CUES worker concurrently (mod.rs:202-203) - I never touched that file.
- `ProjectileDamage` serde derive gap in `damage.rs` (outside my file
  ownership): owner granted me that one line, done as described above.

## Unverified

- No independent live/running-game verification of `nova_probe`'s
  `ordnance_record` against a resumed round - the probe finding above is from
  a static read only, as instructed (do not edit, so no reason to run it).
- `insert_projectile_render`'s tracer/stretch or other turret-render systems
  were read only far enough to confirm the `BulletProjectileRenderMesh`
  requirement (render.rs:237-285); any further implicit dependency those
  systems have on turret-only components was not audited, since that file is
  outside my ownership and the spec only requires a thaw not touch
  `TurretSectionPartOf`/`TurretSectionMuzzleEntity`/the wake.
