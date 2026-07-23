# Review: Fix stale content_lint_gate test (ledger ch4 mutually-exclusive warn gone)

- TASK: 20260723-103523
- BRANCH: fix/content-lint-gate-ledger-ch4

## Round 1

- VERDICT: APPROVE
- REVIEWER: out-of-context

No BLOCKER / MAJOR / MINOR / NIT findings.

Verified independently (out-of-context reviewer; matches in-session):
- Diagnosis reproduced: `content -- lint --target the-ledger` = 0 errors, 0
  warns, 1 acked (the auditor close-spawn, ack_task 20260717-143806, message
  contains "auditor"); no "mutually exclusive" warn. Confirmed in ch4 content
  that the Auditor now spawns from the SELL branch only (the burn branch's spawn
  was removed) - so the warn is legitimately gone, not an accidentally-dropped
  signal.
- `cargo test -p nova_assets --test content_lint_gate` = 2 passed / 0 failed.
- Scope claim confirmed: on the main checkout (on master), `final_tally_claim`
  fails exactly the two named tests (5 passed, 2 failed); this branch's diff
  touches only content_lint_gate.rs + TASK.md, so the final_tally failures are
  correctly deferred to 20260723-115419.
- `cargo fmt --check` clean; no leftover/debug code; the `LintSeverity` import
  is still live (used by the tree test + the external half).

Fix quality: the re-pin is durable, not brittle-for-brittle. The new in-repo
half asserts three independent things - error_count 0, findings+acks scoped to
the-ledger, and the acked auditor exception surfaces - so it fails if
target-mode attribution breaks, if the-ledger regresses an error, or if the ack
vanishes. The pinned ack is a recorded, playtested design decision, the right
kind of signal to anchor on. The external bogus-prototype half is unchanged.
