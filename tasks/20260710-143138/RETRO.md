# Retro: CI smoke gate - taffy panic containment

- TASK: 20260710-143138
- BRANCH: fix/ci-smoke-gate (landed as d25de6a)
- REVIEW ROUNDS: 1 (APPROVE; 2 MINOR, addressed)

## What went well

- Sequencing paid the whole bill: the plan deliberately put this task
  AFTER the examples rework, and the rework dissolved the panic's trigger -
  a hardware-forensics investigation became a one-line gate flip plus an
  honest record. The evidence run came free with a push we needed anyway.
- The reviewer caught that the cited evidence (step conclusion) is
  maskable under continue-on-error and pulled the job log itself - the
  citation now points at unfakeable evidence.

## What went wrong

- The Record's first draft cited the maskable step conclusion as proof.
  Root cause: I read the conclusion from the API without asking what
  continue-on-error does to it - the exact class of gate-semantics blind
  spot this task existed to fix.

## What to improve next time

- Evidence for "X passed in CI" must come from the artifact that cannot
  lie under the workflow's own settings (the log line / the test binary's
  own output), not from a conclusion field whose semantics the workflow
  modifies.

## Action items

- [x] Ledger: added `maskable-ci-conclusions`.
