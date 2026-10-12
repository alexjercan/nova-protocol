# Changelog

## 1.7.0

- Add `example_survey`, the fixed-seed world with a survey sphere 64 km out.
  Its `OnEnter` marks the first body in with `ObjectiveMarkerAttach((target_id:
  "$other_id", ..))`, a generated rock or derelict no handler could name, and
  the mark comes back when the body's sector retires and thaws. Needs a base
  game whose marker actions read `$id` / `$other_id`.

## 1.6.0

- Add `example_prospect`, a berth with a trader alongside and an ore rock
  ahead. One objective completes on each hold event: `OnTraded`,
  `OnCreditsChanged`, `OnRepaired`, `OnJettisoned`, `OnPickedUp` and
  `OnMined`. Needs a base game whose vocabulary carries the six.

## 1.5.0

- Add `example_bounds`, a chapter on the base world generator with a fixed
  seed and a 2 km `bounds` sphere. Leaving it raises `OnExit` on the area
  `bounds`, which the scenario answers with a Defeat. Needs a base game whose
  scenarios take a `world` block.

## 1.4.0

- **(breaking)** Add `example_survey_core`, an item this mod defines, and
  start the arena's player ship with two. Needs a base game whose items are
  content; item ids in ship stock are quoted map keys.

## 1.3.0

- Add `example_freighter_hull`, a `Grammar` under a new id: a hull LINE the
  editor's Generate block lists beside the base warship and collapses when it
  is picked. Longer, blunt-nosed, no weapons in the draw, built out of this
  mod's own plated hull.

## 1.2.0

- The example content is authored in METERS, ten times its old numbers, like
  every other mod on this build. Ships in-tree rather than through the portal,
  so the bump is for consistency: nothing downloads this one by version.

## 1.1.0

- Add explicit structural link points to the overlaid and new hull prototypes.
