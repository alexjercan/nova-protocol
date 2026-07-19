# Retro: nova_probe rename + module split + run-metadata schema

- TASK: 20260719-112231
- BRANCH: refactor/nova-probe-rename (squash-landed as 03828732)
- REVIEW ROUNDS: 1 (APPROVE; one MINOR fixed in-round)

## What went well

- Steps were written with the verifying citations EMBEDDED (bevy
  settings.rs:197 for the main-world adapter resource, perf-web.sh:76 for
  the log-line scrape contract), each verified BEFORE the step was written.
  Implementation became mechanical and the review could re-check claims by
  following the citations instead of re-deriving from scratch.
- Sweeping both the symbol form (`nova_perf`) and the prose form
  (`nova perf`) surfaced the perf-web.sh scraper before the rename could
  break it - the decision to keep the log prefix + env vars stable fell out
  of the sweep, not out of luck.
- The live-vs-historical split (update queued specs and live docs, leave
  closed task records) kept the rename honest without rewriting history.
- The review re-derived the schema column contract mechanically (header vs
  writer vs reader extracted from source and cross-checked) - the one class
  of bug positional CSV parsing invites.

## What went wrong

- The wiki Performance section gained the rename but initially not the new
  user-visible metadata feature (review R1.1) - the doc surface was updated
  for the OLD content, not re-read against the NEW diff. Same shape as
  keep-docs-in-sync-with-code: a ticked docs step is not proof.
- Minor: the first column-order re-derivation script grabbed the wrong
  bracket (regex matched the type signature, not the body) and needed a
  second pass - extraction scripts want an assert on what they extracted
  (the empty-list assert caught it immediately, which is the pattern
  working).

## What to improve next time

- When a diff ADDS a feature to a surface whose docs were just written,
  re-read the doc section asking "does it claim everything the diff now
  does?" - the inverse of prose-from-diff-not-intent's overclaim check.

## Action items

- [x] Lessons: bumped verify-first-plan-steps (embedding citations in Steps
      confirmed valuable) and keep-docs-in-sync-with-code (R1.1 occurrence).
