# Review: Units - display 1 u = 10 m everywhere (m/km, m/s)

- TASK: 20260728-175731
- BRANCH: feat/units-x10-display

## Round 1

Verified the spec end to end against the branch diff. The shared formatter
lives in `nova_ui::units` beside the theme and is reused at all 11 live sites
(every converted call goes through `nova_ui::units::{distance,speed,
closing_speed}`, none re-implemented). Docs sweep complete - glossary redefines
m/km + m/s, all wiki pages converted with consistent x10 values, CHANGELOG line
added, dated history left verbatim. Tests are meaningful and live-system-driven
(`speed_chip_tracks...` -> `50.0 m/s`, `readout_fills_from_the_locked_target`
-> `DST 1.50 km` + `CLS +200.0 m/s`, new `map_range_renders...`); a no-op
system or a dropped x10 would fail them. The NOVA OS map INFO column is
trailing, so variable-width distance strings do not break its alignment
(passing `map_view_table_aligns...`).

Proofs run (out-of-context reviewer + in-session): `cargo check -p nova_ui
-p nova_gameplay` exit 0; `nova_ui` units 5 + doc 3 passed; `nova_gameplay`
hud modules 49 passed (+ flight_status/edge/lock/beacon re-run alone all
green); DoD 3/4 greps show zero player-facing `u`/`u/s` in crates and web.

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MINOR) crates/nova_ui/src/units.rs:29-33 - The m/km switch compares
  the raw product `metres < KM_THRESHOLD_M` before `{:.0}` rounding, so
  displayed metres in 999.5..1000.0 (world units 99.95..100.0) render as
  `"1000 m"` instead of crossing to `"1.00 km"`. Numerically correct but prints
  a four-digit metres string the km branch is meant to prevent. Switch on the
  rounded metres so no distance ever shows as `1000 m`.
  - Response: Fixed - switch is now on `metres.round() < KM_THRESHOLD_M`, so
    99.95 u (999.5 m) reads `1.00 km` and no distance ever prints `1000 m`. The
    boundary test is rewritten as `distance_boundary_never_prints_four_digit_
    metres` (99.95/99.99/100.0 all -> `1.00 km`) plus a 99.94 u -> `999 m`
    case. Verified: nova_ui units 5 + doc 3 green.
- [x] R1.2 (NIT) crates/nova_gameplay/src/hud/beacon_chips.rs:23 - `CHIP_SIZE`
  stays 140px while the max label grew to `"BEACON 1  12.34 km"` (18 chars). The
  chip is `NoWrap`, so a far beacon's km label clips rather than wraps; by the
  repo's own font budget (`READOUT_SIZE`, ~9.3px/char) 18 chars needs ~168px.
  Bump the chip width so km labels fit.
  - Response: Fixed - `CHIP_SIZE` width bumped 140 -> 168 px with a comment
    noting the 18-char `NoWrap` budget. Verified: beacon_chips tests green.

Round-1 verdict was APPROVE (both findings non-blocking); addressed anyway in
the same round since both touch the owner-eyeball display surface. Fixes
verified in-session (nova_ui units + doctests, beacon_chips tests all green).
