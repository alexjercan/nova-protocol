# Retro: Arrival grace

- TASK: 20260717-163042
- BRANCH: feature/arrival-telegraphs (landed on master)
- REVIEW ROUNDS: 2 (REQUEST_CHANGES -> APPROVE; 1 MAJOR, 1 MINOR, 1 NIT)

## What went well

- The leash was the structural template (component-from-config, damage
  override, pure-function early return) and it fit without force: the
  design cost was near zero because the precedent was well built.
- The reviewer's MUTATION testing found the one unpinned load-bearing
  path (Engage demotion - the path EVERY production spawn takes, since
  AIBehaviorState defaults to Engage). The rig had seeded Patrol "to be
  explicit" and thereby dodged the production shape.
- The Bevy Timer finished-flag trap (set_elapsed does not set it; only
  tick does) was caught by the first test run, not by a player.

## What went wrong

- R1.1's root cause: the system rig seeded a HAND-PICKED state instead of
  the production default, so the table and rig both exercised only the
  passive->hold path. production-faithful-rigs (x7 in the ledger) says
  mirror the shipped CONFIGURATION - the shipped configuration here
  included the required-component DEFAULT, which I overrode for tidiness.
- R1.2: a docs step ticked against the wrong file (scenario-system vs the
  guide) - the ticked-step-is-not-proof class again; the reviewer had to
  diff the claim against the branch.

## What to improve next time

- When a rig spawns an entity kind that has required-component defaults,
  seed the DEFAULTS unless the test is specifically about an override -
  the default IS the production shape.

## Action items

- [x] LESSONS.md: production-faithful-rigs bumped (x8, the
  hand-picked-state-vs-required-default variant).
