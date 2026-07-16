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
  `StoryMessage` comms beat at the open and the clear.
- **Mod-shipped binary art** - ships its own skybox and asteroid texture under
  `textures/`, declared in the manifest's `resources`, referenced with the
  `self://` scheme so they resolve against this mod's own folder.
- **A menu backdrop** - `example_menu` is flagged `menu_backdrop: true`, so mods
  can ship their own main-menu ambience, not just playable levels.

## Layout

```
example/
  example.bundle.ron     # manifest: content + resources + meta
  example.content.ron    # section overlay + new section + arena + menu backdrop
  textures/
    nebula.png           # self:// skybox (stacked 1x6 cube faces)
    nebula.png.meta      # RowCount cube reinterpret sidecar (rides along, not listed)
    rock.png             # self:// asteroid texture
```

The textures are PLACEHOLDERS (debug-colored); real art replaces them under
task 20260716-205214, keeping the same paths.

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
  generator, the static `content_lint`, and the in-game content gate.

## Learn more

- `web/src/wiki/dev/guide-make-a-mod.md` - package and publish a mod
- `web/src/wiki/dev/guide-author-scenario.md` - the scenario event/action grammar
- `web/src/wiki/dev/guide-author-section.md` - the section `base`/`kind` grammar
- `web/src/wiki/dev/modding-ron.md` - the data-format reference
- `docs/design/mod-binary-resources.md` - the `self://` design
