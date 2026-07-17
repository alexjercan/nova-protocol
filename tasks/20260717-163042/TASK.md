# Arrival telegraphs: engage_delay on the AI controller + the warning-beat authoring pattern

- STATUS: OPEN
- PRIORITY: 38
- TAGS: spike,v0.7.0,ai,scenario,gameplay

Goal: enemies arrive instead of appearing. An engage_delay/spawn-passive
option on AIControllerConfig: the ship spawns on its patrol/idle routine
and goes hot after N seconds or immediately when fired upon (the leash
machinery's damage-override precedent). Pair with the authored
warning-beat pattern (clock-spaced StoryMessage + marker before the
spawn) and document it as the arrival convention. Mod-facing schema:
failure paths + literal RON syntax in the same change. Spike:
tasks/20260717-155740/SPIKE.md.
