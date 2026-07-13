# Per-source SFX throttle: multiple guns/sources each play

- STATUS: CLOSED
- PRIORITY: 75
- TAGS: v0.4.0,audio,bug

## Goal

Bug (user report): when multiple guns fire, only one plays a sound. Root cause:
`SfxThrottle` in `crates/nova_gameplay/src/audio.rs` keeps ONE global timestamp
per cue-type (`last_turret_fire`/`last_impact`/`last_explosion`), so two distinct
sources firing within the min-interval are collapsed to one - the throttle can't
tell "two guns" from "one gun twice". Same defect hits impact and explosion.
Make the throttle per-source so each source sounds independently while still
collapsing a single source's burst.

## Steps

- [x] Replace the three global timestamp fields with a keyed map:
      `SfxThrottle { last: HashMap<ThrottleKey, f32> }` plus
      `allow(key, now, interval) -> bool` (absent key -> fires; else same
      compare-and-stamp as before) and a `prune(now, window)` that drops idle
      keys so the map stays bounded.
- [x] `ThrottleKey` enum: `TurretFire(Entity)` (keyed by the firing turret, so
      each gun is independent - even two guns on one ship), `Impact(IVec3)` and
      `Explosion(IVec3)` (keyed by a quantized world cell via `area_cell(pos)`, so
      a co-located burst - one ship's blast, one ship's sections dying - collapses
      but distinct locations each sound). Add `SFX_AREA_CELL` constant.
- [x] Make `TurretSectionPartOf` `pub(crate)` in `sections/turret_section.rs`
      (module is already `pub mod`; do NOT add to the public prelude) and read it
      off the projectile in the turret-fire observer to get the turret entity.
- [x] Update the four observers: turret keys `TurretFire(part_of.0)`; impact
      keys `Impact(area_cell(pos))`; explosion keys `Explosion(area_cell(pos))`;
      torpedo launch is unchanged (not throttled).
- [x] Add a `prune_sfx_throttle` system (Update) dropping keys idle > ~2s.
- [x] Tests: `SfxThrottle::allow` fires independently for two different keys in
      the same instant (the crux), re-throttles within the interval, and
      re-fires after it; `area_cell` groups nearby / separates distant positions.
- [x] Verify: fmt, clippy --all-targets, cargo test --workspace, headless
      10_gameplay autopilot (Playing, no panic). Shared CARGO_TARGET_DIR.
- [x] Update `tasks/20260708-162011/NOTES.md` (throttle is per-source:
      turret by entity, area cues by world cell; note the area-cell heuristic).

## Notes

- Depends on: 20260708-162011, -213155, -214821 (all CLOSED) - refines the same
  module.
- Verified: turret projectile carries `TurretSectionPartOf(turret)`
  (turret_section.rs spawn bundle); sections are direct children of the ship root
  (`integrity/glue.rs:115`); listener/source plumbing already exists.
- Area-cell is a heuristic: two sources within one `SFX_AREA_CELL` collapse. A
  fully exact impact/explosion key would be the `IntegrityRoot` of the
  damaged/destroyed entity (its `ChildOf` parent for a section, itself for an
  asteroid) - noted as a possible refinement; the cell keeps it simple and needs
  no hierarchy walk. Turret uses the exact entity key because that is the
  reported bug.

## Outcome

Fixed the "only one gun sounds" bug: the throttle was global (one timestamp per
cue-type), so any second source in the same window was silenced. Replaced it with
a per-source keyed throttle (`SfxThrottle { last: HashMap<ThrottleKey, f32> }`,
`allow`/`prune`). Turret fire is keyed by the firing turret entity
(`TurretSectionPartOf`, made `pub(crate)`), so each gun sounds independently;
impact and explosion are keyed by a quantized world cell (`area_cell`), so a
co-located burst collapses but distinct locations each sound. Added a prune system
to bound the map. Tests: per-key independence (the crux - two guns fire in the
same instant, both allowed), single-key re-throttle, prune-only-idle, and
area_cell grouping. Verified fmt, clippy --all-targets (clean), cargo test
--workspace (7 audio tests), headless 10_gameplay autopilot (Playing, no panic).
Self-reviewed; no findings. Noted future refinement: exact per-IntegrityRoot key
for impact/explosion instead of the area-cell heuristic.
