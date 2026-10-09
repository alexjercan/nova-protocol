# SW-CUETEST report

Scope: the round half of P-T8 (TRANSIENT-GATE.md section 10, `a_resumed_projectile_plays_no_launch_cue`).
Owned file: `crates/nova_ship/src/sections/frozen_rounds.rs` (only file edited).

## What was added

`crates/nova_ship/src/sections/frozen_rounds.rs:126-396` - an inline
`#[cfg(test)] mod tests` at the end of the file:

- `PlaySfxCount` (test-local resource; `ship_audio::test_support::PlayedSfx`/
  `LastPlayed` are `pub(super)` to `ship_audio` and unreachable from here).
- `real_fire_app()` (:155-188) - builds the REAL production chain: the public
  `TurretSectionPlugin { render: true }` (registers the real
  `on_projectile_marker_effect`, `insert_turret_barrel_muzzle_effect`,
  `insert_projectile_render`, `shoot_spawn_projectile`) and the public
  `ShipAudioPlugin` (registers the real `on_turret_fire_play_sfx`). All four
  are `pub(super)` to `turret_section`/`ship_audio` respectively and
  unreachable from the sibling module `sections::frozen_rounds` by Rust's
  module-privacy rules, so the plugins - not a hand-rolled trigger - are the
  only way to drive the real functions.
- `spawn_ship_with_turret()` (:198-229) - a ship + one turret with
  `SectionAmmo::new(1)`, `TurretSectionInput(true)`, no aim point (fail-open
  fire) and no `SectionAnimations` (so `insert_turret_stow` never matches and
  the turret reads as always-deployed), the same fixture shape
  `turret_section/firing.rs`'s own `spawn_firing_turret` test rig uses.
- `seed_muzzle_effect_spawner()` (:239-262) - finds the muzzle's real
  `ParticleEffect` child (built by the real `insert_turret_barrel_muzzle_effect`
  observer) and inserts the `EffectSpawner` that `bevy_hanabi::tick_spawners`
  would otherwise add once the GPU pipeline compiled the effect - a
  render-world step this headless test does not run - built from that SAME
  asset's own `SpawnerSettings`, so the component shape
  `on_projectile_marker_effect` resets is the real one.
- `fire_one_real_round()` (:269-292) - runs the real `shoot_spawn_projectile`
  (via `TurretSectionPlugin`'s `FixedUpdate` registration) for several ticks;
  the one-round magazine, not the tick count, caps the fire at exactly one
  round (the same cap `a_turret_with_ammo_fires_exactly_its_magazine_then_stops`
  in `firing.rs` proves).
- `a_resumed_projectile_plays_no_launch_cue()` (:303-395) - the test itself.

## What it asserts

1. One real fire gives exactly one muzzle-flash cue, observed as
   `EffectSpawner::has_completed()` flipping from `true` (the `once`/
   no-emit-on-start spawner's rest state) to `false` (only `reset()`, which
   `on_projectile_marker_effect` calls, clears it) - and exactly one
   `PlaySfx` trigger, counted by `PlaySfxCount`.
2. The spawner is put back to its "spent" rest state (a fresh `EffectSpawner`
   from the same `SpawnerSettings`, asserted `has_completed() == true` before
   proceeding, so a no-op test is ruled out).
3. A `FrozenRound` is built directly in the test (owner = the firing ship) and
   `thaw_round`-ed. Neither counter moves: `has_completed()` stays `true`,
   `PlaySfxCount` stays `1`.
4. `insert_projectile_render` (still `On<Add, TurretBulletProjectileMarker>`,
   unchanged by SW-CUES) DOES still run on the thaw; the test asserts the
   thawed round got a render child, as the observable stand-in for "no
   `error!` was logged" - see the log-capture note below.

## Log capture

Not practical with the patterns available in this crate's tests - no test
here or in `render.rs`/`combat.rs`/`firing.rs` installs a log subscriber, and
adding one would be a new helper. Asserted the observable state instead (point
4 above: the render child exists, i.e. `insert_projectile_render`'s happy path
ran, not its `error!` branch at `turret_section/render.rs:254-259`).

## Real result

```
test sections::frozen_rounds::tests::a_resumed_projectile_plays_no_launch_cue ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1098 filtered out; finished in 0.01s
```
(`nix develop --command cargo test -p nova_ship --lib frozen_rounds -j 8`)

## Mutation

Moved `on_projectile_marker_effect` (`turret_section/render.rs:15-16,35`) and
`on_turret_fire_play_sfx` (`ship_audio/combat.rs:242-243,253`) back to
`On<Add, TurretBulletProjectileMarker>` (the exact pre-SW-CUES signatures, per
`git diff HEAD~1` on those two files), leaving every other file and every
other observer/test untouched. Backed up both files to `/tmp` first.

**Mutation result: the test still PASSED.** It does not fail under the
mutation.

```
test sections::frozen_rounds::tests::a_resumed_projectile_plays_no_launch_cue ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1098 filtered out; finished in 0.02s
```

### Why, exactly - the task's own anticipated case

This is precisely the scenario the task warned about: "the old observers
return early on a thawed round's missing components." Both old (`On<Add>`)
observers fire on `thaw_round`'s own `commands.spawn(...).insert(...)` (the
`Add` hook does not care which event model the observer used to be on), but
each one's FIRST query read on the thawed round fails, for a reason that has
nothing to do with `On<Add>` vs `On<RoundFired>`:

