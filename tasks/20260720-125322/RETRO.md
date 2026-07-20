# Retro: group --baseline (per-example baseline root)

- TASK: 20260720-125322
- BRANCH: fix/probe-group-baseline
- REVIEW ROUNDS: 1 (APPROVE, one NIT - accepted)

## What went well

- **A user observation surfaced a real gap.** "Shouldn't `--baseline` just try
  its best across a group?" turned out to be a hard-REJECTION, not a partial
  behavior - the tool errored outright. The fix was a clean, small feature.
- **Verified the routing before assuming the change was scoped.** The load-
  bearing question was "does a SINGLE example also route through `run_many`?" -
  if it did, treating `--baseline` as a root there would have broken the
  single-example meaning. Checked `run_spec`: `!resolved.multi -> run(&base)`
  (single) vs `run_many` (group). The two documented meanings of `--baseline`
  fell straight out of the existing dispatch split, so the change was genuinely
  localized to the group path.
- **One e2e sequence proved both paths.** Capture a baseline group, delete one
  example's `frametime.csv`, re-run with `--baseline` -> the kept example
  compares (PASS +1.4%), the deleted one SKIPPED with its diagnostic. Designing
  the fixture to hold one present + one missing covered compare AND skip at once.
- **Skip-not-error fell out of the existing validation.** Setting `opts.baseline`
  only when the csv exists means `run()`'s pre-run baseline validation never
  trips on a group miss - no special-casing needed.

## What went wrong

- Nothing material. The "baseline root matched NONE" warn is code-verified but
  not e2e'd (the e2e hit the partial-match case); logged as NIT R1.1, accepted.

## What to improve next time

- When extending a helper on a SHARED code path (here `run_many`), confirm which
  inputs route through it vs the sibling path (`run`) before assuming the change
  is contained - the single-vs-group dispatch split was the whole safety of this
  change, and it was worth checking rather than assuming.

## Action items

- No new ledger slug: the routing-check observation is covered by the existing
  "verify what routes through the changed code" family; recorded here as a
  cycle note.
- Possible follow-up (NIT R1.1): e2e the wrong-root "matched none" warn if a
  future change touches that path.
