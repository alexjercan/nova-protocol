# Review: RCS fine-adjustment core primitive

- TASK: 20260718-122906
- BRANCH: feat/rcs-core

## Round 1

- VERDICT: APPROVE

Reviewed commit 4ad4cdf7 vs master (2ef18c22). The diff delivers the Goal: a
verb-gated, speed-capped, torque-free translational primitive driven by a
shared `RcsIntent`, with no player-input/HUD/autopilot coupling (correctly
deferred to the sibling tasks). All nine Steps are genuinely done.

Independent verification performed (same-session blind-spot guard):
- Re-derived the per-axis gate `clamp((cap - sign(cmd)*along) / band, 0, 1)` for
  all sign/velocity cases: at `+cap` a `+` command gates to 0, a `-` command
  saturates to 1; symmetric for the negative axis. Matches the "moving forward,
  RCS forward does nothing but backward still works" rule.
- Confirmed `Forces::apply_linear_impulse` acts at the COM (zero torque) AND
  wakes a sleeping body via `try_wake_up()` (avian3d-0.7 query_data.rs:387) - so
  a docked-at-rest ship still responds to the first nudge.
- Confirmed mass-scaling: `effective_inverse_mass * (accel*dt*mass) = accel*dt`
  with no locked axes, so `rcs_accel` is a true mass-independent acceleration.
- `cargo check --workspace --features debug` passes (exit 0); no `match
  FlightVerb` exists anywhere, so the new variant breaks no exhaustiveness.
- 5 new tests pass; no existing test weakened or deleted. The negative
  `rcs_does_nothing_without_the_verb` test is non-vacuous - deleting the verb
  gate makes it fail - and is delivery-guarded by the three positive tests
  sharing the same harness.

No BLOCKER or MAJOR findings. The items below are non-blocking.

- [x] R1.1 (MINOR) crates/nova_gameplay/src/flight.rs:~2000 (`rcs_burn_system`) -
  intent magnitude sets the ACCELERATION, not the terminal speed: any nonzero
  deflection held long enough asymptotes to the full `cap` on that axis (the
  gate only zeroes at `cap`). A feather-touch therefore still creeps to full cap
  speed. This is a defensible reading of the primitive, but the feel decision
  (should a half-deflection target a lower speed, i.e. scale the per-axis cap by
  `|cmd|`?) belongs to the player-input task. Flag it there (20260718-122912) so
  the mapping is chosen deliberately, not inherited by accident. Not a blocker.
  - Response: Flagged in task 20260718-122912's Notes (the feel decision is now
    explicit there) and recorded in NOTES.md. No primitive change - the behavior
    is the deliberate default until the input task decides otherwise.
- [x] R1.2 (MINOR) crates/nova_gameplay/src/flight.rs (RcsSpeedCap) - unlike its
  sibling `FlightSpeedCap`, `RcsSpeedCap` has no author-facing path: scenarios
  set `FlightSpeedCap` via the spaceship spawn config (nova_scenario
  objects/spaceship.rs:359) and a runtime action (actions.rs:909), but nothing
  can author a per-hull RCS cap - only the global `FlightSettings::rcs_speed_cap`
  default applies. Functional as-is, but the inconsistency will bite the first
  scenario that wants a hull with a different RCS ceiling. Either wire it (a
  follow-up task is fine) or add a one-line NOTES.md statement that the global
  default is intentional for now. Not a blocker.
  - Response: Documented in NOTES.md as an intentional deferral - the global
    default is the base primitive; per-hull scenario authoring is a clean
    follow-up if a scenario ever needs a distinct RCS ceiling.
- [x] R1.3 (NIT) crates/nova_gameplay/src/flight.rs (`rcs_burn_system`) - the cap
  is per ship-local axis, so a diagonal intent (e.g. forward+right) can reach a
  combined speed of up to `sqrt(2..3) * cap`. Consistent with the spike's
  per-axis design and fine for docking, but worth a one-line code comment so a
  future reader does not mistake `cap` for a speed-magnitude limit.
  - Response: Added the clarifying comment above the per-axis loop in
    `rcs_burn_system`.

### Round 1 resolution

All findings addressed (R1.1/R1.2 as docs, R1.3 as a code comment) - none
changed runtime behavior, so no re-review round was needed. Verdict stands:
APPROVE.
