# Retro: Ledger campaign-wide pace-map (20260722-214053)

## What went well

- Primitive-counting was the right first move. Grepping each file for the
  pacing vocabulary (`scenario_elapsed`, `engage_delay`, `dwell`, `Objective`,
  `StoryMessage`, `Outcome`) produced the headline findings in one pass: `dwell`
  unused mod-wide, ch1/ch3/ch4 with zero clock pacing, ch4's climax
  untelegraphed. Counts turned "feels rushed" into a structural diagnosis, the
  same reframe the Shakedown pacing pass (20260721-211506) got from reading the
  script before touching it.
- Reusing the earlier structural map (the Plan agent's trace of the ch4 ending
  wiring) meant the diagnostic only had to CONFIRM line refs, not re-derive them.
- The out-of-context review earned its keep on a doc-only diff: it verified every
  load-bearing claim against the RON and caught five accuracy nits (miscounted
  handler blocks, an off-by-one objective count, a mislabelled beacon). None
  would have broken a downstream task, but a load-bearing reference should be
  exact.

## What went wrong / was tricky

- `grep -c "OnEnter"` counts substring matches (comments, prose) not handler
  blocks, so the per-chapter header handler counts were off (ch1 5->4, ch4
  3->2). Fix: count by the actual declaration pattern (`name: OnEnter,`), not the
  bare token. Applied the corrections; the per-chapter BODY text was already
  right, so the error was cosmetic-header only.
- I wrote "THREE objectives" while listing four ids in the same line - a
  self-contradiction the reviewer flagged. Writing prose from the diff (count by
  counting) would have caught it; I counted from memory of the structure.

## Lessons / what to do differently

- When counting handler blocks in RON, grep the declaration keyword
  (`name: <Event>,`), never the bare event token - comments and headers inflate
  a substring count. (Reinforces `write-prose-from-the-diff`.)
- A diagnostic brief is load-bearing; run it through the same out-of-context
  accuracy review as code, and fix even the low-severity nits since four tasks
  will author against it.

## Follow-ups

- None blocking. The five owner playtest questions are batched into the umbrella
  GOAL.md Manual acceptance for the Finish checkpoint. The sibling tasks
  (-214058, -214105, -214110, -214115) author against this brief.
