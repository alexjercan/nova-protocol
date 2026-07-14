# Unify the whole game UI to the web-app theme (menu, HUD, pause, mods, editor - one style)

- STATUS: OPEN
- PRIORITY: 45
- TAGS: ui,polish,v0.6.0

## Goal (user request, 20260714)

The in-game UI colours/typography are ad-hoc and inconsistent across screens
("kind of random"). The web app (`web/src/style.css`) has a cohesive
"industrial HUD" look - deep navy panels, hard 1px borders, sharp 2px corners,
cyan/amber accents, crisp hover, Rajdhani/Inter/JetBrains-Mono type. Adopt ONE
shared style across the whole game UI so every screen matches the web app.

The editor already moved to this palette in task 20260714-204219
(`crates/nova_editor/src/ui/theme.rs` holds the wiki-palette `Color` consts +
metrics). That module is the reference/starting point; this task spreads the same
theme to the rest and de-duplicates the ad-hoc palettes.

## Scope

Screens to bring onto the shared theme:
- `nova_menu` - main menu, Sandbox/New Game buttons, Settings panel, the Mods
  panel + explore placeholder, pause overlay. It currently keeps its own private
  colour consts (`crates/nova_menu/src/lib.rs`, ~line 60: "Same palette as the
  editor sidebar (nova_editor keeps its constants private ...)").
- HUD / overlays owned by `nova_gameplay` / `nova_info` (objectives panel, ammo
  readouts, radar/target chrome, FPS) - align colours + fonts where they are
  UI text/panels, without disturbing diegetic 3D elements.
- `nova_editor` - already themed; just consume the shared module once extracted.

## Likely approach (decide when planning)

- Extract the palette + metrics (and ideally a `button`/panel/`header` widget set)
  into ONE shared location so there is a single source of truth, instead of each
  crate re-declaring colours:
  - Option A: a small `nova_ui` crate depended on by menu/editor/gameplay-hud.
  - Option B: put the theme in an existing low-level crate (e.g. `nova_assets` or
    `nova_gameplay`) that the UI crates already depend on.
  - Weigh against the dependency graph (editor/menu/gameplay all need it).
- Fonts: the web app uses Rajdhani (display) / Inter (body) / JetBrains Mono
  (labels). Decide whether to ship those font assets and a `TextFont` helper, or
  approximate with the current default font. Loading real fonts is the bigger
  win for "matches the web app" but adds assets + a load path (wasm-safe).
- Keep it a restyle: same layouts/behaviour, new colours/spacing/type. No new
  screens.

## Notes

- Reference: `web/src/style.css` (palette CSS variables + `.prose`, `.wiki-*`,
  button/card styles), and `crates/nova_editor/src/ui/theme.rs` (already ported).
- Sweep for existing ad-hoc palettes before adding a shared one: `Color::srgb`
  consts in `nova_menu`, `nova_gameplay` HUD, `nova_info`, `nova_editor`.
- This is a `spike`-free restyle but it is broad; `/plan` should break it per
  screen/crate. Verify each screen via its example / autopilot + an eyeball.
- Depends on nothing hard, but best done AFTER 20260714-204219 lands (the editor
  theme is the template).
