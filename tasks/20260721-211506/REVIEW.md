# Review: Shakedown pacing pass (20260721-211506)

## Round 1 (out-of-context reviewer)

Reviewer examined `git diff master...HEAD` on `content/shakedown-pacing`,
focused on soft-lock / rush bugs in the scripted state machine and the shared
`beat_gate` variable.

### Findings: NONE

Correctness checks, all PASS:

- **Opening soft-lock:** SAFE. Beacon 1 spawns only at the `open_step == 5`
  hand-off; the beat 1->2 handler is gated on `player_enters(beacon_1)`, which
  cannot fire before beacon 1 exists in production. Opening lines cannot
  double-fire or stall on a clock jump (single `open_step` counter, linearly
  advanced).
- **Breather mechanism:** SOUND. `breather_last` (single monotonic var) blocks
  double-fire; a breather skipped when the player blitzes a beat within 4s is
  acceptable, not a soft-lock (beats stay position-gated). The shared
  `beat_gate` re-stamp is safe because beats are monotonic.
- **Expression construction:** CORRECT. `past_gate` builds
  `GreaterThan(scenario_elapsed, Add(beat_gate, delay))` matching the existing
  `gt_num`/`add_one` idioms and lifeline's relative-clock usage.
- **Lazy beacon 1:** VERIFIED. Removed from OnStart; no other code references
  it at start; marker-handoff and geometry tests still hold.
- **Tests:** the walk test correctly pumps the clock (`set_clock`/
  `finish_opening`); the new pacing pin and the updated marker test pin the
  deferral, the voice, and the gate stamps; none pass only by harness artifact.
- **Generated RON:** matches the builder (15 StoryMessage: 11 Halloran + 4
  "You"), not hand-edited.
- **Docs:** CHANGELOG, tutorial.html, and the beat-sheet dev-guide note claim
  nothing the diff does not do.
- **Quality:** no dead code; one-gesture-per-beat preserved; the objective
  simplification kept every key cue ([Alt]/[CTRL]/[G]/[O]/[Z]/[RMB]/[LMB]) in
  the objective while moving flavor to comms; the combat exam stays tight.

## Verdict: APPROVE

The `manual:` DoD item (owner replays the Shakedown and confirms the rush is
gone / the opening reads well) batches for the Finish checkpoint.
