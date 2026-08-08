# Backlog: rethink kill condition - destroying a ship should not require zeroing all section health

- STATUS: CLOSED
- PRIORITY: 0
- TAGS: backlog

## Closed (2026-07-24, merged into 20260722-092320)

Closed during v0.9.0 planning: this and the critical-damage feature
20260722-092320 are the same "when is a ship dead" question and are
architecturally inseparable (see `inseparable-seeded-tasks-remerge` in
LESSONS). The kill-condition rethink is now the first concern of 092320, which
is tagged v0.9.0 as the Goal-B stretch. No work lost - the design lives there.

## Story

Playtest verdict (owner, 2026-07-22): to destroy a ship you currently have to
grind every section's health to zero. That is annoying, and combined with
enemy AI sometimes crashing/getting stuck it can leave the player unable to
finish an objective (a mostly-wrecked ship that will not die). The owner wants
the kill/neutralize condition rethought so a ship that is clearly beaten is
counted as beaten.

This is closely related to the critical-damage feature 20260722-092320 - the
critical-damage predicate (no weapons + no thrusters => combat-dead) is likely
the mechanism that also resolves this annoyance. Kept as a separate backlog
item because "when is a ship dead" (this) and "what critical damage does to
the player + AI" (092320) may be scoped/scheduled separately.

Deliberately deferred to the backlog; not part of the current pacing/
ship-behavior goal (umbrella 20260722-092316).

## Steps

- Confirm today's kill condition in the integrity/sections code
      (nova_gameplay) - what exactly must reach zero for a ship to be
      destroyed.
- Design a beaten-ship condition that does not require every section at
      zero (likely: reuse the critical-damage predicate from 092320, or a
      total-integrity-fraction threshold).
- Harness coverage pinning the new condition.

## Definition of Done

- A ship is destroyed/neutralized under a condition that does not require
  every section's health at zero.
- Deferred: pull from backlog into a real vX.Y.Z tag before scheduling; may
  be merged into 20260722-092320 if they prove inseparable.

## Notes

- Related: critical-damage feature 20260722-092320.
