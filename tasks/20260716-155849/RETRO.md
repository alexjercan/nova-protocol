# Retro: Data-driven menu scenario roles

- TASK: 20260716-155849
- BRANCH: feature/menu-scenario-roles (landed 4313cf96)
- REVIEW ROUNDS: 1

## What went well

- The gen_content pipeline from task 155823 made the data change
  trivial: flag the builder, run one command, get a one-line RON diff
  with parity green. The tooling investment paid off within hours.
- Applying mid-flow lessons: the plan sweeps were re-run untruncated at
  cycle start (mid-flow-lesson-reaudits-the-queue) and found the plan
  complete this time; verify-bevy-api-at-callsite (Single<&mut WyRand> +
  rand::Rng import copied from asteroid.rs, with_seed checked in the
  registry source) avoided API guesswork.
- The review sabotaged the central trust rule (base-only new_game) and
  watched the pin go red - the user's "mods must not set New Game"
  requirement is enforced AND proven.

## What went wrong

- The plan missed all three failure paths that turned out to be the real
  design work: a well-less mod backdrop would deactivate the camera
  forever (menu bricks - the UI renders through it), zero flagged
  backdrops would load no camera at all, and the fallback chain for
  unregistered declarations was unspecified. All were designed and fixed
  in-cycle, but they were discovered by reading stage_menu_camera during
  work, not by the plan.
- One compile round lost to the new struct field breaking exhaustive
  literals - predicted verbatim by the check-all-targets-for-struct-field
  ledger entry, and caught by exactly that command.

## What to improve next time

- A task that exposes a surface to MOD DATA plans its failure paths
  first: enumerate "what breaks when a mod does this badly" (missing
  entity contracts, empty sets, unregistered ids, hostile values) as
  plan steps, not as work-phase discoveries.

## Action items

- [x] Ledger: new mod-facing-surface-plans-failure-paths (x1); bumped
      check-all-targets-for-struct-field (x2).
- [x] Follow-up task filed per user request: more menu backdrops (the
      rotation currently has one member).
