# Ammo/magazine HUD readout: show loaded type, rounds and reload state

- STATUS: OPEN
- PRIORITY: 32
- TAGS: v0.7.0,hud,ui,spike


## Goal

Reframed by the spike (tasks/20260716-123556/SPIKE.md): the diegetic ammo
readout (`hud/ammo_readout.rs`) ALREADY shows loaded bullet type (pip color via
`damage_type_color`) and rounds remaining (turret ring / torpedo bar), at
`HudTier::Instrument`. The one missing signal of the original three is the
**reload state**, which had nothing to show until the reload mechanic
(20260717-085640) existed. This task adds that: surface the `SectionReload`
progress on the EXISTING ring/bar as a reload/recharge visual. Do NOT build a
separate instrument-family corner chip (spike option B2, rejected as duplicate) -
loaded-type + count already ship and a second readout for the same fact is
clutter.

## Steps

- [ ] Extend `drive_ammo_readouts` in `crates/nova_gameplay/src/hud/ammo_readout.rs`
  (`:359`) to also read `Query<&SectionReload>` (Option per section) alongside
  the existing `SectionAmmo`/`LoadedBullet` queries.
- [ ] Compute a per-readout reload state: `reloading` = the section has a
  `SectionReload` whose `progress() > 0` while `rounds < capacity`, plus the
  fraction `progress()`. Keep the current steady lit/dim path when not reloading.
- [ ] Render the reload visual over the existing pips (both `AmmoReadoutKind`):
  while reloading, drive the ring/bar as a progress SWEEP - light pips up to
  `floor(progress * segment_count)` in the loaded hue at a distinct
  reloading alpha (a pulse between `DIM_ALPHA` and `LIT_ALPHA`, or a fixed
  mid alpha), so a discrete on-empty reload reads as the ring filling from empty
  to full and a torpedo bar fills a pip at a time. Reuse `turret_lit_segments`'
  segment count for the ring and `capacity` for the bar; no new pip nodes.
- [ ] Keep the visual purely a function of `progress()` so it works for both
  reload modes (discrete full-reload sweep and continuous regen) without
  branching on the mode.
- [ ] Tests (headless, in `ammo_readout.rs`, extending the existing color/drive
  tests): (1) with a mid-reload `SectionReload` (progress ~0.5) on an empty
  turret, the count of reload-lit pips equals `floor(0.5 * RING_SEGMENTS)` and
  differs from the steady-empty lighting. (2) a torpedo bay part-way through
  rearm lights the expected number of bar pips. (3) with no `SectionReload` (or
  progress 0) the output is byte-identical to today's steady path (no regression
  to the shipped loaded-type/count behavior).
- [ ] Docs: `tasks/20260716-123556/NOTES.md` (the reload-visual design, why it
  rides the existing readout instead of a new chip). Append a Fix record line to
  the spike. CHANGELOG Unreleased line.

## Notes

- Depends on: 20260717-085640 (the `SectionReload` component + progress this
  task renders). Must land first.
- Spike: tasks/20260716-123556/SPIKE.md (option B1; B2 corner chip rejected).
  Release-scope spike: tasks/20260716-122954/SPIKE.md; plan
  docs/plans/20260716-v0.7.0-plan.md strand 3.
- Existing readout internals: `drive_ammo_readouts` (ammo_readout.rs:359) is the
  single ammo-state read point; `turret_lit_segments` maps rounds/capacity ->
  lit ring pips; `LIT_ALPHA` 0.95 / `DIM_ALPHA` 0.16; hue from
  `damage_type_color`. `HudTier::Instrument`, spawned by hud/mod.rs observers.
- The readout is still hidden for `infinite_ammo` weapons (no `SectionAmmo`);
  once shakedown flips finite (20260717-085640) it appears in combat naturally.
- Coordinate styling with diegetic HP (20260711-202901) if both land this
  release.
