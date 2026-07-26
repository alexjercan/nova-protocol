# NOVA OS chin controls: working BRIGHT/SCAN knobs + SND/PWR buttons

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.9.0,feature,ui,hud

## Story

As a player at the NOVA OS monitor, I want the physical-looking knobs and
buttons on the case chin to actually work - brightness, scanline depth, speaker
toggle, power - so the monitor reads as hardware I can touch, exactly like the
PoC's chin controls (`examples/ui/nova_os_terminal_poc.html`, `.controls`:
BRIGHT knob, SCAN knob, SND speaker button, PWR button + LED).

The chin bar itself (casing strip + recessed brand plate with the NovaCRT 9000
logo bottom-left) is geometry owned by 20260726-193219; this task makes the
controls on its right side exist and function.

## Steps

- [ ] BRIGHT knob: 4 detents cycling a whole-screen brightness level (PoC
      `BRIGHT = [0.8, 1, 1.15, 1.3]`). Implement via a brightness uniform on
      `NovaOsCrtMaterial` (`assets/shaders/nova_os_crt.wgsl`) - the overlay can
      darken multiplicatively with a black tint and brighten with an additive
      term - or an equivalent whole-screen treatment. The knob pointer rotates
      to the detent angle (PoC `ANGLES = [-115, -38, 38, 115]`).
- [ ] SCAN knob: 4 detents driving `scanline_strength` (uniform already exists
      on `NovaOsCrtMaterial`), same pointer rotation treatment.
- [ ] SND button: toggles a `NovaOsSoundEnabled` resource plus the lit/unlit
      speaker glyph state. The NOVA OS sound task consumes the resource; this
      button must land cleanly even if it orders first (toggle with no audio
      wired is a visible-state no-op).
- [ ] PWR button + LED: pressing PWR drives the existing animated close
      (`DrawerCloseTransition`), the diegetic twin of the `exit` command.
- [ ] Knob/toggle state persists across drawer open/close within a session
      (resources, not per-spawn component state).
- [ ] Mouse clicks work with the drawer's free cursor; controls stay OUT of the
      terminal's keyboard path (Tab must keep completing at the prompt, never
      focus a chin button).
- [ ] Headless tests: detent cycling updates the material uniform/resource,
      SND flips the resource, PWR sets the close transition.

## Definition of Done

- Clicking BRIGHT/SCAN steps through 4 detents and visibly changes screen
  brightness / scanline depth; the dial pointer rotates per detent. (test:
  `nova_os_chin_knobs_cycle_detents`; manual: compare feel against the PoC)
- SND toggles `NovaOsSoundEnabled` and its lit state. (test:
  `nova_os_snd_toggles_sound_resource`)
- PWR closes the computer through the existing animated close. (test:
  `nova_os_pwr_drives_close_transition`)
- Settings survive close/reopen within a session. (test:
  `nova_os_chin_settings_persist_across_reopen`)

## Notes

- Depends on: 20260726-193219 (chin bar + brand plate geometry).
- Pairs with: the NOVA OS sound task (SND consumer; created alongside this
  task).
- PoC reference: `.knob`/`.dial`/`.power-btn` markup + the `hardware` script
  section (BRIGHT/SCAN/ANGLES arrays, `applyKnob`, sound/power click handlers).
- Epic: `tasks/20260725-104330/TASK.md`.
