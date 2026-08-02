# NOVA OS chin controls: 3D knobs/buttons, sound bulb, orange PWR close animation

- PRIORITY: 44
- TAGS: v0.9.0, feature, ui, hud
- KIND: TASK
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

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

## Story

Bring the chin controls up to the web PoC's tactile 3D quality and fix the
individual affordances the owner called out.

## Steps

- [x] General 3D depth pass on all chin buttons/knobs: gradient fills, inset/
      raised shadows, borders that read as moulded plastic (match the PoC's
      `.power-btn` gradient + inset shadow and `.dial` radial gradient). No
      flat single-color rectangles. Done: `nova_os_chin_button_gradient()`
      (180deg lit->deep + 1px top-highlight lip), near-black `NOVA_OS_BUTTON_BORDER`,
      dial radial dome, glassy bulb cap.
- [x] BRIGHT/SCAN knobs: make the dial faces read as real 3D knobs (radial
      gradient body, raised rim, a pointer tick) matching the PoC, not the
      current flat circle. Done: `RadialGradient` at anchor (-0.16,-0.22) =
      PoC `circle at 34% 28%`, stops DIAL_LIT/MID/DARK, dark inner rim, pointer kept.
- [x] Sound button: stop swapping the "SND ON" / "SND OFF" text on toggle.
      Keep a fixed label and instead convey state with the green bulb/LED
      turning on (lit phosphor) and off (dark). Chose a fixed "SND" legend
      (the owner's "SND ON/OFF" suggestion, minus the now-redundant on/off
      text the bulb carries) - trivially changed if the literal is preferred.
      `NovaOsSoundLabelMarker`/`nova_os_sound_label` removed; bulb driven by
      `nova_os_bulb_color(on)` in sync.
- [x] Power button: on click, turn the button/LED ORANGE, then play a small
      close animation before the drawer collapses. Done: `drive_nova_os_power_led`
      flashes `NOVA_OS_ORANGE` while `NovaOsCloseTransition.closing`, green
      otherwise; the existing slide + raster collapse is the "then close".
- [x] Keep detents/handlers working (BRIGHT/SCAN detent cycling, sound mute
      toggle, power close). Verified by the 5 passing chin-control tests.

## Definition of Done

- Chin controls read as 3D moulded plastic like the PoC; knobs look like real
      dials; sound state is shown by the bulb (label no longer swaps text);
      pressing PWR flashes orange then animates closed. (manual: owner confirms
      against the PoC)
- Touched tests pass. (cmd: nix develop --command cargo test -p nova_gameplay -- nova_os_snd nova_os_pwr nova_os_chin)
      [The planning template's `drawer` filter matches 0 tests in nova_gameplay;
      the chin-control tests live under `hud::nova_os::tests::*`.]

## Close-out

What changed and why:
- All chin controls now use the same moulded-plastic language the casing/screws
  already used (`BackgroundGradient` + `ColorStop`, no new rendering path): a
  180deg lit->deep button gradient with a 1px top-highlight lip and a near-black
  outer border, and a radial dome on the knob dials with an off-centre highlight
  matching the PoC `circle at 34% 28%`. Reused the existing gradient primitives
  rather than adding `BoxShadow`, for consistency with the surrounding code.
- Sound state moved off the label onto a bulb. The label swap
  ("SND ON"/"SND OFF") is gone; the bulb goes lit-phosphor <-> dark-green. This
  also let the SND button border stop encoding state (it was doubling the
  indicator), so the border is now static like PWR's.
- PWR orange flash: a tiny `drive_nova_os_power_led` system tints the LED orange
  whenever `NovaOsCloseTransition.closing` is set, green otherwise. It runs every
  active frame (not on the `resource_changed::<NovaOsMonitorSettings>` gate that
  `sync_nova_os_monitor_controls` uses) because the closing flag lives on a
  different resource. The "then close" animation already existed (slide + CRT
  raster collapse); this only adds the colour cue.

Alternatives considered:
- A brief timed orange flash BEFORE starting the close (delaying `closing`). Not
  taken: the drawer's slide+collapse is already ~0.22s and reads as the close,
  so tinting the LED for the duration of the existing close is simpler and has
  no new timing state. Easy to revisit if the owner wants a pre-close beat.
- Label kept as the literal "SND ON/OFF": dropped in favour of "SND" (see step
  note); documented so it is a one-line change if the owner disagrees.

Difficulties:
- The DoD proof command copied from the sibling casing task
  (`cargo test -p nova_gameplay drawer`) silently matches ZERO tests - the
  nova_os tests are under `hud::nova_os::tests::*`, no "drawer" in the path. The
  first run reported "0 passed ... 690 filtered out" (a green that proves
  nothing). Corrected the filter to the chin-control test names. Worth a lesson:
  a passing test filter that runs 0 tests is a false green.
- An existing test (`nova_os_snd_toggles_sound_resource`) asserted the old
  "SND OFF" label swap; updated it to assert the bulb colour instead (the
  intentionally-changed behaviour), and added a PWR-LED orange test.

Self-reflection: I verified the filter actually ran the tests instead of
trusting the "ok" line - that caught the 0-tests false green. Next time, sanity-
check an inherited DoD proof command against the real test names before relying
on it. The change stayed within the established gradient vocabulary, so no new
DECISION.md was needed.
