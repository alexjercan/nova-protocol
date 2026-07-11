# Turret crosshair (orange square) twitches while tracking

- STATUS: OPEN
- PRIORITY: 80
- TAGS: v0.5.0, turret, bug

## Goal

Playtest bug (user, 2026-07-10): the turret crosshair (orange square pip)
jitters while tracking. Root cause
(docs/spikes/20260711-103527-twitching-family-two-clocks.md): the pip
anchors are driven in Update (turret_lead.rs:78-84) from
`TurretSectionAimPoint`, which is computed in PostUpdate
(turret_section.rs:245-255) - the pip always shows the PREVIOUS frame's
intercept, then gets projected with the stale camera pose fixed in
20260710-231928. The user's "target calculation takes time" hypothesis was
close: the solve is fresh every frame, but its consumption is a frame late.

## Steps

- [ ] Reorder the crosshair chain so the pip consumes THIS frame's
      intercept: with projection now in PostUpdate (after 20260710-231928),
      chain `drive_pip_anchors` after `update_turret_aim_point`
      (turret_section.rs:245-255) and before `ScreenIndicatorSystems`
      within PostUpdate; keep `sync_turret_pips` reconciliation wherever
      it fits best.
- [ ] Audit the aim-point inputs for one-clock consistency
      (turret_section.rs:459-583): the lead solve should read the eased
      target pose (what the player sees) with raw velocities only where
      physics-correct lead demands it; document the chosen convention in
      the code.
- [ ] Regression test: schedule-order assertion or headless frame-step test
      that the pip's consumed aim point equals the aim point computed the
      same frame (fails on the one-frame-stale read).
- [ ] Re-check the solver-oscillation hypothesis after the reorder: track a
      constant-relative-velocity target and assert the aim point moves
      monotonically/smoothly frame to frame (no flip-flop); record verdict.
- [ ] cargo check + fmt + new tests; extend the spike doc fix record.

## Notes

- Evidence: turret_lead.rs:78-84 (Update, before ScreenIndicatorSystems),
  turret_section.rs:245-255 (aim point in PostUpdate after Propagate).
- Depends on: 20260710-231928 (projection moves to PostUpdate first).
