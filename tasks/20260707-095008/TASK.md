# PDC turret test range example (playable + gates + autopilot)

- STATUS: CLOSED
- PRIORITY: 90
- TAGS: v0.4.0,example,turret

The PDC (point-defense) turret section feels clunky to tune because there is no
focused way to exercise it. Build a dedicated example scene that is a small
playable test range for the turret, in the spirit of the gated example scenes in
`~/personal/bevy-common-systems` (e.g. 09_reactor, 13_glide).

Goal: make the turret cheap to iterate on and to regression-test.

## Steps

- [x] Add `examples/08_turret_range.rs` with a single player ship carrying one turret
      section (plus controller + hull). The scenario system's chase camera observes it.
- [x] Spawn target gates: four static gates spread across the turret's firing arc plus
      one that sweeps back and forth across the front. The range points the turret at the
      sweeper so tracking is exercised; a throttled `turret: aim error N deg, M bullets in
      flight` readout scores tracking quality and firing.
- [~] Live tuning sliders: deferred to a small follow-up (20260707-150002) - the aim-error
      telemetry + gizmos already make the aiming legible, and sliders are UI-heavy. Tune
      for now by editing the section config and re-running.
- [x] Wire the BCS autopilot + screenshot harness. Headless run: reached Playing, turret
      tracks + fires (bullets in flight climbs to ~290), cycle complete, no panic.
- [x] Diagnose the clunky aiming: confirmed. The fixed angular-rate slew has no lead, so
      the barrel lags the sweeping gate - aim error catches to ~7 deg then breathes back up
      to ~20 deg and oscillates as the gate reverses. Filed as a fix task (20260707-150001).
- [x] Ran headless via `BCS_AUTOPILOT=1` under Xvfb; telemetry as above.

## Resolution

Added `examples/08_turret_range.rs`: a player turret ship vs. static + sweeping asteroid
gates, with barrel-vs-target aim gizmos (green on target, yellow lagging) and a throttled
aim-error / bullets-in-flight readout, wired to the autopilot + screenshot harness. The
range immediately surfaced the turret's tracking lag (fixed-rate slew, no lead), captured
as follow-up 20260707-150001; live tuning sliders are follow-up 20260707-150002. No-debug
build (harness cfg's out) and clippy are green; headless smoke run tracks/fires with no
panic.

## Notes

Turret aiming lives in `crates/nova_gameplay/src/sections/turret_section.rs`
(`update_turret_target_yaw_system` / `update_turret_target_pitch_system`, the yaw/pitch
rotators with `speed`/`min`/`max`). Reference the gated examples and `docs/dev-harness.md`
in the bevy-common-systems repo for the harness shape.
