# Retro: Fix catalog_matches_disk (smoke-list nova_os examples)

- TASK: 20260727-143752
- BRANCH: fix/catalog-smoke-nova-os
- REVIEW ROUNDS: 1 (in-session, trivial diff, APPROVE)

## What went well

- Discovered mid-flow (during the CRT-frame task, while hunting for a shader
  validation harness) and filed as its own prioritized task rather than widening
  that branch - the right handling for unrelated work found in flight.
- The set-equality assertion did its job: reading its printed `left`/`right`
  diff named BOTH offenders, so the fix was complete instead of a half-fix.

## What went wrong

- I filed and initially fixed for ONE example (`screenshot_nova_os`), assuming
  the filed symptom was the whole gap. The first fix left the test still red
  because a SECOND example (`nova_os_rtt_poc`) was also unlisted. Root cause:
  trusting the motivating symptom instead of reading the full assertion diff up
  front. Reading the `--nocapture` set diff immediately would have shown both.

## What to improve next time

- For a set/list-equality failure, read the printed diff and fix the WHOLE
  symmetric difference in one pass; do not assume the one example that motivated
  the task is the only member missing.

## Action items

- No new ledger entry: this is a specific instance of the existing
  `keep-docs-in-sync-with-code` / catalog-drift discipline (a new example must
  join a smoke list or NOT_SMOKED in the same task that adds it) - the two nova_os
  examples were added by 20260726-180807 / 20260726-193233 without that step.
