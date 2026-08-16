# Arena: more weapon types, ship rosters, better cameras

- STATUS: CLOSED
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

## Closure

Landed 2026-08-16, lane arena-weapons (opus). All three parts:
- Weapons: pdc_pierce joins the shared draw with the old mount weight SPLIT
  (kinetic 1.0 / pierce 0.6) so batteries mix rather than double;
  load_lances re-sources every second mirrored bay pair to
  lance_torpedo_section (phase = seed % 2), so any hull with >= 2 bay pairs
  fields both flavours. Scoreboard counts by flavour off the projectiles.
  Evidence: default bout fired 334 kinetic + 138 pierce rounds and
  8 serpent + 8 lance from AMBER alone.
- Rosters: repeatable --ship TEAM[:STYLE[:SEED]]; teams spawn as lines
  preserving the ~163u opening range; per-team aggregation; R rerolls the
  same roster on fresh seeds, pinned hulls keep theirs. 2v3 mixed-style run
  verified live.
- Cameras per the owner's revised mapping: Q = idle-orbit auto-frame (only
  orbiting mode), E = tactical overview, 1-4 = follow roster slots with
  dead-slot fallback. Q/E collide with nothing (rig verticals are
  Space/ShiftLeft).

Known, out of lane: bouts are over in ~15 s at 750 blast damage - the
lethality data point again; avian3d logs a massless-body warning when a
gutted root despawns (pre-existing).
