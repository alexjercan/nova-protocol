# Torque authority is size-blind: big ships cannot turn

- STATUS: OPEN
- PRIORITY: 48
- TAGS: v0.11.0,ship,combat,physics

## The limitation (investigated 2026-08-17, not yet fixed)

Big ships are 50-100x under-actuated per radian. Owner's report from flying
a wfc arena hull: "the rotation is so slow it's really hard to control them,
trying to fly forward using thrust causes too much torque on them and the
controller cannot manage it."

Root cause, measured:
- The attitude PD is inertia-aware in DEMAND: it scales its correction by
  the world inertia tensor (pd_controller.rs), so it asks for the right
  torque at any size.
- The OUTPUT clamps at the controller section's authored
  `max_torque: 40.0` (basic_controller_section) - an absolute number,
  size-blind. Controller stacking deliberately saturates at 2x
  (STACK_AUTHORITY_LIMIT), so several bridges top out at 80.
- An arena hull: ~200-250 sections at mass 1.0 each over ~11 cells gives
  yaw inertia ~2000. Angular acceleration = 40 / 2000 = 0.02 rad/s2 - a 90
  degree turn takes 10+ seconds. The campaign cast (inertia ~50-200) gets
  0.2-0.8 rad/s2, which is why the number never felt wrong before.
- The thrust-torque half of the complaint is the same weakness: manual burn
  IS torque-nulled (balance_throttles allocates differential throttle about
  the live COM), but transients (spool ramps, one-tick-stale pose,
  quantized allocation) leak torque that a 40-unit budget swats instantly
  on a cast ship and cannot damp on a 2000-inertia hull.

## The fix direction (recommended)

Author torque in ACCELERATION units: reinterpret the authored value as max
angular acceleration; effective torque = alpha_max * inertia, clamped
per-axis against the principal tensor the PD already holds. Handling
becomes size-invariant by default; ship feel is then differentiated
deliberately per prototype. alpha_max ~= 0.5 rad/s2 reproduces the current
cast feel almost exactly, so nothing already-tuned changes.

Alternatives considered and ranked below it: a capital-grade controller
prototype (content-only, but player-built hulls still hit the cliff);
mass-scaled authority per section (hides the lever); raising the stack
limit (fights deliberate anti-stacking doctrine).

## Done when

- the clamp site in pd_controller.rs speaks acceleration units and the
  authored 40.0 is re-expressed with its derivation in a comment
- existing PD + flight lib tests green; a new test pins that two hulls of
  10x different inertia converge a fixed attitude error in comparable time
- a hand-flown arena capital feels controllable (owner playtest)
