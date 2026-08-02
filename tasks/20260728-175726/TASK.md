# Spike: HTML demos - menu widget language + contextual HUD behavior

- PRIORITY: 44
- TAGS: v0.9.0, spike, ui, hud
- KIND: SPIKE
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

The UI-rework epic (20260728-175719) needs its directions picked by the owner before
any Bevy work: how far the NOVA OS look goes on each surface (full CRT-terminal
style vs casing-hardware "physical controls" without scanlines), what a
light-3D button vocabulary looks like, how the flight HUD gets quieter (what
shows when, what grows while in use, what text goes away, what moves into the
computer), and how meters read at 1 u = 10 m. The proven method is a standalone
HTML PoC like `examples/ui/nova_os_terminal_poc.html`: interactive,
browser-openable, cheap to iterate with owner feedback.

## Steps

- [x] Demo 1 `examples/ui/nova_ui_rework_poc.html` - the widget language:
      main menu, pause, settings (audio/graphics/controls), mods browser and
      scenarios picker mocked in the NOVA OS-derived style, plus a widget zoo
      (button states idle/hover/pressed/selected/disabled, segmented control,
      slider, list rows, panel headers, badges) with light-3D treatment
      (gradient faces, lit top edge, deep bottom edge, pressed inset). Include
      a style-intensity toggle where it matters (phosphor-terminal panel vs
      hardware-casing panel) so the owner picks per surface.
- [x] Demo 2 `examples/ui/hud_rework_poc.html` - contextual HUD behavior:
      a starfield backdrop with the flight HUD mocked in the new language,
      driven by situation buttons (idle cruise, AP GOTO burn, combat lock,
      weapons hot + firing, objective posted, comms message, low ammo). Shows
      contextual visibility, size emphasis while in direct use, meters units
      (1 u = 10 m), reduced text (icon-style keybind chips per backlog
      20260710-231927's direction), and how the `~` HUD levels interact with
      the automatic behavior.
- [x] Owner reviews both demos in a browser; iterate on feedback.
- [x] Write SPIKE.md: accepted directions, rejected variants, per-surface
      intensity picks, the contextual HUD ruleset (what shows on which event,
      what grows when, revert timing), the units formatting policy (m/km
      threshold, m/s), the text-reduction list.
- [x] Record load-bearing shape decisions in DECISION.md (ACCEPTED status).
- [x] Refine the epic's remaining children (Steps/DoD) from the accepted
      SPIKE.md; re-merge inseparable pairs if the design says so.

## Definition of Done

1. Both demo files exist and open standalone in a browser (cmd:
   `ls examples/ui/nova_ui_rework_poc.html examples/ui/hud_rework_poc.html`;
   manual: owner reviewed and accepted them).
2. SPIKE.md records accepted directions for: widget vocabulary + light-3D
   treatment, per-surface style intensity, HUD contextual ruleset, text
   reduction list, units policy (manual: owner sign-off).
3. DECISION.md records the load-bearing forks with STATUS: ACCEPTED.
4. The epic's remaining child tasks carry refined Steps/DoD sourced from the
   accepted SPIKE.md (cmd: `tatr check`).

## Notes

Demo code is throwaway-quality but tracked under examples/ui/ like the NOVA OS
PoC (which stayed the canonical visual reference). Reuse the PoC's CSS palette
variables verbatim as the starting tokens (--case-0..3, --phosphor, --amber,
--orange, --screen-0/1, --text, --mono).
