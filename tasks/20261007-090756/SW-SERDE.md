# Sub-worker report: nova_ship/nova_gameplay serde derives (finish) + FrozenFixture collider rebuild

Scope: `crates/nova_ship/`, `crates/nova_gameplay/` only. Did not touch any
other crate's uncommitted edits.

## Files changed

- `crates/nova_ship/src/input/ai/frozen.rs`
- `crates/nova_ship/src/input/ai/threat.rs`
- `crates/nova_ship/src/sections/frozen.rs`
- `crates/nova_ship/src/sections/integrity.rs`
- `crates/nova_ship/src/sections/shell_skin.rs`
- `crates/nova_ship/src/sections/skin_decor.rs`

(No `nova_gameplay` files touched this pass - the previous sub-worker had
already finished every `nova_gameplay` type in scope.)

## Task A: serde derives added

Struct -> `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` +
`#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]`. Enum -> derive
only, no `deny_unknown_fields` (matches the prior sub-worker's pattern, e.g.
`AIBehaviorState`, `RailgunCharge`).

| Type | Kind | Path:line (post-edit) |
| --- | --- | --- |
| `FrozenAI` | struct | `nova_ship/src/input/ai/frozen.rs:25` |
| `FrozenFixture` | struct | `nova_ship/src/sections/frozen.rs:48` |
| `FrozenFixtureKind` | enum | `nova_ship/src/sections/frozen.rs:57` |
| `FrozenSection` | struct | `nova_ship/src/sections/frozen.rs:70` |
| `FrozenWreckFragment` | struct | `nova_ship/src/sections/integrity.rs:168` |
| `FrozenWreckSection` | struct (private) | `nova_ship/src/sections/integrity.rs:177` |

`AIThreat.attacker: Option<Entity>` (`nova_ship/src/input/ai/threat.rs:69`)
got `#[cfg_attr(feature = "serde", serde(skip))]` plus a doc line: "Skipped by
serde: an `Entity` does not survive a save, so a thaw always starts with
`None` (`freeze_ai` already clears it this way too)."

### Entity / Handle / Instant / avian-physics fields found

None beyond the one already named in the task (`AIThreat.attacker`, handled
above). Every other field transitively reachable from `FrozenFixture`,
`FrozenSection`, `FrozenAI`, `FrozenWreckFragment`/`FrozenWreckSection` is
either a plain value, a bevy `Name`/`Transform` (both already serde-capable
under `bevy/serialize`, which `nova_ship/serde` and `nova_gameplay/serde`
both pull in), `EntityId` (already derives `Serialize`/`Deserialize`
unconditionally, `nova_events/src/lib.rs:67`), or a type this crate already
serde-derives (`SectionConfig`, `Health`, `SectionAmmo`, etc.). The one
avian type that was present (`Collider`) is removed, not skipped - see Task
B.

## Task B: FrozenFixture.collider deletion + DecorColliderSize

- `crates/nova_ship/src/sections/frozen.rs:48-53` - `FrozenFixture.collider`
  field deleted.
- `crates/nova_ship/src/sections/frozen.rs:59-63` -
  `FrozenFixtureKind::Decor` gained `collider_size: Vec3`.
- `crates/nova_ship/src/sections/frozen.rs:111-149` (`freeze_fixture`) - no
  longer reads the live `Collider` component; reads the new
  `DecorColliderSize` component for the decor arm only (plate's collider is
  rebuilt from the `ShellShape` it already carries). Panics if a decor
  fixture carries no `DecorColliderSize`.
- `crates/nova_ship/src/sections/frozen.rs:227-241` (`spawn_frozen_fixture`)
  - passes no collider to `frozen_plate_body`; passes `collider_size` to
    `frozen_decor_body`.
- `crates/nova_ship/src/sections/shell_skin.rs:665-677`
  (`frozen_plate_body`) - signature drops `collider: Collider`; body now
  builds `plate_collider(shape.volume())` itself.
