# Review: Lifeline (ch3a) - convoy defense

- TASK: 20260721-160957
- BRANCH: content/lifeline-ch3a

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MINOR) crates/nova_assets/src/scenario/lifeline.rs player-death
  handler - leaves act at 1 while CurrentOutcome is last-write-wins and the
  bell Victory's clock gate is true every pulse: a mutual-destruction trade
  could overwrite the Defeat with a Victory over the queued retry. Suggested:
  set act 3 in the handler.
  - Response: fixed - terminal act 3 set first in the handler (with the
    review comment); harness pin extended with the trade case
    (player_death_retries_the_lane now kills the last wave + crosses the
    bell after the death and asserts the Defeat holds). The same
    pre-existing shape in broadside.rs is filed as its own task
    20260721-182034 (narrower window there; review-the-diff-not-the-repo).
- [x] R1.2 (MINOR) ally_relation_tests - the shipped convoy shape (bare
  SpaceshipRootMarker + Allegiance::Player, no controller marker) had no
  runtime acquisition rig; a candidate-query refactor could silently make
  the convoy untargetable. Suggested: one marker-less acquisition rig.
  - Response: fixed - `a_markerless_player_allegiance_root_is_acquired`
    added; 4 ally rigs green.
- [x] R1.3 (NIT) TASK.md Record - "17-stage walk" is a 14-stage walk (13
  transitions); harness test names were promised in Steps but recorded in
  the Record.
  - Response: fixed - wording corrected, the eight lifeline_convoy test
    names + the chain test recorded in the step text.

Verification notes (out-of-context reviewer): act machine traced in full
against source dispatch semantics (authoring-order handlers, synchronous
variable writes) - four Victory variants mutually exclusive, no
double-spawn/out-of-order/deadlock in the wave gates, clear-gated comms
lines fire exactly once, post-victory hauler death gated; lint re-run (0
errors, zero lifeline findings, no acks); all three suites re-run green;
the no-stacking and layout-floor assertions verified fail-capable; the
example's clock fast-forward verified legitimate at loader.rs:376-381 and
the probe run.log holds the full staged walk with checks.json verdict OK;
every DoD grep re-run verbatim; the T1 verdict judged honored (the
controller:None adaptation openly recorded, claims verified at source);
beat-sheet/ASCII/cast conventions clean; Record honest apart from R1.3.

Pending manual (batched to flow Finish): difficulty first pass (relief
240s, waves 2/3/2, W3 full-gun corvette), stalled-vs-crawling convoy image,
cast names.
