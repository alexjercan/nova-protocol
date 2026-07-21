# Review: base chain voice pass

- TASK: 20260721-160929
- BRANCH: content/base-voice-pass

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

- [x] R1.1 (MINOR) crates/nova_assets/src/scenario/broadside.rs:378-401 - the
  first-corvette-down comms pair's mutual exclusion (each handler gates on
  the OTHER kill flag being 0) had no test pin; a future edit could silently
  produce two lines or zero lines. Suggested: a config-level assertion that
  exactly two such story handlers exist, each with the cross-flag == 0
  filter.
  - Response: fixed - `the_first_kill_line_is_mutually_exclusive_on_the_cross_flag`
    added in broadside_assault.rs, per the suggested shape, tightened to
    require the literal 0.0 comparison so a flipped gate (== 1.0) also
    fails; suite now 14 green. Ticked on the reviewer's own suggested-change
    criteria (config pin, both handlers, cross-flag).

Verification notes (out-of-context reviewer): all DoD proofs re-run verbatim
(speaker grep = broadside + gunship exactly; lint 0 errors with only
pre-existing findings; 13+2 tests green pre-fix; CHANGELOG hit). Victory
variant exclusivity traced through the event-queue semantics (instantaneous
exclusion on hauler_lost, act==1 as the cross-frame second guard). The
shakedown amendment's cited lint rule verified at
crates/nova_scenario/src/lint.rs:257-270 - the amendment is justified, not a
dodge. Double-kill trace: exactly one line, never two or zero. Dead-id sweep
for the removed hauler_lost objective: only the flag const and
absence-asserting tests remain. Master-vs-branch test comparison: nothing
weakened (the post-win objective pin became an equivalent-strength flag
pin). All new lines plain ASCII, tone consistent with the Ledger register.
Record matches the diff exactly.

Pending manual (batched to flow Finish, not blockers): cast names await the
owner's nod; comms pacing feel in play.
