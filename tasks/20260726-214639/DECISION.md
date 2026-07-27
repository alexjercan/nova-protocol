# DECISION - NOVA OS sound asset route

- STATUS: ACCEPTED

## Context

The NOVA OS terminal needs a full voice (key clicks, backspace, enter, ok beep,
error buzz, autocomplete tick, degauss coil, power up/down sweeps, ambient CRT
bed). The PoC (`examples/ui/nova_os_terminal_poc.html`) synthesizes every cue at
runtime via WebAudio. Two routes to bring them in-game:

1. **Pre-rendered WAVs** in `assets/sounds/`, synthesized OFFLINE by a script
   that mirrors the PoC's WebAudio recipes.
2. **Runtime synthesis** via a bevy `Decodable` audio source.

## Decision: pre-rendered WAVs (route 1)

- Every existing UI cue in the game ships as a WAV loaded through the `UiSfx` /
  `SoundBank` convention (`assets/sounds/<name>.wav`). Matching that keeps the
  new cues in one playback path (master volume, the `PlaySfx` command, the
  loop-entity pattern for the bed) with zero new engine machinery.
- The hard web-parity requirement on this family
  (`tasks/20260726-193233/DECISION.md`) makes a WASM-side `Decodable` source an
  extra risk the WAV route simply does not have: the `wav` decode feature
  already ships and is proven on web.
- The cues are retunable: `scripts/gen-nova-os-sfx.py` holds the synth recipes
  (filter type/freq/Q, gain envelope, pitch slides) as constants; edit a recipe
  and re-run to regenerate the WAV. Both the script and the WAVs are checked in.
- Determinism: the generator seeds its noise RNG with a fixed seed, so the same
  recipes render the same bytes and CI can diff a regeneration.

Owner scope call (2026-07-26 plan gate): FULL PoC treatment including
per-keystroke clicks, SND default ON, with the chin SND button
(`NovaOsMonitorSettings::sound_enabled`, landed by 20260726-214617) and master
volume as the opt-outs.
</content>
