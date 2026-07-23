# Fix final_tally_claim survey->picket tests (inherited nova_assets failures)

- STATUS: CLOSED
- PRIORITY: 5
- TAGS: v0.8.0, bug, testing, content

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

- [x] Reproduce: `cargo test -p nova_assets --test final_tally_claim` -> 2
      failed (survey/picket, cast-off/break).
- [x] Trace the survey->picket wiring with real values: instrumented test 1
      after the survey - `surveyed=1.0 picket_gate=6.0 picket_posted=None
      elapsed=30 objs=[]`. Two findings: (1) the picket objective is now
      BREATHE-GATED (`picket_gate = scenario_elapsed + 6`, posted by a separate
      OnUpdate handler once the clock passes it - the defer-objectives pass,
      commit 0ae5c7f9); (2) the KEY bug: `picket_posted` is `None`, so the
      handler's `picket_posted == 0` filter never matches and it never fires.
- [x] Decide: STALE TEST, not a content regression. The content is correct by
      design (announce-breathe-arrive pacing). The `seed_live_claim` helper -
      which claims to "seed the whole OnStart variable block" - DRIFTED: commit
      0ae5c7f9 added `survey_posted/picket_posted/break_posted/picket_gate/
      break_gate` to OnStart, and the helper was never updated, so the gated
      handlers read `None`. Fix is test-only.
- [x] Green `cargo test -p nova_assets` (FULL suite now green, together with the
      landed 103523 content_lint_gate fix).

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

## Close-out (20260723)

Diagnosis (instrumented, real numbers): after the survey, test 1 showed
`surveyed=1.0 picket_gate=6.0 picket_posted=None elapsed=30 objs=[]`. The
picket objective is posted by a breathe-gated OnUpdate handler that filters
`picket_posted == 0` and `scenario_elapsed > picket_gate`. `picket_posted` was
`None` - never seeded - so the filter never matched and the objective never
posted. Root cause: the `seed_live_claim` test helper (doc: "seed the whole
OnStart variable block the way OnStart would") had DRIFTED from OnStart. The
defer-objectives pass (commit 0ae5c7f9) added five variables to the OnStart
block - `survey_posted`, `picket_posted`, `break_posted`, `picket_gate`,
`break_gate` - and neither the helper nor the two tests were updated. Confirmed
0ae5c7f9 touched the content but NOT the test file (git show). Content is
correct by design (the announce-breathe-arrive pacing); this is a STALE TEST /
unfaithful rig, not a content regression.

Fix (test-only):
1. Completed `seed_live_claim` to mirror the current OnStart VariableSet block
   exactly - added the five missing `*_posted`/`*_gate` vars at 0.0. This is
   the primary fix (the `production-faithful-rigs` lesson) and is what unblocks
   the gated handlers.
2. Advanced `scenario_elapsed` past the breathe gates before asserting the
   deferred objectives (picket after its 6s gate in test 1; break_flagship
   after its 8.4s gate in test 2), matching how the file's already-passing
   tests advance the clock. Both changes are needed: the seed lets the handler
   fire, the clock advance satisfies the breathe.

Verify: `cargo test -p nova_assets --test final_tally_claim` 7/7 (was 5/2); the
FULL `cargo test -p nova_assets` is now GREEN end to end (both inherited
failures - this and the landed content_lint_gate fix - cleared); `cargo fmt
--check` clean.

Self-reflection: the instrument-first move was decisive - my initial hypothesis
(just advance the clock) was INCOMPLETE and the `picket_posted=None` dump
redirected me to the real cause (a drifted seed helper) in one step. A
"seed the whole OnStart block" helper that is hand-maintained will silently rot
every time a content pass adds an OnStart variable; worth a durable lesson (see
RETRO / ledger).
