# Ship real web fonts in-game (Rajdhani/Inter/JetBrains Mono) + wasm-safe load path

- STATUS: OPEN
- PRIORITY: 35
- TAGS: v0.7.0,ui

Umbrella: task 20260714-212139. Depends on: 20260714-214111 (nova_ui).

## Goal (deferred from the UI-unification pass)

The game currently uses Bevy's default font everywhere (no font assets shipped).
The web app uses Rajdhani (display), Inter (body), JetBrains Mono (labels/keybinds).
To truly match the web app's typography, ship those font families and route the
UI's `TextFont` through a shared helper. Split out of the palette/metrics
restyle (user chose palette-only first) because it adds assets + a load path.

## Sketch (plan properly when picked up)

- Add the font assets under `assets/fonts/` (licences checked - the web app
  already vendors `promptfont` under `web/src/assets/`; Rajdhani/Inter/JetBrains
  Mono are OFL).
- Load them via the asset system (native + wasm-safe) - likely into `GameAssets`
  or a `nova_ui` font resource - and add a `nova_ui` `TextFont` helper set
  (display/body/mono) so call sites ask for a role, not a handle.
- Route the ~20 `TextFont`/`FontSize` call sites across menu + editor + HUD through
  the helper (defaulting to the right family per role).
- Verify: text renders in the new fonts natively AND on the wasm build (the wasm
  path is the risk); eyeball a capture.

## Notes

- Best done AFTER `nova_ui` exists (20260714-214111) so the font helper has a home,
  and ideally after the menu/HUD restyles so the call sites are already centralized.
- wasm is the hard part: fonts must load through the bundled asset path, not the
  filesystem. Confirm the wasm build actually renders them (only `workflow_dispatch`
  builds wasm today - static review + a manual deploy check).
