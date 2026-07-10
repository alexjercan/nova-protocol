# Gravity indicator in the velocity-sphere family; SOI shell removed

- TASK: 20260710-201514
- MODULE: crates/nova_gameplay/src/hud/velocity.rs (widget family),
  hud/holo_instruments.rs (shell removal)

## What was built

Playtest feedback (user, 2026-07-10): the SOI shell read the same as the
orbit ring. It is gone; gravity now shows the way velocity does.

- The velocity widget became a small family: `VelocityHudSource`
  (Velocity | Gravity) picks what the orbiting cone + shaded sphere
  express, and `VelocityHudPalette` carries the colors (defaults preserve
  the original white/blue; `GRAVITY` is yellow) so the two quantities
  never read as one. The child-spawn observers read the palette instead
  of hardcoding colors.
- The gravity variant points down the dominant well's pull, scales its
  shader magnitude by the felt `well_accel` normalized against
  `max_surface_gravity` (pure helper, unit-tested), and hides itself
  (root Visibility) in flat space. It nests at radius 5.6 outside the
  velocity sphere's 5.0 so the shells never z-fight.
- `sync_soi_shell`, `SoiShellRing`, their constants, test, and cleanup
  wiring were removed from holo_instruments; the ribbon, flip gate, and
  orbit ring stay. The [O] ORBIT cue and the GRAV status line remain the
  text channels for "you are in a well".

## Decisions

- Generalize-not-copy: one widget with a source enum beats a parallel
  gravity_hud module (the shader/orbit machinery is identical). The
  module keeps its historical velocity.rs name to avoid churn; renaming
  to direction_hud is noted as a future cleanup.
- Magnitude normalization by the strength cap means "full sphere" =
  "surface-strength pull" - the same scale the escapability guardrail is
  authored in.

## Verification

- 3 new widget tests (pure magnitude incl. degenerate cap; gravity
  variant points down the well / hides in flat space; velocity variant
  unchanged and never self-toggles) + holo module tests green minus the
  removed shell test; hud module (55) green; the 05_directional example
  updated for the config's new fields (caught by the --examples check).
  fmt + check --workspace --examples clean. Full suite and clippy on CI.

## Difficulties

None material.

## Self-reflection

- Third example-initializer break on a config-struct change; the
  --examples check catches it every time, but adding `..default()` to
  a struct's construction sites when ADDING defaulted fields would avoid
  the breakage class entirely - consider defaulted-field additions being
  paired with a construction-site grep as a matter of course.
