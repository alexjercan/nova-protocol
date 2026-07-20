# Retro: HUD health percent rounds a living sliver to 0%

- TASK: 20260716-165617
- BRANCH: fix/hud-health-percent-ceil (bcs: master, tag v0.19.1)
- REVIEW ROUNDS: 1 (APPROVE)

## What went well

- Extracting the fix into a pure `display_percent(current, max) -> i32`
  helper made both guards (max<=0 -> 0%, 0<percent<1 -> 1%) unit-testable
  without spinning up a Bevy app. Four boundary tests, each of which fails
  with the fix deleted.
- The task carried its own scope: both failure modes (sliver rounding AND the
  NaN%-on-Health{0,0} half added by review R1.3 of 20260716-162701) were
  fixed in one bcs pass, so no sibling backfill was needed.
- Independent re-derivation of all seven boundaries in the review (Python,
  not re-reading the Rust) confirmed the same-session implement+review blind
  spot was covered.

## What went wrong

- Nothing material. One premise-vs-reality gap: TASK.md said "bump the pinned
  rev (e.g. rev 4c81117)", but the actual current pin was a release tag
  (`tag = "v0.19.0"`) - the convention had moved from rev to tag in the most
  recent bump (nova `bb50db75`). Checking the live Cargo.toml before acting
  caught it, so I cut a patch release (v0.19.1, per the Bevy-minor-tracks-
  crate-minor scheme) instead of a bare rev pin. Cost: nothing, because it
  was verified; had I followed the stale premise blindly it would have left
  an inconsistent mix of `tag` and `rev` pins across the five crates.

## What to improve next time

- When a task names a mechanism ("bump the rev", "edit file X"), verify the
  current state of that mechanism in the repo before following it - task text
  is a snapshot from when it was written and conventions drift.

## Action items

- [x] Added `verify-current-convention-not-task-premise` to LESSONS.md.
- No follow-up code work; cross-repo change is complete and landed
  (master 540c9373; bcs v0.19.1 pushed).
