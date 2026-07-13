# Retro: Committed torpedoes join the combat target set

- TASK: 20260712-212742
- BRANCH: feature/torpedo-combat-target (landed 5123576)
- REVIEW ROUNDS: 1 (APPROVE + one NIT)

Process notes only; behaviour in TASK.md, spike 20260712-203235.

## What went well

- The fix flowed from a clear user model (available combat targets + current +
  cycle) into one concept - `is_combat_target = ship || committed torpedo` -
  reused in three places (the ranked cycle set, the sticky `held` gate, the
  tuple). Small, coherent diff.
- Review's independent check paid off again: verified enemy torpedoes actually
  carry `Allegiance::Enemy` in production (they COPY the shooter's allegiance,
  torpedo_section/mod.rs:686) before trusting that the hostile-filtered combat
  set would include them - otherwise the whole feature would have been a no-op
  in-game while passing every test (tests set allegiance explicitly). Same
  "confirm the production data, not just the test rig" muscle as the sticky
  retro.
- Rewrote (not deleted) the test that pinned the OLD "torpedoes stay out"
  contract - it was correctly failing.

## What went wrong

- This whole task existed only because the previous cycle (sticky ship locks)
  shipped with a wrong claim - "torpedoes remain lockable (CTRL+scroll or aim)"
  - that neither the tests nor I checked against the CTRL+scroll cycle's actual
  contents (ships-only). The user caught it in seconds by trying it. Root cause:
  asserted a capability in the CHANGELOG/notes without exercising it. A claim
  that "X is still possible via Y" needs a test or a manual check that Y
  actually does X.

## What to improve next time

- When a change REMOVES a path (here: aim-steal) and the notes claim an
  alternative path still covers the use case, TEST the alternative path
  end-to-end (CTRL+scroll actually reaching a torpedo), don't just assert it.
  This is the producer/consumer lesson again, pointed at the workaround you
  claim exists.

## Action items

- [x] Lessons ledger: covered by the existing `verify-first-plan-steps`
  shared-state-consumer variant (x7) - this is the same root (a claimed
  capability not traced to its mechanism). No new slug.
- Playtest NIT recorded: the 5-slot candidate cycle is now shared by ships and
  torpedoes; a swarm could crowd ships out. Tune if it bites.
