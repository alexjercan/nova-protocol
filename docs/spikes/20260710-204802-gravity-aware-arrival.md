# Spike: How should GOTO/STOP arrival planning account for gravity wells?

- DATE: 20260710-204802
- STATUS: RECOMMENDED
- TAGS: spike, autopilot, gravity, flight

## Question

The arrival solver (task 20260710-193500) plans as if the brake group's
thrust is the only acceleration in play, so a GOTO toward a well body
brakes too late and the ship impacts the surface. What change to the
arrival math makes the ship reliably stop at the standoff inside an SOI,
without destabilizing the legs that work today (flat space, ships,
GotoPos)? A good answer names the formula, where it lives, and what the
telemetry (FLIP marker, ETA) inherits for free.

## Context

- The solver is per-tick: `arrival_desired` in `autopilot_system`
  (crates/nova_gameplay/src/flight.rs) recomputes the desired velocity
  every tick from `arrival_speed_limit(distance, accel, margin, lead)`,
  which solves `v*lead + v^2/(2*a*margin) = d`. `goto_flip_point` and
  `arrival_eta` mirror the same terms into ManeuverTelemetry, so the HUD
  instruments show whatever the solver believes.
- The well is bounded and capped: `well_accel` (gravity.rs) is
  inverse-square, clamped at the surface (max
  `GravitySettings::max_surface_gravity` = 5.0 by construction of
  `GravityWell::from_surface_gravity`), smoothstep-faded to zero at the
  SOI edge. A ship's brake acceleration is typically 15-25 u/s^2, so
  gravity eats up to ~a third of the brake authority near the surface -
  not enough to make stopping impossible, enough to blow through a 50u
  standoff at approach speeds.
- Two error terms, both signed toward the body on an inward leg:
  1. Braking outward against the pull: effective deceleration is
     `a*margin - g_along`, not `a*margin`.
  2. The un-braked lead window (flip rotation + spool, ~1-2s): gravity
     keeps adding `g*lead` of speed and `g*lead^2/2` of distance that the
     current formula does not budget.
- The gravity spike (docs/spikes/20260709-193147, decision 6) accepted
  this error for v1 on the strength-guardrail argument and recorded the
  feedforward as follow-up; the playtest showed the error is fatal near
  big bodies, not small.
- `DominantWell(Entity)` on the ship names the one well that matters
  (single-attractor model), and `autopilot_system` already has
  `q_wells: Query<(&Position, &GravityWell)>` and `Res<GravitySettings>`
  for ORBIT - the solver can read the pull without new plumbing.
- Related queued tasks touch the same code: 20260710-202408
  (surface-relative standoff) changes what `standoff` means; 20260710-
  195954 (GOTO parks into ORBIT) consumes the arrival. Whichever lands
  last re-reads the others.

## Options considered

- **(a) Feedforward from the current position** - each tick, subtract the
  pull at the ship's position (projected on the closing direction) from
  the brake acceleration; let per-tick replanning absorb the rest. Cheap
  and local. But on a descent the pull GROWS toward the goal
  (inverse-square), so the plan is always optimistic exactly when it
  matters: at the flip point (far out) it budgets the weak far-field pull,
  then discovers the strong near-field pull only after committing. The
  replanning correction arrives as a deficit the brake group may no longer
  have room to pay. This is the mechanism of the observed crash, softened
  but not removed.

- **(b) Integrate the pull over the predicted brake arc** - the honest
  version: the well is conservative, so the speed gained falling from the
  flip radius to the standoff radius is exactly the potential difference
  of the clamped/faded profile. Exact for radial legs; but the potential
  of the piecewise profile (core inverse-square, surface clamp, smoothstep
  fade) is a three-branch integral, the leg is not always radial, and the
  per-tick replanning that already exists makes exactness redundant: any
  conservative bound is corrected 60 times a second. High math surface for
  little behavioral gain over (d).

- **(c) Minimum-standoff clamp against well bodies** - a crash guard
  independent of the solver (never plan a rest point inside
  `body_radius + margin`). Worth having, but it is really task 20260710-
  202408 (surface-relative standoff) wearing a helmet: it fixes where the
  ship tries to stop, not whether it can. A hot arrival still overshoots
  through any standoff.

