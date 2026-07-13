# Retro: Bigger edge indicators with target info

- TASK: 20260711-174840
- BRANCH: feature/edge-indicator-info
- REVIEW ROUNDS: 2 (round 1 APPROVE with one MINOR, fixed and verified)

## What went well

- Fast feedback loop: playtest remark to landed feature in one short
  cycle, because the edge-indicator module already had the reconcile,
  markers and tests to hang a label on.
- The review caught a real scheduling subtlety (label mirrored the
  widget's PostUpdate visibility from Update, one frame late) by asking
  the two-clocks question the LESSONS ledger keeps drilling: WHERE is the
  data I read written, and does my slot see this frame's value?

## What went wrong

- The one-frame mirror lag was written in the first place - root cause:
  defaulted to the Update/NovaHudSystems slot out of habit instead of
  checking which schedule produces the input (the widget's own header
  comment states it). Caught in-review, cost one round.

## What to improve next time

- Any system whose input is written in PostUpdate (widget visibility,
  chase-camera-dependent state) must be slotted relative to its producer,
  not dropped into the Update HUD set by default - the two-clocks family
  keeps generalizing beyond transforms.

## Action items

- [x] Bumped the `two-clocks` family note in LESSONS.md with the
  non-transform variant (consumer slot vs producer schedule).
