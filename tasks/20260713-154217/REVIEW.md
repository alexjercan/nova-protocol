# Review - 20260713-154217 inset kill cam

## Round 1 (2026-07-13)

Re-derived option B from the spike, then attacked the state machine and
its interactions cold.

- **State machine audit**: the explicit InsetPanelState enum resolves
  before side effects; every transition audited: Live->death->KillCam
  (frozen pose, pinned by equality), Live->clear-alive->Hidden (the
  discriminator, delivery-guarded), KillCam->fresh-lock->Live
  (preemption), KillCam->expiry->Hidden, any->chrome-hidden->Hidden
  (immediate), NoSignal entered mid-linger kills the cam (a lock exists;
  the live viewfinder in all its forms wins).
- **Gameplay isolation verified**: the diff touches only
  hud/target_inset.rs + the example - no targeting, safety or AI
  surface; the spike's option-D rejection stands structurally.
- **Frame-memory hygiene**: LastFramed inserted only in the Live arm and
  removed in NoSignal/Hidden arms - a HUD respawn, a beacon interlude or
  a manual clear cannot leave a stale pose behind for a later
  false-linger. Both state components live on the panel entity, so
  player-death teardown takes them along by construction.
- **Ordering re-verified**: targeting's validity clear
  (SpaceshipInputSystems) precedes the HUD set; the asteroid husk keeps
  the root lockable until its actual despawn - clear and death land
  together, and the ship path (12_hud_range) despawns the root directly.
- R1.1 (NOTE, accepted): the RTT keeps rendering for KILL_CAM_SECS after
  the kill (~2 s of extra second-scene render) - trivial against the
  panel's steady-state cost, and the point of the feature.
- R1.2 (NOTE, playtest): fast target swaps (kill -> immediate re-lock)
  never show the linger - correct by design (preemption), noted so the
  behavior is not mistaken for a bug.
- HONESTY: the inverted example pin is validated by mechanism (direct
  despawn = the detected signature) but not yet by a live run (user's
  game instance up); recorded in the Outcome with the re-run pointer.

- VERDICT: APPROVE (round 1).
