# Review: Resolve asteroid_field hidden-vs-wiki contradiction

- TASK: 20260721-160842
- BRANCH: fix/asteroid-field-hidden

## Round 1

- VERDICT: REQUEST_CHANGES
- REVIEWER: out-of-context

- [x] R1.1 (MAJOR) tasks/20260721-160842/TASK.md Record (also
  crates/nova_assets/src/scenario.rs:395-400, CHANGELOG.md:17) - the recorded
  git-history evidence is factually wrong: shakedown NEVER chained into
  asteroid_field in any committed state (pickaxe on shakedown.rs across all
  history: NextScenario targets are only shakedown_run and broadside; every
  committed revision of shakedown_run.content.ron has zero asteroid_field
  occurrences). The "mid-story stage reached by chaining from the shakedown
  run" premise was already false when baf56811e introduced the flag. In-session
  re-verification found the actual history: asteroid_field was the ORIGINAL
  New Game scenario (NEW_GAME_SCENARIO_ID in 24491209's parent) and 24491209
  swapped New Game to shakedown_run. Correct the narrative in all three
  surfaces; the UNHIDE decision itself stands (strengthened, if anything).
  - Response: fixed - all three surfaces rewritten to the verified history
    (never-true premise, original New Game scenario, pickaxe hit was the
    NEW_GAME_SCENARIO_ID swap); Record also gains the misread-pickaxe
    reflection.
- [x] R1.2 (MAJOR) crates/nova_assets/tests/broadside_assault.rs:325/544 - no
  test pins asteroid_field's picker visibility, so an accidental re-hide
  passes the whole suite; the repo convention pins exactly this property for
  the siblings (broadside `!hidden` at :325, gunship `hidden` at :544), and
  broadside_assault.rs already includes ASTEROID_FIELD_RON. Add the pin for
  asteroid_field (`!hidden`) and the paired pin for asteroid_next (`hidden`,
  which the Record explicitly asserts stays correct).
  - Response: fixed - new test `the_sandbox_is_listed_and_its_relay_is_not`
    in broadside_assault.rs pins field `!hidden` + thumbnail present + relay
    `hidden`; sabotage-proven against master's RON (see Record).

Verification notes (out-of-context reviewer): all DoD cmd proofs re-run
verbatim and passing; lint 0 errors (1 pre-existing WARN + 2 acks); 27 tests
green across content_ron_parity / broadside_assault / example_scenario; the
four claimed fmt-only files verified byte-identical to rustfmt(master) - the
honesty claim holds; chain-in claim verified repo-wide (only the
asteroid_next relay loop references asteroid_field); thumbnail pattern,
CHANGELOG placement and doc surfaces clean. In-session pass re-verified
R1.1's load-bearing claim directly (24491209 shows the NEW_GAME_SCENARIO_ID
swap, not a chain rewire) before adopting the round.

## Round 2

- VERDICT: APPROVE
- REVIEWER: out-of-context (same reviewer resumed against the new diff)

Both R1 findings verified RESOLVED, no new findings. Reviewer independently
re-derived the history claim (24491209 swapped NEW_GAME_SCENARIO_ID from
asteroid_field to shakedown_run; the constant was introduced at 85049480),
swept all three narrative surfaces for residual chain-rewire wording (none),
re-ran the visibility pin and re-derived its sabotage (master RON -> fails
at broadside_assault.rs:571, branch RON -> passes, worktree clean), and
re-ran the three test files (28 green) + fmt check.

No open manual: DoD items on this task.
