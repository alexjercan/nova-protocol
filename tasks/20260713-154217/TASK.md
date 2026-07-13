# Inset kill cam: freeze-frame linger on target death

- STATUS: OPEN
- PRIORITY: 46
- TAGS: v0.5.0,hud,ux,spike

## Goal

Playtest (2026-07-13): the viewfinder closes on the exact frame the kill
lands. Option B from the spike: when the framed target becomes
UNRESOLVABLE (died - the discriminator vs tap-clear/decay/retarget, whose
targets remain alive), the panel and RTT camera stay up with the camera
FROZEN at its last pose for KILL_CAM_SECS (~2 s), filming the slicer
fragments, then close. A new combat lock preempts the linger instantly;
Chrome-tier hide stays immediate. Presentation-only: no targeting-layer
changes - locks, safety and turrets behave exactly as today.

## Notes

- Spike: docs/spikes/20260713-154023-inset-kill-cam.md (options B vs C/D,
  the free safety-click composition, the open questions: duration feel,
  early hand-off to a live radar search, player-death teardown).
- 12_hud_range kills the target ship at script end - verify its final
  asserts tolerate a lingering panel (plan-time check).
- /plan before implementation.
