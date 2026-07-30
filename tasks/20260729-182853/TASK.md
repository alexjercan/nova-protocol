# Screenshot beat for the objective chip posting (currently unseeable)

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog,ui,hud,tooling,testing

## Story

No harnessed example can show the objective posting animation end to end. The
reveal card takes `REVEAL_TOTAL_SECS` (~3.2 s) to fly and tuck into the
objective stack, and every autopilot walk completes its objectives faster than
that (the lifeline walk completes `screen_convoy` 1.3 s after posting it),
posts none at all (broadside), or ends before the first posting
(menu_newgame's 6 s hold ends during the shakedown's opening conversation,
which posts no objective by design). So the card-to-chip HANDOVER - the most
visually load-bearing moment in the objective surface - is pinned only by
App-driven tests, never seen.

Raised by the out-of-context reviewer of 20260729-163816, who also pointed at
the cheap fix: the screenshot examples already run a timed `at(<secs>, ...)`
script (see `examples/screenshots/screenshot_juice.rs`), and the objective
stack is driven purely off the `GameObjectives` resource, so a few lines that
post an `Objective` at `at(1.0, ...)` and capture at `at(4.5, ...)` would show
the whole motion deterministically, with no scenario logic involved.

## Steps

- [ ] Pick the home: a new small `examples/screenshots/screenshot_objective.rs`
      (preferred - a shipped web capture should not carry a synthetic
      objective) driving the timed script.
- [ ] Post an `Objective` into `GameObjectives` at ~1.0 s and capture at least
      two frames: one mid-card (~2.0 s, the card in flight with NO chip yet)
      and one just after the tuck (~4.0 s, the chip up and popped).
- [ ] Register it in the `[[example]]` catalog and the `examples_smoke`
      screenshots category, and add it to the probe's NOT_PROBED list with a
      reason if it should not be swept.
- [ ] Record in the task what the captures show, and whether the composed
      motion reads well (this doubles as evidence for 20260729-163816's
      manual DoD item, which is currently owner-playtest only).

## Definition of Done

1. cmd: `BCS_AUTOPILOT=1 BCS_REEL=1 NOVA_SHOT_DIR=target/reel cargo run
   --example screenshot_objective --features debug` produces both captures.
2. manual: the mid-card frame shows the card with no chip; the post-tuck frame
   shows the chip carrying the objective text.
3. test: `cargo test --test examples_smoke screenshots` still passes.

## Notes

- 2026-07-30 (task 20260729-211200): the reveal CARD is deleted - the chip is
  the whole posting now, spawning and popping on the posting frame. The
  card-to-chip handover this task was written to capture no longer exists, so
  the Story above is history. What is still unseeable and still worth this
  task: the chip's own arrival (pop -> settle -> breath) and a two-chip stack.
  Re-scope before working it - the capture times collapse from ~2.0 s/4.0 s
  (mid-card/post-tuck) to roughly the posting frame and ~1.5 s later, and DoD 2
  must be rewritten (there is no "mid-card frame").
- Discovered mid-flow during 20260729-163816; not folded into that branch to
  keep it to the feature (flow: new work becomes its own task).
- Also the natural home for a future "two chips stacked" capture, which is
  likewise test-only today.
