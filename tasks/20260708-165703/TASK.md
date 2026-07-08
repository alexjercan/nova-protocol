# Lock-on dwell + acquire/lock cue (HUD)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: v0.5.0,hud,torpedo,spike

Spike: docs/spikes/20260708-165647-weapons-hud.md

Phase 2 (needs a small mechanic + audio). Today the torpedo lock is instant
(`update_spaceship_target_input` sets the target the moment a body is in the cone).
Introduce a short dwell: keep a candidate under the crosshair for N ms before it
becomes a hard lock, shown as a lock-on ring that fills, plus an acquire/lock
audio-visual cue when it snaps. Makes locking legible and satisfying.

Depends on the audio system (162011) for the cue, and changes targeting feel - decide
during planning whether the dwell replaces or augments the instant lock.
</content>
