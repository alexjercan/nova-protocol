# NOVA OS sound: terminal SFX + ambient CRT bed

- STATUS: OPEN
- PRIORITY: 43
- TAGS: v0.9.0,feature,ui,hud,audio

## Story

The PoC terminal is fully audible: key clicks while typing, a backspace tick,
an enter thunk, confirmation beeps, an error buzz, an objective chime, a
degauss coil thump on app launch, power up/down sweeps, and a live-tube ambient
bed (a ~7.8 kHz flyback whine + 50 Hz mains hum) while the screen is on. The
in-game NOVA OS is silent. Give the computer its voice through the existing
audio conventions (`nova_gameplay::audio`, `NovaAudioPlugin`, the `UiSfx`
engine-chrome bank under `assets/sounds/`).

## Steps

- [ ] Record a short DECISION.md on the asset route: pre-rendered WAVs in
      `assets/sounds/` (matches the `UiSfx` bank convention; can be synthesized
      OFFLINE by a small script that mirrors the PoC's WebAudio recipes) vs
      runtime synthesis (a bevy `Decodable` source). Prefer the WAV route
      unless there is a strong reason - every existing UI cue ships that way.
- [ ] One-shot cues: key click (typing), backspace, enter submit, command-ok
      beep, error buzz (unknown command / rejected args), autocomplete tick,
      app-launch degauss coil, computer open (power-up sweep), computer close
      (power-down sweep).
- [ ] Ambient bed: a quiet loop while the drawer is open; starts on open,
      stops on close, and must be pause-safe (the drawer freezes virtual time -
      the thruster/RCS loop-freeze fix from 20260724-102304 R1.1 is the
      precedent to follow).
- [ ] Objective-complete chime while the computer is open reuses
      `UiSfx::ObjectiveComplete`; make sure it does not double-fire with the
      existing HUD cue.
- [ ] Gate every cue on the SND toggle (`NovaOsSoundEnabled`, task
      20260726-214617) and the game's master volume path; pick and record the
      default (PoC defaults OFF because of the browser gesture rule - the game
      has no such constraint, so ON is likely right).
- [ ] Volumes in the informational-tick band (see the volume constants in
      `audio.rs`); typing clicks especially quiet and throttled (`SfxThrottle`
      precedent) so held keys do not machine-gun.
- [ ] Tests: cue fires on submit/error/open/close; bed entity exists only
      while the drawer is open; SND=off silences everything.

## Definition of Done

- Typing, submitting, erroring, opening and closing the computer each produce
  their cue, and an ambient bed hums while it is open. (test:
  `nova_os_sound_cues_fire_on_terminal_events`; manual: an in-game listen
  against the PoC with SND on)
- The bed starts/stops with the drawer and survives the virtual-time freeze
  without stutter or runaway. (test: `nova_os_ambient_bed_tracks_drawer_state`)
- SND off = fully silent NOVA OS. (test: `nova_os_snd_off_silences_cues`)

## Notes

- Pairs with: 20260726-214617 (the SND toggle; this task consumes the
  resource, that task ships the button).
- PoC reference: the `Sound` IIFE in `examples/ui/nova_os_terminal_poc.html` -
  each cue's synth recipe (filter type/frequency/Q, duration, gain envelope,
  pitch slides) is the offline-render spec.
- Epic: `tasks/20260725-104330/TASK.md`.
