# Retro: Robust SFX/juice listener: dedicated camera marker

- TASK: 20260708-224254
- BRANCH: feature/sfx-listener-marker (local branch by user request, merged)
- REVIEW ROUNDS: 1 (APPROVE; two NITs, both addressed in-cycle)

A small, well-specified refactor task (review findings F1/F2 turned into a
marker component), one clean round.

## What went well

- **The "confirm the concrete risk" step paid for itself.** Before writing
  the marker, the editor -> scenario transition was traced through
  `DespawnOnExit` / `OnEnter` ordering, which settled that the old "first
  Camera3d" assumption was latent, not live - recorded in TASK.md so the
  commit message could claim it honestly instead of hedging.
- **Dependency direction decided the marker's home in one check.**
  `nova_scenario` depends on `nova_gameplay`, so the existing (private)
  `ScenarioCameraMarker` could not be the listener signal; a new public
  marker in `nova_gameplay::audio` was the only shape that let the loader
  tag the camera. Checking Cargo.toml first avoided a false start.
- **Tests pin the exact failure mode from the finding.** The new
  `attenuation_listens_from_the_marked_camera_not_any_camera3d` test spawns
  a nearer unmarked `Camera3d` and asserts it loses - the literal scenario
  F1 warned about - rather than only testing the happy path.
- **Honest review order held** (lesson from 225731): findings were written
  to REVIEW.md first, the two NIT fixes applied after, responses and ticks
  last.

## What went wrong

- **A user commit rode the feature branch unnoticed until review.** Working
  on a local branch (no sprout worktree) means sharing the checkout with the
  user; mid-task the user committed an unrelated task re-prioritization
  (dfd4b38) on the branch. It was caught in the review diff and cherry-picked
  onto master before the squash, but only because the diff --stat was read
  file by file. Root cause: local-branch mode removes the isolation the flow
  normally gets from worktrees, and nothing in the merge step guards against
  foreign commits.
- **Local `cargo check` freshness was misleading.** The shared
  CARGO_TARGET_DIR (plus rust-analyzer checking in the background) made
  `cargo check --workspace --all-targets` finish in 0.2s right after edits,
  which proves nothing by itself. The review round re-verified by touching
  the edited files and checking the two examples explicitly.

## What to improve next time

- When the user asks for a local branch instead of a worktree, always
  inspect `git log <default>..<branch>` before squash-merging and land any
  user-authored commits on the default branch separately (cherry-pick), so
  the task's squash commit contains only the task.
- Treat a sub-second workspace check after edits as unverified; force-check
  the specific touched targets (or read the compile evidence from the test
  run) before claiming green.

## Action items

- Playtest knob noted in code, not tasks: none new (all tunables untouched).
- The consolidated v0.4.0 CHANGELOG task (20260710-093420) now also covers
  this task.
