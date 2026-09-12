# Simulation defects: severing, lock validity, projectile ownership, flush races

- STATUS: CLOSED
- PRIORITY: 81
- TAGS: v0.14.0, bug, physics, combat, review

## Goal

Fix the latent defects found by the 2026-09-09 read of `nova_ship` and
`nova_gameplay`: severing, targeting, projectile ownership, destruction
observers, degenerate authored values, and command-flush races. Line
numbers are from the read and will drift.

Every real defect gets a `bug_` range in `examples/systems/` (or a focused
unit test where no live app is needed) that fails before the fix, then the
fix. Land the complete task as one commit. Owner (2026-09-09): "create systems
examples for all edge cases and bugs we find."

## Decisions

- A neutralized hull keeps an existing combat lock. The player must unlock it
  manually. The lock keeps `WeaponsHot` true and may keep turrets working the
  wreck; this is deliberate. Neutralization only removes the hull from threat
  contacts. Do not add `bug_neutralized_lock_drops`.
- Severed sections are obstacles for everyone, including the ship they came
  from and that ship's existing and future projectiles. The current owner
  filtering already has this behavior. Do not track fragment origin, add an
  ownership grace window, or add `bug_own_fragment_not_shot`.
- A pending sever expires immediately if any referenced body no longer exists.
  If every body exists but mass data is missing or zero, retry once, then expire
  and log once. Expose a small `nova_ship` cleanup function for scenario
  teardown to clear both pending-sever resources. Do not add a new reset event
  or move the state onto entities.
- Production has exactly one seeded `GlobalRng`; game code must not create a
  local or fallback RNG. Let destruction and fixture shedding detect a missing
  or duplicate global RNG, log the broken contract once, omit randomized debris,
  and still remove the dead entity and collider. Test that production assembly
  provides exactly one global RNG.
- Remove stale hull-radius components when no live hull can be measured, and
  clear downstream ship-derived hit and integrity envelopes. Skip controller
  retuning until finite mass and radius measurements exist; do not write
  infinite ceilings or invent an arm floor.
- Reject an invalid torpedo bay `fire_rate` at lint and withhold its runtime
  spawner. Do not substitute a default cadence or clamp it to epsilon.
- Check the authored thruster direction at all three autopilot use sites. Skip
  invalid thrusters and use the existing no-live-engines disengagement when no
  valid thruster remains.
- Floor a zero autopilot crumb scale at `f32::EPSILON`.
- Make `TipWalk` advance safely beyond a rejected region before resetting its
  rejection budget. Do not only increase the budget.
- Cap railgun wake emission at 512 particles per emitter per frame. Discard
  excess whole-particle debt and preserve only the fractional remainder.
- Reproduce every item under "Plausible, verify by running" before changing
  production code. Record and close any item that does not reproduce.
- Add live `bug_` ranges only where an app is needed. Use focused unit tests for
  command shape, validation, and arithmetic. Register every new range and its
  outcome slugs in the probe roster.

## Confirmed by reading

- [x] `crates/nova_ship/src/sections/integrity.rs:381,482,487,512` a sever
      batch whose root died before `apply_pending_sever_motion` is pushed
      back onto `PendingSeverMotion` with no retry count or expiry, and
      `recompute_pending_sever_mass` (`:439`) then updates mass properties
      on every live body in every stuck batch every tick, for the rest of
      the process. Neither resource is reset by `teardown_scenario_entities`.
      Expire immediately when a referenced body is gone. If every body exists
      but mass data is missing or zero, retry once, then expire and log once.
      Clear both resources on teardown.
- [x] `crates/nova_ship/src/input/targeting/contacts.rs:273,363` preserves a
      combat lock on a `NeutralizedMarker` hull. This is intended: the player
      must unlock manually. Keep the lock, reticle, `WeaponsHot`, and turret
      operation. The threat arrow remains absent.
- [x] `crates/nova_gameplay/src/rounds.rs:1079` and
      `projectile_hooks.rs:66-74` compare the collider body with the firing
      root. A severed section belongs to a new wreck-fragment body, so existing
      and future projectiles from the former hull can hit it. This is intended:
      severed sections are physical obstacles for everyone.
- [x] `crates/nova_ship/src/sections/torpedo_section/bay.rs:81`
      `1.0 / config.fire_rate` unguarded: an authored `0.0` fires once then
      never, a negative or NaN rate fires every tick. The turret guards the
      same line (`turret_section/setup.rs:74-77`). Reject at lint, guard at
      spawn.
