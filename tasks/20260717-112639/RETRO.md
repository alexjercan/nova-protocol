# Retro: Broadside act-split + cover hardening

- TASK: 20260717-112639
- BRANCH: work/broadside-rework (landed ebe8b77f)
- REVIEW ROUNDS: 1 (APPROVE; 3 MINOR + 2 NIT, all fixed)

## What went well

- Checking generated-vs-authored FIRST: the parity test names the builder
  as the single source, so the classic hand-edit-the-generated-RON mistake
  never started. gen_content ran twice to prove stability before commit.
- Pattern transfer: the ledger_ch2 cycle's computed-geometry-pin approach
  ported almost verbatim (corridor, 6x overlap, station clearance), which
  made this the fastest content cycle of the flow.
- The reviewer's independent recomputation again matched every number and
  produced a regression->test map that doubles as coverage documentation.

## What went wrong

- R1.3: the post-win test pinned the player-death gate but not the hauler
  soft-fail gate - the SECOND handler sharing the same act boundary went
  unpinned. Root cause: the gate-tightening edit touched three handlers
  but the test edit mirrored only the one the old test already covered.
  When a change tightens N parallel gates, the pin count should be N.
- R1.2 recurred in miniature: the same stale infinite-ammo comment existed
  in TWO files (builder + test); I fixed the builder copy and claimed the
  fix complete. A describing-words grep (sweep-then-delete discipline)
  would have caught the twin.
- Workspace-wide `cargo fmt` picked up a sibling task's unformatted
  examples and bundled them into the diff (reviewer flagged as churn; the
  attempted revert restored an older master state and the final fmt
  re-formatted them anyway). Net effect was fine - master ends fmt-clean -
  but the diff carried noise. Scoped formatting (cargo fmt -p) or a
  pre-work fmt-check would keep content diffs content-only.

## What to improve next time

- After tightening or moving a shared gate value, grep for EVERY handler
  and test that references the old value (the act < 3 sweep missed two
  comments and one unpinned handler).

## Action items

- [x] docs/LESSONS.md: new lesson parallel-gates-pin-all (x1); bumped
  sweep-then-delete with the two-file stale comment variant.
