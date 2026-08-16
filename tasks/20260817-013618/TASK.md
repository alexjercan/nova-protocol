# Examples taxonomy: systems is correctness, screenshots is content

- STATUS: IN_PROGRESS
- PRIORITY: 51
- TAGS: v0.11.0,example,harness,refactor

## Goal

Owner-approved examples rework. The categories become purposes:

- screenshots = content/showcase (captures, frametime, snapshots; feeds web).
  Keeps everything it has today.
- systems = correctness ranges. THE DOCTRINE, recorded in repo conventions:
  you find a bug, you write a reproducible systems range, the fix turns it
  green, catalog_drift pins its invariants so a deleted assertion screams.

## The moves

- sections/ dissolves into systems/ (renames allowed for clarity).
- ui/ splits: correctness ranges to systems/, content pieces to screenshots/.
- many_* are DELETED, replaced by four single-file stress ranges in systems/
  (one file per test, owner's call - less code per file):
  1. ~1000 bullets in flight, measured window, exact-count clear
  2. ~1000 torpedoes (guidance + weave + fuzes), same shape
  3. one huge structure (~1000-section ship): skin, health graph, one body
  4. many structures (~100 ships x ~10 sections): churn + spatial queries
  Each: own probe frametime capture, named absurd-scale constants with a
  comment that they must never reflect real content, generous deadlines
  (llvmpipe), invariants on the roster.
- systems ranges get better names + descriptions across the board.
- The invariant roster (catalog_drift) EXTENDS to every systems range, not
  just the former sections ones.
- Fix examples/systems/neutralized_quiet.rs: pre-existing race, fails 4/4 on
  a clean base commit - its step advances on assignment+firing then re-reads
  a frame later while the AI trigger drops for a frame during barrel slew.
- scenario_id example: LEAVE ALONE (retired by the coordinator after the
  --scenario flag lands separately).

## Done when

- probe run walks the new catalog green; catalog_drift green with the
  extended roster; CI's --all-targets compiles everything moved
- the four stress ranges emit per-range frametime numbers
- web/src references to moved example names updated
- the doctrine paragraph lives in the repo conventions
