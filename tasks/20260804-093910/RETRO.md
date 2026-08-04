# Retro: Retire the mainline and POC example runs, reduce screenshots to capture-only

- TASK: 20260804-093910
- BRANCH: refactor/retire-example-runs
- REVIEW ROUNDS: 1

## What went well

- One round, APPROVE, nothing above MINOR. The plan's four `cmd:` proofs were
  confirmed red on the base before work, so "the retirement is complete" was
  mechanically decidable rather than argued.
- DoD 7 (`ls target/reel`) was the only proof that could catch the eager-
  completion hazard, and it did its job. Every other proof and the smoke test
  pass for a producer that exits before its PNG lands.
- DECISION.md settled all three boundary questions (the `fps_exempt` key,
  `NOT_PROBED`'s `render_scale_shot`, how far the conversion reaches) before
  implementation. The review read them as disclosed, not as creep.

## What went wrong

- The Notes claimed no documentation named the retired examples. Wrong twice.
  `web/src/wiki/dev/development.md` was caught during work; `.claude/skills/
  probe/SKILL.md` was not, and shipped stale (R1.1, R1.2) - it still lists
  `gameplay` as a live probe category and carries a `gameplay/broadside` row.
  The decision seemed sound because a sweep HAD been run; the sweep used
  `rg -r`, which is ripgrep's REPLACE flag, so it silently matched nothing to
  recurse into.
- `cargo check --examples` does not compile `tests/`, so a `GAMEPLAY`
  reference in `catalog_matches_disk:149` - outside the two line ranges the
  Steps named - survived the example check and only surfaced under `cargo
  test`. The Steps' line-number precision became a false floor.
- Three copies of `capture_settle_frames` landed next to two other spellings
  of the same idea, in a change whose stated win is that `screenshots/` now
  has one idiom (R1.3).

## What to improve next time

- Breadth: the diff is large (-2242/+777, 19 files) but not wrongly split. The
  retirement and the reduction share exactly one seam - DoD 1 greps the whole
  of `examples/`, which only passes once both halves land - so an independent
  split would have needed a weaker proof. Inherently large, correctly scoped.
- Churn: near zero, and the residue points at the plan's doc step, not the
  work. The plan-time question that would have prevented R1.1/R1.2 is the
  cold-reader one: "which surfaces name this symbol?" answered by ENUMERATING
  them (README, `web/src/wiki`, `AGENTS.md`, `.claude/skills`) rather than by
  one grep whose flags were never checked. A sweep that reports zero hits owes
  a positive control.
- Context: no threshold crossing or compaction observed. Round 1 delegated to
  an out-of-context reviewer, which is where the stale skill docs were found -
  the fresh context read `.claude/skills` because it had no memory of the
  sweep having "already been done."

## Action items

- R1.1/R1.2 are open MINORs and do not block landing; fold the two
  `.claude/skills/probe/SKILL.md` edits into whichever task next touches probe
  docs (`20260804-094006` is the natural home - it already edits probe
  categories).
- `20260804-094006`'s Steps still list the `fps_exempt` key deletion this
  branch took (DECISION 1). Re-read that Step as a verification, not an edit.
- R1.3-R1.5 (settle-frame duplication, a missing blank line, two wrapped
  comments) are NITs; take them opportunistically.

## Landing message

```
refactor(examples): retire gameplay/ + the RTT POC, reduce screenshots to capture-only

Delete examples/gameplay/ (broadside, lifeline), examples/ui/nova_os_rtt_poc.rs
and its example-owned shader, their [[example]] blocks, the orphaned
[package.metadata.nova_probe] table, the "gameplay" CATEGORY_POLICIES row, and
the GAMEPLAY smoke list.

Strip probe enrollment from the six enrolled screenshots/ producers, drop
screenshot_reel's smoke backstop, and convert the five beat-script producers
from wall-clock holds and one-shot booleans onto AutopilotPlugin step
timelines. Load gates are now player_ship_present()/state_is, every capture
owns a step, and each producer ends on a capture step held open for the
asynchronous save_to_disk write.
```
