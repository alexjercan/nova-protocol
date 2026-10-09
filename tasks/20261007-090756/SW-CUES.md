# SW-CUES report

Scope: SW-CUES spec (T1-PLAN.md "SW-CUES spec (D-T2)") and TRANSIENT-GATE.md
section 6, option (a). Owned files only.

## Diffs

- crates/nova_ship/src/sections/turret_section/firing.rs:388-393
  New `let round = projectile.id();` + `commands.trigger(RoundFired { round });`
  right after the allegiance insert, inside `shoot_spawn_projectile`'s spawn
  loop (was firing.rs:335-395 in the spec; spawn block unchanged above it).
  firing.rs:407-416 New `RoundFired { #[event_target] round: Entity }`,
  `#[derive(EntityEvent, Clone, Copy, Debug)]`, documented.
- crates/nova_ship/src/sections/turret_section/mod.rs:33
  `pub use firing::RoundFired;` added next to the existing
  `use firing::shoot_spawn_projectile;`.
  mod.rs:56 `RoundFired` added to the crate's `turret_section::prelude`.
  mod.rs:202-203 Owner-requested: `BulletProjectileRenderMesh` widened from
  private to `pub(crate)` (struct and its tuple field), for sw-rounds'
  frozen_rounds.rs. No prelude export added.
- crates/nova_ship/src/sections/turret_section/render.rs:15-16,35
  `on_projectile_marker_effect` signature changed from
  `add: On<Add, TurretBulletProjectileMarker>` to `fired: On<RoundFired>`;
  body unchanged except `let projectile = fired.round;` (was `add.entity`).
- crates/nova_ship/src/sections/torpedo_section/bay.rs:553-560
  New `commands.trigger(TorpedoLaunched { torpedo });` right after the
  `TorpedoWeave` insert (which already computed `torpedo = projectile.id()`),
  inside `shoot_spawn_projectile`'s launch path.
  bay.rs:584-592 New `TorpedoLaunched { #[event_target] torpedo: Entity }`,
  `#[derive(EntityEvent, Clone, Copy, Debug)]`, documented.
- crates/nova_ship/src/sections/torpedo_section/mod.rs:46
  `pub use bay::{TorpedoBayDoorsMoved, TorpedoLaunched};` (added
  `TorpedoLaunched` to the existing export line).
  mod.rs:57 `TorpedoLaunched` added to the crate's `torpedo_section::prelude`.
- crates/nova_ship/src/sections/torpedo_section/render.rs:807-808,826
  `on_torpedo_launch_effect` signature changed from
  `add: On<Add, TorpedoProjectileMarker>` to `launched: On<TorpedoLaunched>`;
  body unchanged except `let projectile = launched.torpedo;`.
- crates/nova_ship/src/ship_audio/combat.rs:242-243,253
  `on_turret_fire_play_sfx`: `add: On<Add, TurretBulletProjectileMarker>` ->
  `fired: On<RoundFired>`; `q_projectile.get(add.entity)` ->
  `q_projectile.get(fired.round)`.
  combat.rs:298-299,315 `on_torpedo_launch_play_sfx`:
  `add: On<Add, TorpedoProjectileMarker>` -> `launched: On<TorpedoLaunched>`;
  `q_projectile.get(add.entity)` -> `q_projectile.get(launched.torpedo)`.
  No import changes: both events already reach combat.rs through the
  existing `crate::prelude::*` (same path `RailgunFired` uses).

## Test-module fixes (step 4, inside owned files)

- combat.rs `fire_round` helper (was: bare spawn + flush, relying on
  `On<Add, TurretBulletProjectileMarker>`): now spawns, flushes, then
  `world.trigger(RoundFired { round })` + flush.
- combat.rs `a_torpedo_bay_with_a_declared_launch_sound_plays_it_and_silent_without`:
  both the authored and silent arms now capture the spawned torpedo's id and
  trigger `TorpedoLaunched` (+ flush) instead of relying on the Add hook.
