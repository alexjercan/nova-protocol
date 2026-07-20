# Retro: move base-mod content gen to build-time (DECLINED)

- TASK: 20260719-092952
- BRANCH: none (no code; investigated + decided)
- REVIEW ROUNDS: 0 (decision-close, no code to review)

See TASK.md "Decision (2026-07-20)" for the full rationale; process only here.

## What went well

- Investigated the build/asset structure (root `build.rs`, `Trunk.toml` hooks,
  how `nova_meta_gen` is wired) BEFORE writing code, and found the load-bearing
  constraint early: `content_files()` pulls in bevy via the config types'
  `Reflect` derives, so a `build.rs` generator needs `nova_assets` as a
  build-dependency and duplicate-compiles bevy. That surfaced the cost/footgun
  before any throwaway build.rs got written.
- Surfaced the fork to the user with the concrete tradeoffs (build.rs duplicate
  compile + tree mutation vs Trunk hook web-only vs keep-as-is) rather than
  guessing the task's "default recommendation" was still the right call under
  the real costs. The user chose to keep `content gen`.

## What went wrong

- Nothing broke; the task premise (build-time move is worth it) turned out not
  to hold once the bevy-compile cost was weighed. The task's own Notes flagged
  the footgun, so the decline was anticipated by the planner.

## What to improve next time

- A "move X to build-time" task should first check whether the generator drags
  in a heavy dependency (here bevy, via `Reflect`): if it does, a `build.rs`
  duplicate-compiles that dep in the build graph, which usually kills the
  cost/benefit. Check the dep weight before scoping the move, not after.

## Action items

- [x] LESSONS.md: added `build-time-move-weigh-generator-deps` (x1).
- [x] Repointed umbrella 20260718-152304 so the declined `gen -> build-time`
  decision is not re-opened by a future worker.
