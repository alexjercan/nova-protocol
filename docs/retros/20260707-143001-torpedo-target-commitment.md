# Retro: Torpedo target commitment (no re-targeting)

- TASK: 20260707-143001
- BRANCH: feature/torpedo-pn-guidance (PR #31, same PR as the PN work)
- REVIEW ROUNDS: 1 (APPROVE, one NIT documented)

Short, clean cycle. See `tasks/20260707-143001/TASK.md` for the change.

## What went well

- The user's design call resolved the problem at the right level. My first
  instinct (filter bullets out of the aim cast) treated the symptom; the user's
  rule - "a torpedo keeps the first target chosen by the input system, full stop" -
  is simpler, matches the fiction, and automatically covers every variant
  (bullets, debris, re-locks after target death) with one marker component and
  three query filters.
- The regression test encodes the report verbatim: fire with no lock, lock a
  bullet afterwards, assert nothing is assigned. Plus the symmetric
  no-retarget-after-loss case.
- Keeping the example autotargets on the same contract means the harnesses stay
  faithful to the game rule instead of quietly diverging (a torpedo whose gate
  dies now freezes rather than re-homing on the next gate - same as the game).

## What went wrong

- Nothing notable; single review round, no rework. The one design wrinkle (same-
  frame double assignment between an example autotarget and the player system) was
  caught in review and consciously accepted for dev examples.

## What to improve next time

- When a behavior complaint arrives ("it picks bullets up as targets"), ask for
  the intended rule before designing the fix - the user's one-sentence policy
  ("commit at launch") was cheaper and better than the technical filtering I was
  about to propose.

## Action items

- [ ] Future counterplay (flares/decoys) will need a deliberate exception to the
      commitment rule; noted in TASK.md, no task filed (explicitly out of scope).
