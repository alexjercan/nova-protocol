# Retro: base campaign polish + extension (close-out)

- TASK: 20260718-152313
- BRANCH: docs/campaign-close-out (landed 831f35d2)
- REVIEW ROUNDS: 1 (APPROVE, two record-precision NITs, no action)

## What went well

- The close-out found NOTHING to fix beyond a two-line CHANGELOG reorder:
  per-cycle discipline (docs-in-task, lint-per-land, probe-per-scenario,
  harness suites landing WITH the content) left zero debt for the sweep.
- The verification record's every claim reproduced under an out-of-context
  reviewer on the first pass - a direct payoff of writing the record FROM
  the commands' output rather than from memory (write-prose-from-the-diff).

## What went wrong

- Nothing material. The probe index carrying the parent sha (R1.1) is a
  small precision lesson: when evidence is generated pre-commit on a
  docs-only branch, say so in the record instead of leaving a pedant trap.

## What to improve next time

- Run evidence-generating commands AFTER the final content commit when
  cheap, or note the sha delta explicitly in the record.

## Action items

- none; the seven playtest questions batch to the flow Finish gate.
