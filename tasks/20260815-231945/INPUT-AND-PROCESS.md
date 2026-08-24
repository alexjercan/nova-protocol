# Input and process audit (round 4 research for v0.12.0 planning, written 2026-08-24)

Scope: the flight rig, the keybind display mirror, the settings menu, input
injection, gamepad reads, and the headless step-mode hook. All references are
to the current master checkout.

## 1. The flight rig

The rig lives in `crates/nova_ship/src/input/player/flight_rig.rs`. The path
`nova_gameplay/src/input/player.rs` in task 20260714-001140 (TASK.md:65-66) is
stale: `crates/nova_gameplay/src/` has no `input/` directory at all. Task
20260820-174148 names the correct file.

Structure:

- `flight_input_rig()` at `crates/nova_ship/src/input/player/flight_rig.rs:97-260`
  returns one bundle: `FlightInputMarker` plus an `actions!` block. Spawned by
  `on_player_added_spawn_flight_input` (flight_rig.rs:69-84) on
  `Add<PlayerSpaceshipMarker>`, despawned at flight_rig.rs:262-274. One rig per
  player ship; no rig exists in the main menu.
- Actions are typed `#[derive(InputAction)]` structs:
  - `FlightBurnInput` (f32), `AutopilotStopInput`, `AutopilotGotoInput`,
    `AutopilotOrbitInput`, `AutopilotOffInput`, `RcsModifierInput` (bool),
    `RcsAimInput` (Vec2): flight_rig.rs:26-67.
  - `RadarHoldInput` / `RadarClearInput`:
    `crates/nova_ship/src/input/targeting/gesture.rs:20-29`.
  - `ComponentCycleNextInput` / `ComponentCyclePrevInput`:
    `crates/nova_ship/src/input/targeting/component_lock.rs:51-59`.
- Bindings are declared inline with the `bindings!` macro, keyboard and gamepad
  in one list per action:
  - Burn: W / Space / RightTrigger (flight_rig.rs:110-114).
  - Stop: X / East (123). Goto: G / North (132). Orbit: O / South (144).
    Off: Z / West (153).
  - Radar hold and clear: CtrlLeft / CtrlRight / DPadUp with `Hold` and `Tap`
    conditions sharing `RADAR_TAP_SECS` (164-190).
  - Component cycle: brackets / DPad left-right plus mouse wheel with
    `SwizzleAxis` / `Negate` / `Clamp` modifiers (201-225).
  - RCS modifier: ShiftLeft / ShiftRight / LeftTrigger2 (238-242).
  - RCS aim: raw `Binding::mouse_motion()` (255).
- Every action entity already carries a display `Name` such as
  `Name::new("Input: Flight Burn")` (flight_rig.rs:104, 117, 127, ...).

Other action rigs a naming pass must cover:

- Camera rig: `CameraInputRotate` (mouse motion + right stick), `FreeLookInput`
  (AltLeft / LeftTrigger), `CombatInput` (RMB / LeftTrigger2) at
  `crates/nova_ship/src/camera/rig.rs:175-208`, types at 239-250. A second
  copy of FreeLook/Combat bindings exists in the mode test fixture
  (`crates/nova_ship/src/camera/mode.rs:384-388`).
- Editor WASD camera: `crates/nova_ship/src/camera/wasd_controller.rs:36-115`.
- Scenario advance confirm: Enter / DPadDown at
  `crates/nova_scenario/src/loader/lifecycle.rs:226`.
- Per-section weapon rigs built from content `input_mapping`:
  `crates/nova_ship/src/input/player/weapons.rs` (components
  `SpaceshipThrusterInputBinding` etc., weapons.rs:14-16). These are dynamic,
  keyed by section entity, not a fixed action table.

Difficulty of stable string names: low for the fixed rigs. Each action is an
entity spawned in one known place; adding a small `InputActionName("main_drive")`
component (or a registry entry) per tuple is mechanical. Two real design points:

- The dispatcher cannot be purely name -> type generic, because
  `bevy_enhanced_input` actions are typed. The 0.26.0 crate ships a mock API
  (`bevy_enhanced_input-0.26.0/src/action/mock.rs:173-200`) that is generic over
  `A: InputAction`, so the practical shape is a name -> closure registry
  populated where each typed action is declared.
- Per-section weapon actions need derived names (for example
  `fire_<section_id>`), or stay out of phase 1.

## 2. The display mirror

`crates/nova_ship/src/input/reference.rs` is still the location. `KeybindEntry`
at reference.rs:20-30, the hand-authored `KEYBINDS` table at 36-133,
`keybind_reference()` at 135-138. `TODO(20260710-231927)` is in the module doc
at reference.rs:10.

