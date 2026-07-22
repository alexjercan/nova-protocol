# Retro: Ledger ch3 depth (20260722-214105)

## What went well

- The ch1/ch2 pacing pass (20260722-214058) had already established the
  opening-cascade + breather idiom IN THE MOD, so ch3 was "copy the landed
  pattern" not "invent it". The diagnostic brief named the exact gaps (one
  linear position-gated act, decorative debris), so the work was targeted.
- The debris-pinch was the right call for "a second DISTINCT encounter": it
  adds a PILOTING beat next to the existing NAV-2 COMBAT ambush (two different
  textures), makes the previously-decorative 26-rock field load-bearing, and
  keeps the "fighting is optional" contract cleanly - a purely navigational
  hazard forces nothing.
- Pinning the pinch gap as a COMPUTED geometry assertion (gap width derived from
  spawn positions + the real `ASTEROID_GEOMETRIC_FACTOR_MAX`, not a magic
  literal) is the ch2-rig discipline (`author-against-measured-values`). The
  reviewer independently recomputed it (24.06u clear vs ~11u ship-fit need) and
  agreed - a false-green here would have been a soft-lock, so the derivation
  earned its keep.

## What went wrong / was tricky

- The one non-obvious ordering guard was the `arrive3_said` final-leg breather:
  it needs an `act == 1` guard so it cannot fire after YARD sets `act = 2` in
  the Victory handler. Caught it by tracing the act latch (same discipline as
  ch2's deferred Victory) - a deferred comms one-shot must be disqualified by
  the terminal latch or it can fire post-outcome.
- LOW: the pinch CONFIRM line is missable by a very fast pilot (it drops a comms
  line, no soft-lock). Accepted with the test covering it - a cosmetic tail, not
  worth a gate on the objective.

## Lessons / what to do differently

- For a "make the encounter richer" ask, prefer adding a DIFFERENT texture
  (piloting vs combat) over scaling the existing fight - two distinct beats read
  as depth; a harder version of the same beat reads as a difficulty bump. The
  debris-pinch-vs-staggered-contact fork was decided this way (owner confirms at
  Finish).
- Any deferred/breather one-shot in a chapter with a terminal Outcome needs a
  guard on the terminal latch (`act == running`), same as ch2. This is now a
  three-chapter pattern (ch2, ch3, and ch4's burn overlay) - it is the standing
  rule for the mod.

## Follow-ups

- None blocking. Owner playtest question 2 (debris-pinch chosen vs
  staggered-combat-contact at NAV-3) is batched for Finish; the LOW missable
  confirm line is noted there too.
