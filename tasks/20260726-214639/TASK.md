# NOVA OS sound: terminal SFX + ambient CRT bed

- PRIORITY: 42
- TAGS: v0.9.0, feature, ui, hud, audio
- ACTIVITY: COMPOUNDING
- GATES: PLAN REVIEW RETRO
- RESOLUTION: DONE

## Story

The PoC terminal is fully audible: key clicks while typing, a backspace tick,
an enter thunk, confirmation beeps, an error buzz, an objective chime, a
degauss coil thump on app launch, power up/down sweeps, and a live-tube ambient
bed (a ~7.8 kHz flyback whine + 50 Hz mains hum) while the screen is on. The
in-game NOVA OS is silent. Give the computer its voice through the existing
audio conventions (`nova_gameplay::audio`, `NovaAudioPlugin`, the `UiSfx`
engine-chrome bank under `assets/sounds/`).

## Steps

- [x] Record a short DECISION.md on the asset route: pre-rendered WAVs (see
      `DECISION.md`).
- [x] Write the offline generator - `scripts/gen-nova-os-sfx.py` (stdlib `wave`,
      deterministic seeded noise) - rendering each cue from the PoC recipes into
      `assets/sounds/nova_*.wav`. Script + WAVs checked in; retunable via the
      recipe constants at the top.
- [x] Extend `UiSfx` + `UI_SFX_FILES` in `audio.rs` with the 10 new keys and
      their volume constants (informational-tick band).
- [x] Fire the cues from the terminal seams in `drawer.rs`:
      keystroke/backspace in `handle_terminal_keyboard`, enter thunk +
      ok/error/coil by submit outcome, tick on completion, coil on app
      launch/exit, sweeps on open/close (power-down on the close-request rising
      edge, synced with 193233's collapse).
- [x] Ambient bed: `nova_bed.wav` loop entity spawned on `OnEnter(Drawer)`,
      despawned on exit. Exempt from `pause_loops` BY CONSTRUCTION - that system
      queries only `ThrusterLoopSfx`/`RcsLoopSfx`, and the bed carries its own
      `NovaOsBedSfx` marker, so the freeze loop-pause never touches it
      (`audit-state-gates-on-new-entry-path`).
- [x] Objective-complete chime reuses `UiSfx::ObjectiveComplete`: NO duplicate
      added here, so the HUD's existing global cue is the only one - no
      double-fire.
- [x] Gate every cue on `NovaOsMonitorSettings::sound_enabled` and the
      master-volume path (one-shots via `SfxMasterVolume`; the bed via
      `apply_nova_os_bed_volume`). FULL PoC treatment incl. per-keystroke clicks.
- [x] Volumes in the informational-tick band; typing clicks quiet
      (`NOVA_OS_KEY_VOLUME` 0.10) and throttled (`NOVA_OS_KEY_MIN_INTERVAL`).
- [x] Tests: `nova_os_sound_cues_fire_on_terminal_events`,
      `nova_os_ambient_bed_tracks_drawer_state`, `nova_os_snd_off_silences_cues`,
      and `every_ui_sfx_key_has_a_file` (extended to the new cues). Generator
      determinism verified by two runs -> byte-identical (recorded in NOTES).
- [~] Verify on the web build (trunk + in-browser listen): MANUAL/owner
      acceptance - a headless session cannot listen. The `wav` decoder already
      ships on web and every cue rides the same path as existing UI SFX. Work +
      self-reflection recorded in `tasks/20260726-214639/NOTES.md`.

## Definition of Done

- Typing, submitting, erroring, opening and closing the computer each produce
  their cue, and an ambient bed hums while it is open. (test:
  `nova_os_sound_cues_fire_on_terminal_events`; manual: an in-game listen
  against the PoC with SND on)
- The bed starts/stops with the drawer and survives the virtual-time freeze
  without stutter or runaway. (test: `nova_os_ambient_bed_tracks_drawer_state`)
- SND off = fully silent NOVA OS. (test: `nova_os_snd_off_silences_cues`)

## Notes

- Owner scope call (2026-07-26 plan gate): FULL PoC treatment including
  per-keystroke clicks, SND default ON. The asset-route DECISION.md stays an
  implementation step (the recommendation is WAV; the decision record captures
  whatever the generator work confirms).
- Pairs with: 20260726-214617 (the SND toggle; this task consumes the
  resource, that task ships the button).
- PoC reference: the `Sound` IIFE in `examples/ui/nova_os_terminal_poc.html` -
  each cue's synth recipe (filter type/frequency/Q, duration, gain envelope,
  pitch slides) is the offline-render spec.
- Epic: `tasks/20260725-104330/TASK.md`.
