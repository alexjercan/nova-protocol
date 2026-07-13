# Projectile owner collision filter (torpedo launch self-damage fix)

Task: `tasks/20260709-131502` - torpedoes fired from the torpedo bay took contact
damage from the firing ship at spawn (and dealt some back to the bay), often dying
instantly. With the hit-feedback juice this became visible as a hit flash on the
bay every time a torpedo fired.

## What changed

- New `crates/nova_gameplay/src/sections/projectile_hooks.rs`: a shared
  `ProjectileOwner(Entity)` component and a `ProjectileHooks` avian
  `CollisionHooks` impl whose `filter_pairs` skips any contact pair between a
  projectile and the body that fired it.
- `TurretBulletProjectileOwner` (turret_section.rs) and `TorpedoProjectileOwner`
  (torpedo_section) are gone; both bullet and torpedo spawns now attach the shared
  `ProjectileOwner`. The player torpedo-targeting query in `input/player.rs`
  follows the rename.
- `NovaGameplayPlugin` registers `ProjectileHooks` instead of the old
  turret-only `TurretProjectileHooks`.
- The torpedo's two child sections (which carry its colliders - the root has
  none) get `ActiveCollisionHooks::FILTER_PAIRS` so the hook actually runs for
  their pairs.
- `integrity/test_support.rs` gained `unfinished_integrity_physics_app_with`
  (caller-supplied physics plugin group) so tests can register collision hooks
  the way the real app does.

## Why this design

Root cause: the torpedo spawns essentially at the bay spawner transform and
leaves at muzzle speed *relative to the ship*. Its child sections have colliders,
1.0 health, and (auto-added for any collider with `Health`)
`CollisionEventsEnabled`, so the spawn overlap raised `CollisionStart` and
bevy_common_systems' `on_impact_collision_deal_damage` applied impulse/energy
damage on frame one. The arming gate from task 20260707-100003 only gates
detonation, not incoming damage; that task explicitly deferred this case.

Alternatives considered:

- **Second, torpedo-specific hook.** Not possible: avian allows exactly one
  `CollisionHooks` type per app (`with_collision_hooks::<H>()` on
  `PhysicsPlugins`), so the turret hook had to be generalized, not duplicated.
- **Arming-gated (temporary) filter.** Rejected: `filter_pairs` runs when a
  broad-phase pair is created, so a filter whose answer changes while the pair
  persists is not re-evaluated reliably. The permanent owner filter matches the
  semantics turret bullets have had all along; blast damage is a separate sensor
  path and still hurts the owner.
- **Spawn the torpedo further out.** Treats the symptom, breaks the "launched
  from the bay" look, and still fails when the ship rotates during fire.

Deliberately NOT filtered: torpedo-vs-torpedo pairs from the same ship (a salvo
sibling's body is the sibling, not the ship), and everything involving other
ships - enemy PDC fire intercepting torpedoes keeps working.

The hook resolves ownership through two paths: `ProjectileOwner` directly on the
collider entity (turret bullet - root and collider are one entity) or on the
collider's `ColliderOf.body` (torpedo - colliders live on child sections).

## Verification

- Four tests in `projectile_hooks.rs`: three physics-level (real avian stepping,
  hook registered as in the app) - owner overlap deals no damage and leaves the
  torpedo's velocity untouched; the same overlap against a non-owner body does
  deal contact damage; a turret bullet still ignores its owner - plus a direct
  `filter_pairs` call asserting the filter is symmetric in pair orientation.
- A wiring assertion in the range itself (from review round 1):
  `assert_no_owner_pair_damage` panics the run if a torpedo section and a ship
  section ever exchange contact damage, so the headless smoke fails if the
  FILTER_PAIRS flag or the hook registration regresses.
- A/B smoke on `06_torpedo_range` (headless autopilot under Xvfb): on master,
  every launch logs a 0.30-damage impact pair between the torpedo section and
  the ship's bay section; on the branch those owner-pair impacts are gone while
  3 fired / 3 armed / detonations / exit 0 all hold.

## Difficulties

- The review round added a wiring assertion to `06_torpedo_range`
  (`assert_no_owner_pair_damage`, panics on torpedo-vs-owner damage), and its
  first run appeared to show the filter failing intermittently. The real cause:
  the smoke was built with `CARGO_TARGET_DIR` pointed at the main checkout's
  target (per the then-current docs/development.md advice), and the master A/B
  run in between had rebuilt master's `nova_gameplay` into that shared dir - the
  worktree binary linked master's code, which has no filter. Rebuilt in the
  worktree's own target: three consecutive smoke runs clean (0 owner-pair
  events). The docs advice is corrected in this branch; never share a target
  dir across diverged checkouts of the same workspace.

- First version of the "non-owner overlap still collides" control test gave the
  warhead its realistic 1.0 health; the contact damage destroyed it and the
  destroy/explode observer panicked in the headless harness (it needs
  `Assets<StandardMaterial>` and a `GlobalRng` that only exist in the real app).
  Fix: give both sides large health - the invariant is "contact damage lands",
  and staying away from zero keeps the render-facing destroy pipeline out of a
  physics test. Lesson for future physics tests: keep entities alive unless the
  death cascade itself is under test.

## Self-reflection

- The A/B example run (same command on master and branch) was cheap and produced
  the clearest evidence of both bug and fix; worth doing for any physics-feel
  bug with an existing range example.
- Reading the avian source for the exact hook semantics (single hook type per
  app, either-collider flag activation, broad-phase-time filtering) before
  planning avoided a dead-end design (per-kind hooks, arming-gated filter).
