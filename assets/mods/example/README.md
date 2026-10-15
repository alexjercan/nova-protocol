# Example mod

The one **copy-me tutorial mod**: a single, self-contained, playable mod that
demonstrates a little of *everything* a mod can do. Copy this whole folder to
start your own mod.

A mod is the same folder-bundle shape as the base game: a directory with a
`*.bundle.ron` manifest listing its `*.content.ron` files (plus, optionally, the
binary assets it ships).

## What it demonstrates

- **Section overlay** - overrides the base `reinforced_hull_section` by id
  (same id = last-wins replace): more health, a renamed label.
- **New section** - adds `example_plated_hull_section` (a fresh id = add),
  bolted onto the arena ship so you can see it in play.
- **A playable scenario** - `example_arena`: spawns a player ship from base
  prototypes plus this mod's own sections, two destructible targets, an
  objective, and a win check. Ends in a Victory or Defeat `Outcome`, with a
  `NarrativeCue` comms beat at the open and the clear.
- **Timed beats on the scenario clock** - the arena gates a comms nudge
  (25s) and a bonus drifting target (45s) on the engine's reserved
  `scenario_elapsed` variable: threshold filter + a seeded one-shot flag,
  the copyable pattern for breathers, late waves and slow-play nudges.
- **Mod-shipped binary art** - ships its own skybox and asteroid texture under
  `textures/`, declared in the manifest's `resources`, referenced with the
  `self://` scheme so they resolve against this mod's own folder.
- **A menu backdrop** - `example_menu` declares `role: Backdrop`, so mods can
  ship their own main-menu ambience, not just playable levels. It reports its
  act finished with `BackdropDone` after a 150 s timer, and the menu picks
  what plays next.
- **A bounded generated world** - `example_bounds` runs the base world
  generator on a fixed seed (`world: Generated((seed: Some(1701), bounds:
  Some(..)))`). The loader spawns the `bounds` sphere as the scenario area
  `bounds`; the scenario's own `OnExit` handler on it counts the exit and
  ends the run in Defeat. The world keeps streaming past the edge.
- **A hull line** - `example_freighter_hull` is a `Grammar` under a new id: a
  longer, blunt-nosed hauler drawing this mod's own plate and no weapons. The
  editor's Generate block lists it under HULL LINE beside the base warship and
  collapses whichever is picked.
- **A new item** - `example_survey_core` adds a fresh id to the item catalog;
  the arena's player ship starts with two of it in its hold.
- **A hold tour** - `example_prospect` berths the shipped mining tug beside
  a trader with an ore rock ahead. Six `once` handlers complete one objective
  each on `OnTraded`, `OnCreditsChanged`, `OnRepaired`, `OnJettisoned`,
  `OnPickedUp` and `OnMined`, and the sixth ends the run in Victory.
- **A survey contact** - `example_survey` runs the fixed-seed world unbounded
  with a 1 km survey sphere in the -X face of the window. The first body in
  is one the generator placed, whose id no handler can write; the `OnEnter`
  marks it with `ObjectiveMarkerAttach((target_id: "$other_id", ..))`, bound
  from the event payload, and the mark follows the body through its sector's
  retire and thaw.
- **A station** - `example_outpost` is a `Station`: a hull that never flies
  (a collar, a bridge, four of this mod's plates, no thruster), the share of
  its hull pool below which it takes no more damage
  (`integrity: (collapse_threshold: Some(0.5))`, required on a station), the
  services it offers (`Trade`, `Storage`), its storage cap, the stock its hold
  opens with and the credits it trades with, all under `design`. A scenario
  spawns it with `kind: Station((design: Prototype("example_outpost"), ..))`.
- **A start role** - `example_surveyor` is a `StartRole`: a row in the New
  Game wizard that starts the player in the base `light_freighter` with a kit
  of credits, this mod's survey cores and rations. The wizard binds the
  weapon keys from the hull's sections, so the role binds nothing.

## Layout

```
example/
  example.bundle.ron     # manifest: content + resources + meta
  example.content.ron    # section overlay + new section + arena + menu backdrop
                         #   + hull line + item + station
  textures/
    nebula.png           # self:// skybox (stacked 1x6 cube faces)
    nebula.png.meta      # RowCount cube reinterpret sidecar (rides along, not listed)
    rock.png             # self:// asteroid texture
```

The textures are PLACEHOLDERS (debug-colored).

## How to enable

This mod is listed in `assets/mods.catalog.ron` (the installed-mods catalog), so
it ships **installed but disabled by default**. Enable it from the main-menu
**Mods** section - the section palette then shows the buffed "Reinforced Hull
Section (Example Mod)" and the "Plated Hull Section (Example Mod)", and
`example_arena` becomes available in the Scenarios picker. Toggle it back off to
return to the pristine base game.

## How mod-owned art works (`self://`)

- `resources` in the manifest lists the binary files the bundle ships
  (bundle-dir-relative, the same base as `content` and `meta.icon`). Sidecar
  `.meta` files ship automatically and are NOT listed.
- Content references a resource with `self://<path>` (e.g.
  `cubemap: "self://textures/nebula.png"`). At merge time `self://` is rewritten
  to the mod's own folder: `mods/example/...` when shipped, `mods://example/...`
  when installed from the portal - native and web alike. You never hard-code
  your own id.
- A `self://` ref that names no declared resource is rejected by the portal
  generator, the static `content lint`, and the in-game content gate.

## Learn more

- `web/src/wiki/modding/publish-a-mod.md` - publish a finished mod
- `web/src/wiki/modding/scenarios.md` - scenario files and scripting chapters
- `web/src/wiki/modding/sections.md` - the section `base`/`kind` reference
- `web/src/wiki/modding/mod-files.md` - the mod folder and file format
