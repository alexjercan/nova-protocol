# Live radar lock: lock at threshold, retarget while held, stick on release

- STATUS: CLOSED
- PRIORITY: 50
- TAGS: v0.5.0,targeting,input,spike

## Outcome (CLOSED 2026-07-13)

Shipped per plan. The latch and the writes live in `update_radar_search`
(TriggerState::Fired read via the action_held pattern); `RadarState` is now
`{ engaged: Option<RadarSlot>, candidate, acquired }`; the release
observers collapsed into one shared teardown (`close_radar_search`).
Notes against the plan:

- Pause semantics REPHRASED for the live model: the old pin "a release
  during pause drops the commit" has no commit to drop any more - the new
  pin is "paused radar neither latches nor retargets (set gating, verified
  run_if(Unpaused) via configure_pause_gating); a pre-pause acquisition was
  a completed acquisition and sticks; release during pause still tears
  down".
- The Q1a A/B (raise-inside-the-tap-window latches COMBAT) is fail-first BY
  CONSTRUCTION, not executed against the old code: press-latching had no
  `engaged` field to compile against; the test documents that the old model
  routed this gesture to TRAVEL (the retired same-frame edge).
- The gesture e2e rig got MORE production-faithful as a side effect: the
  old rig stuffed `RadarState.candidate` by hand; the writes moving into
  the search forced real bodies + the real split-camera look ray through
  the REAL picker in every gesture test.
- An engaged combat sweep holds CombatDecay at zero every frame (not just
  on changed writes) - the strongest reading of F12.
- 12_hud_range now pins the lock LIVE under the sweep (before release).

Verified: 468 nova_gameplay lib tests (41 targeting, incl. 10 gesture e2e);
fmt + check clean; 12_hud_range / 10_gameplay / 03_scenario autopilots
exit 0.

## Goal

Playtest feedback (2026-07-13): waiting for CTRL release to commit is an
annoyance. Rework the radar gesture to A1: press starts the radar (nothing
written inside the 0.25 s tap window, so tap-clear is untouched); at the
Hold threshold the slot is latched from the CURRENT raised stance (Q1a,
kills the same-frame RMB+CTRL edge) and written with the current candidate;
every held frame the slot retargets live (existing hysteresis; keep-last on
empty sweep, Q2a); release just ends the radar - the lock sticks.

Questionnaire ANSWERED 2026-07-13: all recommended (Q1a threshold latch,
Q2a keep-last).

## Steps

- [x] Rework `RadarState` (targeting.rs:110): replace `combat: bool` with
      `engaged: Option<RadarSlot>` (new `enum RadarSlot { Travel, Combat }`;
      `None` = inside the tap window, nothing latched) plus
      `acquired: bool` (first-write-of-gesture bookkeeping for the LockOn
      cue). Update the D2 press-latch doc comments to the threshold model.
- [x] `on_radar_start` (targeting.rs:725): stop reading `WeaponsRaised`;
      insert `RadarState::default()`. Capability + pause gates unchanged
      (the deny CUE and its stale comment stay with 20260713-110311).
- [x] `update_radar_search` (targeting.rs:577) absorbs the commit: add
      `Query<&TriggerState, With<Action<RadarHoldInput>>>` (the
      `action_held` pattern, camera_controller.rs:762) plus
      `Option<&WeaponsRaised>` and `&mut TravelLock`/`&mut CombatLock`/
      `&mut CombatDecay` on the player. Per frame with `RadarState`
      present: update the candidate (existing pick + hysteresis); when the
      hold reports `TriggerState::Fired`: latch `engaged` from the CURRENT
      raised stance if still `None` (Q1a), then with a `Some` candidate
      write the engaged slot (set-if-neq; NEVER write `None` - keep-last,
      Q2a), reset `CombatDecay` to 0 on every combat-slot write (F12), and
      on the FIRST write of the gesture emit a new
      `RadarLockAcquired { combat: bool }` message (consumer lands in
      110311) and set `acquired`.
- [x] Verify `update_radar_search` does not run while paused (the
      pause-drop guarantee moves from the commit observer to system gating;
      check the set it runs in), and that release observers still tear down
      during pause.
- [x] Shrink `on_radar_commit` (targeting.rs:767) to teardown only and
      collapse it with `on_radar_cancel` (targeting.rs:807) into one shared
      teardown helper observed on both `Complete` and `Cancel` - no slot
      writes remain in observers.
- [x] Minimal HUD adaptation (lock_crosshairs.rs:206
      `drive_radar_candidate` + module doc): the box shows only while
      `engaged` is `Some`, colored by the engaged slot (the full F11
      visual rework - adornment around the solid crosshair - is 110311's).
      Grep for other `RadarState` field readers and adapt.
- [x] Tests (rewrite the gesture family in targeting.rs, one filter per
      run): threshold writes the slot while still held; retarget follows
      the look mid-hold; release changes nothing and removes `RadarState`;
      empty sweep keeps the last target (Q2a); tap never writes and still
      clears; the exact-boundary frame re-pins threshold-commit (F9);
      raise-after-press inside the tap window latches COMBAT (Q1a,
      fail-first A/B against press-latching); never-saw-a-candidate is a
      no-op (D1); a retarget write resets `CombatDecay` (F12);
      `RadarLockAcquired` fires exactly once per gesture; no writes while
      paused; capability-denied hold stays inert (existing test green).
- [x] Re-pin the 12_hud_range radar stage: assert the lock exists while
      CTRL is still held (threshold-commit), keep the safety asserts.
- [x] cargo fmt + cargo check; new/rewritten test filters; the three
      autopilots (12_hud_range, 10_gameplay, 03_scenario).

## Notes

- Spike: docs/spikes/20260713-110039-show-dont-tell-radar-ux.md (strand A1,
  adversarial F1/F3/F9/F12, questionnaire Q1/Q2 - answered).
- Retarget churn verified safe (F3): no Changed<lock> reactors; consumers
  poll; the focus dwell resets by comparison (tick_lock_focus,
  targeting.rs:925).
- Safety goes hot at the threshold in combat mode - no new exposure (RMB
  already held when the combat slot latches). Auto-track-follows-sweep when
  RMB is released mid-hold is the same class as today's post-commit
  behavior, just sooner (playtest note).
- 20260713-110311 depends on this task.
