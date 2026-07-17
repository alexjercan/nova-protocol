# Review: multi-muzzle firing (task 215857)

- VERDICT: APPROVE
- REVIEWER: flow review pass on the working tree (uncommitted, atop 606c3576)

## What was verified

- **Shared magazine (the key behavior).** Read the refactored
  `shoot_spawn_projectile`: the per-turret setup (weapons-hot gate, loaded round,
  ship pose) is hoisted out of the muzzle loop; the muzzle Entity list is copied
  out before the inner loop so `q_muzzle`/`commands` borrow freely; the turret's
  single `Option<&mut SectionAmmo>` is drawn by every muzzle's burst
  (`ammo.as_deref_mut()` inside the shared `for muzzle` loop). A magazine that
  empties on an earlier muzzle's burst stops the later muzzles the same tick.
  Confirmed by the test asserting `per[0] + per[1] == 3` for a twin barrel over a
  3-round mag (shared, not 3-per-barrel).
- **Per-muzzle timers.** Each muzzle ticks its own `TurretSectionBarrelFireState`
  and gates on `input && is_finished()`; the sub-tick lead + burst math is
  unchanged, just run per muzzle. Timers still tick when safe/empty (ready on
  release/reload), matching the old single-muzzle semantics.
- **Aim.** `update_turret_target_joints_system` now loops every muzzle's chain at
  the shared aim point - shared joints agree for symmetric barrels, private
  joints refine per muzzle (spike default). `update_turret_aim_point` stays on the
  primary muzzle (one lead point per turret). A shared-chain twin barrel points
  both muzzles within 5 deg (test).
- **Blast radius.** `TurretSectionMuzzleEntity` (primary) preserved, so
  ai.rs/turret_lead.rs/audio.rs are untouched and nova_assets checks clean. Bare
  test rigs that build turrets without the observer now also insert
  `TurretSectionMuzzles(vec![muzzle])` (required by the fire query).

## Findings

None blocking. The primary-muzzle approximation for the lead point and the AI
alignment gate is documented and correct for shared-chain mounts; independent
arms with widely-separated muzzles would want per-muzzle leads, but no such mount
ships and the seam is noted (spike open question). Not a defect.

## Verdict

APPROVE. Docs (dev wiki muzzle bullet + a Combat & Weapons CHANGELOG line) landed
in the same change. Pending the independent turret-tests + workspace check going
green before landing.
