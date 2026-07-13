# Torpedo takes no contact damage from its own ship at launch: unify projectile owner collision filter

- STATUS: CLOSED
- PRIORITY: 95
- TAGS: v0.4.0,bug,torpedo

Reported in play: torpedoes fired from the torpedo bay section immediately take
damage or are destroyed at spawn. With the combat-juice work the damage is now
visible as a hit flash right at the bay the moment a torpedo fires.

Root cause (verified in code): the torpedo root spawns essentially at the bay
spawner transform (`projectile_position + spawner_exit_velocity * 0.01`) with
`RigidBody::Dynamic` and two child sections (controller, thruster) that each get a
1x1x1 cuboid collider, 1.0 health and `CollisionEventsEnabled` (auto-added to any
collider with Health by bevy_common_systems). The torpedo leaves the bay at muzzle
speed relative to the ship, so when its child colliders overlap the firing ship's
section colliders at spawn, avian raises `CollisionStart` and
`on_impact_collision_deal_damage` (bevy_common_systems integrity plugin) applies
impulse/energy damage from the relative velocity. With 1.0 health per torpedo
section, the torpedo dies instantly; the ship's own bay/hull section can eat the
same contact damage. The arming gate (task 20260707-100003) only gates
detonation, not incoming contact damage - that task explicitly deferred hull
clipping ("revisit if torpedoes are seen clipping the hull").

Turret bullets already solve exactly this with avian collision hooks:
`TurretProjectileHooks::filter_pairs` skips any pair where one collider is a
bullet whose `TurretBulletProjectileOwner` equals the other collider's
`ColliderOf.body`, enabled per-entity via `ActiveCollisionHooks::FILTER_PAIRS`.
Torpedoes have no such filter. avian registers exactly ONE `CollisionHooks` type
per app (`plugin.rs:36`), so the fix is to generalize the existing hook, not add
a second one.

Expected: firing a torpedo never contact-damages the torpedo or the firing ship
at launch; the torpedo flies out, arms, and detonates normally. Contact
collisions with every other body (targets, asteroids, enemy PDC fire) keep
working.

## Steps

- [x] Add a shared `ProjectileOwner(pub Entity)` component and a `ProjectileHooks`
      `CollisionHooks` impl in a new `crates/nova_gameplay/src/sections/projectile_hooks.rs`,
      exported through the sections prelude. `filter_pairs` resolves each collider's
      owner by looking for `ProjectileOwner` on the collider entity itself OR on its
      `ColliderOf.body` (torpedo colliders are children of the owning root), and
      returns false when that owner equals the other collider's `ColliderOf.body`
      (check both orientations of the pair).
- [x] Replace `TurretBulletProjectileOwner` (private, turret_section.rs) and
      `TorpedoProjectileOwner` (pub, torpedo_section/mod.rs:149, used by
      input/player.rs:267 and its tests) with `ProjectileOwner`. Keep the marker
      components (`TurretBulletProjectileMarker`, `TorpedoProjectileMarker`) as the
      type discriminators in queries.
- [x] Register the generalized hook in `crates/nova_gameplay/src/plugin.rs`:
      `with_collision_hooks::<ProjectileHooks>()` replacing `TurretProjectileHooks`.
- [x] In `shoot_spawn_projectile` (torpedo_section/mod.rs), add
      `ActiveCollisionHooks::FILTER_PAIRS` to both torpedo child section entities
      (the entities that carry the colliders - the flag on the collider-less root
      does nothing).
- [x] Physics-level integration tests (avian stepping, cf.
      `integrity/glue.rs::physics_tests` and `integrity/test_support.rs`; the test
      app must register `PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>()`):
      (1) a torpedo spawned overlapping its owner ship takes no damage and the ship
      sections take none; (2) the same torpedo overlapping a NON-owner body still
      collides (damage or contact reported); (3) a turret bullet still ignores its
      owner (regression for the renamed hook).
- [x] Verify end to end with the torpedo range: headless
      `BCS_AUTOPILOT=1` run of `examples/06_torpedo_range.rs` under Xvfb still
      reports 3 fired / 3 armed / 3 detonated with no torpedo dying at spawn and no
      spawn-time hit flash/damage on the player ship. Done as an A/B run: on master
      every launch logs a 0.30-damage impact pair torpedo-section <-> bay-section
      (the damage that feeds the hit flash); on the branch those owner-pair
      impacts are gone, cycle intact, exit 0 both runs.
- [x] Document the decision (owner filter is permanent, turret parity; why a
      state-dependent/arming-gated filter was rejected) in the task resolution and
      `tasks/20260709-131502/NOTES.md` (retro follows via
      /compound). Also added a CHANGELOG entry under Fixed.

