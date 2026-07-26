# NOVA OS sound: terminal SFX + ambient CRT bed

- STATUS: OPEN
- PRIORITY: 42
- TAGS: v0.9.0, feature, ui, hud, audio

## Story

The PoC terminal is fully audible: key clicks while typing, a backspace tick,
an enter thunk, confirmation beeps, an error buzz, an objective chime, a
degauss coil thump on app launch, power up/down sweeps, and a live-tube ambient
bed (a ~7.8 kHz flyback whine + 50 Hz mains hum) while the screen is on. The
in-game NOVA OS is silent. Give the computer its voice through the existing
audio conventions (`nova_gameplay::audio`, `NovaAudioPlugin`, the `UiSfx`
engine-chrome bank under `assets/sounds/`).

## Flow State

- FLOW STEP: PLANNING

## Steps

- [ ] Record a short DECISION.md on the asset route: pre-rendered WAVs in
      `assets/sounds/` (matches the `UiSfx` bank convention; can be synthesized
      OFFLINE by a small script that mirrors the PoC's WebAudio recipes) vs
      runtime synthesis (a bevy `Decodable` source). Prefer the WAV route
      unless there is a strong reason - every existing UI cue ships that way,
      and the hard web-parity requirement on this family (see
      `tasks/20260726-193233/DECISION.md`) makes WASM-side `Decodable` an
      extra risk the WAV route simply does not have.
- [ ] Write the offline generator - `scripts/gen-nova-os-sfx.py` (numpy or
      stdlib `wave`) - that renders each cue from the PoC's synth recipes
      (filter type/frequency/Q, gain envelope, pitch slides) into
      `assets/sounds/nova_*.wav`. Check IN both the script and the WAVs, so a
      cue is retunable by editing recipe constants and re-running.
- [ ] Extend `UiSfx` + `UI_SFX_FILES` in
      `crates/nova_gameplay/src/audio.rs` with the new keys and their volume
      constants: key click (typing), backspace, enter submit, command-ok beep,
      error buzz, autocomplete tick, app-launch degauss coil, computer open
      (power-up sweep), computer close (power-down sweep).
- [ ] Fire the cues from the terminal seams in `drawer.rs`:
      keystroke/backspace in `handle_terminal_keyboard`, ok/error on submit
      (`TerminalCommandResult` branch), tick on completion, coil on app
      launch/exit, sweeps on the open/close transitions - synced with
      193233's power collapse so the sweep and the raster animation read as
      one event.
- [ ] Ambient bed: a quiet `nova_bed.wav` loop entity spawned on
      `OnEnter(PauseStates::Drawer)`, despawned on exit, playing on the REAL
      clock. Follow the `audit-state-gates-on-new-entry-path` lesson: grep the
      freeze wiring (the thruster/RCS loop-freeze from 20260724-102304 R1.1)
      and prove the bed is exempt from the loop-pause the way drawer UI is,
      not silenced by it.
- [ ] Objective-complete chime while the computer is open reuses
      `UiSfx::ObjectiveComplete`; make sure it does not double-fire with the
      existing HUD cue.
- [ ] Gate every cue on `NovaOsMonitorSettings::sound_enabled` (default ON per
      `tasks/20260726-214617/DECISION.md`) and the master-volume path; per the
      owner's gate call this is the FULL PoC treatment - per-keystroke clicks
      included - with the chin SND button and master volume as the opt-outs.
- [ ] Volumes in the informational-tick band (see the volume constants in
      `audio.rs`); typing clicks especially quiet and throttled (`SfxThrottle`
      precedent) so held keys do not machine-gun.
- [ ] Tests: cue fires on submit/error/open/close; bed entity exists only
      while the drawer is open; SND=off silences everything; the generator
      script is deterministic (same recipes -> same bytes) so CI can diff.
- [ ] Verify on the web build too (the `wav` feature already ships the
      decoder): a trunk run with an in-browser listen. Record the work +
      self-reflection in `tasks/20260726-214639/NOTES.md`.

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
