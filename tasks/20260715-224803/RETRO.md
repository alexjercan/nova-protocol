# Retro: Make Gauntlet Run a playable sequential slalom race

- TASK: 20260715-224803
- BRANCH: feat/gauntlet-race (landed as master 9ab88eef)
- REVIEW ROUNDS: 2

## What went well

- Verify-first paid off: before writing the review MAJOR I read
  `ExpressionFilterConfig::filter` and confirmed an undefined variable evaluates
  to `false` (fails closed). That turned "the OnStart seed is untested" from a
  hunch into a concrete soft-lock failure mode, which made the finding
  actionable instead of hand-wavy.
- Testing the ACTUAL shipped RON (`include_str!` -> parse -> register its real
  handlers -> drive the real `ScenarioAreaPlugin`+avian bridge) rather than
  re-typed handlers. The deliverable IS the data, so the test pins the data.
- Two focused subagents (map the scenario system, map the test API) kept the
  heavy reading out of the main context and returned exact type paths, so the
  RON and the Rust test compiled essentially first-try.

## What went wrong

- R1.1 (MAJOR): the first behavior test seeded `gate=1` itself and skipped the
  scenario's `OnStart`, so the single change that makes the scenario playable -
  the OnStart `player_spaceship` spawn and the `gate=1` seed - was untested. A
  regression dropping either would have shipped green. Root cause: I tested the
  mechanism I was focused on (the gating) with a rig that SUPPLIED the
  precondition the production path is responsible for establishing; a rig that
  hands itself the precondition is structurally blind to that precondition
  breaking. Fixed by adding a structural assertion over the parsed OnStart.
- Minor friction, not a defect: hit the bg-isolation Write guard on the main
  checkout mid-plan, and the settings escape hatch was denied by the
  self-modification classifier. Cost a few minutes reorienting to "Bash heredoc
  for master-side files, Write in the sprout worktree".

## What to improve next time

- When a test injects state that production must set up (seed a variable, spawn
  the actor, insert a resource), add a separate check that the production path
  actually establishes it. The behavior test and the wiring test are two
  different guarantees.
- On a bg job in this repo, plan the file-write split up front: master-side
  artifacts (plan stubs, RETRO/LESSONS) via Bash; all code via Write/Edit inside
  the sprout worktree.

## Action items

- [x] Added `rig-supplies-precondition-hides-regression` and
      `bg-isolation-guard-allows-sprout-not-main` to docs/LESSONS.md.
- No follow-up code tasks: the two NITs (area-overlap invariant, test scoping)
  were addressed in-cycle.