- `crates/nova_ship/src/sections/skin_decor.rs:559-564` - new
  `DecorColliderSize(pub Vec3)` component, next to `ShipDecorMarker`, with a
  doc comment explaining why (avian `Collider` doesn't serialize).
- `crates/nova_ship/src/sections/skin_decor.rs:577-586` (`decor_body`, fresh
  spawn) - inserts `DecorColliderSize(fixture.collider)` alongside
  `ShipDecorMarker`.
- `crates/nova_ship/src/sections/skin_decor.rs:593-614`
  (`frozen_decor_body`) - signature drops `collider: Collider`, takes
  `collider_size: Vec3`; inserts `DecorColliderSize(collider_size)`
  alongside `ShipDecorMarker`; body now builds `decor_collider(collider_size)`
  itself.
- Deleted nothing else: `frozen_plate_body`/`frozen_decor_body` themselves
  stayed (still the thaw-side constructors `spawn_frozen_fixture` calls);
  neither became unused. No other now-dead helper was found.
- Doc comments on `frozen_plate_body`, `frozen_decor_body`, and the
  `freeze_fixture` panic doc updated to stop naming the removed `collider`
  field / drop the stale `Collider` mention.
- Removed the now-unused `use avian3d::prelude::Collider;` import from
  `frozen.rs` (nothing else in that file reads `Collider` once the field is
  gone).

### Round-trip test assertion - NOT added, reporting instead

Per the task's own fallback instruction ("If no existing test covers
fixtures, stop and report rather than adding one"): I searched
`crates/nova_ship/src` for `freeze_fixture`, `FrozenFixture`,
`frozen_plate_body`, and `frozen_decor_body` outside `frozen.rs`,
`shell_skin.rs`, `skin_decor.rs` themselves, and in every `tests.rs`/`mod
tests` block in the crate. The only existing freeze/thaw test that calls
`freeze_section`/`thaw_section` is
`crates/nova_ship/src/sections/railgun_section/tests.rs:425` (`a_railgun_thawed_mid_charge_resumes_its_bolt`),
and its fixture ship has no `ShipSkinMarker`/`ShipDecorMarker` children at
all - `freeze_fixtures` on it always returns an empty `Vec`, so it exercises
`FrozenSection` but never touches a `FrozenFixture`. There is no dedicated
`frozen.rs`/`shell_skin.rs`/`skin_decor.rs` test file either. So no existing
test covers a plate or decor fixture round trip; I did not add a new test
function. A fixture/`DecorColliderSize` round-trip assertion is new test
surface and needs owner sign-off on where it should live (a new
`frozen.rs` test module vs. extending `shell_skin.rs`'s or
`skin_decor.rs`'s existing `mod tests`).

## Checks

1. `nix develop --command cargo check -p nova_gameplay -p nova_ship --features nova_ship/serde,nova_gameplay/serde --tests --message-format short -j 4`
   -> `Finished `dev` profile [optimized + debuginfo] target(s) in 12.74s` (no errors/warnings besides the pre-existing `proc-macro-error2` future-incompat notice). PASS.
   Also ran default-feature (`serde` off) `cargo check -p nova_ship --tests` to confirm the unconditional `FrozenFixture`/`DecorColliderSize` change doesn't break the non-serde build: same clean `Finished`. PASS.
2. `nix develop --command cargo test -p nova_ship --features serde --lib frozen -j 4`
   -> ```
   running 3 tests
   test input::player::wheel::tests::a_frozen_or_suspended_flight_takes_no_zoom ... ok
   test physics::pd_controller::tests::moderate_spin_despins_with_frozen_command ... ok
   test physics::pd_controller::tests::fast_roll_despins_with_frozen_command ... ok

   test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1099 filtered out; finished in 0.66s
   ```
   These 3 are every test whose *name* substring-matches "frozen" in the
   crate; none of them is a `FrozenFixture`/fixture round trip (consistent
   with the "no existing test covers fixtures" finding above). PASS, 0
   failures.
3. `nix develop --command cargo fmt -p nova_ship -p nova_gameplay`
   -> ran clean; a follow-up `cargo fmt -p nova_ship -p nova_gameplay -- --check` produced no diff. PASS.

## Fixture round-trip test

Scope: `crates/nova_ship/src/sections/frozen.rs` only. No helper needed a
`pub(crate)` bump - `plate_body`, `decor_body`, `SkinPlate`, `StyleFixtureConfig`,
`FixturePlacement` and `shell_shape::prelude::FULL` were already `pub` (or, for
`FrozenFixture`/`freeze_fixtures`/`spawn_frozen_fixture` themselves, already
reachable as private items of the same module the new test lives in).

Added the one test Task B's own report flagged as missing sign-off for:
`crates/nova_ship/src/sections/frozen.rs:328`
(`a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with`, in a new
`#[cfg(test)] mod tests` at the end of the file, gated `#[cfg(feature =
"serde")]`).

It spawns one plate (`shell_skin::plate_body`) under a bare section entity and
one decoration (`skin_decor::decor_body`, authored `collider: Vec3::new(0.3,
1.25, 0.7)`) under that plate - the same parent/child nesting
`spawn_ship_skin`'s own doc comment states production uses - on a bare `World`,
with no app or plugin. It captures each original entity's `Collider::aabb(Vec3::ZERO,
Quat::IDENTITY)`, calls the real `freeze_fixtures(&world, section)`, round-trips
the resulting `Vec<FrozenFixture>` through `ron::to_string`/`ron::from_str`, then
thaws it by calling the real `spawn_frozen_fixture` under a fresh root entity via
`world.commands()...with_children(...)` + `world.flush()`. It asserts:

1. The thawed decoration's `DecorColliderSize` equals `DecorColliderSize(Vec3::new(0.3,
   1.25, 0.7))` exactly (`assert_eq!` at `frozen.rs:403-408`).
2. The thawed plate's and the thawed decoration's `Collider::aabb(...)`
   `min`/`max` each equal the corresponding original entity's `min`/`max`
   exactly (four `assert_eq!`s).

### Check 1 - pass

```
running 1 test
test sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1102 filtered out; finished in 0.00s
```

### Check 2 - deliberate fail, then restore

Temporarily changed `freeze_fixture`'s decor arm (`frozen.rs`) from
`collider_size: *collider_size` to `collider_size: *collider_size + Vec3::ONE`:

```
running 1 test
test sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with ... FAILED

---- sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with stdout ----

thread 'sections::frozen::tests::a_frozen_plate_and_decor_thaw_with_the_colliders_they_froze_with' (3046577) panicked at crates/nova_ship/src/sections/frozen.rs:402:9:
assertion `left == right` failed: the authored collider size did not survive the freeze/thaw round trip
  left: DecorColliderSize(Vec3(1.3, 2.25, 1.7))
 right: DecorColliderSize(Vec3(0.3, 1.25, 0.7))

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1102 filtered out; finished in 0.02s
```

Restored the line to `collider_size: *collider_size` exactly, re-ran check 1,
and it passed again (`ok. 1 passed`). `git diff -- crates/nova_ship/src/sections/frozen.rs`
afterward shows `freeze_fixture` matching Task B's own diff exactly (the
`DecorColliderSize` read, the `collider_size: *collider_size` line, nothing
else) - the deliberate break left no trace.

### Check 3

`nix develop --command cargo fmt -p nova_ship` ran clean; it only reformatted
the new test (line wraps on the two multi-field struct literals), nothing in
the restored `freeze_fixture`.
