# Weapon reload/regen mechanic so finite ammo is non-terminal

- STATUS: CLOSED
- PRIORITY: 34
- TAGS: v0.7.0,weapons,spike

## Goal

Finite ammo is technically supported (`SectionAmmo { rounds, capacity }`,
one pool per weapon section) but effectively never used: with no way to
replenish, running dry permanently disables the weapon, so almost every
scenario sets `infinite_ammo: true` and the whole ammo/readout system stays
dormant. Add a cheap replenishment mechanic so a spent magazine comes back on
its own, turning magazine size from a softlock risk into a fire-pacing knob -
the thing that makes finite ammo safe to turn on broadly.

Direction (from the spike): **auto-reload on empty/idle, timer-driven, no new
input binding**, with a continuous-trickle degenerate setting available. Refill
uses the documented seam (`rounds = capacity` on reload complete, or
`rounds += k` per tick). Keep the invariant "no `SectionAmmo` component =
unlimited ammo" so `infinite_ammo` scenarios and every headless firing test are
unchanged. Default catalog weapons (turret 150/500 rounds, torpedo 6) to
generous capacity + short auto-reload so dry is a brief cadence beat, never a
death. Prove it by flipping `shakedown_run` from `infinite_ammo: true` to
finite + auto-reload and confirming the weapon recovers.

Expose a `Reloading`/progress state that the diegetic ammo readout can read -
that is the sibling task (20260716-123556) that surfaces it on the ring/bar.

## Steps

- [x] Add a `SectionReload` component in
  `crates/nova_gameplay/src/sections/ammo.rs` (prelude-exported alongside
  `SectionAmmo`), holding the reload descriptor + runtime progress:
  `reload_time: f32` (seconds per cycle), `rounds_per_cycle: u32` (rounds a
  completed cycle restores, clamped to `capacity`), `only_when_empty: bool`
  (true = discrete auto-reload that starts only at 0 rounds = A3; false =
  continuous regen whenever below capacity = A1), and runtime `elapsed: f32`.
  Add a ctor and a `progress(&self) -> f32` = `(elapsed / reload_time).clamp(0,1)`
  for the HUD. Derive `Component, Clone, Copy, Debug, Reflect`.
- [x] Add a `tick_section_reload` system in `ammo.rs` over
  `Query<(&mut SectionAmmo, &mut SectionReload)>` + `Res<Time>`: if
  `rounds >= capacity` reset `elapsed = 0`; else if `only_when_empty && rounds > 0`
  reset `elapsed = 0` (waiting to run dry); else advance `elapsed += dt` and on
  `elapsed >= reload_time` do `rounds = (rounds + rounds_per_cycle).min(capacity)`
  and `elapsed -= reload_time`. Pure timer + refill on the documented
  `rounds = capacity` seam; adds rounds only, never removes.
- [x] Register the type and schedule the system in
  `SpaceshipSectionPlugin::build` (`sections/mod.rs:121`, next to the existing
  `register_type::<ammo::SectionAmmo>()`): `register_type::<ammo::SectionReload>()`
  and `add_systems(FixedUpdate, tick_section_reload)`. FixedUpdate so refill
  shares the fixed clock the fire systems (`shoot_spawn_projectile`) consume on;
  `Res<Time>` there is the fixed clock. Ordering vs the shoot systems is not
  required (add-only vs consume-only on the same component in the same schedule);
  note this in a comment.
- [x] Add reload config to `TurretSectionConfig`
  (`turret_section.rs:33`/Default `:160`): a single optional
  `reload: Option<SectionReloadConfig>` (or flat `reload_time`/`rounds_per_cycle`/
  `only_when_empty` fields), `serde(default, skip_serializing_if=...)`, Default
  `None`. Attach `SectionReload` where the magazine is built
  (`turret_section.rs:553`, inside the existing
  `if let Some(capacity) = config.ammo_capacity` block) so a weapon with no
  magazine (unlimited / `infinite_ammo`) never gets a reload either - the
  "no `SectionAmmo` = unlimited" invariant is preserved untouched.
- [x] Mirror the same config field + attach in `TorpedoSectionConfig`
  (`torpedo_section/mod.rs:115`/Default `:137`) and its build site
  (`torpedo_section/mod.rs:471`, inside the `ammo_capacity` block).
- [x] Set forgiving catalog defaults in `crates/nova_assets/src/sections.rs`:
  `better_turret_section` (500 rounds, `:167`) and `light_turret_section`
  (150 rounds, `:228`) get a discrete on-empty full reload
  (`only_when_empty: true`, `rounds_per_cycle: capacity`, `reload_time` ~2.5-3s -
  tune by feel); `torpedo_section` (6 rounds, `:259`) gets slow individual
  rearm (`only_when_empty: false`, `rounds_per_cycle: 1`, `reload_time` ~4s) so a
  spent bay steadily rearms. Mirror in the RON catalog
  `assets/base/sections/base.content.ron` (turret `:86`/`:147`, torpedo `:181`).