## Resolution

Implemented as planned. New `sections/projectile_hooks.rs` holds the shared
`ProjectileOwner(Entity)` component and the single avian `CollisionHooks` impl
(`ProjectileHooks`); it resolves ownership on the collider entity itself (turret
bullet) or via `ColliderOf.body` (torpedo child sections) and skips owner pairs in
both orientations. Turret and torpedo spawns both attach `ProjectileOwner`;
`TurretBulletProjectileOwner`, `TorpedoProjectileOwner` and the old
`TurretProjectileHooks` are gone; the torpedo's two child sections opt in with
`ActiveCollisionHooks::FILTER_PAIRS`. `integrity/test_support.rs` gained a
hook-capable app builder for the tests.

Verified with 3 new physics-level tests (owner overlap: no damage, motion
unperturbed; non-owner overlap: damage lands; turret-bullet regression) and an A/B
headless run of `06_torpedo_range` (master: every launch deals 0.30 contact damage
both ways between torpedo and bay; branch: no owner-pair impacts, fire/arm/detonate
cycle and exit 0 unchanged).

Honest test scope: the 4 new tests were run locally and pass; `cargo check
--workspace` and `cargo fmt` are green; the full test suite and clippy were NOT run
locally, per the AGENTS.md instruction to defer them. Review round 1 (R1.2) found
that no in-repo PR workflow actually runs them (.github/workflows has only
deploy-page.yaml and release.yaml), so where the suite runs is surfaced to the
user in the flow report rather than assumed here.

Difficulty hit: the control test first used a realistic 1 hp warhead, whose death
dragged the render-facing destroy/explode observer into the headless harness and
panicked (missing `Assets<StandardMaterial>`/`GlobalRng`). Reworked to high health
on both sides - the invariant is "contact damage lands", not the death cascade.

Review round 1 additions: a wiring assertion in `06_torpedo_range`
(`assert_no_owner_pair_damage` observer, fails any run where a torpedo section
and a ship section exchange contact damage - the smoke now regression-tests the
FILTER_PAIRS flag and hook registration, not just the hook logic), a direct
`filter_pairs` orientation-symmetry test (4 tests total now), and de-tupled hook
queries. While verifying the observer, a false alarm surfaced a real tooling
trap: sharing CARGO_TARGET_DIR between the worktree and the main checkout linked
master's stale nova_gameplay into the worktree smoke binary (master code, no
filter, "intermittent" owner damage). docs/development.md's worktree-cache
advice is corrected on this branch; the smoke is clean (3/3 runs, 0 owner-pair
events) when built in the worktree's own target.

Reflection: reading avian's hook semantics up front (one hook type per app,
either-collider activation, broad-phase-time filtering) shaped the design
correctly on the first try; the A/B example run was the cheapest, clearest
evidence of the fix and is worth repeating for physics bugs with an existing
range example. Details in `tasks/20260709-131502/NOTES.md`.

## Notes

- Relevant files:
  - `crates/nova_gameplay/src/sections/torpedo_section/mod.rs` (spawn:
    `shoot_spawn_projectile` ~348-521; `TorpedoProjectileOwner` at 149)
  - `crates/nova_gameplay/src/sections/turret_section.rs` (hook 266-294, bullet
    spawn 822-838)
  - `crates/nova_gameplay/src/plugin.rs:36` (single hook registration)
  - `crates/nova_gameplay/src/input/player.rs:267` (torpedo owner query)
  - bevy_common_systems `src/integrity/plugin.rs` (`on_impact_collision_deal_damage`,
    `on_collider_of_spawn_insert_collision_events`) - external crate, read-only here.
- Decision: the owner filter is permanent for the projectile's lifetime (same
  semantics turret bullets already have). A torpedo that loops back passes through
  its own ship instead of contact-damaging it; blast damage is a separate sensor
  path and still hurts anything in radius including the owner. An arming-gated
  filter was considered and rejected: avian evaluates `filter_pairs` when a
  broad-phase pair is created, so a filter that changes answer mid-overlap is not
  re-evaluated reliably.
- Deliberately NOT filtered: torpedo-vs-torpedo pairs from the same ship (owner
  check compares projectile owner to the other collider's body, and a salvo
  sibling's body is the sibling torpedo, not the ship). Fire-rate spacing makes
  this a non-issue today; revisit only if salvo self-collisions show up.
- One avian `CollisionHooks` type per app: this is why the turret hook is
  generalized instead of adding `TorpedoProjectileHooks` alongside it.
- Verify while implementing: whether avian adds `ColliderOf` when collider and
  body share one entity (turret bullet root). The hook's "collider entity itself
  OR ColliderOf.body" lookup covers both cases regardless.
