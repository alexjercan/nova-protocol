# Review: Fix final_tally_claim survey->picket tests (inherited nova_assets failures)

- TASK: 20260723-115419
- BRANCH: fix/final-tally-survey-picket

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

No BLOCKER / MAJOR / MINOR / NIT findings.

Verified independently (out-of-context reviewer; matches in-session):
- Diagnosis is a genuine stale rig, not a papered-over content regression: the
  OnStart VariableSet block in final_tally.content.ron seeds all 5 added vars at
  0.0 (survey_posted, picket_posted, break_posted, picket_gate, break_gate), so
  the completed `seed_live_claim` MIRRORS real OnStart (15 keys matching in
  order, + engine-provided scenario_elapsed), inventing nothing. `git show
  0ae5c7f9` confirmed that commit added the vars to content but never touched
  the test.
- Tests remain meaningful (would-it-fail-without-it): the assertions still
  assert the picket / break_flagship objective actually posts; both new lines
  are load-bearing (the seed satisfies the `*_posted == 0` filter, the clock
  advance satisfies `scenario_elapsed > *_gate`). Remove the content handler and
  the tests fail.
- `cargo test -p nova_assets --test final_tally_claim` = 7 passed / 0 failed;
  the FULL `cargo test -p nova_assets` is green end to end (every binary 0
  failed); `cargo fmt --check` clean; no leftover debug code; diff touches only
  final_tally_claim.rs + TASK.md.

Reviewer note (addressed): the seed helper's doc comment cited the
defer-objectives pass by a task id while the close-out cited the commit.
Corrected the code comment to cite commit 0ae5c7f9 (the change the diagnosis
verified against).
