# Review: loop gate readiness + mid-cycle done

- TASK: 20260720-014142
- BRANCH: fix/loop-gate-readiness (+ bcs v0.19.5)
- ROUND: 1

## What I tried to break

- **The exact field crash**: forced playable loops (window >> cycle) now
  run clean twice over (2 loops full capture; and pre-fix the same rig
  reproduced the crash, then the backstop panic, then the deadline race -
  each layer verified fixed by the next run).
- **Guard erosion**: the looped flag relaxes ONLY looped cycles;
  cycle 1 and every clean-pass run keep full completion enforcement, and
  the smoke suite (no capture, no loops) is untouched.
- **Measurement honesty**: the seed wait now counts inside the reload
  interval (excluded + reported); mid-cycle finish cuts dead idle frames
  that would have padded the tail.
- **Sentinel contracts**: the mid-cycle line keeps "cycle complete, no
  panic" phrasing; smoke greps unaffected (that path needs a capture).

## Findings

- R1.1 (NIT, accepted): readiness variables are named per example
  ("beat", "target_down") - a rename would silently turn the gate into
  a deadline wait. The names sit next to the scripts that own them.
- R1.2 (recorded): forced-loop e2e knobs (FRAMES=2000) are documented in
  the close-out, not automated; S-strand follow-ups can pin them.

## Verdict

- VERDICT: APPROVE - land.
