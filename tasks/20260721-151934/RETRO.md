# Retro: fix cargo-run-launches-probe + ambiguous-glob-reexport

- TASK: 20260721-151934
- BRANCH: fix/cargo-run-and-glob (landed 8e80dca1)
- REVIEW ROUNDS: 1 (in-session, mechanical verified regression revert)

## What went well

- Both regressions had a fast, empirical repro/verify: `cargo run -- --help`
  (game vs probe) and `cargo build -p nova_core` (glob warning), so the fix was
  confirmed by running the exact failing commands, not reasoning.
- The `default-members` removal was the escape hatch my OWN meta_gen retro had
  already documented ("drop this step if the footgun is unwanted") - the design
  had recorded its own reversal path.

## What went wrong (the real lesson)

- I INTRODUCED `default-members` in the meta_gen relocation (6f41f47a), my retro
  FLAGGED it as a footgun, the out-of-context review APPROVED it - and it STILL
  shipped a regression that broke the single most-documented user command
  (`cargo run` -> play the game). Root cause: `default-members` on a workspace
  whose ROOT is a package changes what a bare `cargo run`/`build` targets - from
  the root game package to the whole member set, which resolved to the `probe`
  bin. And it bought nothing: meta_gen is not a game dependency, so a bare build
  of the root package never compiled it anyway.
- The REVIEW GAP: the meta_gen review verified the NEW behavior ("bare build
  skips meta_gen") but never ran the DOCUMENTED command (`cargo run` launches
  the game). A build-config change was checked for what it added, not for what
  it must preserve.

## What to improve next time

- After changing workspace/build config (`default-members`, `[[bin]]`,
  `default-run`, features), RE-RUN THE DOCUMENTED USER COMMANDS - the README
  quickstart, `cargo run` - not just verify the intended new behavior. A config
  change is judged by what it preserves as much as what it adds.
- Prefer NOT adding `default-members` to a root-package workspace: a leaf tool
  that is not a game dependency is already skipped by bare builds; the key only
  adds an allowlist-maintenance footgun and re-targets bare `cargo run`.

## Action items

- [x] LESSONS.md: added `default-members-retargets-bare-cargo-run` and the
  general `re-run-documented-commands-after-build-config-change`.
