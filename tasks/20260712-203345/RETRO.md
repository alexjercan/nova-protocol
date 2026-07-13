# Retro: InsetZoomable - ship/torpedo/asteroid inset scope

- TASK: 20260712-203345
- BRANCH: feature/inset-zoomable-scope (landed 4aa02d8)
- REVIEW ROUNDS: 2 (round 1 APPROVE + one NIT; round 2 APPROVE)

Process notes only; details in TASK.md and tasks/20260710-104421/NOTES.md.

## What went well

- Observer-over-spawn-site. The plan assumed a single "ship spawn path" to
  edit; ships actually acquire `SpaceshipRootMarker` in several places
  (integrity glue, scenario). Authoring the flag via `On<Add,
  SpaceshipRootMarker>` / `On<Add, TorpedoTargetChosen>` observers (the repo's
  existing `on_add_entity_with` pattern) covered every ship with two lines and
  no spawn-site hunt. Only the cross-crate case (asteroids in nova_scenario)
  needed a bundle line, because nova_gameplay cannot observe nova_scenario's
  marker.
- Reuse paid off. The framing generalization needed exactly what
  `screen_indicator::target_world_aabb` already did (union non-sensor collider
  AABBs over the subtree, returning None for sensor-only bodies like beacons).
  Making it `pub(crate)` beat writing a second AABB walker.
- Independent verification closed the load-bearing claim cheaply: grepping every
  `InsetZoomable` authoring site + confirming the beacon bundle carries only
  `BeaconMarker` proved beacons cannot get the flag by construction, which the
  gate relies on.
- Checked the "framing regression" worry with arithmetic instead of vibes: both
  the old section-spread radius and the new AABB-corner radius clamp to
  `INSET_MIN_DISTANCE` for a small ship, so there is no framing change - avoided
  a needless constant retune.

## What went wrong

- Minor: the plan's Steps named "the ship spawn path in sections/" as if it were
  one site; it was not. No cost here (the observer approach sidestepped it), but
  it is the same class as writing a plan from a model of the code rather than
  the code.

## What to improve next time

- To attach a derived/flag component to every entity of a kind, default to an
  `On<Add, KindMarker>` observer rather than editing spawn sites - it is
  spawn-site-agnostic and matches the codebase. Only fall back to bundle edits
  across a crate boundary the observer cannot reach.

## Action items

- [x] Lessons ledger updated (`observer-over-spawn-site`).
