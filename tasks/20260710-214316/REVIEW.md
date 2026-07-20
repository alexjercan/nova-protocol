# Review - Holo ribbon terminates at the arrival park point

Branch: fix/ribbon-park-point, commits 53d26f3..f9fef5e.
Scope: ManeuverTelemetry.park_point (flight.rs), ribbon endpoint
(hud/holo_instruments.rs), test-helper literals
(hud/maneuver_instruments.rs), tasks/20260710-214316/NOTES.md.

## Round 1 (2026-07-11)

Independent re-derivation of the formula: outside the envelope
`park = goal - closing_dir * standoff` puts the point exactly
`arrival_standoff + radius` from the center on the closing line, which
matches the rest point `goto_desired_velocity` flies
(`remaining = distance - standoff`). Inside, `min(standoff, distance)`
gives `goal - closing_dir * distance = position` - the ship itself, so
the instrument draws no leg the computer will not fly. STOP publishes
`park_point == goal`, its rest point: correct, and the ribbon's STOP
behavior is unchanged. The envelope test in the new integration
assertion (`numbers.distance <= arrival_standoff`, surface-relative)
is algebraically the same gate as production's `goto_arrived`. The
fail-first evidence (ribbon at [0,0,-300] vs park [0,0,-250], the full
50u standoff) is recorded in the docs entry. Consumer sweep: only
flight.rs, holo_instruments.rs, maneuver_instruments.rs touch
`ManeuverTelemetry`; the destination readout chip still anchors at the
goal center, which is right - it marks the destination, not the stop.
The inside-envelope sample carries a delivery guard
(`expect("the leg passes through the park envelope before release")`),
so the null-shaped assertion cannot pass vacuously.

### Findings

- **R1.1 (MINOR)** flight.rs,
  `goto_standoff_is_surface_relative_for_sized_targets`: the round
  added `let standoff = ...arrival_standoff` mid-test, but the
  pre-existing identical binding after the release loop now shadows it
  redundantly. Delete the second binding; the next reader should not
  have to check whether the two values differ.
- **R1.2 (MINOR)** flight.rs, `arrival_desired`: the closure now
  normalizes `to_target` four times (`closing_speed`, `park_point`,
  `closing_dir`, and the `flip_point` map). Hoist one
  `let closing_dir = to_target.normalize_or_zero();` above the branch
  and reuse it everywhere (the else branch guarantees
  `distance > standoff > 0`, so `normalize_or_zero` is exact there).
  Behavior-identical; removes a re-derivation the reviewer had to
  check three times.
- **R1.3 (INFO, no change requested)**: in the degraded
  no-stopping-plan state (`brake_accel == 0` outside the envelope) the
  ribbon now ends at a park point the computer cannot promise to
  reach. This is strictly less of an over-promise than the old center
  endpoint, and flip/eta already go blank there; noting for the record
  only.

- VERDICT: REQUEST_CHANGES (R1.1, R1.2)

## Round 2 (2026-07-11)

Commit 542c863 addresses both findings. R1.1: the shadowed second
`standoff` binding is gone; the mid-test binding now feeds the
park-envelope loop and the final park assertion alike. R1.2: a single
`closing_dir = to_target.normalize_or_zero()` above the branch feeds
`closing_speed`, `park_point`, `brake_dir`, `gravity_along` and the
`flip_point` map; in the else branch `distance > standoff > 0`, so the
zero-fallback never engages there and the substitution for the old
`to_target.normalize()` is exact. The full goto_ test family (13 tests,
including arrival settling and well arrivals that exercise the flip
map) is green after the refactor. R1.3 stands as recorded, no change
requested.

- VERDICT: APPROVE