- `on_projectile_marker_effect` requires `&TurretSectionMuzzleEntity`
  (`turret_section/render.rs:18-21`, the query `q_projectile`). Its missing-match
  branch is `error!` + `return` at `turret_section/render.rs:51-57`, before
  `effect_spawner.reset()` at line 124 - so `has_completed()` never flips.
- `on_turret_fire_play_sfx` requires `&TurretSectionPartOf`
  (`ship_audio/combat.rs:244`, the query `q_projectile`). Its missing-match
  branch is a silent `return` at `ship_audio/combat.rs:254-256`, before the
  `commands.play_sfx_at(...)` call - so `PlaySfxCount` never increments.

`thaw_round` (`frozen_rounds.rs:96-102`, its own doc comment) never attaches
`TurretSectionPartOf` or `TurretSectionMuzzleEntity` - by design (D-T2): those
two components exist only to feed the launch cues and are not part of a
round's gun-agnostic flight state, so a thaw never had them under EITHER
observer wiring. The `On<Add>` -> `On<RoundFired>` change in SW-CUES stops the
cue from firing on the THAW's `Add` hook in the general case (any round that
DOES carry those components, e.g. a copy-constructed test fixture, would have
tripped the old observers); it is not what keeps THIS specific `thaw_round`
output silent. For this round, the attribution components' absence already
did that job under the old wiring too.

I did not weaken or reshape the test to force a failure (e.g. by attaching
`TurretSectionPartOf`/`TurretSectionMuzzleEntity` to the frozen/thawed round,
which `thaw_round` never does and would misrepresent production behavior).

### Restore

```
cmp /tmp/render.rs.bak crates/nova_ship/src/sections/turret_section/render.rs   -> byte-identical
cmp /tmp/combat.rs.bak crates/nova_ship/src/ship_audio/combat.rs                -> byte-identical
```
Re-ran the test against the restored files: passes (see "Real result" above,
re-verified after restore).

## Checks run

- `nix develop --command cargo check -p nova_ship --tests -j 8` - clean (one
  pre-existing unrelated warning at `frozen.rs:318`, not touched).
- `nix develop --command cargo test -p nova_ship --lib frozen_rounds -j 8` -
  `1 passed; 0 failed`.
- `nix develop --command cargo fmt -p nova_ship` - reformatted only
  `frozen_rounds.rs` (confirmed by diffing a pre-fmt copy of `render.rs`/
  `combat.rs` against the post-fmt files: byte-identical; `cargo fmt -p
  nova_ship -- --check` is clean).
- Did not run workspace-wide tests or Clippy.
- Did not stage or commit anything; the index was never touched.

## Unverified / flagged

- The torpedo half of P-T8 is not in this file, per the brief (no torpedo
  thaw exists yet - T2).
- **The mutation did not falsify the test** (see above). The test is real and
  passes against the real production wiring, but it does not, by itself,
  distinguish `On<RoundFired>` from `On<Add>` for THIS round shape, because
  `thaw_round`'s omission of `TurretSectionPartOf`/`TurretSectionMuzzleEntity`
  already silences the old observers too. Flagging this to the owner rather
  than reshaping the test.
