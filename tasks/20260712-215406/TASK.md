# Separate combat vs travel lock modes (mode toggle)

- STATUS: OPEN
- PRIORITY: 20
- TAGS: v0.6.0, targeting, navigation, spike

## Goal

The one lock resource (`SpaceshipPlayerTargetLock`) does double duty: a sticky
COMBAT target (turret aim feed) and an aim-driven TRAVEL/GOTO designator. They
want different feels and different eligible pools, and cramming both into one
cycle risks clutter (see task 20260712-215402). Separate them so combat and
travel each get a clean pool and feel.

Direction (see spike, option C1 - the recommended sketch): a Combat <-> Travel
MODE toggle on one lock. Combat mode: ships + committed torpedoes, sticky,
CTRL+scroll cycle (today's behaviour). Travel mode: wells, beacons, asteroids,
clumps - aim/cycle, non-sticky, GOTO-oriented. The mode decides the eligible
pool and the stickiness; turrets consume the lock only in combat mode, GOTO in
travel mode. Least new state, no second reticle. Reconsider two separate lock
resources (C2) only if the single-lock toggle feels too modal.

## Notes

- Spike: docs/spikes/20260712-215256-combat-travel-lock-separation.md (Part C).
- Root fix that the near-term cyclable-nav-bodies stopgap (20260712-215402)
  subsumes; the user steered "keep combat-only for now", so this is parked at
  v0.6.0 until picked up.
- Consumers to re-route by mode: turret aim feed + GOTO (`AutopilotAction::Goto`,
  input/player.rs:848) + torpedo designation, all reading
  `SpaceshipPlayerTargetLock` today.
- Open: what key/gesture toggles the mode; how the HUD shows the current mode;
  whether the focus dwell / inset apply in travel mode.
