# Live radar lock: lock at threshold, retarget while held, stick on release

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,targeting,input,spike

## Goal

Playtest feedback (2026-07-13): waiting for CTRL release to commit is an
annoyance. Rework the radar gesture to A1 from the spike: press starts the
radar as today (nothing visible or written inside the 0.25 s tap window, so
tap-clear is untouched); at the Hold threshold the latched slot is WRITTEN
with the current candidate; every held frame the slot retargets live
(existing hysteresis); release just ends the radar - the lock sticks. D1
becomes "never saw a candidate while held = slot untouched".

## Scope (adversarial round folded in)

- Slot latch moves from CTRL press to the Hold threshold per Q1 recommended
  (F1: kills the recorded same-frame RMB+CTRL sharp edge - raised derives in
  Update, the Start observer runs PreUpdate; at threshold the raised state is
  settled). If the user picks Q1(b), keep press-latching and carry the edge.
- Empty-sweep behavior per Q2 recommended: keep-last (the lock stays on the
  last valid target; tap remains the only clear).
- A retarget write resets the 30 s CombatDecay (F12: any lock write is
  combat activity, so a long sweep cannot cross the decay boundary
  mid-gesture).
- Boundary re-pin (F9): sub-threshold release (tap) fires clear having never
  written; the threshold-frame release must not both lock and clear; the
  gesture e2e family and the 12_hud_range script re-pin threshold-commit
  instead of release-commit.
- Retarget churn is verified safe (F3): no Changed<lock> reactors exist,
  consumers poll, the focus dwell resets by comparison
  (tick_lock_focus, targeting.rs:925).

## Notes

- Spike: docs/spikes/20260713-110039-show-dont-tell-radar-ux.md (strand A1 +
  adversarial round F1/F3/F9/F12, questionnaire Q1/Q2).
- Blocked on the questionnaire answers (Q1, Q2); recommended defaults above
  make the task plannable on "all recommended".
- Main surface: targeting.rs (update_radar_search absorbs the commit and the
  latch; on_radar_start no longer decides the slot; on_radar_commit shrinks
  to radar teardown).
- Safety goes hot at the threshold in combat mode - no new exposure, RMB is
  already held when the combat slot latches; auto-track-follows-sweep when
  RMB is released mid-hold is the same class as today's post-commit
  behavior, just sooner (playtest note).
- /plan before implementation.
