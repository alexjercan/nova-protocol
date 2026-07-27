# Review: NOVA OS chin controls (BRIGHT/SCAN knobs + SND/PWR)

Branch: `feature/nova-os-chin-controls` (one commit `c88fc97c` on `master`).
Reviewer: out-of-context. Re-derived all load-bearing claims from the code.

`cargo check -p nova_gameplay -p nova_menu` is clean (only pre-existing
proc-macro-error2 future-incompat noise). Only the TASK.md checkbox flips are
uncommitted, as expected.

## Summary

The branch delivers all four DoD items with real, mechanism-exercising tests.
The knob detent math mirrors the PoC exactly, the BRIGHT/SCAN values reach the
CRT shader uniforms the WGSL actually consumes, defaults preserve the shipped
out-of-box look, persistence is serde-default tolerant with a real round-trip
test, and the terminal keyboard path is untouched. Findings below are all NIT /
MINOR - no correctness or DoD gaps.

## Positive confirmations (verified, non-trivial)

- DoD 1 (knobs): `nova_os_chin_knobs_cycle_detents` triggers `Activate` on the
  actual `Button` entity, then asserts (a) the resource detent advanced, (b) the
  dial `UiTransform.rotation` reached `NOVA_OS_KNOB_ANGLES[i]`, and (c) the
  material `data.brightness` / `data.scanline_strength` uniforms followed. It
  also asserts the 4-detent wrap back to the default and that SCAN cycles
  independently of BRIGHT. This is a genuine end-to-end test, not presence-only;
  each assertion would fail if the mechanism broke. drawer.rs:6218-6350.
- Uniforms actually reach the shader: `assets/shaders/nova_os_crt.wgsl` reads
  `material.scanline_strength` (line 107) and `material.brightness` (line 129),
  and `animate_nova_os_crt` writes both every frame from
  `settings.brightness()` / `settings.scanline_strength()` (drawer.rs:2479-2480).
- Wrap + angle correctness: `cycle()` uses `(index + 1) % len` (drawer.rs:349,
  353), matching PoC `(brightIndex + 1) % BRIGHT.length`. `NOVA_OS_BRIGHT_DETENTS`
  == PoC `[0.8, 1, 1.15, 1.3]` and `NOVA_OS_KNOB_ANGLES` == PoC ANGLES exactly.
- Defaults preserve the shipped LOOK: BRIGHT default detent 1 == 1.0 (neutral),
  SCAN default detent 2 == `NOVA_OS_CRT_SCANLINE_STRENGTH` (0.06), the same value
  the material `Default` already shipped (drawer.rs:752, 107, 163). Because
  `animate_nova_os_crt` now unconditionally overwrites both uniforms, this
  equality is what keeps the out-of-box brightness/scanline unchanged - I
  confirmed the default settings resolve to exactly the prior constants, so the
  feature does not silently alter the shipped image. Sound defaults ON.
- DoD 2 (SND): `nova_os_snd_toggles_sound_resource` asserts default ON, flips the
  flag, and asserts the label text became "SND OFF" via the real
  `sync_nova_os_monitor_controls` path (the rig runs that system in Drawer
  state). drawer.rs:6353-6390.
- DoD 3 (PWR): `nova_os_pwr_drives_close_transition` asserts `closing` starts
  false and flips true after Activate. PWR sets the same `DrawerCloseTransition`
  the `exit` command uses (drawer.rs:2058-2060), so it is the diegetic twin.
- DoD 4 (persistence): `settings_store` round-trip test now uses NON-default
  values (bright 3, scan 0, sound off) and asserts both the raw round-trip and
  the rebuilt `NovaOsMonitorSettings` (settings_store.rs:214-244). The
  missing-field test confirms serde `#[serde(default = ...)]` tolerance so an old
  settings file still loads (settings_store.rs:284-290). `nova_menu` snapshots
  (`persist_settings_on_change`) AND applies on load (`load_persisted_settings`),
  mirroring the MasterVolume wiring, with `is_added` guarding the init frame.
- Input hygiene: `handle_terminal_keyboard` reads `KeyboardInput` messages
  directly (drawer.rs:1886-1917), not bevy focus; Tab is matched there
  (drawer.rs:1843). The chin buttons carry no `TabIndex` and use pointer-driven
  `Activate` observers, so they cannot steal the Tab-completion path. Confirmed
  the dial/pointer/label children are all `Pickable::IGNORE` so only the parent
  `Button` takes the pick.
- State hazards: `sync_nova_os_monitor_controls` is gated on
  `resource_changed`, which fires on the init frame; its queries just iterate
  (empty before spawn = harmless no-op), and it only mutates existing entities -
  no panic. `setup_drawer` takes `settings: Option<Res<..>>` and falls back to
  `unwrap_or_default()` (drawer.rs:3064-3065), so bare-app rigs without the
  resource still spawn. `animate_nova_os_crt` takes a non-optional
  `Res<NovaOsMonitorSettings>`, but every rig that schedules it inits the
  resource (drawer.rs:1728, 6163, 6221) and the plugin always inits it, so no rig
  panics.
- Spawn-time reconciliation on reopen: dials spawn already rotated to
  `settings.dial_angle(knob)` and the SND button spawns with the correct
  lit/label state (drawer.rs:3728, 3808-3833), so a reopen shows the saved
  position even though `resource_changed` will not fire.

## Findings

### NIT - `NOVA_OS_CASE_LIT` doubles as the "unlit/muted" color
`nova_os_lit_color(false)` returns `NOVA_OS_CASE_LIT` (drawer.rs:3892-3897), and
the same constant is the knob dial's rim border. The name reads as "lit" but here
it is the muted/off state of the SND indicator. Not wrong visually, just a
slightly confusing name at the call site. Suggestion: a one-word local comment
or a `NOVA_OS_CASE_EDGE`-style alias would read clearer. Non-blocking.

### NIT - PWR sets `closing = true` unconditionally
`on_nova_os_power_button` sets `close.closing = true` even if a close is already
in flight (drawer.rs:2058-2060). Harmless (idempotent) and matches the PoC's
fire-and-forget handler, but worth a one-liner noting it is intentionally
idempotent. Non-blocking.

### MINOR - SCAN top detent 0.20 vs the documented "obviously aggressive" intent
`NOVA_OS_SCAN_DETENTS = [0.0, 0.03, 0.06, 0.20]` (drawer.rs:107). The doc comment
calls index 3 "a heavy, obviously aggressive raster" per the 2026-07-27 owner
call, and 0.20 vs the 0.06 default is a >3x jump, so this reads as intended. I
could not eyeball it (the RTT scene OOMs local lavapipe, per the task's
`gpu-example-local-skip`), so this is flagged only as an unverified-on-hardware
item for the owner's AFTER-shot acceptance, not a code defect. No change needed
in code.

## Verdict rationale

All four DoD tests exist, exercise the real mechanism, and would fail if broken.
Defaults preserve the shipped look, uniforms are consumed by the WGSL,
persistence is old-file tolerant, and the terminal keyboard path is provably
untouched. No MAJOR correctness or DoD gap. Findings are NIT/MINOR only.

- VERDICT: APPROVE
