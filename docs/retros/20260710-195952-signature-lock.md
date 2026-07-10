# Retro: Signature-gated lock (long range only sees large objects)

- TASK: 20260710-195952
- BRANCH: signature-lock (squash-merged to master as 560f90e)
- REVIEW ROUNDS: 1 (APPROVE with 3 MINOR + 1 NIT, all fixed in-round)

## What went well

- **Gating at the collection point was the whole design.** One filter_map
  change and every lock consumer (cone pick, heat fallback, torpedo
  commit, turret feed, GOTO designation, HUD) inherited the model with
  zero changes - the review traced all of them and found nothing to fix
  downstream.
- **The review caught a fiction hole the implementation had rationalized.**
  "Point defense needs torpedoes" had quietly become "torpedoes visible
  at 20km" - the smallest object in the game as the only full-range small
  thing. The reviewer quantified the actual need (AI launch 1000u, heat
  fallback 550u) and the fix was a settings field.
- **Hysteresis is now reflexive** - the reviewer flagged the boundary
  strobing, but the fix pattern (incumbent holds past the gate) was
  already house idiom (5th instance) and cost minutes.

## What went wrong

- **Two conflicting user reports briefly coexisted in one comment.**
  TARGETING_MAX_RANGE had been raised the same day for "designate from
  across the play area", and the new gate half-reversed that; the stale
  comment documented the losing side until review. Same-day requirement
  churn needs the comment updated in the same diff that changes the
  behavior.
- The old intrinsic-classes test survived one edit pass unnoticed
  (formatting mismatch in a scripted replace) and failed loudly - caught
  in-cycle, but scripted multi-part edits keep proving fragile on
  fmt-reflowed code; targeted Edit calls on the exact current text are
  the reliable path.

## What to improve next time

- When a change narrows behavior that a recent change widened, grep for
  the widening commit's rationale (comments, docs) and reconcile both
  texts explicitly - requirements that pass each other in the night
  confuse the next reader.

## Action items

- [ ] Playtest knobs: signature_range_per_unit 30, unsigned 15u,
  torpedo_lock_range 2500u, range_hysteresis 1.15.
- [ ] Next in queue: 20260710-201514 (yellow gravity indicator replacing
  the SOI shell), then 20260710-193500 (gravity-aware arrival, spike
  first), 20260710-195954 (GOTO parks into ORBIT).
