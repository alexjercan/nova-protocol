# Balance a single off-center main drive with off-axis counter-torque thrusters

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.4.0,handling,physics

Follow-up to the thrust-balancing task (20260709-155920,
docs/2026-07-09-thrust-balancing.md). That task balances torque by differential
throttle *within the firing set* - the engines already pushing toward the burn.
It cannot help the most common damage case: a ship with one centered main drive
that loses a side section, so the COM shifts and the lone drive is now
off-center with nothing in the firing set to trim against. It fires at full and
the PD holds the residual (or pulls past its cap).

The realistic fix is to let the flight computer optionally fire an *off-axis*
thruster (retro/lateral, outside the burn's alignment cone) purely to produce a
counter-torque that nulls the main drive's torque about the COM. This is the
full "control allocation" endgame: solve for a per-engine throttle vector over
ALL live engines against a desired wrench, not just the forward set.

## Steps

- [ ] Decide the trade with the user: a counter-torque burn adds a sideways net
      force the maneuver did not ask for (unless paired symmetrically), so the
      allocation must weigh straightness vs. unwanted translation. Confirm
      whether to constrain net perpendicular force to zero (needs an opposing
      pair) or allow a bounded lateral drift.
- [ ] Generalize `balance_throttles` (or add a sibling) to a wrench-space
      allocation: choose u_i in [0,1] over all live engines to match the desired
      force along the burn AND null torque about the live COM, minimizing
      off-axis force. Reuse the projected-gradient structure.
- [ ] Physics test: a single main drive on a damage-shifted hull holds its
      heading under burn by recruiting a lateral, within the centered tolerance.

## Notes

- The firing-set-only boundary is documented in
  docs/2026-07-09-thrust-balancing.md ("Scope and boundaries").
- Watch fuel/thrust honesty: a recruited lateral spends thrust that does not go
  toward the goal - that cost should stay legible, not hidden.

