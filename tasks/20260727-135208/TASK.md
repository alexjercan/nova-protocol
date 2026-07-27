# NOVA OS chin controls: 3D knobs/buttons, sound bulb, orange PWR close animation

- STATUS: OPEN
- PRIORITY: 44
- TAGS: v0.9.0,feature,ui,hud

Playtest feedback on the NOVA OS chin controls (the knobs + buttons on the
case chin). They read flat/ugly next to the web PoC; make the whole control
row feel 3D, and fix specific per-control behaviors.

Code: `crates/nova_gameplay/src/hud/nova_os.rs` - knob spawn
`spawn_nova_os_knob()` ~3578-3651, sound button `spawn_nova_os_sound_button()`
~3654-3691 (indicator ~3670, label ~3681), power button
`spawn_nova_os_power_button()` ~3695-3726, shared chin-button node
`nova_os_chin_button_node()` ~3728-3742; sound handler ~1819-1846, power
handler ~1828-1833 (drives `NovaOsCloseTransition`). Reference look:
`examples/ui/nova_os_terminal_poc.html` `.knob`/`.dial` ~758-796,
`.power-btn` ~798-831.

## Flow State

- FLOW STEP: PLANNED
- PLAN STATUS: APPROVED

## Story

Bring the chin controls up to the web PoC's tactile 3D quality and fix the
individual affordances the owner called out.

## Steps

- [ ] General 3D depth pass on all chin buttons/knobs: gradient fills, inset/
      raised shadows, borders that read as moulded plastic (match the PoC's
      `.power-btn` gradient + inset shadow and `.dial` radial gradient). No
      flat single-color rectangles.
- [ ] BRIGHT/SCAN knobs: make the dial faces read as real 3D knobs (radial
      gradient body, raised rim, a pointer tick) matching the PoC, not the
      current flat circle.
- [ ] Sound button: stop swapping the "SND ON" / "SND OFF" text on toggle.
      Keep a fixed "SND ON/OFF"-style label and instead convey state with the
      green bulb/LED turning on (lit phosphor) and off (dark). Update the
      indicator, not the label text.
- [ ] Power button: on click, turn the button/LED ORANGE, then play a small
      close animation before the drawer collapses (brief orange flash tied to
      the existing `NovaOsCloseTransition`, not an instant close).
- [ ] Keep detents/handlers working (BRIGHT/SCAN detent cycling, sound mute
      toggle, power close).

## Definition of Done

- Chin controls read as 3D moulded plastic like the PoC; knobs look like real
      dials; sound state is shown by the bulb (label no longer swaps text);
      pressing PWR flashes orange then animates closed. (manual: owner confirms
      against the PoC)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay drawer)
