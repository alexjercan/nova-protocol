# Fix conformance: add STATUS line to DECISION.md for tasks 20260726-214639 and 20260727-015156

- PRIORITY: 0
- TAGS: backlog, chore
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

`tatr check --ledger` flags `bad-decision-status` on two landed tasks whose
DECISION.md files have no STATUS line:

- tasks/20260726-214639/DECISION.md
- tasks/20260727-015156/DECISION.md

Add the appropriate `- STATUS: ACCEPTED` (or the correct status) line to each so
`tatr check --ledger` is clean. Pre-existing, surfaced at the Finish of flow
20260726-214708. Do not rewrite the decision content - just add the missing
header field.
