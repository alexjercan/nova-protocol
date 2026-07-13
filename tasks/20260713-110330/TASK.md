# Live radar lock: lock at threshold, retarget while held, stick on release

- STATUS: OPEN
- PRIORITY: 50
- TAGS: v0.5.0,targeting,input,spike

## Goal

Playtest feedback (2026-07-13): waiting for CTRL release to commit is an
annoyance. Rework the radar gesture to A1 from the spike: press starts the
radar as today (provisional only inside the 0.25 s tap window, so tap-clear
is untouched); at the Hold threshold the latched slot is WRITTEN with the
current candidate; every held frame the slot retargets live (existing
hysteresis; keep-last on empty sweep); release just ends the radar - the
lock sticks. D1 becomes "never saw a candidate while held = slot untouched".

## Notes

- Spike: docs/spikes/20260713-110039-show-dont-tell-radar-ux.md (strand A1,
  incl. the adversarial quirks: safety hot at threshold in combat mode,
  auto-track-follows-sweep when RMB released mid-hold, dwell resets).
- Keep-last vs follow-to-none is a playtest knob; default keep-last.
- Main surface: targeting.rs (update_radar_search absorbs the commit;
  on_radar_commit shrinks to radar teardown), gesture e2e + 12_hud_range
  script re-pin threshold-commit instead of release-commit.
- /plan before implementation.
