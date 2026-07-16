# Retro: Gate the OnUpdate scenario pulse (fire_on_update) on Unpaused

- TASK: 20260716-231855
- BRANCH: gate-fire-on-update-pause (landed on master as 3990fa43)
- REVIEW ROUNDS: 1 (APPROVE)

See TASK.md for what/why and REVIEW.md for the correctness argument; this is
process only.

## What went well

- Verified the load-bearing assumption in source before deciding per-system.
  The task claimed the sibling trackers are "frozen anyway"; rather than trust
  it, I read `pause_clocks` (nova_menu/lib.rs:321) and confirmed both pause
  paths freeze `Time<Virtual>` (delta 0), which is exactly why
  `track_orbit_holds` / `track_player_locks` are safe left ungated. This is
  `verify-engine-guarantees-in-source` paying off - the "walk each system"
  direction became a decision backed by the pause mechanism, not a guess.
- Designed the test as a real delivery guard, not a mirror of the neighbor.
  It deliberately does NOT pause the clock - only flips `PauseStates` - so it
  isolates the state gate: delete the fix and the Paused phase keeps
  incrementing, failing the assert. The Unpaused + resume phases prove the
  pulse still fires otherwise (no "nothing happens" ambiguity).
- Kept docs in the same task: the two dev-wiki `OnUpdate` "every frame" rows
  and a CHANGELOG Fixes line, caught proactively rather than in review
  (`keep-docs-in-sync-with-code`).
- A warnings-surfaced build caught the `SystemCondition::and` deprecation
  (the task literally suggested `.and(...)`); switched to `.and_then(...)`
  before landing (`warnings-clean-before-land`).

## What went wrong

- First test run used `cargo test -p nova_scenario --lib` in isolation and
  failed to compile - the serde round-trip tests need `ScenarioConfig:
  Serialize`, which only exists under the `serde` feature that workspace
  unification normally provides. Root cause: ran the crate solo without
  grepping the ledger, where `crate-solo-tests-miss-unified-features` already
  documented this exact trap (twice). Cost: one ~8min cold compile before I
  added `--features serde`. This is now the third occurrence.

## What to improve next time

- Before any `-p <crate>` test run, grep docs/LESSONS.md for the crate name -
  this specific gotcha was already written down and would have saved the cold
  compile. For nova_scenario specifically: `--features serde` (or a unifying
  sibling / workspace-wide).

## Action items

- [x] Bumped `crate-solo-tests-miss-unified-features` to x3 and moved it to
  Pending promotions (proposed target: work skill / docs/development.md) so
  the crate-solo trap stops recurring.
- No follow-up code work: the sibling systems were walked and correctly left
  ungated; nothing deferred.