- combat.rs `a_ships_whole_salvo_is_one_report_and_two_ships_are_two`: each of
  the 8 bare-spawned torpedoes now triggers `TorpedoLaunched` (+ flush) in the
  spawn loop.

## Outside callers found (searched crates/ and examples/ for bare
`TurretBulletProjectileMarker` / `TorpedoProjectileMarker` spawns)

None rely on a cue. Everything outside the owned files either:
- queries the marker on rounds/torpedoes fired through the real turret/bay
  fire paths (screenshots/lesson_combat_*.rs, loop_torpedo_blast.rs,
  loop_round_types.rs read path, system_*.rs, stress_*.rs - no bare spawns of
  the marker in these), which still gets the cue because the real fire path
  now triggers the event; or
- bare-spawns the marker for unrelated behavior (damage, radar/AI targeting,
  point defense, physics, scenario loading, snapshot content) and never
  registers `on_projectile_marker_effect`, `on_turret_fire_play_sfx`,
  `on_torpedo_launch_effect`, or `on_torpedo_launch_play_sfx` in its test app,
  so it never observed the old Add-based cue either. Checked:
  crates/nova_gameplay/src/rounds.rs, crates/nova_hud/src/edge_indicators.rs,
  crates/nova_probe/src/capabilities/snapshot.rs,
  crates/nova_scenario/src/loader/lifecycle.rs,
  crates/nova_ship/src/sections/cargo_intake_section/tests.rs,
  crates/nova_ship/src/sections/torpedo_section/projectile.rs,
  crates/nova_ship/src/input/ai/{acquisition,guns,threat,torpedo}.rs,
  crates/nova_ship/src/input/{player/intent,targeting/*,point_defense/*}.rs,
  examples/screenshots/loop_round_types.rs (`spawn_comparison_rounds` supplies
  its own `Mesh3d`/`MeshMaterial3d` directly, which both the old Add observer
  and the new `on_projectile_marker_effect` already skip via the
  `q_direct_mesh.contains` early return - unaffected either way).
  None of these needs a change.
- turret_section/render.rs's own test module spawns the marker several times
  (round_render_app etc.) but those tests register `insert_projectile_render`
  / `stretch_round_tracers`, not `on_projectile_marker_effect`; unaffected.
- torpedo_section/render.rs's own test module similarly tests
  `insert_torpedo_controller_render`, not `on_torpedo_launch_effect`;
  unaffected.

## Owner-requested addition (mid-turn)

`BulletProjectileRenderMesh` in turret_section/mod.rs:202-203 widened to
`pub(crate)` (struct and field), for `frozen_rounds.rs` under SW-ROUNDS. No
prelude export. Done.

## Checks

- `cargo check -j 8 -p nova_ship --tests`: clean (one pre-existing unrelated
  warning in sections::frozen - unused `use super::*` at frozen.rs:318, not an
  owned file, not touched).
- `cargo test -j 8 -p nova_ship --lib turret_section`: `test result: ok. 83
  passed; 0 failed; 0 ignored; 0 measured; 1015 filtered out`.
- `cargo test -j 8 -p nova_ship --lib torpedo_section`: `test result: ok. 70
  passed; 0 failed; 0 ignored; 0 measured; 1028 filtered out`.
- `cargo test -j 8 -p nova_ship --lib ship_audio`: `test result: ok. 54
  passed; 0 failed; 0 ignored; 0 measured; 1044 filtered out`.
- `rustfmt --edition 2024` run on all 7 owned/touched files; reordered a few
  `use` groups (e.g. `pub use firing::RoundFired;` split onto its own line
  before `use firing::shoot_spawn_projectile;`). Re-ran all three test
  commands after formatting; same pass counts.

## Unverified / not done

- No new tests added, per spec ("No new tests. P-T8 comes later, after the
  thaw exists.").
- Did not touch `nova_probe` `snapshot.rs:1388` turret-ref read (out of scope
  for SW-CUES; that is SW-ROUNDS's item per T1-PLAN.md).
- Did not run workspace-wide tests or Clippy.
