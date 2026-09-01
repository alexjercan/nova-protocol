# Teach the AI to fly a railgun lance run

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

Split out of `20260824-125947` on 2026-09-01, at owner direction: the
railgun ships with AI use ALLOWED, but the AI aims it with the crude
gate the section landed with, not with a behavior built for it.

## What shipped

`railgun_ai_input` commits a shot when the hull axis already points
inside a cone of the AI's chosen target and the target sits inside the
slug's reach. The AI does not FLY to make that true - it fires when its
orbit happens to sweep the nose across a target. A raider therefore
lands the occasional lance hit and never sets one up.

## What this task wants

A lance run: break the standoff orbit, roll onto the target's line,
commit through the charge, fire, and peel off. The verb the player has
to answer, and the reason a spinal gun is frightening on an enemy hull.

- The charge window is the tell. The run must be readable from the
  cockpit before the slug arrives, not after.
- Decide what a raider does when the run is spoiled mid-charge: hold
  the shell, or dump it downrange.
- Whether the AI weighs the shot against its own recoil (a lance run
  costs the attitude budget the orbit needs back).