- [x] `crates/nova_gameplay/src/integrity/explode.rs:313`
      `detach_destroyed_body` takes a plain `Single<&mut WyRand>`, so an
      unmatched `Single` silently skips the only owner of a destroyed
      section's despawn and zero-health sections stay standing with live
      colliders. `crates/nova_ship/src/sections/fixture.rs:261`
      `shed_dead_fixtures` has the same shape. Use `Option<Single>` as
      `turret_section/firing.rs:88` does. A missing or duplicate global RNG is
      an assembly error: log once, create no fallback randomness or randomized
      debris, and still remove the dead entity and collider.
- [x] `crates/nova_ship/src/sections/hull_radius.rs:71-77` a root that
      loses every live section keeps its last `HullRadius`, so the
      attitude envelope tunes a surviving controller to a hull ten times
      its size. Remove or zero it when the arms map has no entry.
- [x] `crates/nova_gameplay/src/audio/voice.rs:275,302,499` plain
      `despawn()` under a doc comment that says the writes here must be
      the `try_` variants; the Retry teardown flush is the race the
      comment names. Panic under the default error handler.
- [x] `crates/nova_gameplay/src/lifetime.rs:137` `on_insert_despawn_entity`
      uses `despawn()` where its sibling at `:105` uses `try_despawn()`
      with the panic named. Mod-facing only today.
- [x] `crates/nova_ship/src/physics/attitude.rs:82` and
      `controller_section.rs:461-478` a root with no `HullRadius` yet
      (the spawn frame, a stub) gets INFINITY ceilings written onto every
      `PDController` and the load clamp is disabled for that tick. Floor
      the write or skip the tick.
- [x] `crates/nova_gameplay/src/rounds.rs:593` `TipWalk::next` stops at
      `REJECT_BUDGET` 16 even when the tip has not advanced, so a round
      crosses a dense debris cloud unharmed for that step. Advance the
      origin past the rejected set, or budget per advance.

## Plausible, verify by running

- [x] `crates/nova_gameplay/src/integrity/neutralize.rs:146,153` plain
      `insert` on a root that `:170` writes with `try_insert` for the
      stated reason. Align.
- [x] `crates/nova_ship/src/sections/integrity.rs:406` plain `insert` on a
      section read in the same loop while `:168,172` use `try_insert`.
      Align.
- [x] `crates/nova_ship/src/flight/autopilot.rs:1096` raw `.normalize()`
      on a thruster direction; a degenerate authored rotation makes
      `tail_dv` NaN and the burn never completes. Use `try_normalize`
      and reject the rotation at lint.
- [x] `autopilot.rs:757` `error / crumb_band` with no floor; 0/0 on station
      writes a NaN `RcsIntent`. Floor the band.
- [x] `crates/nova_ship/src/sections/railgun_section/wake.rs:460`
      `owed_particles(covered)` has no per-frame cap: a hitch during a
      slug's flight requests tens of thousands of particles in one frame.
      Cap per frame.

## Judged solid, leave alone

`flight/guidance.rs` and `flight/thrusters.rs` normalisation, the PD
controller's dead-target zeroing, torpedo target-death freeze, turret
hinge rejection, occlusion endpoints, gravity hysteresis, damage speed
floors, the ship audio `Local` resets.

## Proof

Every fix was reproduced first: the fix was applied, the test written, the
production lines temporarily reverted, and the test observed to FAIL before
being restored to green.

### Defects fixed, and the test that pins each

- Pending sever expiry - `nova_ship` `sections::integrity::physics_tests`:
  `a_sever_whose_root_died_first_is_dropped_rather_than_held_forever`,
  `a_sever_that_never_gets_mass_data_retries_once_then_expires_with_one_line`
  (one `warn!`, driven through `run_system_cached` because `CapturedLog` is
  thread-local and the schedule is multi-threaded),
  `clearing_pending_severs_empties_both_stages`. `PendingSeverBatch` gained
  `waits: u8` with `SEVER_MASS_RETRIES = 1`; teardown queues the exported
  `clear_pending_severs` after `resume_player_control`.
- Torpedo bay `fire_rate` - `torpedo_fire_interval` returns `Option`;
  `a_bay_that_authors_no_usable_fire_rate_is_built_without_a_launcher`, and
  `nova_scenario` lint `a_torpedo_bay_without_a_usable_fire_rate_is_an_error`.
- Missing global RNG - `detach_destroyed_body` and `shed_dead_fixtures` take
  `Option<Single<&mut WyRand, With<GlobalRng>>>`, log once through a `Local`
  latch, and still remove the dead entity:
  `a_destroyed_body_is_taken_off_its_hull_even_with_no_global_rng`,
  `a_spent_fixture_comes_off_even_with_no_global_rng`, and the assembly pin
  `the_gameplay_assembly_provides_exactly_one_global_rng`.
