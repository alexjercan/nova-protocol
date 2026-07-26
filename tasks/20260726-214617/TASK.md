# NOVA OS chin controls: working BRIGHT/SCAN knobs + SND/PWR buttons

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.9.0, feature, ui, hud

## Story

As a player at the NOVA OS monitor, I want the physical-looking knobs and
buttons on the case chin to actually work - brightness, scanline depth, speaker
toggle, power - so the monitor reads as hardware I can touch, exactly like the
PoC's chin controls (`examples/ui/nova_os_terminal_poc.html`, `.controls`:
BRIGHT knob, SCAN knob, SND speaker button, PWR button + LED).

The chin bar itself (casing strip + recessed brand plate with the NovaCRT 9000
logo bottom-left) is geometry owned by 20260726-193219; this task makes the
controls on its right side exist and function.

## Flow State

- FLOW STEP: PLANNING

## Steps

- [ ] Add a `NovaOsMonitorSettings` resource in
      `crates/nova_gameplay/src/hud/drawer.rs` (exported via the prelude):
      `bright_detent`, `scan_detent`, `sound_enabled`. Defaults match the PoC
      boot state + the owner's gate call: detents (1, 2), sound ON.
- [ ] Spawn the controls into the chin's reserved controls-row slot from
      20260726-193219: two knobs (dial node + pointer child rotated via
      `UiTransform`), the SND speaker button with lit/unlit glyph + "SND
      ON/OFF" label, the PWR button with LED. Use the `Button` +
      `observe(Activate)` pattern the app close control already uses.
- [ ] BRIGHT knob: 4 detents cycling a whole-screen brightness level (PoC
      `BRIGHT = [0.8, 1, 1.15, 1.3]`), wired to the brightness-multiply
      uniform the RTT sampling shader (20260726-193233, lands first) reserves
      for this task - a true `filter: brightness()` equivalent, exact for the
      >1.0 detents an overlay could only fake. The knob pointer rotates to the
      detent angle (PoC `ANGLES = [-115, -38, 38, 115]`).
- [ ] SCAN knob: 4 detents driving the scanline-strength uniform in the same
      sampling shader, same pointer rotation treatment.
- [ ] SND button: flips `sound_enabled` plus the glyph state. The NOVA OS
      sound task consumes the flag; this button must land cleanly even if it
      orders first (toggle with no audio wired is a visible-state no-op).
- [ ] PWR button + LED: pressing PWR drives the existing animated close
      (`DrawerCloseTransition`), the diegetic twin of the `exit` command.
- [ ] Persist per `DECISION.md`: add serde-defaulted fields to
      `PersistedSettings` (`crates/nova_menu/src/settings_store.rs`), snapshot
      + apply alongside the existing `MasterVolume` wiring in `nova_menu`, so
      the detents and SND survive a restart on native and web.
- [ ] Mouse clicks work with the drawer's free cursor; controls stay OUT of the
      terminal's keyboard path (Tab must keep completing at the prompt, never
      focus a chin button).
- [ ] Headless tests: detent cycling updates the resource + material uniform,
      SND flips the flag (default ON), PWR sets the close transition, and the
      settings-store roundtrip covers the new fields (mirror the existing
      `settings_store` unit tests).
- [ ] Capture AFTER shots with `screenshot_nova_os` (knobs at non-default
      detents so the effect is visible), eyeball them, and record the work +
      self-reflection in `tasks/20260726-214617/NOTES.md`.

## Definition of Done

- Clicking BRIGHT/SCAN steps through 4 detents and visibly changes screen
  brightness / scanline depth; the dial pointer rotates per detent. (test:
  `nova_os_chin_knobs_cycle_detents`; manual: compare feel against the PoC)
- SND toggles the monitor sound flag and its lit state, and defaults ON.
  (test: `nova_os_snd_toggles_sound_resource`)
- PWR closes the computer through the existing animated close. (test:
  `nova_os_pwr_drives_close_transition`)
- Knob detents + SND survive a game restart via the settings store. (test:
  `settings_store` roundtrip covers the monitor fields; manual: dial a knob,
  relaunch, the setting held)

## Notes

- DECISION: `tasks/20260726-214617/DECISION.md` - persistence via the
  settings store, SND default ON (owner, 2026-07-26 plan gate).
- Depends on: 20260726-193219 (chin bar + brand plate geometry) and
  20260726-193233 (the RTT sampling shader the BRIGHT/SCAN uniforms live on -
  re-slotted ahead of this task on 2026-07-26 so the knobs wire to the final
  shader once instead of the superseded overlay first).
- Pairs with: the NOVA OS sound task (SND consumer; created alongside this
  task).
- PoC reference: `.knob`/`.dial`/`.power-btn` markup + the `hardware` script
  section (BRIGHT/SCAN/ANGLES arrays, `applyKnob`, sound/power click handlers).
- Epic: `tasks/20260725-104330/TASK.md`.
