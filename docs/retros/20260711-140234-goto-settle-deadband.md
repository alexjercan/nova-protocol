# Retro: GOTO arrival hunt - settle deadband

- TASK: 20260711-140234
- BRANCH: fix/goto-settle-deadband (squash-merged as 1db8390)
- REVIEW ROUNDS: 1 (APPROVE; one MINOR docstring fix in-round)

The cycle where the fix worked but the spike's explanation of WHY was
wrong twice, and the fail-first regression caught both.

## What went well

- **Exact-number A/B oracles falsify no-op fixes instantly.** The
  desired==0 scoping left the terminal spin BIT-FOR-BIT unchanged
  (0.6727556) - not "still failing" but "provably never touched the
  failing phase". A "did it improve" assertion would have shipped a
  placebo knob. Same pattern one step later caught the urgency confound.
- **The regression surfaced a finding bigger than the task**: arrival
  dynamics are wiring-dependent (the shipped Update-copy staleness is
  accidental dither that breaks the re-aim limit cycle; the same-tick
  harness wiring phase-locks it). That warning now gates the sibling
  task 20260711-140241 in its TASK.md, including a legitimate
  falsification-close exit.
- **Guard migration instead of guard weakening**: the two arrival guards
  that failed encoded the old 0.5 contract; both were rewritten to
  derive from settings.settle_deadband with the reasoning in comments,
  keeping their primary assertions intact.

## What went wrong

- **The spike's deadband experiment was confounded**: the global
  attitude_deadband raise moved TWO readers at once (the crumb band and
  the urgency denominator), and the spike attributed the whole effect to
  the band. Root cause: the spike concluded from the A/B without
  grepping every reader of the knob it turned. Cost: two failed fix
  variants in this cycle before the coupling was found.
- **The spike also measured its deadband variants under one wiring only**
  (production), so the wiring-dependence stayed invisible until this
  cycle's regression ran on the harness wiring. A 2x2 (wiring x band)
  would have caught both confounds at spike time for one extra minute of
  runtime.

## What to improve next time

- Before concluding a knob-turning experiment, grep every reader of the
  knob; if it has more than one, either scope the variant to a single
  reader or state the coupling in the conclusion.
- When a diagnostic already runs multiple wirings/variants, cross the
  factors instead of holding one fixed - confounds live in the cells
  you did not run.

## Action items

- [x] Wiring warning + falsification-exit recorded on 20260711-140241
      (TASK.md) and in the spike fix record.
- [x] Ledger: `confounded-knob-experiment` seeded (x1);
      `fail-first-regression-ab` bumped (the falsifying instance);
      `production-faithful-rigs` bumped (the regression's rig had to be
      re-wired to production mid-cycle).
