# Notes: base chain voice pass

Design record for task 20260721-160929 (spike tasks/20260721-155249/SPIKE.md,
"Polish pass"). What shipped and the conventions the ch3 tasks inherit.

## The cast (crates/nova_assets/src/scenario/cast.rs)

One constant per speaker, single-line rename, placeholders pending the
owner's nod:

- `Capt. Halloran` - the Ceres Queen's captain; the friendly recurring voice.
- `Rust Tally` - the gang gunship's channel (the vessel speaks, keeping the
  gang's BOSS voice fresh for chapter three's Tallyman).
- `Belt Relay` - dispatch; speaks when no character can (e.g. after
  Halloran's ship dies).

## Voice rules applied (and to reuse in ch3)

- Objectives are imperative goals; comms lines carry all story.
  ("Find the hauler Ceres Queen." + Halloran's distress line, not a
  three-sentence objective.)
- One StoryMessage per beat; mid-fight beats gate on the act machine so a
  line fires exactly once (first-corvette-down: two handlers, each gated on
  the OTHER kill flag still being 0 - mutually exclusive by construction).
- The banner carries the closing line (lint forbids StoryMessage beside
  Outcome). This is WHY the shakedown epilogue keeps its hook in the
  Victory banner rather than gaining a comms line: the hook fires in the
  same handler as the Outcome. The voice pickup instead happens at
  Broadside's OnStart - Halloran's spoken distress IS the call the
  shakedown banner promised, which reads as continuity, not a gap.
- Conditional flavor via a scenario-local flag: `hauler_lost` (0/1), raised
  by the soft-fail beat, read by mutually-exclusive gated Victory handlers
  whose banner variants acknowledge the fate. Variables are
  scenario-scoped, so EACH part tracks its own hauler across the
  checkpoint restage (a known, accepted continuity seam of the chained
  arena - same as the hauler respawning at all).

## The soft-fail beat changed shape

Before: hauler death pushed a `hauler_lost` HUD OBJECTIVE ("The Ceres Queen
is gone. Make it cost them."). After: it raises the flag, unmarks, and
Belt Relay speaks the line. Rationale: it was never a goal, it was voice.
The test `hauler_death_on_a_live_act_pushes_the_soft_fail_beat` now pins
flag + comms + the objective's ABSENCE; `victory_banner_reflects_the_haulers_fate`
drives both branches of both parts through the act machine.
