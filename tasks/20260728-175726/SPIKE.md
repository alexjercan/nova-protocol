# SPIKE: UI-rework directions (menu widget language + contextual HUD)

Status: ACCEPTED by owner 2026-07-28 (three review passes on demo 1, two on
demo 2). Source of truth for the epic's remaining children
(20260728-175719). Artifacts: `examples/ui/nova_ui_rework_poc.html` (demo 1),
`examples/ui/hud_rework_poc.html` (demo 2), both browser-openable.

## Method

Two standalone HTML PoCs, iterated live with the owner (same method that
produced `nova_os_terminal_poc.html`). Both reuse that PoC's palette tokens
verbatim (`--case-*`, `--phosphor`, `--amber`, `--orange`, `--mono`). Demo 2
also imported real key glyphs (see Key glyphs below). Every state was eyeballed
rendered (chromium screenshots) per `render-output-eyeball`.

## Accepted directions

### 1. Visual language - phosphor terminal, widgets ARE the screen

- The NOVA OS **phosphor terminal** look is the PRIMARY skin for ALL player UI
  (menus, settings, mods, scenarios, pause, HUD chrome). The flat hardware
  **casing** look is kept as a SECONDARY skin/alternative, not the default.
- Phosphor is NOT just a background swap. Every widget re-renders as a
  **CLI/terminal-drawn element**: flat 1px phosphor-bordered buttons with a `>`
  cursor marker, **inverted** selection (phosphor fill, dark glyphs), segmented
  ASCII-block meter sliders (no glossy knob), bracketed `[TAG]` badges, dashed
  header rules, terminal list rows. This was the load-bearing correction on
  demo 1: controls must look like they live on the screen, not like 3D buttons
  glued to glass.
- The hardware skin keeps the light-3D vocabulary (gradient faces, lit top
  edge, deep bottom shadow, pressed inset) for anyone who prefers it.

### 2. Menus mirror the shipped layouts, scene stays the focus

- **Main menu**: a COMPACT panel in the **bottom-right corner** over the LIVE
  `menu_backdrop` scene (ship orbiting a well). Non-modal, Factorio-style: the
  gameplay scene is the focus, not a full centered panel. Buttons: New Game /
  Sandbox / Scenarios / Mods / Settings / Exit.
- **Settings**: a single panel with stacked sections AUDIO (volume) / GRAPHICS
  (quality tiers) / CONTROLS (read-only keybind reference, keyboard + gamepad),
  mirroring `build_settings_body`. Not tabs.
- **Mods** and **Scenarios**: identical **two-pane 85% modals** (left scrollable
  list, right detail pane). Mods: Installed / Explore-online tabs, enable
  checkboxes, detail = dependencies/adds/actions. Scenarios: **collapsible
  campaigns** ([+]/[-]) + flat non-campaign rows, detail = thumbnail + Play.
  Both lists SCROLL (the current scenario scroll is broken - fix it).
- **Pause**: centered panel - Resume / Retry / Settings / Back to Main Menu /
  Exit.

### 3. Contextual flight HUD - quieter, show-by-relevance, grow-in-use

- **Idle cruise is near-empty**: velocity shader + speed chip + a dim keybind
  dock + the status bar. Nothing else.
- **Show-by-relevance**: each element appears only while its situation is live:
  - autopilot burn -> AP mode chip + destination marker + ETA/distance readout;
    speed chip emphasises during the burn.
  - combat lock -> reticle ON the target + DST/CLS readout riding it + the
    top-right target-zoom PiP.
  - weapons hot -> ammo groups; firing emphasises the lock readout + reticle.
  - objective posted -> objective chip pops (grows) then settles to a slim
    breathing chip.
  - comms message -> a short comms card, ~5s dwell, then fades.
- **Grow-in-use then settle**: the element in direct use scales up (~1.14x)
  while active and relaxes afterward (action ends, or a settle timer for
  one-shot events: objective ~1.2s pop, comms ~5s hold).
