# SW-SHED: shed fixture freeze/thaw (TRANSIENT-GATE.md 4.4, T3 names)

## Step 0 answers

- **Durable style key**: `ShipStyle(pub Option<String>)`
  (`crates/nova_ship/src/sections/skin_style.rs:577`), a `Component` on the
  ship root naming a `ShipStyleConfig` by id out of `GameStyles`
  (`skin_style.rs:563,583`). Resolved by `ShipStyle::resolve` via
  `GameStyles::get_style`. The record's `style` field is `Option<String>`
  (the inner id), not a new `ShipStyleId` type - no such type exists.

- **How a shed plate gets its colour today**: `dress_skin_plate`
  (`shell_skin.rs:1050-1078`) fires on `On<Add, ShipSkinMarker>` and resolves
  the style by walking UP `ChildOf` from the plate via the private
  `worn_style` (`shell_skin.rs:1112-1125`) to find the nearest ancestor's
  `ShipStyle`. **A shed plate has no ancestor**: `shed_dead_fixtures`
  (`fixture.rs:353`, pre-existing) removes the fixture's only `ChildOf` as
  part of shedding it, so by the time anything freezes the drifting debris,
  there is nothing left to walk. Per the gate's instruction, I stamped the
  style at shed time instead of reading it at freeze time - see below.

- **Shed fixture's grace**: its own `TempEntity(SHED_LIFETIME_SECS)` /
  `TempEntityState`, inserted at shed time (`fixture.rs:364`, pre-existing,
  constant `SHED_LIFETIME_SECS = 12.0` at `fixture.rs:40`). **Not**
  `ChunkGrace` - that type (`nova_gameplay::integrity::chunk::ChunkGrace`) is
  the carved-chunk/detached-piece re-collide timer; a shed fixture is
  "kinematic, colliderless, and untouchable for the seconds it lives" per
  `fixture.rs:214-221`, the opposite case `ChunkGrace` documents itself
  against (`fixture.rs` doc, same block). `nova_gameplay::prelude::SavedLifetime::of`
  reads a `TempEntity`'s authored total and remaining seconds
  (`crates/nova_gameplay/src/lifetime.rs:85-95`).

- **Components on a shed fixture holding an `Entity` or a runtime handle**:
  `ShedFixtureMarker(pub Entity)` (`fixture.rs:190`, attribution only - names
  the section it came off, a dangling handle once that section despawns, by
  design). No other handle-bearing component lands on the fixture itself: a
  decor's `WorldAssetRoot(Handle<Scene>)` is inserted by `dress_skin_decor`
  (`skin_decor.rs:639-652`) from the `AssetRef` on `ShipDecorMarker`, rebuilt
  fresh on every dress rather than carried; a plate's `SkinSurfaceMarker`
  mesh/material children (`Mesh3d`/`MeshMaterial3d` handles) live on CHILD
  entities, not the fixture itself, and are likewise rebuilt fresh by
  `dress_skin_plate` on thaw - neither is part of the frozen record, matching
  `freeze_fixture`'s existing `is_fixture` filter.

## Implementation

### `fixture.rs` (stamp at shed time)

No new type: reused the existing `ShipStyle` component as the stamp, since
`shell_skin.rs`'s own `worn_style` already checks the entity itself before
walking its ancestors, so the SAME logic that finds a style on an ancestor
also finds it on the shed fixture once stamped on its own entity.

- `fixture.rs:194-216`: new private `ancestor_ship_style(start, q_parents,
  q_style) -> ShipStyle`, the same ancestor walk as `worn_style`
  (re-implemented, not called: `worn_style` is private to `shell_skin.rs` and
  outside my file ownership), returning an owned `ShipStyle` (`ShipStyle(None)`
  when no ancestor carries one).
- `fixture.rs:294`: `shed_dead_fixtures` gains a `q_style: Query<&ShipStyle>`
  parameter.
