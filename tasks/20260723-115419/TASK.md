# Fix final_tally_claim survey->picket tests (inherited nova_assets failures)

- STATUS: OPEN
- PRIORITY: 5
- TAGS: v0.8.0,bug,testing,content

## Story

As a developer, I want `cargo test -p nova_assets` green on master. Two tests in
`crates/nova_assets/tests/final_tally_claim.rs` FAIL on master (confirmed
20260723, they fail identically in a clean checkout - INHERITED, not caused by
the campaign-picker run that discovered them):

- `the_survey_is_a_one_shot_travel_lock_gate` (line ~379): after
  `travel_lock(&app, "anchorage_bow")`, `surveyed == 1.0` (the survey fires) but
  the assertion "the survey posts the picket objective" fails - no objective
  with id `picket` is present in `GameObjectives`.
- `the_cast_off_waits_for_survey_pickets_and_the_breathe` (line ~444): the
  paired cast-off/breathe gate, same survey->picket flow.

So the survey fires (`surveyed` flips) but the picket objective no longer posts.
Likely the final_tally content or its survey->picket-objective wiring changed
(the 20260722 ledger ch3/ch4 rework touched this area) and the test's
expectation went stale, OR a real objective-posting regression slipped in.

## Steps

- [ ] Reproduce: `cargo test -p nova_assets --test final_tally_claim` -> 2
      failed (survey/picket gate).
- [ ] Trace the survey->picket wiring with real values: does the final_tally
      content still post a `picket` objective on survey, or did the objective id
      / handler change? Compare the test's expectation against the CURRENT
      generated `final_tally.content.ron` and its builder.
- [ ] Decide: stale test (update the expectation to the objective the scenario
      actually posts now) vs real regression (fix the content/handler so the
      picket objective posts). Record which, with the evidence.
- [ ] Green `cargo test -p nova_assets`.

## Definition of Done

- `cargo test -p nova_assets --test final_tally_claim` passes. (cmd: that test)
- Full `cargo test -p nova_assets` green (together with 20260723-103523's
  content_lint_gate fix). (cmd: `cargo test -p nova_assets`)
- The fix names whether the test was stale or the content regressed, with the
  survey/objective evidence.

## Notes

- Discovered during 20260723-103523 (content_lint_gate fix) verify step; that
  branch touches only content_lint_gate.rs and is not the cause.
- Fails identically on master (checked out master, same 2 failures at
  final_tally_claim.rs:379/444).
- Related rework: the 20260722 ledger ending/pacing pass (tasks 20260722-214110
  and siblings).
