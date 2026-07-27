# Retro: Fix conformance DECISION.md STATUS lines

- TASK: 20260727-112529
- REVIEW ROUNDS: 0 (trivial doc chore, no review)

## What happened

Two landed/in-flight tasks tripped `tatr check --ledger` with
`bad-decision-status`. `20260726-214639/DECISION.md` had no STATUS line
(added `- STATUS: ACCEPTED`, the task is CLOSED). `20260727-015156/DECISION.md`
had a `STATUS: PROPOSED (...)` line that was neither a `- ` bullet nor an
accepted value - `tatr check` only allows `ACCEPTED` or `SUPERSEDED by <ref>`.
Set it to `- STATUS: ACCEPTED` and kept the "task still OPEN, awaiting plan
gate" nuance as a separate note line.

## Lesson

`tatr check` DECISION.md STATUS is a closed enum (`ACCEPTED` / `SUPERSEDED by
<ref>`), and must be a `- ` bullet - `PROPOSED`/`DRAFT` and un-bulleted lines
fail. Captured as `decision-status-enum` in the ledger.

## Action items

- [x] Both DECISION.md files fixed; `tatr check --ledger` clean.
