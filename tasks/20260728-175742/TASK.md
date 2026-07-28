# HUD restyle + on-screen text reduction

- STATUS: OPEN
- PRIORITY: 36
- TAGS: v0.9.0,ui,hud

## Story

The flight HUD chrome (status bar, objective hint, comms panel, keybind hint
cluster, marker chips) still speaks the old flat language and carries the bulk
of the on-screen text. Restyle it to the accepted direction and cut text per
the spike decisions: icon-style keybind chips (folds backlog 20260710-231927),
slimmer objective/beacon chips, comms density, with detail delegated to the
NOVA OS computer.

## Steps (refined from SPIKE.md, 2026-07-28)

This is the HUD LOOK (SPIKE.md D4 + text reduction); the automatic
show/emphasis BEHAVIOUR is the sibling 20260728-175747, layered on top. Demo 2
(`hud_rework_poc.html`) is the reference. Lands before 175747.

- [ ] Restyle HUD chrome to the phosphor language: status bar, objective/beacon
      chips, comms panel, readouts (speed / mode / destination / lock DST-CLS /
      target-zoom PiP) per demo 2. Keep the velocity-direction shader and the
      top-right target-zoom PiP.
- [ ] Replace the 7-row keybind cluster with the contextual ICON-CHIP DOCK using
      the imported FREE Input Prompts Alt glyphs
      (`examples/ui/assets/input-prompts/`, relocated to the game asset home per
      backlog 20260728-214929 if that lands first); dim / available / hot states
      driven by `FlightVerbHints`. Folds backlog 20260710-231927.
- [ ] Text reduction: slim the objective/beacon chips (glyph + name + range),
      cut comms density (short cards + dwell), anchored verb cues use glyphs;
      move full detail (ship status, objective list, map, log) to NOVA OS.
- [ ] Apply the units policy on HUD readouts (m / km / m/s) - coordinate with
      the units child 20260728-175731 so the format helper is shared, not
      duplicated.

## Definition of Done (refined 2026-07-28)

1. render eyeball: HUD screenshots per changed element (status bar, dock, chips,
   comms, readouts, target-zoom) in the phosphor language.
2. test: a widget-tree assertion that the keybind dock renders the correct key
   glyph + availability state per `FlightVerbHints` (would fail if the dock were
   a no-op).
3. cmd: `grep` shows the old 7-row `[KEY] VERB` text cluster is gone; recorded
   here.
4. manual: owner playtest verdict that on-screen text density dropped and the
   HUD reads in the new language.
