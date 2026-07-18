# Changelog

All notable changes to the Gauntlet Run mod. Versions are the `meta.version`
in `gauntlet.bundle.ron`; the portal keeps every published version.

## 1.2.0

- Re-skin onto the base-game racer/cargo prototypes now that ships are reusable
  prototypes shared by mods and menus.
- Drop the `demo` dependency: depend on `base` alone. v1.0.0's `demo` dep
  silently overrode `reinforced_hull_section` health (200 -> 400) and forced
  players to enable the demo arena. The crash tolerance is now base's honest
  200-health hull, and the mod no longer rides a scenario slated for removal.

## 1.1.0 - Gauntlet Run 2.0

- Rebuilt from a thin four-gate slalom into a real parkour course: six ordered
  gates across three escalating acts (warmup / slalom / hazard).
- Invulnerable asteroids crowding the racing line; an act-3 gravity well to
  sling or avoid.
- Per-act `SetSkybox`; crossing FINISH declares Victory, wrecking your hull
  declares Defeat with a Retry.
- Reference base art via `self://` + `dep://base` after base art moved under
  `assets/base/`.

## 1.0.0

- First PORTAL mod: published to the static mod portal by `nova_portal_gen`,
  not shipped inside the game's `assets/`.
- One scenario, `gauntlet_run` - a playable sequential slalom race and a worked
  example of the data-driven scenario vocabulary (`ScatterObjects`, `Asteroid`,
  ordered `OnEnter` gates, `SetSkybox`, `Outcome`).
