# Retro: Torpedo arming gate (self-detonate-on-spawn fix)

- TASK: 20260707-100003
- BRANCH: feature/torpedo-arming
- PR: #27 (open against master, not merged)
- REVIEW ROUNDS: 1 (APPROVE, one deferred NIT)

See `tasks/20260707-100003/TASK.md` for what changed; this retro is about how the
working went.

## What went well

- Applied the previous retro's lessons directly: redirected the long build/test
  output to files instead of piping through `tail` (so progress was visible), and
  did the small task-file edits by hand rather than firing rapid `tatr` calls.
- Checked blast radius before firing: grepped every `TorpedoSectionConfig`
  construction site *before* adding config fields, which surfaced the one non-
  `..default()` literal in `nova_assets/src/sections.rs`. Updating it in the same
  pass meant the full build was green on the first try instead of failing on a
  missing field.
- Made the arming rule a pure `TorpedoArming::tick(dt, pos) -> bool` method, so the
  core logic is unit-testable with no App at all, and added two system-level tests
  that exercise `torpedo_detonate_system` itself (un-armed survives, armed
  detonates). The tests map one-to-one onto the reported bug, so a regression would
  be caught by name.
- Single clean review round - the design matched the task's suggested shape, so
  there was nothing to rework.

## What went wrong

- Minor path friction: I first read the torpedo file from the main checkout path,
  then had to re-read it from the sprout worktree path because Edit tracks
  file-reads per absolute path. A few wasted reads. Root cause: reading from the
  repo I was thinking in, not the worktree the work lives in.

## What to improve next time

- In a sprout worktree, read files from the worktree path from the very first read,
  so Edit never needs a redundant re-read.

## Action items

- [ ] NIT R1.1 (deferred, not a change here): the default `arm_distance` (5.0) is
      below the proximity-fuze radius `BLAST_RADIUS * 0.5` (15.0), so a target
      5-15u ahead can still detonate the torpedo soon after it arms. Tune/confirm in
      the torpedo test range (`20260707-100001`) alongside the blast-param work
      (`20260706-162913`) and guidance (`20260525-133021`); no new task needed, it is
      already covered by those.
- [x] Verified the fix with unit + integration tests since the range task does not
      exist yet.
