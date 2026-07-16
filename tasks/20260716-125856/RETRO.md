# Retro: Scenario outcome frame - Victory/Defeat action + overlay

- TASK: 20260716-125856
- BRANCH: feature/scenario-outcome-frame (landed 9a27efac)
- REVIEW ROUNDS: 2 (round 1 REQUEST_CHANGES: 1 MAJOR, 3 MINOR, 4 NIT; round 2 APPROVE)

## What went well

- The out-of-context review earned its cost again: it caught the one real
  MAJOR (pause/unpause over a live Victory overlay re-locking the cursor)
  that the implementer's own eyeball and 7 green tests all missed, and it
  independently verified regeneration consistency and test non-vacuity
  instead of trusting the close record.
- Verify-first planning meant zero mid-implementation surprises: the linger
  mechanism, teardown seam, and pause-overlay patterns were all read before
  design, so the action + overlay dropped onto existing seams (the
  HintEmphasis command pattern, the teardown reset class) without rework.
- The throwaway probe example (a 60-line copy of the example-12 harness)
  bought a real rendered eyeball for ~10 minutes of work; the rig is
  recorded in NOTES.md so review/the slice can recreate it.
- Mid-cycle user feedback ("scuffed to just press enter" -> real buttons)
  arrived BEFORE the overlay was built and was folded in at design time -
  reading feedback against what is not yet built is much cheaper than a
  review round.

## What went wrong

- R1.1's root cause: the eyeball evidence was Defeat-only because Defeat was
  the CONVENIENT variant to stage - and Defeat is exactly the variant that
  masks the cursor bug (dead ship = empty player query = no re-grab). The
  evidence variant was chosen by staging cost, not by what it could hide.
- `cargo test -p nova_scenario` alone fails to compile (pre-existing: its
  serde tests lean on workspace feature unification) - cost one confused
  compile cycle before the sibling-crate workaround; now recorded in
  NOTES.md.
- The scripted REVIEW.md response-filling (python replace) produced doubled
  "Response: Response:" prefixes that needed a sed fix-up - the replace
  "succeeded" while emitting malformed text.

## What to improve next time

- Pick probe/eyeball variants adversarially: enumerate the feature's
  variants (Victory/Defeat, queued/unqueued) and stage the one with the MOST
  live actors/interactions - or both when cheap - not the easiest one.
- In this repo, never run a single crate's tests solo; pair it with a
  feature-unifying sibling or run workspace-wide as CI does.
- After any scripted text edit, read the result (or grep the exact new
  text), not just the exit status.

## Action items

- [x] R1.8 follow-up recorded on the slice task 20260708-203659 (retrofit
      asteroid_field chain outcomes).
- [x] Ledger: bumped `out-of-context-review-pass` and
      `verify-scripted-edits-applied`; added `probe-the-adversarial-variant`
      and domain lesson `crate-solo-tests-miss-unified-features`.