- [x] Flip the proof scenario off infinite ammo: set `infinite_ammo: false` in
  both `assets/base/scenarios/shakedown_run.content.ron:37` and the Rust builder
  `crates/nova_assets/src/scenario/shakedown.rs:415`. Invert the guarding test
  `the_new_game_player_has_infinite_ammo` (`shakedown.rs:2066`) to assert finite
  ammo (rename accordingly); it exists precisely so the flag cannot flip
  silently, so it must move with the decision.
- [x] Tests (headless, in `ammo.rs`): (1) a discrete on-empty `SectionReload`
  does NOT tick while `rounds > 0`, then after the magazine empties, `reload_time`
  of ticks refills `rounds` to `capacity`. (2) a continuous regen
  (`only_when_empty: false, rounds_per_cycle: 1`) adds exactly one round per
  `reload_time` and clamps at `capacity` (never overfills). (3) a full magazine
  keeps `elapsed` at 0 and never exceeds `capacity`. (4) `progress()` returns
  0 at rest and approaches 1 as a cycle nears completion. Use `Time` advanced by
  a manual delta (see the `TimeUpdateStrategy::ManualDuration` rigs already in
  `turret_section.rs:1724`). Also add/adjust a turret + torpedo build test
  asserting `SectionReload` is attached iff `ammo_capacity` is `Some` and reload
  is configured.
- [x] Docs: `tasks/20260717-085640/NOTES.md` - the mechanism (one timer, two
  behaviors via `only_when_empty`/`rounds_per_cycle`), why FixedUpdate + add-only
  avoids a schedule conflict with consume, the catalog tuning chosen and why,
  and the shakedown flip. Append a Fix record line to
  `tasks/20260716-123556/SPIKE.md`. Add a CHANGELOG Unreleased line.

## Notes

- Spike: tasks/20260716-123556/SPIKE.md (option A3 + A1, and why A2 manual
  reload / A4 pickups are deferred).
- Keystone: unblocks 20260716-123556 (reload-state on the readout).
- Ammo pool + seam: crates/nova_gameplay/src/sections/ammo.rs
  (`SectionAmmo`, `capacity` is the refill reference; "future reload is
  `rounds = capacity`" is noted there).
- Fire gates already read the pool: turret `shoot_spawn_projectile`
  (turret_section.rs), torpedo `shoot_spawn_projectile`
  (torpedo_section/mod.rs) both `try_consume()` and skip on `is_empty()`.
- Infinite-ammo strip point: crates/nova_scenario/src/objects/spaceship.rs:207
  (`infinite_ammo` -> `ammo_capacity = None` -> no component).
- Catalog capacities to re-tune: crates/nova_assets/src/sections.rs
  (turret 500 / 150, torpedo 6).
- Scenario to flip as proof: assets/base/scenarios/shakedown_run.content.ron
  (`infinite_ammo: true`).
- Open question for playtest: trigger reload strictly on-empty vs on idle-since-
  last-shot (avoids mid-burst stutter with small mags). One-line difference.
- Stepless: run /plan before /work (tatr convention).

## Implementation record

Landed on branch `feature/ammo-reload`. `SectionReload`/`SectionReloadConfig`
+ `tick_section_reload` (FixedUpdate, add-only) in `sections/ammo.rs`; `reload`
config on turret/torpedo attached alongside `SectionAmmo`; forgiving catalog
defaults (turrets discrete reload-to-full ~3s/~2.5s, torpedo bay continuous
1/4s regen) in `nova_assets/sections.rs` + the RON mirror; Shakedown Run flipped
to finite ammo (RON + Rust builder + inverted guard test). Design, the
FixedUpdate/add-only reasoning, the `Time<Virtual>` `max_delta` test-clock
gotcha, and self-reflection are in `tasks/20260717-085640/NOTES.md`.

Chose the on-empty vs idle-timeout open question in favor of a plain
`only_when_empty` flag (on-empty for turrets, continuous regen for the bay)
rather than an idle timer - simpler, and mid-burst stutter is avoided by
generous magazines rather than a delay. Docs updated: CHANGELOG, dev wiki
`sections.md` + `guide-author-section.md`, spike Fix record.

Verification (per the standing skip-local-full-suite instruction): `cargo check
--workspace --all-targets` clean (no warnings in touched crates); `cargo fmt`
clean; new + affected tests green - `sections::ammo` 9/9, the two fire->reload
integration tests, `content_ron_parity` 2/2, `content_lint_gate` 2/2, the
inverted shakedown guard. Full suite runs in CI.
