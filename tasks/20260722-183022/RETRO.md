# Retro: pre-commit fmt guard (20260722-183022)

Outcome: shipped a tracked `.githooks/pre-commit` `cargo fmt --check` guard
that makes rustfmt drift impossible to land. APPROVE round 1, no blockers.

## What went well

- **Read the tool before designing around it.** The task listed three seams
  (git hook / `/work` skill / `sprout land` preflight). Instead of guessing,
  reading the `sprout` source settled the whole design: `cmd_land` commits with
  a plain `git commit` (no `--no-verify`) and rolls back on hook failure by
  design (its own comment names "hook failures"). That single fact collapsed
  the three seams into one - a repo hook covers the land path for free - and
  turned "which seam" from a user question into a verified fact.
- **The test caught my own bad proof.** My first real-repo reject check used an
  ORPHAN `.rs` file; the hook passed because `cargo fmt` only formats
  module-reachable files. That was my test being wrong, not the hook - and it
  surfaced the real property worth stating: the hook has exactly CI's coverage
  (same `cargo fmt` blindness), which is the design goal, not a gap. Re-proved
  with a module-reachable file (real rustfmt diff, commit refused).
- **Dogfooded the guard.** Armed `core.hooksPath` mid-task, so this task's own
  commits (and the land) ran through the hook - exercising the docs-only skip
  path in the real repo.

## What went wrong

- `git checkout -- <file>` restores from the INDEX, not HEAD. After I staged a
  misformat probe, that "restore" left the staged bad version in the tree. Had
  to fix with `git restore --staged --worktree`. A test that mutates a tracked
  file must restore from HEAD (`git restore --staged --worktree` or
  `git checkout HEAD -- <file>`), and must re-verify `git status` is clean -
  which is exactly how the miss was caught.

## What to improve next time

- When probing rustfmt behavior, use a MODULE-REACHABLE file - an orphan `.rs`
  is invisible to `cargo fmt`, so it silently proves nothing.
- Any throwaway mutation of a tracked file needs a HEAD-restore + a clean
  `git status` assertion, not an index-restore.

## Follow-ups

- [ ] NIT (from review): the hook's `nix develop` fallback is unverified in
      practice (CI and dev both have bare cargo). Low value; not filed as a task
      unless a NixOS-without-devshell commit ever trips it.
- [x] LESSONS `lint-gate-is-the-last-step` annotated SHIPPED with this task id.
