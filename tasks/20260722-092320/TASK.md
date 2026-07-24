# Backlog: critical-damage state - a ship is combat-dead when weapons+thrusters are destroyed

- STATUS: OPEN
- PRIORITY: 40
- TAGS: v0.9.0,gameplay,feature

## Story

Playtest verdict (owner, 2026-07-22): killing a ship today means grinding
every section's health to zero, which is tedious and sometimes leaves you
stuck (see the sibling kill-condition backlog item 20260722-092326). The
owner wants a real notion of a ship being "out of the fight": once its
weapons AND thrusters are gone it can neither shoot nor maneuver, so it
should count as combat-dead / mission-neutralized even with hull sections
still intact. This applies to the PLAYER too (a new lose/critical condition
when you can no longer fight), not just AI ships.

This is a NEW FEATURE, deliberately deferred to the backlog. Filed now so it
is not lost; not to be implemented in the current pacing/ship-behavior goal
(umbrella 20260722-092316).

## Steps

- [ ] Spike the critical-damage model: what counts as "can no longer fight"
      (all turret/torpedo sections destroyed AND all thruster sections
      destroyed? controller destroyed?). Decide the exact predicate per
      faction and whether it differs for the player.
- [ ] Decide the consequence: AI ship -> despawn / drift-as-wreck / surrender
      state and stops being a target-to-finish; player -> a new critical /
      defeat outcome path.
- [ ] Integrate with the outcome system without regressing
      `outcome-is-last-write-wins-close-the-act` (every terminal path sets a
      terminal act).
- [ ] Harness coverage: a scenario walk that destroys only weapons+thrusters
      and asserts the ship is neutralized without zeroing hull health.

## Definition of Done

- A ship with no working weapons and no working thrusters is treated as
  neutralized (AI: removed as an active combatant; player: critical/defeat
  path), hull health notwithstanding.
- Deferred: pull from backlog into a real vX.Y.Z tag before scheduling.

## Notes

- Related: kill-condition rethink 20260722-092326 (the immediate annoyance),
  integrity/sections system in nova_gameplay.

## Merged in the kill-condition rethink (2026-07-24, v0.9.0 planning)

Absorbed sibling 20260722-092326 (now CLOSED): "destroying a ship should not
require zeroing every section's health." Same question, inseparable, so this
task now owns BOTH angles - the immediate annoyance (a mostly-wrecked ship that
will not die) and the critical-damage model. The critical-damage predicate
(no working weapons + no working thrusters => combat-dead) is the likely
mechanism for both; the beaten-ship threshold gets designed here too. Tagged
v0.9.0 as the Goal-B STRETCH: cut first if the cockpit-HUD work (Goal C) runs
long. Full DoD/steps to be defined in the v0.9.0 planning pass.