Parity test: `reference_rows_track_the_flight_rig` at
`crates/nova_ship/src/input/player/hints.rs:352-449`. It spawns the real rig,
reads each action's first keyboard `Binding` live, and asserts the reference
row contains that key's label; `display_label` (hints.rs:388-403) panics on any
unmapped key so a remap cannot drift silently. Only the 8 flight/targeting rows
are pinned; the camera, comms, and system rows are untested static prose by
design (reference.rs:12-15).

Does naming rig actions delete the mirror? Partially, not by itself:

- The FLIGHT and TARGETING rows (reference.rs:36-98) become derivable from
  named actions, and the parity test dies with them. But reference.rs:3-9
  states the real blocker: the settings panel renders in the main menu, where
  no rig exists (the rig spawns with the player ship). Naming alone does not
  fix that; the settings screen needs a bindings source that exists outside a
  live scenario (a persistent bindings resource/registry the rig is built FROM,
  which is also what rebinding needs anyway).
- The CAMERA, COMMS, and SYSTEM rows are not enhanced-input actions today.
  Comms V/B are raw key reads (`crates/nova_hud/src/comms_panel.rs:275-278`),
  pause Esc/Start (`crates/nova_menu/src/pause.rs:55`), HUD backquote/Select
  (`crates/nova_hud/src/lib.rs:392-405`), NOVA OS Tab/RightThumb
  (`crates/nova_os_ui/src/terminal/input.rs:48-74`). The mirror only fully dies
  when those become named too, or their rows are declared fixed.
- A second hand-maintained mirror exists and is the bigger risk:
  `flight_rig_reserved_sources()` at
  `crates/nova_ship/src/input/player/hints.rs:164-195` hardcodes every rig
  source for conflict checking (pinned by
  `flight_rig_reserves_exactly_these_sources`,
  `crates/nova_ship/src/input/player/intent.rs:611`). Once bindings are
  player-mutable this list must be computed from the live rig, not a constant.
- `crates/nova_hud/src/key_glyphs.rs` also references TODO(20260710-231927)
  (key-glyph coverage) and will need the same live source.

## 3. The settings menu today

- Shared body: `build_settings_body` at
  `crates/nova_menu/src/settings.rs:72-217`. Sections: AUDIO master-volume
  slider (bevy `ui_widgets::Slider`, settings.rs:109-131, mirrored to
  `MasterVolume` by `on_volume_slider_change` 319-327, label sync 334-343);
  GRAPHICS preset segmented row (153-166); CONTROLS read-only keybind reference
  rendered from `keybind_reference()` (170-192, row builder 347-380);
  INTERFACE UiSkin segmented row (199-216).
- Two entry points, one body: main-menu overlay
  (`crates/nova_menu/src/menu_ui.rs:146-191`, `SettingsPanel`, 460px panel,
  toggled by `on_settings` settings.rs:31-46) and pause overlay
  (`crates/nova_menu/src/pause.rs:272-319`, `PauseSettingsPanel`).
- Widgets: bevy `ui_widgets` `Activate` observers throughout. Menu buttons are
  `nova_ui::widget::menu_button` wrapped in `crates/nova_menu/src/widgets.rs:37-51`;
  setting rows are `segmented_option` + `ButtonValue<T>` + `Selected` driven by
  the app-global `button_on_setting::<T>` observer (imported at
  `crates/nova_menu/src/lib.rs:25`).
- Persistence (already survives restarts): `PersistedSettings` at
  `crates/nova_menu/src/settings_store.rs:16-36`, RON via
  `nova_assets::persist`, key "settings" (settings_store.rs:102; native config
  dir, wasm localStorage per 100-102). Startup load
  `load_persisted_settings` settings.rs:224-237; debounced save
  `persist_settings_on_change` settings.rs:252-279 (15-frame debounce, 244);
  exit flush 295-311. Serde defaults make adding a `bindings` field
  non-breaking; the partial-file test pattern to copy is
  settings_store.rs:213-230.
- Existing tab UI to reuse: the mods screen has real tabs. `ModsActiveTab`
  resource at `crates/nova_menu/src/mods.rs:51-52`, click handler `on_mods_tab`
  at mods.rs:226-249 (writes the resource, moves `Selected`, list refresh keys
  off the resource change). Visuals: `segmented_container` /
  `segmented_option` at `crates/nova_ui/src/widget/segmented.rs:34-50`. A
  tabbed settings panel is this pattern verbatim.