- `fixture.rs:342-345`: `let style = ancestor_ship_style(fixture, ...)`,
  computed BEFORE the `try_remove::<(ChildOf, Collider)>()` command is even
  queued (reads the live `ChildOf` chain while it still exists).
  `fixture.rs:369`: `style` added to the existing `try_insert` bundle.

### `frozen.rs` (the T3 shed-fixture pair)

- `frozen.rs:345-371`: `pub struct FrozenShedFixture { pub fixture:
  FrozenFixture, pub style: Option<String>, pub translation: Vec3, pub
  rotation: Quat, pub linear: Vec3, pub angular: Vec3, pub grace:
  Option<f32> }` - matches the gate's section 4.4 sketch and the T3-names
  decision exactly. Fields are `pub` (unlike `FrozenFixture`'s own, which stay
  module-private): the save collector outside this crate needs `record.grace`
  directly to build a `SavedLifetime` and insert `resumed_lifetime`, the same
  way it reads `FrozenRound`'s public fields.
- `frozen.rs:388-437`: `pub fn freeze_shed_fixture(world: &World, entity:
  Entity) -> Result<FrozenShedFixture, TransientFreezeFault>`. Its own entry
  (does not call `freeze_fixture`): skips the `HealthZeroMarker` refusal on
  `entity` itself (a shed fixture at zero health has no `ChildOf` left to
  leave, so the normal "awaiting its own shed" refusal would block it
  forever), but still calls the existing `freeze_fixtures` for `entity`'s OWN
  children (a greeble still bolted to a shed plate), which DOES preserve that
  refusal for them - they can still legitimately be mid-shed on their own.
  Panics naming the entity if `Transform`, `Health`, `ShipStyle`,
  `LinearVelocity` or `AngularVelocity` is missing. `grace` comes from
  `SavedLifetime::of(world, entity).map(|l| l.remaining)`.
- `frozen.rs:461-513`: `pub fn thaw_shed_fixture(commands: &mut Commands,
  record: &FrozenShedFixture) -> Entity`. Rebuilds via `frozen_plate_body`/
  `frozen_decor_body` exactly as `spawn_frozen_fixture` does, with `ShipStyle`
  riding in the SAME initial `commands.spawn((...))` bundle as the
  `ShipSkinMarker`/`ShipDecorMarker` - not a follow-up `.insert()` - because
  the `Add<ShipSkinMarker>` observer that dresses a plate fires the moment the
  bundle lands, and a thawed fixture has no ancestor to walk, so the style has
  to already be on the entity for `dress_skin_plate` to find it. No asset or
  material access needed outside `Commands`: the fresh `Collider` for
  `CenterOfMass` is read back via one `entity.queue(|mut entity: EntityWorldMut|
  ...)` closure, the same deferred pattern `thaw_section` already uses for its
  animation/hinge resolve (`frozen.rs:423-430` in the existing code) - no
  signature change needed, so no stop. `entity.remove::<Health>()` strips
  `Health` (present because `frozen_plate_body`/`frozen_decor_body` require
  it as a parameter): a shed fixture can never take another hit once its
  collider is gone, and inserting a fresh `Health` on a brand-new entity
  would be exactly the "this just happened" shape the gate's D-T2 reasoning
  warns about for rounds. No `TempEntity`/lifetime insert, no ammo/section
  state - matches the brief exactly.
- `frozen.rs:16-47`: import additions (avian3d velocity/body/collider types,
  `SavedLifetime`/`TransientFreezeFault` from `nova_gameplay::prelude`,
  `fixture::prelude::ShedFixtureMarker`, `skin_style::ShipStyle`) and the
  prelude export (`freeze_shed_fixture`, `thaw_shed_fixture`,
  `FrozenShedFixture` added alongside the existing `freeze_section`/
  `thaw_section`/`FrozenFixture`/`FrozenSection`).

### Test

