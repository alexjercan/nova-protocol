# NOVA OS sound - work notes

## What was built

The in-game NOVA OS terminal now has the PoC's full voice.

- **Asset route**: pre-rendered WAVs (see DECISION.md). `scripts/gen-nova-os-sfx.py`
  (stdlib-only, no numpy) renders 10 `nova_*.wav` cues from the PoC `Sound` IIFE
  recipes: hand-rolled RBJ biquad filters for the noise bursts, oscillators with
  exponential pitch slides + envelopes for the tones. Deterministic (fixed noise
  seed) so CI can diff a regeneration.
- **audio.rs**: 10 new `UiSfx` variants + `UI_SFX_FILES` entries (loaded
  automatically by `nova_assets::register_sounds`, which iterates the array) +
  tick-band volume constants. `every_ui_sfx_key_has_a_file` extended to cover all
  15 variants (it was non-exhaustive, missing even CommsLine).
- **drawer.rs cue wiring**: `submit`/`complete`/`exit_app` now return a semantic
  result (`TerminalSubmitOutcome`, `bool`) so the pure model stays audio-agnostic
  and the bevy layer picks the cue. `handle_terminal_keyboard` fires: nova_key on
  a keystroke (throttled), nova_back on backspace/delete, nova_enter on submit
  then nova_ok/nova_error/nova_coil by outcome, nova_tick when Tab actually
  advances the prompt. `on_nova_os_app_close` + `close_drawer_from_menu_keys` fire
  nova_coil on a real app exit. `start_nova_os_sound` (OnEnter Drawer) plays
  nova_powerup + spawns the bed; `play_nova_os_power_down` fires nova_powerdown on
  the close-request rising edge (synced with the raster collapse, which also
  starts then - not OnExit, which is the end of the collapse).
- **Ambient bed**: a `NovaOsBedSfx` loop entity. Exempt from the sim-freeze
  loop-pause BY CONSTRUCTION: `audio::pause_loops` queries only `ThrusterLoopSfx`
  / `RcsLoopSfx`, so the bed's own marker keeps it playing while the drawer is
  open. `apply_nova_os_bed_volume` drives its sink from MasterVolume + the SND
  toggle (0 when muted), so toggling SND live silences it without a despawn.
- **Gating**: every one-shot goes through `play_nova_os_cue`, which early-returns
  when `sound_enabled` is false; master volume rides `SfxMasterVolume` (one-shots)
  and `apply_nova_os_bed_volume` (bed).

## Decisions / reasoning

- Enter thunk + outcome cue (ok/error/coil) both play on a submit: the Story
  lists "an enter thunk, confirmation beeps, an error buzz" as distinct cues, so
  the thunk is the submit cue and the outcome layers on top. The recipes differ
  in pitch so they read as one "thunk-beep".
- Power-down on the `DrawerCloseTransition::closing` RISING EDGE, not OnExit:
  OnExit(Drawer) fires only after the collapse animation finishes, but the sweep
  must sync with the collapse that STARTS when close is requested.
- Objective chime: reused the HUD's existing global `ObjectiveComplete` cue by
  adding NOTHING here - the surest way to avoid a double-fire.

## Bugs hit

- `AudioSink::set_volume` needs `&mut AudioSink` (not `&`, unlike `pause()`), so
  `apply_nova_os_bed_volume` queries `&mut AudioSink`.
- `Volume` is `bevy::audio::Volume`, not in the prelude - needed an explicit
  import (matching `audio.rs`).
- The key-click throttle used a `Local<f32>` defaulting to 0.0, which would
  wrongly throttle the FIRST click when `elapsed_secs() < 0.03`. Switched to
  `Local<Option<f32>>` so the first click always fires.

## Verification

- `nova_os_sound_cues_fire_on_terminal_events`, `nova_os_ambient_bed_tracks_drawer_state`,
  `nova_os_snd_off_silences_cues`, `every_ui_sfx_key_has_a_file`: 4 passed.
- Generator: ran twice, all 10 WAVs byte-identical (determinism for CI diff).
- Regression: nova_gameplay terminal/drawer/nova_os tests + `cargo check --workspace`.
- Web listen: MANUAL owner acceptance (headless can't listen); the `wav` decoder
  ships on web and cues ride the same path as existing UI SFX.

## Self-reflection

- Delegating the WAV generator to a subagent in parallel with the Rust wiring was
  a good split - the generator is a self-contained, verifiable artifact.
- Reused `objective_feedback::sfx_app`'s SoundBank + PlaySfx-capture pattern for
  the cue tests (`reuse-known-good-stack`), which made the "which cue fired"
  assertions mechanical and avoided an audio device.
- Threading the new `Res<NovaOsMonitorSettings>` into `handle_terminal_keyboard`
  meant the shared `terminal_command_app` rig needed the resource; caught it by
  reasoning about the system's new params up front and adding it to
  `init_terminal_input_resources` (one edit, all existing terminal tests kept
  green) rather than discovering panics test-by-test.
</content>
