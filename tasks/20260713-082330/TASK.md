# Travel/combat lock slots + deliberate radar: CTRL hold/release/tap, componentized locks, capability flag

- STATUS: OPEN
- PRIORITY: 56
- TAGS: v0.5.0, targeting, input, hud, spike

## Goal

Replace passive lock acquisition with deliberate radar locking, on two
coexisting sticky lock slots (spike 20260713-082207):

- **TravelLock + CombatLock as ship-root COMPONENTS** (not player resources) -
  needed for AI parity and respawn hygiene.
- **Radar gesture**: hold CTRL = radar on, live-retargets to the look ray's
  best candidate; release = the lock COMMITS (travel slot in normal mode,
  combat slot while RMB is held); tap CTRL = clear (normal mode: ALL locks;
  combat mode: combat lock only). Tap threshold is an ordinary tap-vs-hold
  const (~0.25 s) - CTRL stops being a modifier anywhere else, so the
  bare-modifier Chord bug class (20260711-173237) does not apply.
- **Deliberate only**: remove the every-frame cone auto-pick, the close-range
  signature auto-acquire, and the sticky `held` gate (stickiness becomes
  inherent - nothing ever re-picks a committed lock; locks clear on
  death/despawn/out-of-range/tap). The candidate collection + LockSignature
  range gates + angle-from-ray pick survive as the radar's picker, run only
  while CTRL is held, reading the active-look-ray accessor (20260713-082324).
- **Unified pool**: anything lockable serves both slots; retune range knobs
  toward the user's examples (small asteroids ~200 m, debris ~5 m).
- **Lock-capability flag** on the ship computer (the controller-provided verb
  flag pattern, spike 20260712-143551): a computer without it cannot radar-lock
  (deny cue).
- **HUD baseline**: white crosshair on the travel lock, red slightly-smaller
  on the combat lock (overlappable), radar-active provisional cue while CTRL
  is held.
- **Retire the old CTRL roles**: CTRL+scroll target cycling and the
  CTRL free-aim raw read (player.rs:434, :625) - free-aim's job moves to RMB
  manual gunnery in 20260713-082337. Wheel = component fine-lock cycling is
  UNCHANGED. Focus dwell / component lock / target inset re-key onto the
  CombatLock component.

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md.
- Depends on: 20260713-082324 (active look ray).
- The dead family's componentization analysis applies (closed 20260712-223035 /
  -215957: port surface - 12_hud_range compiles against resources, shipless
  verb hints, reticle None-writes, pause-latch); consult at plan time.
- Consumer routing (G/GOTO, guns, torpedoes, safety) lands in
  20260713-082337, not here - this task delivers slots + radar + HUD with the
  existing consumers reading the combat slot compatibly.
