# Review - 20260713-110330 live radar lock

## Round 1 (2026-07-13)

Re-derived the model from the spike (A1 + Q1a/Q2a/F9/F12) and read the
diff cold, then traced every consumer of the changed surfaces.

Checked and sound:

- **Write-path unification**: the slot writes exist in exactly ONE place
  (`update_radar_search`); both release observers reduce to the shared
  teardown. Grepped: no other writer of TravelLock/CombatLock in the
  gesture path remains.
- **Tap-window contract**: `engaged: None` until `TriggerState::Fired`;
  the Tap clear and the Hold latch derive from the one shared
  `RADAR_TAP_SECS`, and the boundary e2e pins the exact 5x50 ms frame with
  the lock already live.
- **Keep-last (Q2a)**: a `None` candidate never writes; pinned with a
  delivery guard (the test asserts the picker actually saw empty space).
- **Q1a**: latch reads the CURRENT stance at threshold; the
  raise-inside-the-tap-window e2e documents the retired press-latch edge.
  The A/B is by construction (the old model cannot compile the new field),
  honestly recorded in the Outcome.
- **Pause**: `update_radar_search` sits in the pause-gated
  `SpaceshipInputSystems` (configure_pause_gating wired into the e2e app,
  production wiring); the rephrased pause pins carry delivery guards (the
  same sweep retargets live in the sibling test).
- **F12**: decay held at zero every engaged-combat frame, not just on
  changed writes - stronger than the plan asked.
- **Churn re-verified**: no `Changed<TravelLock>`/`Changed<CombatLock>`
  readers exist (F3 still true post-diff); `tick_lock_focus` resets by
  comparison, so per-frame equality writes are inert.
- **Consumers traced**: player.rs reads `RadarState` presence only
  (HOLD_FIRE_DURING_RADAR); GOTO captures at [G] (pinned elsewhere);
  torpedo commit reads CombatLock (a live mid-sweep commit now targets the
  sweep's current pick - consistent with "the lock is live", noted for the
  110311 playtest); the inset keys off focus and is untouched until 110311.
- **Rig honesty**: the e2e rig no longer stuffs candidates - real bodies,
  real split-camera ray, real picker, decoy turret rig retained; the cue
  counter drains messages across frames (double-buffer safe).
- 468 lib tests, fmt, check, three autopilots exit 0 re-confirmed.

Findings:

- R1.1 (NIT, fixed this round): the `RadarLockAcquired` doc said "first
  written" but the cue also fires on an equality-skip re-acquisition;
  reworded to "first resolves a candidate". Behavior is the desired one
  (a re-acquire gesture still cues) - doc only.
- R1.2 (NOTE, no action): `hold_fired` is a global any() over
  `Action<RadarHoldInput>` states, not scoped to the ship's rig - the same
  single-player assumption as camera_controller's `action_held`; fine
  until multiplayer, not this task's problem.

- VERDICT: APPROVE (round 1, with R1.1 folded in).
