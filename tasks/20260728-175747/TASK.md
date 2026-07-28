# Contextual HUD: show-by-relevance + grow-in-use

- STATUS: OPEN
- PRIORITY: 34
- TAGS: v0.9.0,ui,hud,gameplay

## Story

Full HUD always-on feels bloated; no HUD is unplayable; the `~` levels are a
manual workaround. Per the accepted spike ruleset, make importance-driven
visibility automatic: elements appear when their situation is live (combat
lock, weapons hot, AP burn, objective posted) and grow while in direct use,
shrinking back after. Decide what happens to the `~` tiers (keep as an
override, or simplify) per the spike.

## Steps (refined from SPIKE.md, 2026-07-28)

The automatic BEHAVIOUR layered on the restyled HUD (sibling 20260728-175742,
which lands first). Ruleset = SPIKE.md D3/D5; demo 2 (`hud_rework_poc.html`) is
the reference.

- [ ] Show-by-relevance on top of `HudTier`: each element auto-appears while its
      situation is live and hides otherwise - AP mode chip + destination readout
      on `Autopilot`; reticle + lock readout + target-zoom on `CombatLock`; ammo
      groups on weapons-hot; objective chip on objective post; comms card on a
      story line. Idle cruise shows only velocity shader + speed + dim dock +
      status bar.
- [ ] Grow-in-use emphasis: the element in direct use scales up (~1.14x) and
      reverts - continuous emphasis while the action holds (firing, burn),
      settle-timer for one-shots (objective pop ~1.2s, comms hold ~5s then fade).
- [ ] Simplify the `~` control to On / Cinematic (drop All/Minimal/None):
      On = full auto-contextual HUD, Cinematic = clean screen. Migrate the
      existing HudVisibility cycle + its keybind/tests.
- [ ] Keep the velocity-direction shader always on and the target-zoom PiP on
      lock (these are not auto-hidden).

## Definition of Done (refined 2026-07-28)

1. test: App-driven tests per rule - situation event -> element shown/grown ->
   reverts when the situation clears (and one-shots settle on their timer).
2. test: the `~` control toggles On/Cinematic and Cinematic clears the HUD;
   the old three-level cycle is gone (grep recorded here).
3. cmd: `cargo run -p nova_probe -- run <playable example>` passes with the
   contextual HUD active (before/after evidence attached).
4. manual: owner playtest verdict that the HUD is quieter and surfaces the right
   things at the right time.
