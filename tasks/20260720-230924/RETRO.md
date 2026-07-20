# Retro: remove the nova_portal_gen crate

- TASK: 20260720-230924
- BRANCH: refactor/remove-portal-gen (landed a8b0c833)
- REVIEW ROUNDS: 1 (out-of-context APPROVE; 2 MINOR fixed, 2 NIT accepted)

See TASK.md Outcome for what changed; process only here.

## What went well

- The "preserve coverage BEFORE deleting" discipline
  (`deleted-content-tests-carry-engine-coverage`) was the whole task and it
  held: the 15 publish-gate rejection cases were re-homed onto the production
  tool (gen-portal.py, driven by a Rust subprocess test) before the crate was
  deleted, so no gate lost its guard.
- The out-of-context reviewer did the check that mattered for a DELETION: a
  case-by-case comparison of the deleted `generate.rs` against the new
  `gen_portal_gate.rs` (confirming none dropped) and an independent right-reason
  run of gen-portal.py on three fixtures - not just "the tests pass".

## What went wrong

- The re-homed gate coverage shipped WEAKER than the original: the subagent's
  rejection rows asserted only a non-zero exit, where the deleted test asserted
  the specific error reason (`err.0.contains(...)`). A gate that started
  rejecting for the wrong reason (or a parse error masking a membership check)
  would have passed green. Root cause: a coverage re-home was scoped to "same
  cases, same pass/fail" without carrying the assertion FIDELITY across. Caught
  by review (R1.1), fixed by adding a `stderr_contains` per row mapped to
  gen-portal.py's actual messages.
- Secondary: the python3 self-skip would have silently dropped the only gate
  coverage if CI ever lost python3 (R1.2) - fixed to hard-fail under `CI`.

## What to improve next time

- Re-homing test coverage onto a different tool (Rust lib -> subprocess) must
  preserve the ASSERTION, not just the case: if the original checked WHY it
  failed (error string), the port checks the same, or the coverage silently
  degrades to "something went wrong". Count of cases is not fidelity of cases.

## Action items

- [x] LESSONS.md: added `re-homed-coverage-keeps-assertion-fidelity` (x1).
- [x] All references to nova_portal_gen swept; the crate is fully gone. (The
  152247 follow-up "remove the crate" is now done.)
