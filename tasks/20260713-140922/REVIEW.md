# Review - 20260713-140922 OnLock scenario events

## Round 1 (2026-07-13)

Re-derived the contract from the spike + the orbit tracker's review
history, then read the diff cold.

- The plan's "once per acquisition" was WRONG against the codebase's own
  precedent: the orbit tracker documents (R1.1, 20260712-110730) why
  one-shot bridge events soft-lock beat-gated scenarios. The
  implementation deviates correctly (acquire + 5 s echo) and the Outcome
  records it; the beat-sheet task's stale-lock pin must be written
  against THIS semantics (a stale held lock eventually re-fires - beat
  guards, not event dryness, own ordering).
- Player scoping verified against the real hazard (the AI combat mirror)
  and pinned e2e with the AI ship locked on the SAME filtered id.
- Id-less target: quiet, echo retries (orbit R1.2 parity) - pinned with
  the prior fires as delivery guards.
- `tick_lock_slot` is pure and pins the full state machine including
  clear-re-arm; the e2e drives the REAL pipeline (handlers, filters,
  variables) exactly like the orbit test.
- The events mirror OnEnter's info shape, so EntityFilterConfig works
  unchanged - no filter extension needed (the two-variant decision).
- NIT (accepted): the echo period constant is shared by both slots; a
  per-slot knob can come with a real need.

VERDICT: APPROVE (round 1).