- **(d) Worst-point feedforward: budget the pull at the goal, plus the
  lead-window terms** - evaluate the well's pull at the GOAL position
  (the standoff point - the strongest pull on a monotonic inward leg,
  since `well_accel` is monotonic decreasing in r above the clamp),
  project it on the closing direction, and clamp at >= 0 (a pull that
  helps braking is ignored, never subtracted into extra confidence). Feed
  that `g_along` into the solver as: effective brake `a_eff = a*margin -
  g_along`, lead-window speed gain `v_brake = v + g*lead` and distance
  `v*lead + g*lead^2/2`. The arrival quadratic stays closed-form; `g = 0`
  reduces every formula to today's byte-for-byte. Conservative everywhere
  on an inward leg (plans for surface-strength pull from the first tick),
  which costs a slightly earlier flip and slower final approach - the
  safe direction to be wrong in, and per-tick replanning re-tightens the
  plan as the real pull catches up to the budgeted one. If `a_eff <= 0`
  (brake weaker than the well) the solver has no plan; degrade explicitly
  rather than dividing by a sliver.

- **Do nothing** - rejected by playtest; GOTO at the Gravity Rock is a
  crash today, and GOTO is the verb the HUD teaches first.

## Recommendation

Option (d), implemented as a `gravity_along: f32` parameter threaded
through the four pure helpers (`arrival_speed_limit`,
`goto_desired_velocity`, `goto_flip_point`, `arrival_eta`;
`stop_rest_distance` gets it too so STOP's rest-point telemetry stops
lying inside a well), computed once per tick in `arrival_desired`:

- `g_along = max(0, dot(g_vec_at_goal, closing_dir))` where `g_vec_at_goal
  = well_accel(|goal - well|) * normalize(well_pos - goal)` for the ship's
  `DominantWell`, else 0. Evaluating at the goal (not the ship) is the
  load-bearing choice - it bounds the pull over the whole remaining leg
  for the common case (goal at a standoff of the well body itself).
- Modified arrival rule: solve `v*lead + g*lead^2/2 + (v + g*lead)^2 /
  (2*(a*margin - g)) = d` for v (quadratic, closed form); zero/no-plan
  when `a*margin <= g`.
- Flip point gains the matching terms: `flip_from_goal = standoff +
  v*lead + g*lead^2/2 + (v + g*lead)^2/(2*(a_margin - g))`.
- `ManeuverTelemetry.brake_accel` reports the effective (gravity-reduced)
  value, so the FLIP marker, decel chip and ETA correct themselves with
  no HUD changes - the seam the telemetry refactor was built for.
- Degradation: if `a_eff <= 0` the leg cannot guarantee a stop; keep
  flying the (unreachable) desired velocity but publish telemetry with
  `flip_point/eta = None` and log once. Do not silently disengage - the
  pilot sees the instruments go blank, which is the honest signal.

Runner-up notes: (b) is the upgrade path if a future well is strong
enough that the worst-point bound over-brakes annoyingly (the bound's
cost grows with SOI size); (c)'s geometry half is already queued as
20260710-202408 and composes cleanly with this - (d) fixes the dynamics,
202408 fixes the geometry.

## Open questions

- Grazing legs: a GOTO whose path passes NEAR the well but whose goal is
  outside it has its worst pull mid-path, not at the goal. The goal-point
  bound under-budgets there. Deferred: the dominant-well fade makes the
  transverse encounter brief, per-tick replanning corrects through it,
  and the failure mode is a sloppy stop, not a crash (the body is not at
  the goal). Revisit if playtests show it.
- STOP inside a well still completes and hands back control while
  falling (intentional, per the gravity spike); this task only makes its
  rest-point telemetry honest, not its semantics.
- Whether `min_approach_speed` (floor on the desired speed) should also
  be gravity-adjusted; left alone - it only matters inside the last few
  units where the standoff gate ends the leg.

## Next steps

- tatr 20260710-193500 (this task): implement option (d); /plan owns the
  step breakdown, citing this doc.
