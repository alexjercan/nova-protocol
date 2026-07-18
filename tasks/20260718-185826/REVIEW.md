# Review: RCS mouse control delta-driven instead of virtual-joystick accumulate

- TASK: 20260718-185826
- BRANCH: feat/rcs-delta-control

## Round 1

- VERDICT: APPROVE

Small, well-contained change reversing the held-direction accumulate to a
delta-driven SET + per-tick decay. Diff reviewed against master; the
`flight::` + `input::player::tests::rcs` suites run green (77 passed, 0
failed), including the two new/changed tests.

Independently re-verified the load-bearing claims (same-session review, so
this is the required cross-check):

- Chain ordering. The comment claims the intent is "spent before it decays".
  Confirmed: the FixedUpdate chain is `autopilot_system, manual_burn_system,
  rcs_burn_system, decay_player_rcs_intent` (flight.rs:524, mirrored in the
  test harness at :2989). `rcs_burn_system` reads `RcsIntent` and applies the
  impulse, THEN `decay_player_rcs_intent` shrinks it. Correct order - decaying
  first would waste a fraction of every tick's command.
- Gate correctness. `decay_player_rcs_intent` queries `With<RcsActive>`.
  `RcsActive` is only the player's SHIFT modal; the autopilot writes its
  `RcsIntent` on ships without `RcsActive` (task 20260718-122932) and rewrites
  it every loop, so the decay must skip it. The query does. No autopilot
  regression in the suite.
- Both tests fail without their fix (meaningful, not vacuous):
  - `rcs_mouse_motion_sets_intent_from_the_delta_only_while_active`: under the
    old accumulate, a second (10,0) motion would give `intent.x = first + 0.2`,
    so `(intent.x - 0.2).abs() < 1e-4` fails. The harness runs no decay, so the
    shrink is provably from the SET, not the decay - a clean isolation.
  - `player_rcs_intent_decays_when_input_stops_but_autopilot_intent_does_not`:
    remove the decay system and the player's 0.8 intent persists (> 1e-3),
    failing the first assertion; the paired autopilot-proxy assertion (no
    `RcsActive`, stays > 0.5) guards that the gate is doing the discriminating,
    not a blanket decay.

Notes / non-blocking observations:

- NIT flight.rs decay: `intent.0 *= 0.4` then snap-to-zero below `1e-4`
  (length_squared), i.e. length < 0.01. Fine - it just guarantees the ship
  coasts rather than creeps at a sub-milli intent. No change needed.
- The autopilot ship in the flight test is a proxy (a bare `RcsIntent` with
  nothing rewriting it), not a live `autopilot_system` run. That is the right
  scope for THIS test - it isolates the decay gate. The live autopilot RCS
  behavior is already covered by the settle tests from 20260718-122932, which
  still pass here.
- Sensitivity/decay are single tunable constants (`RCS_AIM_SENSITIVITY = 0.02`,
  `RCS_PLAYER_INTENT_DECAY = 0.4`); a live feel-retune is cheap follow-up if
  needed, not a blocker. NOTES.md documents both.

No BLOCKER/MAJOR/MINOR findings. Ship it.
