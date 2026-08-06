# Retro: Cleanup and maintenance: close the engine gaps the screenshot pipeline routed around

- TASK: 20260805-185103
- BRANCH: task/20260805-185103 (landed straight on `master`)
- REVIEW ROUNDS: 3

## What went well

The out-of-context reviewer earned its keep three times over. Round 1's two
BLOCKERs were a type-ownership move (`bcs::Health` -> `nova::Health`) that
`cargo check` cannot see, because both types existed and both compiled. A
reviewer holding the implementer's context would have read the same import and
seen the intent rather than the symbol.

Round 3 turned the same trick on a fix. R2.1 asked for subject counts so an
empty query stops reading as a clean gate; the counts then proved, at runtime,
that the round-1 repoint actually works - 4, 6, 4, 17 and 10 health subjects
across the five sections. That is evidence rounds 1 and 2 never had, because
neither re-ran the probe.

Sabotage-checking tests before claiming them (R1.13, R3.3) caught nothing
broken but cost minutes and made both claims real.

## What went wrong

**A doc sweep declared complete, three rounds running.** R1.9 claimed a
repo-wide bcs-attribution sweep; it had swept `crates/` only, and round 2 found
`examples/`. R1.14 made the identical claim with the identical gap. Round 2
diagnosed it correctly - "record the sweep as the command that produced it, not
as prose" - and round 3 still found three more in `crates/`, two of them
rustdoc on the very modules the step took ownership of. The remedy was applied
to the new finding (R2.2) and never retrofitted to the old one.

Root cause: a sweep is reported as a conclusion ("the mentions left are real
ones") instead of as a reproducible command with its path list. A conclusion
cannot be re-run by a reviewer; it can only be believed or redone from scratch.

**A guarantee restored in name.** R1.2 repointed `combat_burst_driver`'s
`Health` and was ticked as restoring "keeps every combatant alive". The import
was right and the guarantee still false: destruction observes
`HealthZeroMarker`'s INSERT, so a once-per-frame post-damage top-up can never
deliver immortality, and writing full HP onto a spent pool forged a
full-HP-yet-destroyed entity that the health-bounds invariant read as clean.
Fixing the symptom the finding named let a ticked box outrun the behaviour.

Worth recording: round 3's suggested fix (`try_remove` the marker) was also
wrong, for the same reason. Re-deriving the observer's timing before choosing
between the reviewer's two options is what found it.

**Steps landed straight on `master`,** interleaved with three other tasks, so
`cafae048..HEAD` was 61 commits of which 33 were this task's. Every round had
to reconstruct scope by grepping the task ID. Six in-range commits carry no
task scope, two of them landing mid-review.

## What to improve next time

- A type-ownership migration owes a cross-crate grep for the OLD path as a
  step-closing proof, not a hope. Both blockers were one `rg` away.
- Report a sweep as its command and path list. "I grepped X, Y, Z and these
  are the hits" is checkable; "the remaining mentions are legitimate" is not.
- When a finding names a symptom, re-derive the guarantee behind it before
  ticking. R1.2 named a stale import; the broken promise outlived the fix.
- Re-run the runtime verdict in the review round that claims it. Rounds 1 and
  2 both stood on the implementer's recorded probe grades while R1.1/R1.2 meant
  the health invariant behind those grades was checking nothing.

## Action items

- Two `manual:` DoD items remain open and are the owner's, not the review's:
  the child set is complete, and the capture-run flicker is gone. Neither is
  resolvable by review and neither was self-ticked.
- DoD item 1 ("every step above has a child task") cannot be ticked as written:
  no task carries `PARENT: 20260805-185103`. Raised in round 1, unchanged.
- bevy-common-systems is at `6f09461`, unpushed and untagged, per the owner's
  ruling. Nova's pin stays `v0.19.5`. Tagging and pushing are the owner's call.
- R2.1's subject counts are report-only by design. A run whose health invariant
  examines zero entities still grades green in CI; only a human reading
  `checks.json` sees the 0. A gate is possible - `catalog_drift.rs` already
  knows which examples own a combat roster - but no step asked for one.

## Landing message

```
chore(20260805-185103): close the engine gaps the screenshot pipeline routed around

The screenshot examples worked around engine gaps rather than closing them.
This closes them: one capture idiom (`shoot`, the reel deleted), a uniform
scene settle, runtime coverage declared by WIRING instead of by category, the
smoke suite replaced by a probe correctness sweep in CI, camera authority
ordered into explicit phases, and nova taking ownership of health, integrity
and persistence instead of borrowing bevy_common_systems'.

Three review rounds. The last two were spent on a type-ownership move that
compiled cleanly while silently disabling the health invariant in every run;
invariants now report how many subjects they examined, so an empty query is
visible instead of reading as a clean gate.
```
