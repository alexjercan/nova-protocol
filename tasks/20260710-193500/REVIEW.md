# Review: GOTO arrival planning is gravity-blind

- TASK: 20260710-193500
- BRANCH: fix/gravity-aware-arrival

## Round 1

- VERDICT: REQUEST_CHANGES

Verified sound (derived/re-run independently by the reviewer): the
substitution algebra and quadratic root in arrival_speed_limit (g=0
reduces bit-identically to the old form; the check equation closes),
agreement of the three sibling helpers with the flip identity, the
recomputed test constants (103.142857 / 20.976 / 53.1429), caller units
(raw accel+margin vs pre-margined brake_accel - critically not the
gravity-reduced local, no double subtraction), closure safety (only
called with distance > standoff; NaN cannot arise), q_wells filter,
STOP-degradation unreachability of the 1e-3 eta sliver, no helper call
sites outside flight.rs, integration-test determinism, no weakened
pre-existing assertions, and the honesty check (budget force-zeroed ->
test fails at 12u from a 40u body's center) reproduced by the reviewer.

- [x] R1.1 (MAJOR) flight.rs (arrival_eta) - the promised eta=None
  degradation is not implemented: with effective <= 0 goto_flip_point
  returns None and arrival_eta falls into its braking-regime fallback
  (2*remaining/v), so the pilot sees a confident ETA on a leg the
  computer just declared unstoppable, while the code comment and docs
  claim "flip/eta are None". Return None from arrival_eta when
  brake_accel - g <= 0, and add a unit test.
  - Response: fixed - arrival_eta returns None up front when
    `brake_accel - g <= 0` (both regimes), doc updated ("a blank chip
    beats a confident lie"), unit tests added for the coast and braking
    distances at g=10.
- [x] R1.2 (MINOR) flight.rs (arrival_desired) - the degradation debug!
  fires every tick while degraded; the task step says "debug! once".
  Implement once-per-leg logging or amend the docs.
  - Response: fixed - the ship query's Has<ManeuverTelemetry> became
    Option<&ManeuverTelemetry>; the log is gated on the previously
    published plan still having brake authority, so it fires once per
    degradation entry. Docs state the exact semantics.
- [x] R1.3 (MINOR) docs + TASK.md Resolution - "the decel chip
  self-corrected" describes a consumer that does not exist:
  ManeuverTelemetry.brake_accel has zero readers in the workspace.
  - Response: fixed - docs and Resolution now say the field is
    write-only in the HUD today and forward-correct for a future decel
    readout; the in-code comment at the publish site says the same.
- [x] R1.4 (MINOR) flight.rs (ManeuverTelemetry.brake_accel field doc) -
  still reads "margin applied, zero inside the standoff"; it is now
  gravity-reduced and also zero outside the standoff when degraded.
  - Response: fixed - field doc rewritten with both facts and a pointer
    to the spike.
- [x] R1.5 (MINOR) TASK.md step 5 - "a control run in flat space must
  keep today's arrival frames" overstates what was tested (no
  frame-parity harness exists); and the reviewer did not verify the
  "332 pre-existing tests pass" claim.
  - Response: fixed - step text now says the control is the untouched
    pre-existing arrival tests (behavior, not frame parity). The 332
    figure is the implementer's own full `cargo test -p nova_gameplay
    --lib` run in the worktree immediately after the arity change and
    before the new tests landed; it stands.
- [x] R1.6 (NIT) flight.rs (gravity_along) - fold(0.0, f32::max) takes
  the strongest single well, under-budgeting overlapping SOIs; sum of
  positive contributions is strictly more conservative and no more code.
  - Response: taken - per-well components are clamped at >= 0 and
    summed; comment updated.
- [x] R1.7 (NIT) flight.rs (orbit_desired_velocity) - the "v_circ
  already balances the well" justification is looser than it sounds
  during ring capture.
  - Response: taken - comment rewritten to name the real mitigations
    (stable band above the fade, bounded nudge, per-tick reissue,
    unchanged from master) instead of the loose claim.
- [x] R1.8 (NIT) flight.rs (goto_desired_velocity doc) - the min_approach
  floor still applies in the near-goal band where the limit dips below
  it under a survivable pull; the docstring only covered the
  full-degradation case.
  - Response: taken - docstring names the band and why the floor is
    intentional there.

## Round 2

- VERDICT: APPROVE

All eight round-1 findings verified resolved against commit 8178d3a (the
reviewer re-ran flight 54 / input::ai 73 / hud 55 and re-checked the
arrival_eta guard placement, the once-per-leg log gate semantics including
borrow soundness of the Option<&ManeuverTelemetry> capture, and the
per-well clamp-then-sum shape - signed summing would have let an opposing
well bank credit). No new findings at MAJOR or MINOR.

One non-blocking NIT left open by agreement: the log gate reads a
previously published inside-standoff brake_accel of 0.0 as "already
degraded", so a ship that falls through the standoff of an unstoppable
well and climbs back out loses one debug line on re-entry. Accepted as is.
