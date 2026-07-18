# RCS error-relative mode for autopilot ORBIT station-keep (needs primitive redesign)

- STATUS: CLOSED
- PRIORITY: 1
- TAGS: v0.7.0,feature,flight,spike

## Goal

Let the autopilot use RCS for ORBIT station-keeping (and any correction while the
ship moves faster than the fine-adjust cap). Split out of the autopilot-RCS task
(20260718-122932), which delivered only the GOTO/STOP terminal settle because:

- `rcs_burn_system` caps ABSOLUTE along-axis speed at `rcs_speed_cap` (2 u/s):
  the gate is `along = velocity.dot(world_axis)`, absolute velocity.
- ORBIT moves at `circular_orbit_speed = sqrt(mu/r)` ~= 2.5-6 u/s (>2 u/s cap),
  so a prograde RCS push gates to zero and a retrograde one BRAKES the orbit.
  RCS as-built cannot express "add a small correction while already moving at
  orbital speed".

The fix needs a PRIMITIVE change, not just an autopilot hookup, so it is its own
task (and probably its own spike):

- A target-relative / error-relative RCS mode: cap the CORRECTION relative to a
  supplied desired velocity (`|v - desired| < cap`), not the absolute speed - so
  the same primitive can trim a fast-moving orbit by a sub-cap delta.
- Must NOT regress the player-facing absolute-cap mode (the SHIFT+mouse
  fine-adjust feel, task 20260718-122912). Likely a second mode/flag on
  `rcs_burn_system` or a sibling system, chosen deliberately.

## Design (spike folded in, see NOTES.md)

The absolute along-axis cap in `rcs_burn_system` is what excludes ORBIT. The
fix is to cap the along-axis component of `(velocity - reference)` where
`reference` is a supplied world-frame reference velocity that DEFAULTS TO ZERO.
Zero reference == today's absolute cap, so the player fine-adjust mode and the
STOP/GOTO terminal settle are byte-for-byte unchanged. The autopilot supplies
`reference = desired` (orbital velocity) only for ORBIT, so RCS trims the
sub-cap residual `|v - v_orbit|` instead of fighting the orbital speed. Chosen
over a second sibling burn system (duplicates the taper math) and over a
mode-flag on RcsIntent (a reference velocity is the actual physical quantity).

## Steps

- [x] Add `RcsReference(Vec3)` component (world-frame reference velocity, on the
  ship root) in flight.rs near `RcsSpeedCap`; doc that absent/zero == absolute
  cap. Register it with `register_type` in BOTH the real plugin (near line 513)
  and the flight test harness.
- [x] `rcs_burn_system`: add `Option<&RcsReference>` to `q_ship`; inside the
  axis loop compute `along = (velocity - reference).dot(world_axis)` with
  `reference` defaulting to `Vec3::ZERO` when absent. Update the cap comment:
  it now caps the along-axis RESIDUAL relative to the reference, not the
  absolute speed.
- [x] `autopilot_system`: set an `is_orbit` bool true in the
  `AutopilotAction::Orbit` match arm. Add `Option<&mut RcsReference>` to the
  ship query. Keep the existing settle gate as `use_rcs_settle`; add
  `use_rcs_orbit = rcs_granted && rcs_cap > 0 && is_orbit && error_speed <
  rcs_cap && error_speed > 1e-3`; `use_rcs = use_rcs_settle || use_rcs_orbit`.
  Set `rcs_reference = if use_rcs_orbit { desired } else { Vec3::ZERO }` and
  write it to `RcsReference` (insert if absent) every autopilot tick, exactly
  as `RcsIntent` is written. `rcs_command` uses the SAME `error / rcs_cap`
  formula (error = desired - v) for both branches.
- [x] Cleanup (`shared-primitive-clear-on-handoff`): zero `RcsReference` in
  `on_autopilot_removed_cool_engines` alongside the RcsIntent zeroing, so a
  stale orbital reference never leaks into the player's absolute-cap mode.
- [x] Tests (flight.rs): (a) rewrite `orbit_never_engages_rcs` into
  `orbit_engages_rcs_only_to_trim_a_sub_cap_residual` - orbit near the ring
  engages RCS with `RcsReference == desired` and a cold main drive, but does
  NOT engage while spinning up from rest (residual > cap); (b) add
  `rcs_relative_cap_trims_a_fast_moving_reference` - a headless burn unit test:
  a ship at ~5 u/s with `RcsReference` ~5 u/s and a small trim intent gets a
  real impulse, and WITHOUT the reference the same command gates to zero;
  (c) add `orbit_rcs_reference_clears_on_disengage`. Confirm the player
  (`rcs_holds_the_cap_forward_but_reverses_freely`, etc.) and STOP
  (`stop_terminal_brakes_via_rcs`, `rcs_settled_*`) tests still pass unchanged.
- [x] NOTES.md design record (serves as the spike output for Fork 4's deferred
  error-relative question).

## Notes

Spike: tasks/20260718-122508/SPIKE.md (Fork 4). Parent: 20260718-122932 (GOTO/STOP
terminal RCS landed). The incompatibility is documented there and in
tasks/20260718-122932/NOTES.md. The spike's deferred error-relative question is
resolved in this task's NOTES.md rather than a separate spike doc - the design
uncertainty was narrow (the cap reference frame) and the code map made it
mechanical. RCS is verb-gated so the mainline campaign (RCS withheld) is
unaffected: orbit-RCS only engages on RCS-granting hulls.

## Close-out (2026-07-18)

Delivered. `RcsReference(Vec3)` rebases the RCS cap onto `v - reference`;
absent/zero reproduces the absolute cap exactly, so player fine-adjust and the
STOP/GOTO settle are untouched. The autopilot writes `reference = desired`
(orbital velocity) only in the `is_orbit` branch, gated on the residual
`error_speed < cap`, so RCS trims a fast orbit by a sub-cap delta while the
main drive still handles spin-up and large ring corrections. Reference is
written every autopilot tick (zero when not trimming) and zeroed on disengage.

Verification: full `flight::` suite 74 passed, 0 failed. New: the primitive
unit test `rcs_relative_cap_trims_a_fast_moving_reference` (5 u/s ship trimmed
with a 5 u/s reference pushes; without the reference the same command gates to
zero - fails if the reference term is reverted), the contract test
`orbit_engages_rcs_only_to_trim_a_sub_cap_residual` (no RCS while spinning up
from rest; once orbiting, `|v - reference| <= cap` and `reference > cap`), and
the off-ramp `orbit_rcs_reference_clears_on_disengage`. The pre-existing
`orbit_engages_from_near_rest_and_holds_the_ring_for_a_lap` still holds the
ring for a full lap, now exercising the RCS trim path - the integration proof
that the trim is stable, not just present.

Difficulty: the one real risk was regressing the 60+ existing flight tests.
The reference-defaults-to-zero identity is what made that safe by construction
(every existing path leaves the reference unset). The only behavior test that
had to change was `orbit_never_engages_rcs`, whose asserted non-behavior this
task intentionally reverses; rewritten to pin the new sub-cap-trim contract.