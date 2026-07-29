# RETRO - Scenarios picker: pin pane widths + indent campaign members

- STATUS: DONE
- Rounds: 3 review rounds (9 findings, 8 applied, 1 recorded-not-taken)

## What went well

- **Reproducing in the REAL app first paid for itself immediately.** The
  diagnosis (a flex row shrinks every shrinkable item) was reasoned from the
  code, but the numbers - list pane 141..331 px purely from the selection -
  came from a 60-line example that drives the shipped menu. That rig then
  proved the fix (HELD at 331/481) and is now a permanent CI gate. A unit test
  could not have done any of it: headless, every text node measures zero and
  nothing overflows.
- Fail-first was done properly on both tests by temporarily undoing the fix
  and recording the failure text (`left: 1.0, right: 0.0`;
  `left: Px(0.0), right: Px(24.0)`).
- Copying `menu_newgame` verbatim as the rig's skeleton (`reuse-known-good-stack`)
  meant the harness wiring, the error-handler swap and the probe plugins were
  right the first time.

## What went wrong

- **I eyeballed my own screenshot and did not see the defect in it.** The
  indent shipped as a SHIFT, not an inset: a `list_row` is `width: percent(100)`
  and a margin sits outside that box, so every indented row was 24 px wider
  than its pane and crossed the details divider. It is visible in the capture I
  reviewed and called good; the out-of-context reviewer found it by measuring
  border x-positions in the same PNG. Looking at a render is not the same as
  CHECKING it against a specific expectation - I looked for "is the indent
  there" and got a yes, instead of asking "where does each edge now land".
- **The rig I built as evidence was not wired to fail.** A CHANGED verdict was
  only an `error!` line, and the smoke suite greps for reach-Playing, so the
  exact regression the example exists to catch would have passed CI. Evidence
  that nothing enforces is a demo, not a gate.
- **A completion guard that could never fire.** I copied broadside's
  `guard_run_completion` but not its `.self_completing()`, so the guard read
  `AppExit` in `Last` while the only writer ran later in `Last`. It looked
  right, the comment claimed it worked, and it was dead. Copying a PATTERN
  means copying the whole mechanism, not the part that is visible at the call
  site.
- Two of the three review rounds were spent on the harness, not the fix. The
  fix itself was two lines.

## Lessons

- `check-the-render-against-a-stated-expectation`: when a layout change is
  verified by a screenshot, write down WHAT to look for before opening it (this
  edge lands there, these two borders line up) and check each item. "It looks
  right" found nothing; "where does the row's right border land vs the header's"
  found a shipped overlap in the same image.
- `evidence-rig-must-be-able-to-fail`: a harness added as proof must ASSERT
  under the harness env (panic/non-zero exit) and be wired into a suite that
  runs it, or it proves nothing after the day it was written. Also applies to
  "no measurements" paths - a rig that measured nothing must not read green.
- `copy-the-whole-mechanism-not-the-call-site`: when copying a harness pattern
  from a sibling example, diff the WHOLE wiring (plugin builder flags included),
  and PROVE the guard fires by forcing the failure once. Cutting the runway to
  5 s and watching exit 101 took one minute and turned a dead guard into a live
  one. Kin of `advertised-is-not-wired`.
- `verify-inherited-reds-before-owning-them`: two reds surfaced that were not
  mine (`catalog_matches_disk` from an un-smoke-listed `widget_zoo`;
  `screenshot_nova_os` exiting early). Checking each against a clean master
  checkout first kept one as a two-line merge-integration fix and routed the
  other to its own task (20260729-222131) instead of widening this one.

## Follow-ups

- 20260729-222131: `screenshot_nova_os` exits before completing its cycle
  (inherited smoke red).
- Recorded, not filed: this box intermittently SIGSEGVs at process teardown in
  the nvidia driver after a clean harness exit (2 of 8 runs). Local only; CI
  runs lavapipe.
