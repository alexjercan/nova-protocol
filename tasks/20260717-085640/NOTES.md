# Notes: weapon auto-reload/regen mechanic

## What shipped

- `SectionReloadConfig` (authored) + `SectionReload` (runtime) in
  `crates/nova_gameplay/src/sections/ammo.rs`, prelude-exported.
- `tick_section_reload` system, registered in `SpaceshipSectionPlugin`
  (`sections/mod.rs`) on `FixedUpdate`.
- `reload: Option<SectionReloadConfig>` on `TurretSectionConfig` and
  `TorpedoSectionConfig`, attached alongside `SectionAmmo` at each build site
  (only when `ammo_capacity` is `Some`).
- Forgiving catalog defaults in `nova_assets/src/sections.rs` and the RON
  mirror `assets/base/sections/base.content.ron`: turrets discrete
  reload-to-full (~3s / ~2.5s), torpedo bay continuous 1-per-4s regen.
- Shakedown Run flipped `infinite_ammo: false` (RON + Rust builder), and its
  guard test inverted to pin finite ammo.

## Design decision: one timer, two behaviors

The spike (tasks/20260716-123556/SPIKE.md) wanted both discrete auto-reload
(A3) and continuous regen (A1) from one mechanism. `SectionReload` carries
`reload_time`, `rounds_per_cycle`, `only_when_empty`:

- discrete reload-on-empty = `only_when_empty: true`, `rounds_per_cycle` =
  capacity: the magazine sits until empty, then one cycle refills to full;
- continuous regen = `only_when_empty: false`, `rounds_per_cycle: 1`: a round
  trickles back every cycle whenever below capacity.

`is_reloading()` decides whether a cycle accumulates; `advance()` (the pure
core of the system) ticks `elapsed` and refills, looping so a long `dt` (or
tiny `reload_time`) completes multiple cycles exactly instead of dropping the
surplus. `progress()` is the 0..1 value the HUD readout (task 20260716-123556)
will draw a reload state from.

## Why FixedUpdate + add-only needs no ordering

`tick_section_reload` only ever grows `rounds` (toward capacity);
`shoot_spawn_projectile` only ever shrinks it. Both mutate `SectionAmmo` in the
same `FixedUpdate` schedule, but because one is add-only and the other
subtract-only, the per-tick result is order-independent - so no explicit
ordering constraint is declared. Chose FixedUpdate (not Update) so the refill
shares the fixed clock the fire systems consume on; `Res<Time>` there is the
fixed clock.

Kept the "no `SectionAmmo` = unlimited" invariant untouched: reload is attached
only inside the existing `if let Some(capacity) = config.ammo_capacity` block,
so an unlimited weapon (including any `infinite_ammo` ship) gets no reload and
every headless firing rig that never asked for ammo is unchanged.

## Difficulties

- **Test-clock quirk.** The through-the-schedule system test first failed
  because `Time<Virtual>` clamps its per-update delta to `max_delta` (0.25s by
  default), swallowing a 1s `ManualDuration` step - the magazine never
  accumulated a full cycle. The repo's fire-rate test rigs dodge this by using
  a fast weapon whose timer re-arms even under the clamp; here I raised
  `max_delta` on the inserted `Time<Virtual>` so the manual duration passes
  through deterministically. The per-step timing itself is covered by the pure
  `advance()` unit tests, which don't touch the clock. Diagnosed by reading the
  actual `left/right` assert values (0 vs 6) instead of theorizing about
  first-frame deltas.

## Tests

- Pure logic (`ammo.rs`): discrete waits-for-empty-then-full; continuous adds
  one-per-cycle and clamps; long step completes multiple cycles exactly; full
  mag stays at rest; `progress()` rises 0->1 and clamps; the scheduled system
  refills through an App.
- End-to-end recovery: `an_auto_reloading_turret_fires_again_after_running_dry`
  (fires past one magazine) and `a_regenerating_bay_rearms_and_launches_past_its_magazine`,
  each the A/B partner of an existing "caps at magazine, forever" no-reload test.
- Behavior pin: `the_new_game_player_has_finite_reloading_ammo` (inverted guard).
- Unchanged content parity: `content_ron_parity` + `content_lint_gate` green,
  so the Rust catalog and RON agree on the new `reload` fields.

## Self-reflection

The one-timer/two-behaviors shape kept the surface small and made the readout
task purely additive (it reads `progress()`). Next time, reach for the
`max_delta` clamp explanation immediately when a `ManualDuration` rig under-
advances - it has bitten the repo before (the fire rigs carry a comment about
it), so it should have been the first hypothesis, not the third.
