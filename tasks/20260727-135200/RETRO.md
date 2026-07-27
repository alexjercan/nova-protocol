# Retro: NOVA OS block caret over the first completion letter

- TASK: 20260727-135200
- BRANCH: feature/nova-os-ghost-caret
- REVIEW ROUNDS: 2 (R1 REQUEST_CHANGES -> R2 APPROVE)

## What went well

- The out-of-context reviewer earned its keep: it caught a MAJOR drift bug
  (caret positioned by `chars * 0.6em`, where 0.6em is the block width not the
  glyph advance) that my own tests could not have caught because they reused the
  same formula. This is exactly the structural blind spot the out-of-context
  default exists for.
- The fix (measure `ComputedNode` width like the PoC) is strictly better than
  the original - font-agnostic and drift-free - and turned out cleaner (a small
  dedicated system) than the char-count math it replaced.
- Round 2 was cheap and decisive: a focused re-review confirmed the mechanism
  change and the strengthened test, plus one cosmetic test-clarity nit.

## What went wrong

- The first implementation trusted a pre-existing constant doc comment
  ("`font_size * 0.6` is exactly one cell") without verifying it against the
  actual font. 0.6em was the decorative caret WIDTH; the real glyph advance is
  narrower, so the caret would have drifted ~a full cell by ~6 typed chars.
  Root cause: inherited an unverified assumption AND wrote a test that asserted
  against the same assumed formula, so the test was tautological on exactly the
  thing that was wrong.

## What to improve next time

- When a layout value depends on font/render metrics, MEASURE it (ComputedNode)
  rather than multiply by an assumed em-fraction. And make the test assert
  against an INDEPENDENT measurement (a stamped ComputedNode, a real layout
  pass), never re-derive the production formula in the test.

## Action items

- [x] Added ledger lesson `test-must-not-reuse-the-formula-under-test` (x1).
- [x] Fixed the misleading `NOVA_OS_CARET_WIDTH_FRACTION` doc comment so the
  next reader is not led into the same assumption.
- [x] Corrected the DoD filter off the bogus template `drawer`.
