# Retro: remove the scatter_density graphics-preset lever

- TASK: 20260718-004834
- BRANCH: remove-scatter-density (squash-landed as 48db8c8c)
- REVIEW ROUNDS: 1 (APPROVE, two NITs addressed same round)

See TASK.md + NOTES.md for what/why and the verification log; this is process
only.

## What went well

- Checked the primary data before trusting the brief. The task (and the
  baseline report it came from) asserted the lever "never fires for the shipped
  scenes". Reading the actual scene `.ron` files showed that is only true for
  shakedown_run; asteroid_field and broadside carry real `ScatterObjects((count:
  20/24))` blocks the lever DID thin. That correction reshaped the rationale
  (removal restores counts, it is not a no-op) and went into the commit + NOTES
  instead of shipping a misleading "dead lever" story.
- Swept for callers and checked blast radius before deleting: confirmed no serde
  persistence of GraphicsBudget, that particle consumers only read `.particles`,
  and read the upstream `EventWorld` trait source to confirm `world_to_state_system`
  has no default impl (so the empty override is required, not dead code).

## What went wrong

- `cargo test -p nova_scenario --lib` failed to COMPILE (`ScenarioConfig:
  Serialize` unsatisfied in loader.rs), which briefly read like my change broke
  something. Root cause: I ran the targeted crate test without the `serde`
  feature, which the crate's round-trip tests need; it is only enabled when the
  crate is built through the workspace. CI runs `cargo test --workspace --features
  debug`, so it never hits this. Cost one diagnostic round to rule out my diff.

## What to improve next time

- When running a targeted `-p <crate> --lib` test on a crate with an optional
  `serde` (or other) feature, match CI's feature set: `--features serde` (or test
  via the workspace). A bare targeted run can fail to compile on feature-gated
  test code and masquerade as a regression.

## Action items

- [x] Bumped `verify-stale-brief-against-tree` in docs/LESSONS.md (this is a second
  occurrence: a baseline/brief claim was partly stale vs the live scene data).
- [x] Added `match-ci-feature-set-in-targeted-tests` to docs/LESSONS.md.
- No follow-up code task: 20260718-004809 was closed WON'T DO; render-scale lever
  tuning already has its own task/worktree.
