# Implementation notes

Branch: `fix/objective-gap-matches-comms-dwell` (not committed - awaiting review).

## What changed and why

Owner playtest: objectives were still appearing while the conversation that
introduced them was on screen, even after the pacing pass (20260722-092421)
wired `gated_once` across the mainline. Root cause: the gap was a fixed
`BEAT_GAP = 4.0s` while a comms line holds the screen `COMMS_DWELL_SECS = 8.0s`,
so the objective posted four seconds before the line finished.

Owner decision (asked during implementation): apply the rule uniformly - on a
beat transition, complete the previous objective, play the line, and post the
next objective/beacon only once the line has finished.

### 1. Gap now derives from the comms dwell (fixes lifeline/broadside/final_tally)
- `nova_gameplay` comms_panel: `COMMS_DWELL_SECS` and `COMMS_FADE_OUT_SECS` made
  `pub` (exported via the module prelude), the single source of truth.
- `nova_assets` pacing: `BEAT_GAP = (COMMS_DWELL_SECS + COMMS_FADE_OUT_SECS)`
  (8.4s), so the objective posts as the introducing line finishes and fades.
  `nova_assets` already depends on `nova_gameplay`, so the constant is
  referenced directly - the gap and the dwell cannot drift.

These three scenarios already ordered line-then-objective, so the constant
change was the whole fix for them.

### 2. Shakedown restructured (line-then-objective)
Shakedown was the reverse: reaching a beacon completed the previous objective
AND posted the next in the same frame, with the comms "breather" line 4s LATER.
Restructured every navigation beat (2-10):
- The transition now completes the previous objective, plays the beat's line,
  releases immediate teardown (governor, GOTO grant, prev-hint deemphasis), and
  stamps the gate.
- A new `beat_setup(beat, actions)` handler (replacing `breather`) posts the
  objective, spawns the beacon, hands off the marker and lights the next hint a
  beat later, once the line finishes. Latched on `setup_last` (renamed from
  `breather_last`).
- `BREATHER_DELAY` -> `BEAT_DELAY = BEAT_GAP` (also fixes the beat-12 scavenger
  telegraph, which used the old 4.0).

### Hazards handled
- **Crate pickups (beat 3):** crates exist from OnStart, so a pickup during the
  intro line would count against an unposted objective and beat_setup would then
  overwrite the tally. Guarded the three pickups on `setup_last == 3`.
- **Derelict (beat 9):** `[Z]` is granted from the start, so a fast break-away
  could exit the coast ring (beat 9 -> 10) before the delayed setup. The derelict
  spawn AND its marker hand-off stay at the transition (only the objective text
  waits), so beat 10 always has a hulk to paint and no marker strands.
- **Test/emphasis ordering:** kept `beat_setup` interleaved right after each
  transition in beat order, so the marker-hand-off and emphasis-pairing config
  tests (which assert action order) stay green unchanged.

## Verify
- `cargo check` + `cargo fmt --check`: clean.
- `cargo test -p nova_assets --lib scenario::` : 21 pass (shakedown 15/15,
  including the end-to-end beat walk; lifeline/broadside/final_tally).
- `cargo test -p nova_gameplay --lib hud::comms_panel` : 5 pass.
- Updated the two integration walkers to advance the clock past each gate
  (`settle_beat` helper; shared `walk_to_rehearsal` for the fight tests).
- probe: `lifeline` OK, `broadside` OK, `menu_newgame` OK (boots the real
  shakedown_run: reached Playing, 0 invariant violations / 0 errors over 295
  frames).

## Self-reflection
The initial scope ("bump a constant") was wrong: the exploration missed that
shakedown uses the reverse ordering, which only surfaced on reading the actual
beacon handlers. Reading the handlers AND their tests before editing paid off -
the emphasis-order and marker-hand-off tests would have broken silently if the
setup handlers had been appended at the end instead of interleaved. The two
delayed-objective hazards (crate race, derelict soft-lock) were caught by
reasoning through fast-player skip windows, then pinned by the guard + the
transition-spawn choice; worth checking every "world already interactable while
the objective is delayed" case in future pacing work.