`frozen.rs:626` `a_resumed_shed_fixture_keeps_its_art_style_and_grace`:
builds a real styled ship (`ShipStyle(Some("raider"))` on a root, a
`GameStyles` resource with a distinctive dyed `SurfaceFinish`) with
`ShipSkinPlugin { render: true }` + `NovaHealthPlugin` + `TempEntityPlugin` +
a seeded `WyRand`, spawns a real plate and a real decor fixture as children
of one section via `plate_body`/`decor_body`, kills both through the REAL
`HealthApplyDamage` -> `on_damage` -> `shed_dead_fixtures` path (one
`app.update()`, mirroring `fixture.rs`'s own `kill()` helper), then for each:
freezes, RON round-trips, despawns the original, thaws, and asserts collider
bounds/size, the stamped style's colour on a dressed surface, grace and pose
equality, the placeholder `ShedFixtureMarker`, and no `Health`.

## Verification (real output)

```
$ nix develop --command cargo test -p nova_ship --lib --features serde frozen -j 8
running 8 tests
test sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with ... ok
test sections::frozen::tests::a_resumed_shed_fixture_keeps_its_art_style_and_grace ... ok
test input::player::wheel::tests::a_frozen_or_suspended_flight_takes_no_zoom ... ok
test sections::frozen_rounds::tests::a_resumed_projectile_plays_no_launch_cue ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_keeps_its_target_arming_and_cold_launch ... ok
test physics::pd_controller::tests::moderate_spin_despins_with_frozen_command ... ok
test physics::pd_controller::tests::fast_roll_despins_with_frozen_command ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1105 filtered out; finished in 0.77s
```

```
$ nix develop --command cargo test -p nova_ship --lib fixture -j 8
running 15 tests
test sections::skin_decor::tests::the_reach_of_a_fixture_is_what_it_would_take_alone ... ok
test sections::skin_decor::tests::a_plate_takes_one_piece_and_the_first_fixture_wins ... ok
test sections::shell_skin::tests::a_plate_spawns_as_a_destructible_fixture_and_not_a_section ... ok
test sections::damage_cracks::tests::a_fixture_mesh_is_never_captured ... ok
test sections::fixture::tests::a_spent_fixture_comes_off_the_ship_still_wearing_its_own_art ... ok
test sections::fixture::tests::a_shed_fixture_keeps_the_place_it_was_standing ... ok
test sections::fixture::tests::a_shed_fixture_leaves_carrying_the_ships_motion ... ok
test sections::fixture::tests::a_fixture_is_shed_once_and_then_left_alone ... ok
test sections::fixture::tests::shed_cladding_tumbles_about_the_shape_it_was_wearing ... ok
test sections::fixture::tests::shed_cladding_takes_no_colliders_with_it ... ok
test sections::fixture::tests::a_plate_killed_by_the_tick_comes_off_in_that_tick ... ok
test sections::fixture::tests::a_spent_fixture_comes_off_even_with_no_global_rng ... ok
test sections::fixture::tests::a_hull_stripped_all_at_once_sheds_over_several_ticks_and_loses_nothing ... ok
test sections::fixture::tests::a_frame_that_banks_several_fixed_steps_still_pays_for_one_frame ... ok
test sections::fixture::tests::a_shed_plate_turns_about_its_seat_and_not_about_the_hull_it_left ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1089 filtered out; finished in 0.04s
```
(all 15 pre-existing, no regression from the new `q_style` query parameter.)

```
$ nix develop --command cargo check -p nova_world_base --tests -j 8
Finished `dev` profile [optimized + debuginfo] target(s) in 17.70s
```
(clean, no warnings printed beyond the pre-existing `proc-macro-error2` future-incompat notice unrelated to this change.)

`nix develop --command cargo fmt -p nova_ship` run; diff is confined to
`crates/nova_ship/src/sections/{fixture.rs,frozen.rs}` (`git diff --stat`:
`fixture.rs | 31 ++`, `frozen.rs | 586 +++...`).

## Notes for the main worker / mutation hint

- Mutation to prove the new test actually exercises the stamp: in
  `fixture.rs`'s `ancestor_ship_style`, change the `Ok(style) =>
  style.clone()` arm's early-return to instead fall through and keep walking
  past the first match (i.e. always return `ShipStyle(None)` from the loop,
  or delete the `style` line from the `try_insert` tuple in
  `shed_dead_fixtures`). Either mutation makes the thawed plate dress with
  the engine's default colour instead of the style's dye, and the new test's
  "wears_the_dye" assertion fails.
- One encountered-and-resolved compile error outside my ownership during this
  session: `crates/nova_ship/src/sections/frozen_rounds.rs:446` referenced a
  not-yet-renamed `TorpedoSectionSpawnerEntity` (should be
  `TorpedoSectionSpawnerMarker`) while another worker was mid-edit on
  `torpedo_section`. I did not touch it; it resolved itself on retry a minute
  later and all builds above are from the resolved state.
- I did not stage or commit anything; `fixture.rs` and `frozen.rs` are left
  modified in the working tree per instructions.

## Round 2

Scope: TRANSIENT-GATE.md section 12, "T2/T3 worker deviations (owner,
2026-10-09 ...)", the two `FrozenShedFixture` items.

### Claim 1: `FrozenShedFixture.grace` is deleted

- Evidence: `crates/nova_ship/src/sections/frozen.rs:344-359` (struct, before:
  had a `pub grace: Option<f32>` field and a doc paragraph naming
  `SHED_LIFETIME_SECS`); `frozen.rs:384-436` (`freeze_shed_fixture`, before:
  filled `grace` from `SavedLifetime::of(world, entity)`); `frozen.rs:457`
  (`thaw_shed_fixture` doc, before: named `record.grace`).
- Change: the field, its fill, and the three doc mentions of `grace` or
  `SHED_LIFETIME_SECS` in those two functions' docs are gone. The struct now
  carries only `fixture`, `style`, `translation`, `rotation`, `linear`,
  `angular`. The doc on `thaw_shed_fixture` now says the save collector
  inserts the countdown from its own `FrozenTransient::lifetime`, the field
  that already exists on `nova_world_base::save::transients::FrozenTransient`
  (`crates/nova_world_base/src/save/transients.rs:49-54`) and that
  `SavedLifetime::of` fills at freeze time
  (`crates/nova_gameplay/src/lifetime.rs:85-95`, read by
  `freeze_transients`, `crates/nova_world_base/src/save/transients.rs:206-208`).
- Blast radius: `FrozenShedFixture` is not yet a variant of
  `FrozenTransientType` (the collector's `TODO(20261007-090756)` at
  `nova_world_base/src/save/transients.rs:182-189` still panics on
  `TransientKind::ShedFixture`), so no other file reads `.grace`. Confirmed
  with `grep -rn "FrozenShedFixture" crates` outside this report: only
  `frozen.rs` itself.

### Claim 2: the test keeps a lifetime check against the collector's own pattern

- Evidence (before): `frozen.rs:738-743` read `SavedLifetime::of(...).remaining`
  into `plate_grace_before`/`decor_grace_before` and asserted
  `frozen_plate.grace == Some(plate_grace_before)` at `frozen.rs:816-817` -
  both gone with the field.
- Change: `a_resumed_shed_fixture_keeps_its_art_style_and_grace`
  (`frozen.rs:622`, name unchanged) now:
  - reads the full `SavedLifetime` on the LIVE fixture before freeze
    (`plate_lifetime_before`/`decor_lifetime_before`, `frozen.rs:738-741`);
  - after `thaw_shed_fixture`, inserts `resumed_lifetime(plate_lifetime_before)`
    / `resumed_lifetime(decor_lifetime_before)` on the thawed entity
    (`frozen.rs:765-783`), the same bundle
    `nova_world_base::save::transients::spawn_resumed` inserts on every other
    resumed transient (`nova_world_base/src/save/transients.rs:419-425`:
    `commands.entity(entity).insert(resumed_lifetime(transient.lifetime))`);
  - asserts `SavedLifetime::of(thawed) == lifetime_before` for both
    (`frozen.rs:818-827`).
  - added one import, `resumed_lifetime`, to the test's local
    `nova_gameplay::prelude` use (`frozen.rs:626-628`).
- Named mutation (not run): swap either `resumed_lifetime(plate_lifetime_before)`
  / `resumed_lifetime(decor_lifetime_before)` call for a fresh countdown
  (e.g. `resumed_lifetime(SavedLifetime { total: plate_lifetime_before.total,
  remaining: plate_lifetime_before.total })`, the "thaw ignores the lifetime"
  shape named in the brief) and the new `SavedLifetime::of(thawed) ==
  lifetime_before` assertion fails: the thawed entity's `remaining` comes back
  equal to `total` instead of the saved partial countdown.

### Item 3: task/report citations in comments

- Checked `frozen.rs` and `fixture.rs` for the listed citation shapes
  ("gate", "D-T2", "section 4.4", "TRANSIENT-GATE", "SCOUT-*") and for any
  `other_file.rs:NNN` line-number reference, with
  `grep -n -iE "gate|D-T[0-9]|SCOUT|TRANSIENT-GATE|[a-zA-Z_]+\.rs[`]?[,:]?[0-9]+" frozen.rs fixture.rs`.
  None of the committed comments in either file carry a task id, a report
  filename, or a line-number reference into another file - the predecessor's
  own citations of that shape lived only in `SW-SHED.md`, not in the code.
  `fixture.rs`'s comments on `ancestor_ship_style` and `shed_dead_fixtures`
  (`fixture.rs:194-215`, `fixture.rs:217-285`) already name the item (`ChildOf`,
  `HealthZeroMarker`, `NovaRoundSystems`, `PhysicsSystems::Last`, and so on)
  rather than a line number, so nothing there needed a change. No edit was
  made to `fixture.rs` in this round.

### Test output (real, both green on a settled build)

```
$ nix develop --command cargo test -p nova_ship --lib --features serde frozen -j 8
running 8 tests
test sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with ... ok
test sections::frozen::tests::a_resumed_shed_fixture_keeps_its_art_style_and_grace ... ok
test input::player::wheel::tests::a_frozen_or_suspended_flight_takes_no_zoom ... ok
test sections::frozen_rounds::tests::a_resumed_projectile_plays_no_launch_cue ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_with_a_frozen_target_keeps_its_last_known_position ... ok
test sections::torpedo_section::frozen::tests::a_resumed_torpedo_keeps_its_target_arming_and_cold_launch ... ok
test physics::pd_controller::tests::moderate_spin_despins_with_frozen_command ... ok
test physics::pd_controller::tests::fast_roll_despins_with_frozen_command ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1105 filtered out; finished in 0.68s
```

```
$ nix develop --command cargo test -p nova_ship --lib fixture -j 8
running 15 tests
... (all 15 pre-existing fixture tests) ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1089 filtered out; finished in 0.03s
```

`nix develop --command cargo fmt -p nova_ship` run; diff stays confined to
`crates/nova_ship/src/sections/frozen.rs` (`fixture.rs` carries only the
predecessor's unchanged `+31` from Round 1 - this round touched no line of
it).

### Unverified

- `cargo check -p nova_world_base --tests` - SKIPPED on the owner's mid-turn
  instruction: the main worker is editing `nova_world_base` and it will not
  compile for a while. Not run this round.
- Both `nova_ship` checks above FAILED to compile on the first several
  attempts in this round (errors in `torpedo_section/frozen.rs` and
  `frozen_rounds.rs`: missing `controller_health`/`thruster_health` fields,
  a `&Entity` pattern mismatch, a `.copied().copied()` double-call) - all in
  files this task explicitly says not to touch, and all from another
  worker's in-progress edit on the same T2/T3 worker-deviations entry. The
  green runs above are from a build taken a few minutes later once that
  edit settled; I made no change to any file outside
  `crates/nova_ship/src/sections/frozen.rs`.
- Whether `FrozenShedFixture` (and its thaw) is wired into
  `nova_world_base`'s `FrozenTransientType`/`spawn_resumed` is outside this
  round's scope and still shows as the standing `TODO(20261007-090756)` at
  `nova_world_base/src/save/transients.rs:182-189`.
