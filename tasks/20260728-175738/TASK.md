# Menus + editor adopt the reworked widget language

- STATUS: OPEN
- PRIORITY: 38
- TAGS: v0.9.0,ui,menu,editor

## Story

With the shared theme/widgets reworked, every consuming screen gets brought to
the accepted look: the main menu panel, settings body, mods two-pane browser,
scenarios picker, pause/outcome overlays, and the editor rail + component
cards + drawer + tooltip adopt the same language per the demo mockups.

## Steps (refined from SPIKE.md, 2026-07-28)

Layouts mirror the shipped screens (SPIKE.md D2); demo 1
(`nova_ui_rework_poc.html`) is the visual reference. Depends on 20260728-175734
(shared widgets) landing first.

- [ ] Main menu: compact bottom-right corner panel over the live
      `menu_backdrop` scene (stays non-modal so the scene is the focus);
      buttons New Game / Sandbox / Scenarios / Mods / Settings / Exit in the
      phosphor language.
- [ ] Settings: single panel with stacked AUDIO / GRAPHICS / CONTROLS sections
      (mirrors `build_settings_body`) using the reworked slider / segmented /
      keybind-reference rows.
- [ ] Mods: the two-pane modal (Installed / Explore-online tabs, scrollable
      list with enable checkboxes, right detail pane = dependencies / adds /
      actions) restyled to the new widgets.
- [ ] Scenarios: the two-pane modal with collapsible campaigns ([+]/[-]) + flat
      rows, right detail = thumbnail + description + Play; FIX the broken list
      scroll.
- [ ] Pause + outcome overlays: Resume / Retry / Settings / Back to Main Menu /
      Exit in the new language.
- [ ] Editor: rail + component cards + drawer + tooltip adopt the shared
      widgets per demo 1.

## Definition of Done (refined 2026-07-28)

1. render eyeball: updated screenshot captures for each restyled screen
   (`screenshot_ui`, `menu_newgame` families; add captures for mods/scenarios
   if none exist - building the rig is step 1 per `render-output-eyeball`).
2. test: live-tree tests where behaviour changed - scenario campaign
   collapse/expand and the mods enable-toggle drive the right tree.
3. cmd: scenarios list scroll verified (the reported broken scroll no longer
   reproduces); note the check here.
4. manual: owner eyeballs every restyled screen in-engine (menus, settings,
   mods, scenarios, pause, editor) - no screen still shows the old flat theme.
