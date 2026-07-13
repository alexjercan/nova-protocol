# Weapons safety + RMB manual gunnery + consumer routing (travel->GOTO, combat->guns)

- STATUS: OPEN
- PRIORITY: 54
- TAGS: v0.5.0, targeting, input, combat, spike

## Goal

The combat half of the deliberate-radar design (spike 20260713-082207):

- **Weapons safety** ("weapons raised") as ship state: safety OFF while
  (RMB held) OR (a CombatLock exists); ON otherwise. With safety ON the
  triggers do nothing (turrets AND torpedo launch - the torpedo gate is a
  chosen default, playtest flag). Gating lands in the input observers
  (player.rs:1054/:1128 area), not bindings data. Route off the RAISED flag
  from 20260713-082324, never the camera enum.
- **RMB manual gunnery**: while RMB is held, turrets follow the look ray
  (manual aim - absorbs the retired CTRL free-aim role, and manual WINS over
  an existing combat lock while held, preserving the side-shot); with RMB
  released and a CombatLock held, turrets auto-track the lock via the existing
  three-tier feed.
- **Consumer routing**: G/GOTO + nav hints read the TravelLock; clearing the
  travel lock (tap in normal mode) DISENGAGES an engaged GOTO; guns, torpedo
  commit, focus dwell, component fine-lock and the target inset read the
  CombatLock (inset/focus re-keying starts in 20260713-082330; this task
  finishes the verb/weapon side). Torpedoes with no combat lock stay dumb-fire
  (once safety is off).
- **AI parity**: AI controllers drive the same components (locks, raised,
  safety) through their own logic; a computer without the lock capability
  cannot lock (scenario difficulty flavor).
- **HUD**: safety indicator (weapons-hot vs safety-on) + keybind hint rows
  (radar, clear, raise).

## Notes

- Spike: docs/spikes/20260713-082207-deliberate-radar-locking.md.
- Depends on: 20260713-082330 (slots + radar), which depends on -082324.
- Safety truth table and gesture flows are in the spike; do not re-derive.
- The dead family's round-3 findings on fire gating and raised-state routing
  (spike 20260712-222610 round 3, deltas 2/4) were adversarially reviewed and
  carry over.
