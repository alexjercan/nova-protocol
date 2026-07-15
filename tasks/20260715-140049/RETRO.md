# Retro: e2e proof for SetSkybox: load a real cubemap and assert the swap

- TASK: 20260715-140049
- BRANCH: test/skybox-swap-e2e
- REVIEW ROUNDS: 1 (APPROVE, no blockers)

## What went well

- **Front-loaded the whole chain before writing a line.** Read the action, the
  applier, the locked bcs rev's `SkyboxPlugin` observer, the two model tests
  (demo_scenario / cubemap_meta), and verified every export/prelude membership
  the test would import. Payoff: the test compiled and passed first try and the
  review found zero blockers - the ~5m cold build was spent once, not iterated.
- **Checked the isolation base before sprouting.** The swap target
  `cubemap_alt2.png` was an unpushed local-only commit; confirmed `git remote`
  + the asset's commit and used `sprout new` (off local HEAD) rather than a
  fresh/origin worktree, so the asset was present. A fresh-from-origin worktree
  would have failed the test on a missing file.
- **Falsifiability verified independently**, not assumed: traced that the final
  assertion needs BOTH the applier (only `SkyboxConfig` inserter) and the bcs
  observer (only `Skybox` inserter), so a broken bridge times out - a real
  regression pin, with an explicit `swapped != initial` guard against a vacuous
  pass.

## What went wrong

- **First background test run died instantly: `cargo: command not found`.** Root
  cause: launched the build assuming cargo was on PATH, but this repo's toolchain
  is provided only by the nix flake dev shell (`nix develop`), which also sets
  `LD_LIBRARY_PATH` for the bevy link. Cost was trivial (a seconds-long failed
  launch, no wasted rebuild), but it's a repeatable trap for any cargo invocation
  in this repo from a non-interactive shell.

## What to improve next time

- In this repo, always invoke the toolchain as `nix develop --command cargo ...`
  (and same for clippy/fmt) - never bare `cargo`. Recorded as a lesson.

## Action items

- [x] LESSONS.md: `nix-flake-toolchain-not-on-path`.
- [x] LESSONS.md: `isolate-off-head-for-unpushed-deps`.
- No follow-up code tasks; the two review NITs (event-dispatch fidelity, the
  load-failure branch) are noted in REVIEW.md as optional future tests, not filed.