- Existing rebind flow to reuse (two copies already exist, both for weapon
  sections, both keyboard/mouse only):
  - Editor: `crates/nova_editor/src/keybind.rs`. `EditorRebind` armed-target
    resource with an arming-click release guard (keybind.rs:16-23),
    `apply_section_rebind` (197-279): captures the next key or mouse press,
    Escape cancels (217-221), waits out the arming click (224-229), refuses
    flight-rig conflicts via `binding_conflict` (184-190) and stays armed for
    another try. Behavior tests at 281-623.
  - NOVA OS ship app: `crates/nova_os_ui/src/ship/rebind.rs`
    (`apply_ship_rebind`, same shape, `reserved_conflict` at the top of the
    file).
  Neither captures gamepad buttons. Gamepad capture needs a
  `ButtonInput<GamepadButton>` (or gamepad-entity) read added to the same
  capture loop; the conflict rule must come from the live rig (see section 2).

What a tabbed settings + rebinding flow needs, concretely: a tab strip
(mods pattern), a CONTROLS tab whose rows come from named actions instead of
`KEYBINDS`, per-row click-to-arm (EditorRebind pattern generalised from
section entities to action names), capture extended to gamepad, conflict check
against live bindings, a persisted `bindings: map<action_name, bindings>`
field in `PersistedSettings`, and a startup apply that patches the rig's
`Binding` child entities (bindings are entities under each action's
`Bindings` relationship, as `update_flight_verb_hints` reads them at
hints.rs:227-242).

## 4. Input injection

- `crates/nova_autopilot/src/input.rs`:
  - `press_key` / `release_key` (input.rs:74-85) write
    `ButtonInput<KeyCode>::press/release` directly, so the press is
    `just_pressed` the same frame.
  - `press_mouse` / `release_mouse` (124-131) go through `set_mouse_button`
    (460-482): writes `ButtonInput<MouseButton>` directly plus the
    `WindowEvent::MouseButtonInput` wrapper for picking, deliberately NOT the
    raw `MouseButtonInput` message (would re-apply a frame late).
  - `type_text` (99-121), `scroll_lines` / `scroll_pixels` via `turn_wheel`
    (143-190, writes both `MouseWheel` and the `WindowEvent` wrapper),
    `move_cursor` (writes `Window::cursor_position` plus `CursorMoved`).
- Scheduling: the driver runs in `PreUpdate` after `InputSystems`
  (`crates/nova_autopilot/src/autopilot.rs:390`), so synthesized input lands
  after bevy clears `just_*` edges and before game systems read it
  (input.rs:20-25). A named-action dispatcher can share this path exactly:
  `apply(name, phase)` resolves name -> live `Binding` -> the matching
  press/release helper, injected at the same point. Caveats: wheel/motion
  bindings and gamepad-only bindings have no `press_key` equivalent (gamepad
  synthesis is deliberately absent, input.rs:27-29); for those the
  enhanced-input mock API (section 1) is the cleaner injection seam.
- Hardcoded `KeyCode` in examples the dispatcher replaces:
  - `examples/screenshots/shared/hollow.rs:509-523` (`hold_radar` /
    `release_radar` press `KeyCode::ControlLeft` raw), used by
    `examples/screenshots/screenshot_combat_lock.rs:119` and the other
    screenshot ranges through `hollow`.
  - 38 direct `press_key(KeyCode..)` / `press_mouse(MouseButton..)` call sites
    across 7 example files: `examples/playable/widget_zoo.rs`,
    `examples/systems/system_nova_os.rs`,
    `examples/systems/system_hud_indicators.rs`,
    `examples/screenshots/shared/ui_walk.rs`,
    `examples/screenshots/loop_player_flight.rs`,
    `examples/systems/system_player_path.rs`,
    `examples/systems/system_ship_editor.rs`.

## 5. Existing raw gamepad reads

All are `Option<Res<ButtonInput<GamepadButton>>>` guards (headless apps have no
input plugin):

- `crates/nova_menu/src/pause.rs:30`, used at 52-55: pause toggle on Start.
- `crates/nova_hud/src/lib.rs:395`, used at 397-401: HUD level cycle on Select.
  (Task 20260714-001140 says `nova_gameplay/hud`; the crate is `nova_hud`.)
- `crates/nova_editor/src/lib.rs:344`, used at 348-350: back-to-editor on
  LeftThumb (F1/L3).
- `crates/nova_editor/src/placement.rs:89,110,725`: `placement_binds` captures
  a held pad button as a freshly placed section's binding (defaults
  RightTrigger / RightTrigger2, placement.rs:104-127).
- `crates/nova_os_ui/src/terminal/input.rs:50` (`toggle_nova_os`, RightThumb,
  56-58) and input.rs:85 (`close_nova_os_from_menu_keys`, Start, 103-105).

