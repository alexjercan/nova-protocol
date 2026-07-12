# Retro: Shakedown Run playtest round 1 fixes

- TASK: 20260712-110730
- BRANCH: fix/shakedown-playtest-1 (landed as d6f4a8c)
- REVIEW ROUNDS: 3 (agent APPROVE overturned by live playtest -> REQUEST_CHANGES x2 -> APPROVE)

## What went well

- The user's playtest findings were filed verbatim with root causes
  verified in code BEFORE planning (standoff 50u vs trigger 40u; ring =
  max(band, engage radius)), so the two blockers had one-line-explainable
  fixes and the OnOrbit design fell out of the real mechanism instead of
  gate-size tweaking.
- The R2.3 crate-overlap invariant test from the previous cycle
  immediately caught the regression my own 70u-trigger fix introduced.
  Pinned "unreachable" invariants pay for themselves within days.
- The reviewer folded the recurring-OnOrbit MINOR into a semantics
  improvement that also defused the early-orbit-into-beat-4 landmine.

## What went wrong

- A runtime BLOCKER shipped past both my tests and an agent APPROVE: the
  objectives panel spawn tuple carried two Node components (bevy panics
  on duplicate components in a bundle - NOT last-wins, which reviewer
  and implementer both assumed). The styling test spawned the BARE bcs
  panel, so the production tuple was never executed by any test. The
  user hit the panic on New Game within the hour.
- Worse: my round-1 "fixes" to both tests were silent no-ops - python
  str.replace against formatter-reflowed bodies matched nothing, and I
  reported them as done in the commit message, REVIEW.md and TASK.md
  without reading the files back. The reviewer caught both as MAJORs in
  round 2. Root cause: writing responses from the intended edit, not
  the verified file.

## What to improve next time

- After ANY scripted edit (python/sed), verify it applied: assert the
  replace count in the script, or grep the expected new text before
  claiming it. A str.replace that matches nothing is indistinguishable
  from success.
- UI spawn paths belong in tests exactly as production composes them
  (helper functions both sides call) - a test that hand-assembles a
  simpler bundle can stay green while the real one panics.
- Do not assume bundle semantics: duplicate components in one bundle
  panic; insert-on-existing replaces. Check which one a "override"
  actually is.

## Action items

- [x] LESSONS.md: new `verify-scripted-edits-applied`; bumped
      `would-it-fail-without-it` (bare-panel test) and extended
      `reuse-production-helpers-in-tests` with the spawn-path variant.
- [x] Round-2 feedback (planetoid gravity reach, bullet impact
      knockback, beat-scoped speed cap) filed as the next task.
