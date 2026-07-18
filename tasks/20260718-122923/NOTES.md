# RCS HUD indication - design / fix record

Task: 20260718-122923. Spike: tasks/20260718-122508/SPIKE.md.

## What shipped

The RCS-active PALETTE on the velocity sphere (the diegetic "I am nudging by
hand under the cap" cue), in crates/nova_gameplay/src/hud/velocity.rs:

- `VelocityHudPalette::RCS_ACTIVE`: a violet (indicator/sphere srgba
  0.72,0.45,1.0) distinct from manual blue, autopilot cyan, and gravity yellow.
- `desired_velocity_palette(engaged, rcs_active)`: pure fn, RCS wins over engaged
  (the two are mutually exclusive on the player ship - entering RCS disengages
  the autopilot - but the precedence is pinned).
- `sync_engaged_palette` now also reads the target ship's `RcsActive` (a
  `Query<(), With<RcsActive>>`), so the sphere flips to violet while SHIFT is
  held. Material-push path unchanged.
- Keyed on `RcsActive` (the player SHIFT modal), NOT `RcsIntent`: so when the
  autopilot later drives `RcsIntent` (task 20260718-122932) the sphere still
  reads ENGAGED (computer-flown), which is correct - that falls out for free.

## Scope: the cap ring was split out (task 20260718-144939)

The task title said "palette + cap ring". I delivered the palette and SEEDED the
cap ring as a separate task, because:

- The sphere is fixed-radius; speed drives a shader magnitude, not the radius, so
  "a ring at the cap" has genuinely underspecified visual semantics (latitude
  ring? torus child? shell?) - a design decision, not a mechanical add.
- It is new geometry/shader work whose "does it read right" can only be judged by
  eye in the running game (a playtest), which this headless flow cannot do.
  Shipping unverifiable visual geometry would be dishonest.

The palette alone delivers the core "RCS is active" indication.

## Difficulties / surprises

- Changing `desired_velocity_palette`'s arity broke the existing
  `desired_velocity_palette_picks_by_engagement` unit test - updated it (renamed
  to `_picks_by_state`) to the 2-arg signature covering all three states.
- Fresh-worktree first build was ~5 min (full crate recompile); later runs in the
  worktree are incremental.

## Tests (all green)

- `desired_velocity_palette_picks_by_state` (pure: manual/engaged/rcs + RCS wins).
- `velocity_palette_follows_rcs_active` (integration, mirrors the autopilot one):
  RcsActive -> RCS_ACTIVE, release -> default, and an engaged autopilot without
  RcsActive still reads ENGAGED.
- The whole `hud::velocity` module suite (8 tests) stayed green - the existing
  `velocity_palette_follows_the_autopilot` still passes after the arity change.

Per repo policy the full suite / clippy run in CI; ran check, fmt, and the
hud::velocity module suite locally.