Plus the enhanced-input gamepad bindings inside the rigs (sections 1 and 2).
Implication for v0.12.0: settings rebinding on the rig covers the flight pad
bindings; the raw reads above are system chords outside any rig, so either they
get named actions too, or the settings screen lists them as fixed.

## 6. Step-mode hook context

- `AppBuilder::assemble` at `crates/nova_core/src/lib.rs:186-238`. The
  headless branch (202-230): disables `WinitPlugin` (210), adds
  `ScheduleRunnerPlugin::default()` (211) - which is
  `RunMode::Loop { wait: None }` (bevy_app-0.19.0/src/schedule_runner.rs:33) -
  and adds `SyncWorldPlugin` (229) with the documented undrained-queue leak
  (~24 bytes per synced spawn/removal, lib.rs:213-228). That leak is
  load-bearing for long driven sessions, as 20260820-174148 already flags.
- `NORENDER_ENV = "NOVA_NORENDER"` at lib.rs:455; `headless()` is
  unconditional (182-184).
- Hook options for "advance N ticks, emit snapshot, block for input":
  1. Replace the plugin at lib.rs:211 with a custom runner
     (`app.set_runner`): tick N, call `capture_snapshot(&mut World)`
     (`crates/nova_probe/src/capabilities/snapshot.rs:4-9`, no IO by design;
     its module doc at snapshot.rs:7-9 already anticipates exactly this mode),
     write the line, block on the channel.
  2. Keep `ScheduleRunnerPlugin` and block inside an exclusive system in
     `First`. Simpler, but the block happens mid-frame and the runner still
     owns exit handling.
  Either way, blocking stretches wall-clock time, so determinism requires
  `TimeUpdateStrategy::ManualDuration` while stepping - the pattern the test
  suite already uses (for example `crates/nova_hud/src/turret_lead.rs:372`).
  Note the known wrinkle: the first manual update is dt 0.
- Injection point for channel input is already proven by the autopilot:
  `PreUpdate` after `InputSystems` (`crates/nova_autopilot/src/autopilot.rs:390`).

Not confirmed / open:

- Whether the enhanced-input mock API composes with `Hold` / `Tap` conditions
  (the radar gesture) the same way a real key press does. Needs a spike; the
  press-the-bound-source path avoids the question for keyboard bindings.
- How bevy 0.19 exposes gamepad state for synthesis (the raw reads use a
  global `ButtonInput<GamepadButton>` resource; whether writes there reach the
  enhanced-input gamepad bindings was not verified).
- No stdin reader exists anywhere yet (only the snapshot module's doc mentions
  it).

## What this means for v0.12.0

1. Create the bindings registry first, not the names first. A resource that
   owns per-action `(name, keyboard binds, gamepad binds)` and BUILDS the
   flight rig from it solves three problems at once: the settings menu can
   read bindings in the main menu (no rig there today), rebinding has a single
   write target, and persistence is one serde field. Naming without the
   registry still leaves reference.rs alive.
2. Scope phase-1 names to the fixed rigs: flight + targeting (11 actions),
   camera (3), scenario advance (1). Leave per-section weapon actions and the
   raw-read system chords (pause, HUD, NOVA OS, comms) for a later pass; list
   them as fixed rows in the settings screen.
3. Replace `flight_rig_reserved_sources()` (hints.rs:164-195) with a
   registry-derived computation in the same change that makes bindings
   mutable, or the editor/NOVA OS conflict checks go stale on the first remap.
4. Build the settings tabs from the mods-tab pattern (`ModsActiveTab` +
   `on_mods_tab`) and `segmented_container`; build the rebind capture from
   `apply_section_rebind`, generalised to action names and extended with a
   gamepad-button capture branch. Both flows are tested code already in tree.
5. Persist bindings as a new serde-defaulted `PersistedSettings` field keyed
   by action name; add the partial-file test copying
   settings_store.rs:213-230. Apply loaded bindings by patching the rig's
   `Binding` child entities on rig spawn.
6. Route the dispatcher through the autopilot injection point (`PreUpdate`
   after `InputSystems`): `apply(name, phase)` resolves the registry and calls
   the existing press/release helpers for keyboard/mouse sources. Spike the
   enhanced-input mock API before committing to it for wheel/motion/gamepad
   sources.
7. Port `hollow::hold_radar` / `release_radar` and the 7 hardcoded example
   files to the dispatcher as the proof-of-done for phase 1 (matches the task's
   own DoD).
8. Step mode: prefer the custom-runner option at nova_core lib.rs:211 with
   `TimeUpdateStrategy::ManualDuration`, and treat the `SyncWorldPlugin` leak
   (lib.rs:213-228) as a blocker to re-check before advertising long driven
   sessions.
