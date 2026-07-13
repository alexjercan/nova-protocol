# Live radar lock: lock at threshold, retarget while held, stick on release

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,targeting,input,spike

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

- [ ] Rework `RadarState` (targeting.rs:110): replace `combat: bool` with
      `engaged: Option<RadarSlot>` (new `enum RadarSlot { Travel, Combat }`;
      `None` = inside the tap window, nothing latched) plus
      `acquired: bool` (first-write-of-gesture bookkeeping for the LockOn
      cue). Update the D2 press-latch doc comments to the threshold model.
- [ ] `on_radar_start` (targeting.rs:725): stop reading `WeaponsRaised`;
      insert `RadarState::default()`. Capability + pause gates unchanged
      (the deny CUE and its stale comment stay with 20260713-110311).
- [ ] `update_radar_search` (targeting.rs:577) absorbs the commit: add
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
- [ ] Verify `update_radar_search` does not run while paused (the
      pause-drop guarantee moves from the commit observer to system gating;
      check the set it runs in), and that release observers still tear down
      during pause.
- [ ] Shrink `on_radar_commit` (targeting.rs:767) to teardown only and
      collapse it with `on_radar_cancel` (targeting.rs:807) into one shared
      teardown helper observed on both `Complete` and `Cancel` - no slot
      writes remain in observers.
- [ ] Minimal HUD adaptation (lock_crosshairs.rs:206
      `drive_radar_candidate` + module doc): the box shows only while
      `engaged` is `Some`, colored by the engaged slot (the full F11
      visual rework - adornment around the solid crosshair - is 110311's).
      Grep for other `RadarState` field readers and adapt.
- [ ] Tests (rewrite the gesture family in targeting.rs, one filter per
      run): threshold writes the slot while still held; retarget follows
      the look mid-hold; release changes nothing and removes `RadarState`;
      empty sweep keeps the last target (Q2a); tap never writes and still
      clears; the exact-boundary frame re-pins threshold-commit (F9);
      raise-after-press inside the tap window latches COMBAT (Q1a,
      fail-first A/B against press-latching); never-saw-a-candidate is a
      no-op (D1); a retarget write resets `CombatDecay` (F12);
      `RadarLockAcquired` fires exactly once per gesture; no writes while
      paused; capability-denied hold stays inert (existing test green).
- [ ] Re-pin the 12_hud_range radar stage: assert the lock exists while
      CTRL is still held (threshold-commit), keep the safety asserts.
- [ ] cargo fmt + cargo check; new/rewritten test filters; the three
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