- **KEEP** (owner, explicit): the velocity-direction shader (shaded heading
  sphere + cone around the ship, always on, recolours in autopilot) and the
  top-right locked-target zoom PiP (identify view: DST/CLS/KIND/HULL).
- **Multiple weapon groups**: ammo shows one group per weapon (PDC-1, PDC-2,
  TUBE-1, TUBE-2 ...), low-ammo emphasis on the near-empty ones.

### 4. Text reduction

- The **7-row `[KEY] VERB` keybind cluster collapses to a contextual icon-chip
  dock**: real key-glyph icons, dim when unavailable, lit when available, filled
  when hot. This folds backlog 20260710-231927 (keybind hint icons).
- Objective/beacon chips: slim (glyph + name + range), breathing only when just
  posted. Comms: short cards with a dwell, not a wall of text.
- **Detail moves into NOVA OS**: full ship status, objective list, map, log stay
  in the computer's commands, not on the flight screen.

### 5. `~` HUD levels -> On / Cinematic

- The old All / Minimal / None triple is SIMPLIFIED to two: **On** (full
  auto-contextual HUD) and **Cinematic** (clean screen for screenshots/immersion).
  Auto-hide already does what Minimal did, so the middle tier is dropped.

### 6. Units policy - 1 u = 10 m everywhere

- Display scale: 1 world unit = 10 metres. Distance: `< 1000 m` -> integer
  metres (`840 m`); `>= 1000 m` -> kilometres, 2 decimals (`1.24 km`). Speed:
  metres/second (`142 m/s`). Closing speed: signed m/s (`+38 m/s`). Orbit radius
  spoke: metres.
- The unit `u` / `u/s` is RETIRED from the player surface (HUD, NOVA OS output)
  and the wiki glossary. Display-only: physics/content/AI values are untouched.
  (Detailed plan already in child 20260728-175731.)

### 7. Key glyphs

- Imported the **FREE Input Prompts pack** (JulioCacko, **CC0**) `Keyboard_Mouse`
  set in the **Alt** (primary), **Dark**, **White** styles, full glyph range,
  under `examples/ui/assets/input-prompts/` with a provenance `NOTICE.md`. Alt =
  dark rounded keycaps with a white glyph; reads well on the phosphor HUD.
- The HUD dock + verb cues use the Alt glyphs. Broader adoption across the real
  game HUD + web key-UI is backlog **20260728-214929**.

## Rejected / deprioritised variants

- **Hardware casing as the primary skin** - owner prefers phosphor; casing kept
  only as a secondary alternative.
- **Phosphor that only swaps the panel background** - rejected; widgets must be
  CLI-rendered.
- **Full centered main-menu panel** - rejected; keep the live scene as the focus
  with a corner menu.
- **`~` triple All/Minimal/None** - simplified to On/Cinematic.
- **Other icon styles (Vintage/Retro/Blanks) and the gamepad sets** - not
  imported (gamepad carries a trademark caveat; see NOTICE).

## Per-surface intensity picks

| Surface | Skin |
|---------|------|
| Main menu, pause, settings, mods, scenarios | Phosphor terminal (primary); hardware casing selectable |
| Flight HUD chrome | Phosphor terminal, contextual |
| NOVA OS monitor | Its own CRT (unchanged, canonical reference) |

## Refinement of the epic's remaining children

Done in this spike (see each TASK.md): 20260728-175734 (theme+widgets),
20260728-175738 (menus+editor), 20260728-175742 (HUD restyle+text reduction),
20260728-175747 (contextual HUD). The two HUD children stay separate: 175742 is
the LOOK (restyle + dock + units on chrome), 175747 is the BEHAVIOUR (visibility
+ emphasis ruleset); 175742 lands first as the visual base, 175747 layers the
automatic behaviour on top. Units child 20260728-175731 was already detailed.
