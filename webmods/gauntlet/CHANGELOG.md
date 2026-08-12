# Changelog

All notable changes to the Gauntlet Run mod. Versions are the `meta.version`
in `gauntlet.bundle.ron`; the portal keeps every published version.

## 1.7.0

- Declares a typed `Scenario(Elapsed)` watch for the run timer. Required by the
  read-only scenario query format; the implicit `scenario_elapsed` engine
  variable no longer exists.

## 1.6.0

- The mod ships its OWN picker thumbnail instead of borrowing base's asteroid
  texture, so the Scenarios picker shows the course rather than a rock every
  other scenario also showed. Placeholder art for now: real art overwrites the
  same path.

## 1.5.0

- The act-3 gravity well is authored by MASS, matching the base game's new
  gravity rules: one number now sets both how hard the well pulls and how far
  it reaches. The well pulls as hard as it did, over a shorter range - so the
  sling is a tighter, later commitment than it was, and the racing line past it
  is more forgiving. Required: the old `surface_gravity` field no longer
  exists, and a copy still authoring it would race past a dead well.

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