- Stale hull size - `publish_hull_radii` removes `HullRadius` and
  `HullEnvelopeRadius` from a root it can no longer measure, and the two
  envelopes published from them unless the body carries its own `BodyRadius`:
  `a_hull_that_loses_every_section_drops_its_size_and_the_envelopes_from_it`,
  `a_hull_with_its_own_body_radius_keeps_the_envelopes_it_can_still_answer`.
- Infinite attitude ceilings - fixed at the WRITER rather than at
  `attitude.rs`: `update_controller_stack_tuning` skips a root with no finite,
  positive inertia and arm, so every controller keeps the tuning it had.
  `a_hull_that_lost_its_last_section_keeps_the_tuning_it_had`,
  `a_root_nothing_has_measured_yet_is_left_on_its_authored_seed`.
- Flush races - `voice.rs` (3 sites), `lifetime.rs`, `neutralize.rs` (2) and
  the section re-parent in `sections/integrity.rs` now use the `try_` variants.
- `TipWalk` reject budget -
  `a_round_crossing_more_near_misses_than_it_can_hold_still_hits_what_is_behind_them`.
- Railgun wake cap - `WAKE_SPAWN_CAP = 512`, whole-particle debt discarded:
  `a_hitch_long_enough_to_owe_thousands_spawns_the_cap_and_carries_no_debt`.
- Degenerate thruster rotation - see the decision below; pinned by
  `a_thruster_with_a_degenerate_authored_rotation_does_not_poison_the_burn`
  and `a_ship_whose_only_engine_is_directionless_disengages`.

### Ruled out, production reverted

- `autopilot.rs:757` crumb band. DOES NOT REPRODUCE. Two tests passed
  identically with and without the `f32::EPSILON` floor: `rcs_capable`
  already gates on `error_speed > 1e-3`, so the 0/0 the item describes is
  unreachable. The floor and its test were reverted.
- A second degenerate-section-rotation lint. DUPLICATE. The red run surfaced
  the EXISTING error from `derive_link_point_graph` ("rotation Quat(0.0, 0.0,
  0.0, 0.0) must have unit length"), so the added check was removed and
  `a_section_mounted_on_a_degenerate_rotation_is_an_error` was rewritten as a
  pin on the rule that already exists.

### Decisions taken while implementing

- NO new `examples/systems/bug_` range. Every defect is reachable from a
  headless rig, and `PendingSeverMotion` is private to `nova_ship`, so a live
  app would prove less than the unit tests do. Consequence: no `Cargo.toml`
  example entry, no `outcome:` markers, no roster slugs, no
  `SYSTEMS_INVARIANTS` change.
- The task's claim that the `despawn()` sites "panic under the default error
  handler" is WRONG for Bevy 0.19: `EntityCommands::despawn` is
  `queue_handled(..., warn)`, so it WARNS - which still fails a probe clean
  pass. `insert` does panic, which is what the `neutralize.rs` and section
  re-parent items were. Both doc comments in `lifetime.rs` were corrected.
- The degenerate thruster direction had FOUR more readers than the three the
  item names: `thruster_impulse_system` (which is what actually put a NaN
  impulse on the hull, so a collider-less test rig still panicked inside
  avian), `manual.rs`, `authority.rs` and `camera/framing.rs`. The guard now
  lives in `sections/thruster_section.rs` as `engine_direction_local` /
  `engine_direction` and every reader goes through it.
- `publish_hull_radii`'s removal pass is scoped `With<ComputedCenterOfMass>`:
  it must not take a size off a root this pass never published one for.

### Verification

- `cargo check --workspace --all-targets --keep-going`: clean.
- `cargo test -p nova_gameplay --lib`: 322 passed, 1 ignored.
- `cargo test -p nova_scenario`: 416 + 1 passed, 1 ignored.
- `cargo test -p nova_ship --lib`: 961 passed, 0 failed.
- `probe run system_hull_scaling,system_section_severing,system_thrust_and_plume,system_railgun_lance
  --correctness-only`: all four OK, 6/8 measured each, `invariants_held` and
  `log_clean` PASS on every one (`probe-runs/27010fb39/index.html`).
- `mdbook build` and `web: npm run ci`: both clean.

### Documentation

- `web/src/create/sections.md`: the torpedo bay `fire_rate` contract.
- `docs/architecture.md`: the one-seeded-`GlobalRng` assembly contract and what
  a system that cannot find it does.
- `docs/sections.md`: the attitude stack skips a root it cannot measure.
- `CHANGELOG.md`: one Combat entry, one Ships & Sections entry marked
  `**(breaking)**`, one Internals entry and five Fixes entries.
