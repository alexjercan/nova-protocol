# Implementation plan: multi-muzzle firing (task 215857)

Builds on the landed joint-tree core (commit 606c3576). Spike:
tasks/20260717-214834/SPIKE.md (multi-muzzle option A). Everything in
crates/nova_gameplay/src/sections/turret_section.rs unless noted. Keep
`cargo check -p nova_gameplay` green + `cargo fmt`; read `cargo build -p
nova_gameplay` warnings (lesson warnings-clean-before-land). Do NOT commit.

## Goal

Today the tree can hold N muzzles but `insert_turret_section` records only the
first (and warns on more), and the fire/aim path drives that one. Make a mount
fire and steer ALL its muzzles, per-muzzle fire timers, over one shared section
magazine.

## 1. Carry all muzzles

Add:
```rust
/// Every muzzle (fire point) of a turret, in tree DFS order. The section-wide
/// fire/aim path iterates these; `TurretSectionMuzzleEntity` stays as the
/// PRIMARY muzzle (the first) for the single-point consumers (lead HUD pip, the
/// aim-point lead solve, AI alignment gate).
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct TurretSectionMuzzles(pub Vec<Entity>);
```
In `insert_turret_section`: after collecting `muzzles`, keep the existing
primary selection (first muzzle -> `TurretSectionMuzzleEntity`) but DROP the
">1 muzzle" warning (multi-muzzle is now supported), and also insert
`TurretSectionMuzzles(muzzles.clone())` on the turret root. Empty-tree error path
stays.

## 2. Fire all muzzles (shoot_spawn_projectile)

Refactor so the per-TURRET work runs once and the per-MUZZLE work loops:
- Per turret (unchanged, computed once): weapons-hot gate, `(bullet_kind,
  bullet_damage)` from LoadedBullet/config, the spaceship pose tuple
  `(position, rotation, lin_vel, ang_vel, center, allegiance)`.
- Query `&TurretSectionMuzzles` instead of `&TurretSectionMuzzleEntity`.
- For EACH muzzle entity in the list: tick its `TurretSectionBarrelFireState`,
  gate on `input && fire_state.is_finished()`, compute its muzzle pose via
  `local_pose_in_root(muzzle, spaceship, &q_chain)`, run the existing sub-tick
  burst loop, stamping `TurretSectionMuzzleEntity(muzzle)` on each bullet (the
  firing muzzle, for the audio per-gun throttle).
- SHARED MAGAZINE: the turret's `Option<&mut SectionAmmo>` is drawn by every
  muzzle. The empty-magazine gate and the per-shot `try_consume()` both read the
  ONE ammo; a mag that empties on muzzle A's burst stops muzzle B this tick too.
  Keep `ammo.as_deref_mut()` reachable across the muzzle loop (borrow the
  `Option<&mut SectionAmmo>` mutably for the whole turret iteration).
- `q_muzzle.get_mut(muzzle)` per muzzle inside the loop (still `With<
  TurretSectionBarrelMuzzleMarker>`). Handle the missing-muzzle Err by
  continue-ing that muzzle, not the whole turret.

Watch the borrow checker: iterate the muzzle Entities by value (copy out of the
`Vec` first, e.g. `let muzzles: Vec<Entity> = muzzles.0.clone()` or iterate
`&**muzzles`), so `q_muzzle`/`commands` are free to borrow inside.

## 3. Steer all muzzles (update_turret_target_joints_system)

Iterate every muzzle of the turret, not just `TurretSectionMuzzleEntity`. Query
`&TurretSectionMuzzles`; for each muzzle, run the SAME per-chain Jacobi CCD pass
that exists today (compute the muzzle pose, walk muzzle->root, step each
articulated joint). Shared joints (twin barrel sharing yaw+pitch) get written by
each muzzle's pass and agree to sub-degree for symmetric barrels; independent
arms each steer their own private joints. This is the spike's default ("shared
joints aim at the target, private joints refine per muzzle"). Leave
`update_turret_aim_point` computing the lead point from the PRIMARY muzzle
(`TurretSectionMuzzleEntity`) - one lead point per turret is fine; all muzzles
share it.

## 4. AI + HUD

Leave `input/ai.rs` (`on_projectile_input`) and `hud/turret_lead.rs` on the
PRIMARY muzzle / single aim point - a shared-chain twin barrel aligns together,
so the primary-muzzle alignment gate is representative. Only touch them if the
compiler forces it. Audio is per-projectile (per firing muzzle) already.

## 5. Tests (in turret_section.rs tests)

- TWIN-BARREL SPAWN + SHARED MAG: build a config whose barrel joint has TWO
  muzzle children (lateral offsets, e.g. (+0.1,0,-0.5) and (-0.1,0,-0.5)), each
  MuzzleConfig fire_rate say 10, with a small shared `ammo_capacity` (e.g. 3).
  Spawn via the observer, hold the trigger, step FixedUpdate frames, and assert:
  (a) both muzzles produce bullets (count bullets whose
  `TurretSectionMuzzleEntity` equals each muzzle - both > 0), (b) total bullets
  spawned == the shared capacity (the magazine is shared, not per-muzzle: 3 total,
  not 6). Reuse the existing fire-path test harness/patterns in this module.
- (Optional) TWIN-BARREL AIM: both muzzles of a shared-chain twin barrel point
  at the target within a few degrees after stepping (extends the convergence
  helper to check both muzzles).
- Update the existing single-muzzle >1-warn expectation if any test asserted it.

## 6. Docs

Update the dev wiki turret section (web/src/wiki/dev/guide-author-section.md):
the `muzzle` bullet currently says "steers and fires ONE muzzle; multi-muzzle
mounts are coming" - change to describe that a turret now fires ALL its muzzles
sharing the section magazine, each at its own fire_rate. Add a CHANGELOG line
under Combat & Weapons (player-facing: multi-barrel PDCs now possible) OR Modding
& Mod Portal (authoring). Keep it short and game-centric.

## 7. Verify

`cargo fmt`; `cargo check -p nova_gameplay`; `cargo build -p nova_gameplay 2>&1
| tail` (read warnings); `cargo test -p nova_gameplay turret 2>&1 | tail`.
Report deviations honestly. Do NOT commit.
