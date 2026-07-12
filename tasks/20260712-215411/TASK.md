# Asteroid-clump travel waypoints (long-range GOTO to a cluster)

- STATUS: OPEN
- PRIORITY: 10
- TAGS: v0.6.0, targeting, navigation, spike

## Goal

At really long ranges, individual asteroids are sub-pixel; let the player LOCK a
CLUMP of asteroids as a single travel waypoint and GOTO it, instead of picking
one rock in the field.

Direction (see spike, Part B): synthesize a "clump" target from spatially
clustered asteroids (a centroid + aggregate signature) that is lockable /
GOTO-able at long range (option B1). If the clustering / synthetic-entity
lifecycle proves heavy, fall back to authored region-waypoint markers per
cluster in the scenario data (option B2) as the cheap first cut.

## Notes

- Spike: docs/spikes/20260712-215256-combat-travel-lock-separation.md (Part B).
- Future / lowest priority; do NOT block the near-term cyclable-nav-bodies work
  (20260712-215402) or the mode toggle (20260712-215406) on this.
- Open: clump form/dissolve hysteresis as rocks move/die; GOTO to a moving
  centroid; clumps are almost certainly travel-only (not combat targets).
- Depends on / pairs with: the combat/travel separation (20260712-215406) -
  clumps are a travel-mode concept.
