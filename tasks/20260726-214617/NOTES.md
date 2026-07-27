# NOVA OS chin controls - work notes

## What was built

Working BRIGHT/SCAN knobs + SND/PWR buttons on the NOVA OS monitor chin, wired
into the controls-row slot reserved by 20260726-193219.

- `NovaOsMonitorSettings` resource (drawer.rs, exported via the hud prelude):
  `bright_detent`, `scan_detent`, `sound_enabled`. Defaults mirror the PoC boot
  state: BRIGHT detent 1 (= neutral 1.0), SCAN detent 2 (= the shipped scanline
  look), sound ON. Accessors clamp a possibly-corrupt persisted index.
- Detent tables as consts: `NOVA_OS_BRIGHT_DETENTS = [0.8, 1.0, 1.15, 1.3]`
  (PoC `BRIGHT`), `NOVA_OS_SCAN_DETENTS = [0.0, 0.03, 0.06, 0.12]` (scaled to
  the in-game shader's `scanline_strength`, whose default 0.06 sits at index 2),
  `NOVA_OS_KNOB_ANGLES = [-115, -38, 38, 115]` (PoC `ANGLES`).
- Chin controls spawn into `NovaOsControlsRow`: two knobs (a `Button` with a
  rotating dial + pointer child and a caption), the SND speaker button
  (lit/unlit indicator + "SND ON/OFF" label), and the PWR button + green LED.
  Each knob's click observer is attached via `EntityCommands::observe` (the
  `observe()` bundle type differs per system, so it cannot be a shared bundle).
- BRIGHT/SCAN uniforms: `animate_nova_os_crt` now reads the settings and writes
  `brightness` + `scanline_strength` onto the sampling material each frame -
  a true brightness multiply (exact for the >1.0 detents an overlay could only
  fake) and the scanline-depth uniform.
- `sync_nova_os_monitor_controls` (gated on `resource_changed`) rotates each
  dial pointer to its detent angle and re-lights/relabels the SND button on
  live changes; spawn-time visuals are set directly from the settings so a
  reopen shows the saved dial position.
- PWR drives the existing `DrawerCloseTransition` (the diegetic twin of `exit`).
- Persistence: `PersistedSettings` (nova_menu settings_store) gains
  serde-defaulted `nova_os_bright_detent` / `nova_os_scan_detent` /
  `nova_os_sound_enabled`; `load_persisted_settings` and
  `persist_settings_on_change` snapshot/apply the monitor resource alongside
  `MasterVolume`, so detents + SND survive a restart on native and web.

## Decisions / reasoning

- SCAN detents were NOT the PoC's [0, 0.18, 0.34, 0.52]: those were CSS-overlay
  opacities; the in-game shader's `scanline_strength` darkens far harder, so the
  range is scaled and index 2 pinned to `NOVA_OS_CRT_SCANLINE_STRENGTH` (0.06)
  to preserve the shipped default look exactly.
- Controls stay out of the terminal keyboard path: Tab completion reads
  `KeyboardInput` directly (not bevy focus), and the chin buttons carry no
  `TabIndex`, so a knob click never steals the prompt. This mirrors the existing
  app-close `Button`.

## Bugs hit

- `BorderRadius` is a field of `Node` in bevy 0.19, not a standalone component;
  placing it in a spawn tuple gave a cascade of "not a Bundle" errors. Moved
  every `border_radius` into its `Node`.
- Two knobs' `observe(system)` bundles have different types, so a shared `let
  observer = match {...}` failed to unify. Attached the observer via
  `EntityCommands::observe` per branch instead.
- Test rig: `nova_os_font` panics when an `AssetServer` is present but
  `Assets<Font>`/`Assets<Image>` are not registered (the async load resolves a
  loader for an unregistered type). Fixed by `init_asset::<Font>()` +
  `init_asset::<Image>()` + `init_asset::<NovaOsCrtMaterial>()` alongside
  `AssetPlugin`, mirroring `spawn_drawer_shell_with_crt`'s callers - NOT by
  dropping `AssetPlugin` (which `init_asset` needs). The material-uniform
  assertion still spawns its OWN surface + `ComputedNode` because headless
  MinimalPlugins runs no UI layout, so the RTT-spawned surface has no
  `ComputedNode` and `animate_nova_os_crt` skips it.

## Verification

- `cargo test -p nova_gameplay --lib` (the 3 new tests): 3 passed.
- `cargo test -p nova_menu --lib settings_store`: 4 passed.
- `cargo test -p nova_gameplay --no-run`: full test target compiles (the
  `Option<Res<NovaOsMonitorSettings>>` change keeps every existing setup_drawer
  rig green without edits).
- `cargo fmt` clean; `cargo check --workspace` clean.

## Self-reflection

- The two test-rig panics (font load, then the AssetPlugin/init_asset dance)
  cost two build cycles. The `reuse-known-good-stack` lesson applies: I should
  have copied `spawn_drawer_shell_with_crt`'s asset setup VERBATIM from the
  start instead of reasoning about which `init_asset` calls were needed. The
  working render-capable rig was right there.
- Making `setup_drawer`'s new resource param `Option<Res<_>>` avoided editing
  ~8 existing test rigs - a good call surfaced by grepping the setup_drawer
  call sites before running, rather than discovering the panics test-by-test.

## Tests

- `nova_os_chin_knobs_cycle_detents`: clicks cycle the detent (wrapping at 4),
  rotate the dial pointer, and drive the brightness + scanline uniforms.
- `nova_os_snd_toggles_sound_resource`: SND flips the flag (default ON) and the
  label.
- `nova_os_pwr_drives_close_transition`: PWR sets the animated close.
- settings_store round-trip covers the three new NOVA OS fields + the
  `nova_os_monitor()` rebuild; the partial-file test proves serde defaults.
</content>
</invoke>
