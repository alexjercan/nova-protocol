# Settings menu content: graphics quality, keybinds, audio

- STATUS: CLOSED
- PRIORITY: 45
- TAGS: v0.7.0, ui, menu, spike

Goal: fill the Settings placeholder panel with real content: visual quality
options (relates to task 20260525-133013, spawn-less visual mode), keybinds,
and audio volume. Deferred to backlog; the main menu task 20260711-180426
ships only an empty panel with a Back button.

Notes:
- Spike: tasks/20260711-180500/SPIKE.md
- Parent task: 20260711-174915


- 2026-07-13 (deliberate-radar spike 20260713-082207, decision D6): keybinds
  in settings should cover the radar-era bindings (radar hold/tap, raise,
  wheel section-cycle) and gamepad alternatives (press-toggle radar) - see
  task 20260710-231927 for the remap mechanics; this task owns the settings
  UI surface.

## v0.7.0 scope (20260716, spike tasks/20260716-122954/SPIKE.md)

Pulled into v0.7.0 (p45). Scope for this release: audio volume, graphics
quality preset (consumes the low-end spawn-less mode 20260525-133013, tuned
against the perf baseline 20260716-123551), and a READ-ONLY keybind reference.
Full remapping + hint icons stay backlog (20260710-231927). Plan:
docs/plans/20260716-v0.7.0-plan.md, strand 3.

- 2026-07-16 (v0.7.0 planning, user note): the settings panel should be
  reachable from the PAUSE menu too, not only the main menu - same modal,
  both entry points.

- 2026-07-16 (user decision, /flow): settings PERSIST cross-platform (native
  RON config file + web localStorage), mirroring the mod_prefs/mod_cache
  cfg-guarded stack in nova_assets. Keybinds are read-only, nothing to persist.

- 2026-07-16 (user feedback, mid-/work): audio volume is a real draggable
  SLIDER (bevy's headless `ui_widgets::Slider`), not the discrete segmented
  control first built. Graphics quality stays segmented (genuinely discrete
  tiers). Changed before landing.

## Implementation plan (20260716)

Design settled from two code surveys (menu + audio/input/graphics backends).
Key backend facts:
- Audio: all SFX + the thruster loop play through bevy `AudioPlayer` sinks, so
  a single Master volume wired to bevy's `GlobalVolume` scales everything with
  one knob (there is no separate music track - Master is the honest
  granularity). audio.rs volume constants stay; GlobalVolume multiplies them.
- Graphics: no quality resource exists and the low-end mode (20260525-133013)
  and perf baseline (20260716-123551) are still unimplemented siblings BELOW
  this task in priority. So this task DEFINES the `GraphicsQuality` enum and a
  real-but-minimal wiring against what exists today: `JuiceSettings`
  (master_enabled + shake.enabled + flash.enabled). Each tier is distinct and
  observable: High = shake+flash on; Medium = shake off, flash on; Low =
  juice master_enabled off. Documented seam: task 20260525-133013 extends the
  Low/Medium tiers with particle (hanabi) + asteroid-scatter gating once the
  baseline names what is expensive. This avoids advertised-but-unwired.
- Keybinds: the flight input rig (nova_gameplay input/player.rs `flight_input_rig`)
  is the single source of truth, but it only exists during Playing. The Settings
  panel renders in MainMenu (no rig), so the read-only reference is authored as
  static data co-located with the rig, PINNED by a parity test against the rig's
  actual bindings via the existing `binding_label()` helper (authored-vs-derived).
- Persistence stack to reuse: crates/nova_assets/src/mod_prefs.rs (native
  dirs::config_dir + RON, web web_sys localStorage, cfg-guarded).

Layering: `Settings` resource is read by nova_gameplay (audio, juice) so it and
its apply systems live in nova_gameplay (no new deps, no serde requirement).
Persistence (serde DTO + disk/localStorage) and the UI live in nova_menu, which
already reaches nova_assets and nova_gameplay.

### Steps

- [x] 1. nova_gameplay: add `settings.rs` - `Settings { master_volume: f32,
      graphics_quality: GraphicsQuality }` resource (Clone, PartialEq, Reflect,
      Default) and `GraphicsQuality { Low, Medium, High }` (Default = High).
      Register types for reflection.
- [x] 2. nova_gameplay: `SettingsPlugin` - init_resource::<Settings>(), and
      apply-on-change systems (guarded on Option resources for minimal rigs):
      Settings -> `GlobalVolume` (master_volume), Settings -> `JuiceSettings`
      (tier mapping above). Add to the gameplay plugin group so it always runs.
      Unit tests: tier->JuiceSettings mapping; volume->GlobalVolume.
- [x] 3. nova_gameplay input: add a read-only keybind reference data source
      (action label + keyboard label + gamepad label) co-located with
      `flight_input_rig`, covering flight/autopilot/radar hold-tap/section-cycle
      + gamepad alternatives. Parity test: spawn the rig, assert each entry's
      keyboard label is among the action's real bindings (bei app.finish/cleanup).
- [x] 4. nova_menu: `settings_store.rs` - serde DTO mirroring Settings, native
      (dirs::config_dir/nova-protocol/settings.ron) + web (localStorage)
      load/save, cfg-guarded, mirroring mod_prefs.rs. Add deps: dirs, ron, serde,
      wasm web-sys. Load at Startup into the resource; save on Settings change.
- [x] 5. nova_menu: extract a shared settings-panel content builder (AUDIO
      volume SLIDER via bevy's headless `ui_widgets::Slider`, GRAPHICS quality
      segmented buttons, CONTROLS read-only keybind list). Replace the MainMenu
      Settings stub with it. Reads/writes the resources; interactions apply
      immediately.
- [x] 6. nova_menu: add a Settings entry to the PAUSE menu that opens the SAME
      shared panel (DespawnOnExit(Paused)); Back returns to the pause menu.
- [x] 7. Verify: full check suite (fmt/check/tests, workspace as CI does), plus
      an eyeball of the rendered panel from both entry points where practical.
- [x] 8. Docs: CHANGELOG line (player-facing), and update the low-end task
      20260525-133013 + plan doc noting the tier seam this task leaves open.
