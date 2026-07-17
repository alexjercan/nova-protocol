# Beat-sheet pass - design record

Task 20260717-163058, spike tasks/20260717-155740/SPIKE.md item 4 - the
content pass that makes the three engine tasks' mechanics actual.

## What shipped

- Two new content_lint arms mechanize the convention: >1 StoryMessage
  per handler warns (one line per beat), StoryMessage beside an Outcome
  warns (the dead-line trap - frozen behind the overlay's pause, dropped
  by the chained teardown). The shipped tree had NINE violations; it now
  lints clean, and the arms hold the line for future content.
- Seven dead lines folded into their Outcome overlay messages (ledger
  ch1/ch2a/ch2b/ch3/ch4 both endings + the example arena): the writing
  survives, the dead delivery does not. Two clauses were trimmed on
  purpose (review R1.1): ch1's "Bring it home slow, Kestrel." and
  BURNED's "I'll even pay." are Okono's comms voice - second/first
  person aimed at the pilot - and read wrong in an impersonal banner.
  SOLD's "Try to look surprised when the yard asks how." works in
  banner voice and was restored.
- ch2a/ch2b's double openings split: the teach line now arrives 8s after
  the contact call via a clock-gated one-shot (seeded flag), while the
  graced arrivals are still inbound.
- Arrival graces: engage_delay Some(8.0) on ch2a/ch2b/ch3's ambush
  ships; Some(5.0) on broadside's corvettes and shakedown's scavenger
  (builders + gen_content, run twice, parity green). The ch4 Auditor
  deliberately stays hot - its entrance is the acked drama.
- The beat-sheet convention + pacing toolbox documented in
  guide-author-scenario.md; CHANGELOG entry.

## Verification

- content_lint CLEAN over the whole tree with the new arms active (the
  acceptance criterion itself; 9 -> 0 violations); the pre-existing
  acked ch4 dual-spawn warning is the only output.
- content_ron_parity 2/2 (builders regenerated, stable across two runs);
  ledger_ch2_encounter 12/12; broadside_assault 11/11; balance_audit
  unchanged (0 errors / 0 warnings / 2 acked - graces do not move
  spawns); nova_scenario --features serde green (incl. the new lint
  tests); workspace --all-targets green; fmt last. Full suite on CI.
