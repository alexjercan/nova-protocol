# Arena: more weapon types, ship rosters, better cameras

- STATUS: IN_PROGRESS
- PRIORITY: 52
- TAGS: v0.11.0,example,combat,wfc

## Goal

Owner-approved arena upgrades, one lane.

1. WEAPON VARIETY: the wfc draw list hand-picks kinetic PDCs and the
   serpent-default bay only. Add pdc_pierce_turret_section to the draw, and
   give the arena Lance capability (per-bay type modification, deterministic
   per seed/config) so fights show all four weapon flavours. This also stages
   the owner's decoy doctrine (serpents drain PD, a lance finishes).
2. SHIP ROSTERS: a repeatable `--ship` CLI arg carrying style and team (lane
   designs the syntax), so the arena spawns MULTIPLE ships per side instead
   of exactly two. Draft, scoreboard and the fight-happens autopilot
   predicate must scale to teams.
3. CAMERAS remapped: 1 = idle-orbit auto-frame (the only mode that orbits),
   2 = high-level tactical overview, 3/4/5 = follow one ship each.

## Done when

- a driven multi-ship run logs shots and damage per team with pierce fire
  and at least one lance launch observed
- `--ship` rosters spawn styled, teamed ships without restart
- the camera row behaves as mapped, orbit still exclusive to camera 1
