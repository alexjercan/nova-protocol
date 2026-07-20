# Retro: window-sized fps deadline + example AppExit propagation

- TASK: 20260720-115935
- BRANCH: fix/probe-deadline-window
- REVIEW ROUNDS: 1 (APPROVE, one NIT - accepted tradeoff, not actioned)

## What went well

- **The user's question reframed the fix.** "Why is 120s hardcoded, can't we
  just increase the timeout?" forced the right model: it is not hardcoded (a bcs
  env default), and the answer is not a bigger flat number but sizing the
  deadline to the requested WORK. Explaining the two-timer model (in-process
  bcs deadline vs probe's supervisor kill) and why a flat bump is wrong BEFORE
  coding produced the clean design. A good reminder that a user's "why can't
  we just..." is often pointing at the real design question.
- **Diagnosis found a second latent bug.** Tracing the deadline turned up that
  every example's `main()` did `app.run();` and discarded the `AppExit` - so the
  completion protocol's whole "error-exit naming laggards" failure signal was
  defeated (the process exited 0). Fixed both in one task; the field failure was
  really two defects wearing one symptom.
- **One cheap e2e proved both fixes.** `BCS_HARNESS_DEADLINE=1 probe run
  scenario --fps` forces an immediate expiry, which at once proves Fix A's
  operator-override (1s beat the sized 195s) AND Fix B's propagation
  (`run_completed: exit "Error(1)"`, `process_exit` FAIL). Designing the test to
  hit both levers at once beat two separate runs.

## What went wrong

- **My verification script parsed the wrong path.** I read `<out>/scenario/
  checks.json`, but a single-run `--out <dir>` writes the run dir DIRECTLY (no
  `<name>/` subdir - that layout is only for multi-run sweeps). The background
  command exited 1 on the python KeyError even though both probe runs succeeded;
  I had to re-read stderr to see the verdicts. Root cause: assumed the multi-run
  dir layout for a single-example run.

## What to improve next time

- When scripting a check that parses probe output, confirm the run-dir layout
  first (single `--out` = the run dir itself; multi-run = `<out>/<name>/`), or
  read the verdict probe already prints to stderr instead of re-opening
  checks.json.

## Action items

- [x] LESSONS.md: add `deadline-scales-with-the-work` (a hang-detector timeout
  sized to the requested work, not a flat constant).
- Fix B is another instance of the `signal-guarantee` lesson (20260720-014142):
  a failure SIGNAL is worthless if a layer in between silently drops it - here
  `app.run();` swallowed `AppExit::error`. Noted, not a new slug.
- Possible follow-up (NIT R1.1): a progress-aware deadline (reset while frames
  still accrue) would catch hangs fast AND allow slow captures - a bcs-side
  change, not filed unless it recurs.
