# Retro: unship screenshot-reel - example-embedded scenario, not a mod

- TASK: 20260715-151551
- BRANCH: refactor/unship-reel (landed on master as 1e5fbce1)
- REVIEW ROUNDS: 1 (APPROVE, two MINORs + two NITs fixed on-branch)

## What went well

- The mid-flow user request was filed as its own task with consequences
  enumerated at filing time (orphaned tests, doc cites, leftover-files lesson),
  so planning was a fill-in rather than a discovery pass.
- The synthetic-catalog rig (in-memory hidden decl reusing the loaded demo
  bundle) replaced the departed shipped subject with LESS machinery than the
  originally-sketched fixture file tree - and the reviewer's mutation analysis
  confirmed all three tests fail with their mechanisms deleted.
- Verification exceeded the plan for free: the example's own autopilot smoke
  (built by the earlier screenshot-showcase task, with a built-in scene-loaded
  panic guard) proved the embedded path loads the real scene on an isolated
  Xvfb display.

## What went wrong

- All four review findings were the same root cause: the sweep for the deleted
  mechanism covered markdown docs and code SYMBOLS but not prose inside
  rustdoc/comments in files the diff did not touch (ModEntry.hidden rustdoc,
  the harness plugin rustdoc) or in the untouched half of a touched file (the
  smoke probe's "mod enable" panic text). sweep-then-delete says grep
  "describing words" - "mod" as a describing word for the reel was too generic
  to grep, but "screenshot-reel" in rustdoc WAS greppable and missed because
  the sweep filtered to markdown.
- The plan step said "keep the ReelLoaded once-guard" - written from memory of
  the old polling shape; OnEnter needs no guard. Caught during implementation
  and the step was updated, but it shows plan steps that prescribe carrying a
  structure forward should be re-derived at the new design, not copied.

## What to improve next time

- When sweeping for a deleted mechanism, run the symbol grep over ALL file
  types (rs, md, toml, workflows) and read the hits in comment/rustdoc context
  - do not filter the sweep to docs folders.

## Action items

- [x] LESSONS.md: sharpened `sweep-then-delete` (rustdoc/comment prose in
  all file types) and bumped it; bumped `out-of-context-review-pass`.
