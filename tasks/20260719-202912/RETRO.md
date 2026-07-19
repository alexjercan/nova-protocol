# Retro: harness mute

- TASK: 20260719-202912
- BRANCH: feature/harness-mute (squash-landed as ed826dae)
- REVIEW ROUNDS: 1 (APPROVE; 2 NITs recorded)

## What went well

- Seam analysis BEFORE filing paid for itself twice: reading all
  `factor()` callers first exposed that the obvious mute site (inside
  `factor()`) would have corrupted persisted settings (settings_store
  saves `factor()` to disk) and lied in the menu UI - the output_gain
  split was designed in the task body, not discovered in review. And the
  "insert GlobalVolume(0) at startup" naive plan was rejected on paper
  (apply_master_volume overwrites it on frame 1) instead of by a
  confusing debugging session.
- Env-to-resource promotion for testability: the plan's per-call env read
  was upgraded mid-implementation to a `HarnessMute` resource resolved
  once at plugin build, because per-call env reads would have made the
  App tests raceable (parallel tests sharing process env). Tests inject
  the resource; the precedence logic is a pure fn with a combo table -
  zero env mutation anywhere in the test suite.
- The coverage proof was one grep: every AudioPlayer/AudioSink in the
  repo lives in a single file, so "three masked call sites = the whole
  output surface" is a checkable fact, not a hope.
- Zero consumer changes: smoke suite and probe both already set
  BCS_AUTOPILOT for their children, so the auto-mute rode an existing
  contract - the feature landed without touching either harness.
- Honest verification split: tests prove the resource->mixer wiring; the
  user's ear proved the actual silence (the one check the machine cannot
  do was explicitly assigned, not silently skipped).

## What went wrong

- Nothing structural. Two small frictions: three Edit calls targeted
  files read only in the PREVIOUS task's worktree (same path shape,
  different checkout) and were rejected until re-read - worktree-hopping
  invalidates file-read state; and the first fmt run reflowed the just-
  written prelude export, a no-op but a reminder to fmt before reviewing
  diffs.

## What to improve next time

- When a task's plan involves masking a value, enumerate ALL readers of
  that value first (the factor()-callers grep) - "who else trusts this
  number" is the question that found the persistence trap, and it
  generalizes to any settings/gain/scale masking work.

## Action items

- None new; NITs live in REVIEW.md (env-only escape hatch; harmless
  mute/pause overlap).
