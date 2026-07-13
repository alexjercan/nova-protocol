# Review - 20260713-110311 show-don't-tell lock HUD

## Round 1 (2026-07-13)

Re-derived B1/C1 + Q3a-Q8a from the spike and read the diff cold; traced
every state surface a player can be in against what now renders.

State-coverage trace (the core of this review - the text block used to
carry ALL of this):

- Combat lock on a zoomable body -> inset viewfinder (presence = guided),
  frame red + ticks while hot. Pinned in tests + live in 12_hud_range
  (viewfinder asserted MID-SWEEP, before the dwell - the fail-first pin
  against the old focus gate: the rig strips the dwell to zero).
- Combat lock on a BEACON -> NO-SIGNAL panel (Q4a), no second render;
  reticle ticks still carry hot. Pinned with a zoomable delivery guard.
- Raised-manual, NO lock (the F4 hole) -> hot lead pips + no other
  surface; color-only exception documented in code.
- Reduced HUD tiers -> Chrome gate intact (existing test still green);
  reticle/pips live in other layers and still carry the state.
- Travel sweep -> box label name+distance (Q6a); combat sweep -> inset
  caption, box label EMPTY (no doubled text) - both pinned.
- Clears -> ghosts (slot-colored, target-anchored; the toast message
  always carries a Some target, verified at the writer) + LockOff.
- Deny -> buzz + centered flash (F7 gap actually closed this time; the
  stale 082337 comment is gone).

Findings:

- R1.1 (REAL, fixed this round): `play_lock_cues` gated on
  `.read().next()` without draining - a second same-frame message would
  have replayed the cue on the NEXT frame, contradicting the collapse
  comment. In practice one tap = one toast, but the code now drains with
  `.count()` so the comment is true.
- R1.2 (DOC, fixed this round): target_inset's module header still
  described the focus-dwell gate; rewritten to inset-on-lock.
- R1.3 (NOTE, no action): `pulse_no_signal` runs while the overlay is
  hidden - one node, set-per-frame on a BorderColor; not worth a run
  condition.
- R1.4 (NOTE, playtest): the ghost anchors a live entity; a target that
  DIES the same frame as the clear despawns the anchor and the widget
  hides the ghost mid-pop. Rare and self-healing (the ghost still expires
  by timer); flagged for the 090653 playtest pass rather than engineered
  around.
- R1.5 (verified): run_system_once/MessageReader trap hit once in the
  deny-flash test and was fixed with the ledger's registered-system
  pattern during work, before review.

Honesty checks: the Outcome records the Q6a drafting-slip correction, the
pip color-only exception, the dropped center-screen ghost fallback (dead
code path - the writer guarantees Some), and the mid-task user fold-in
(panel below the status bar). All verified against the diff.

VERDICT: APPROVE (round 1, with R1.1/R1.2 folded in).
