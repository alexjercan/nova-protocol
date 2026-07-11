# Review: Quiet the GOTO arrival hunt (settle deadband)

- TASK: 20260711-140234
- BRANCH: fix/goto-settle-deadband

## Round 1

- VERDICT: APPROVE (R1.1 resolved in-round)

- [x] R1.1 (MINOR) crates/nova_gameplay/src/flight.rs (settle_deadband
  docstring) - "STOP's rest precision is unaffected" is stated
  unconditionally, but it only holds for AXIAL residuals (the shipped
  single-centered-drive case); the updated recruit test on this very
  branch demonstrates a lateral residual releasing at up to the band on
  a damage-shifted hull. Qualify the sentence so the docstring does not
  overpromise.
  - Response: qualified - the docstring now scopes the exact-rest claim
    to axial residuals and names the lateral-release contract explicitly.

Verification performed beyond reading the diff:

- Independently confirmed the confound claim (the branch's central
  correction): the diagnostic run on this build reproduces the spike's
  global-0.75 row BITWISE (done=971, terminal 0.0973/0.0309, release
  0.583/0.047) with the leg-scoped band + urgency, proving the scoped
  implementation is exactly the experiment the spike measured, on rest
  legs.
- Checked the falsification evidence: desired==0 scoping left terminal
  spin bit-for-bit unchanged (0.6727556) - conclusive that the scoping
  never influenced the failing phase; the numbers in TASK.md match the
  runs.
- Guard updates are contract migrations, not weakenings: both new bounds
  derive from settings.settle_deadband with a small margin and keep
  their primary assertions (recruit > 0.2; park-point distance bounds)
  untouched. The old 0.5 bounds encoded the pre-band contract.
- ORBIT keeps the tight band on BOTH paths that used to read
  attitude_deadband (fine gate and urgency denominator) - verified in
  the match arms.
- Ran the flight module (59 green + 1 ignored diagnostic) and ai module
  (86 green) suites in the worktree; fmt and check clean. Full workspace
  suite deferred to CI per the user's standing instruction - reported
  honestly, not skipped silently.
- The production-wired rig choice is correct and now load-bearing: the
  wiring-dependence finding means a same-tick rig would validate a game
  we do not ship. The warning handed to 20260711-140241 (including the
  legitimate falsification-close option) is the right routing for the
  conflict this creates.

Note for the next cycle (not a finding): the regression uses diag_app /
diag_ship; when 140241 deletes the goto_wobble_diagnostic, those helpers
must survive (the compiler will enforce it).
