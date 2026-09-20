# Export an SFX sidecar from a capture loop

- STATUS: OPEN
- PRIORITY: 0
- TAGS: backlog

Record what a capture loop HEARD, beside the frames it recorded, so an editor
can rebuild the mix from the engine's own numbers.

A loop capture is silent today. `LoopCapturePlugin` stages one PNG per rendered
frame and encodes a webm; nothing observes the audio. Live audio capture is not
an option: an armed run pins `TimeUpdateStrategy::ManualDuration` to `1/fps` and
renders about 38x slower than real time, so there is no real-time signal to
record. The loop must be reconstructed from data instead.

## User facts

- Export audio from a capture loop in the same shape as the frame export: a
  named shot produces its media next to the frames.
- The game mixes by distance, so position has to survive into the export.
- Prove it outside the engine first, because no Nova release is due. Adopt it
  here afterwards.
- Backlog. Land it in the version after v0.14.0.

## Status: the producer exists and works, outside the engine

Proven end to end on 2026-09-20 in the content-machine capsule, against
v0.14.0 with NO engine change:
`content-machine:capture/nova-protocol/v0.14.0/src/sfx_sidecar.rs`.

Five loops of the store trailer's cold open recorded sidecars beside their
webms - 146 to 271 frames each, 177 to 765 voices each, every named sample
copied out. The rendered cut carries the mix: mean -17.8 dB, peaks -0.6 dB,
and the dynamics track the picture (storm and collapse at -21.8 dB, the lance
and the detonation at -15.5 dB).

So this task is no longer "build an exporter". It is "adopt the proven one and
remove the two things a producer outside the engine cannot do".

## Agent findings

Code evidence, all verified 2026-09-20.

- `crates/nova_gameplay/src/audio/voice.rs:324` `drive_sfx_voices` already
  resolves the complete per-frame mix. For each `SfxVoice` it calls
  `place_voice(voice, mixer.bus_gain(voice.route), listener, &point)` and, at
  `voice.rs:366`, writes `placement.gain * master` to the sink.
- `VoicePlacement` (`voice.rs:159`) is `{ level, gain, emitter }`. `level` is
  the bus gain and distance rolloff before pan compensation and is what the
  audible-threshold gate and the voice cap rank on; `gain` folds the pan
  compensation in. The distance mixing the owner asked about is already in
  these numbers.
- `SfxListenerMarker` sits on the scenario camera
  (`crates/nova_scenario/src/loader/lifecycle.rs:421`), which is the camera a
  trailer rig poses. The ear is already where the shot is.
- `SfxVoice` / `PlaySfx` (`crates/nova_gameplay/src/audio/`) are the only way
  game code makes a sound; a test fails if anything constructs a raw
  `AudioPlayer`. One seam to observe, no second path.
- `crates/nova_autopilot/src/loops.rs` owns `LoopRecorder` and its phases
  (`Recording` / `Draining`), the staging directory
  `.loop-frames/<loop>/`, and `loop_capture_drive` in `Last`.
- `nova_autopilot` depends on bevy alone, and `nova_gameplay` does not depend
  on it. Neither crate can see the other. `nova_debug` already depends on both
  and owns the capture harness, so the exporter goes there and adds no new
  dependency edge.

Two findings that the capsule proof turned up, both now settled:

- The sink is NOT a usable source. An armed run is `HarnessMute`d
  (`settings.rs:396` logs it, `MasterVolume::output_gain` at `settings.rs:76`
  returns `0.0`), so `Mixer::master_gain` is zero and anything read back off a
  sink is silence. The number to export is the placement gain WITHOUT the
  master, which is also the right editorial answer: a trailer must not carry
  the capture machine's output volume.
- A voice's entity lifetime is a real-time artifact, not a simulation fact.
  `ONE_SHOT_SINK_GRACE` (`voice.rs`) retires a sinkless one-shot after two REAL
  seconds, and an armed capture runs about 38x slower than real time. Measured:
  `railgun_fire` produced exactly THREE rows. The format therefore plays a
  one-shot's whole sample and holds its last recorded gain, and only a loop is
  bounded by its envelope.

## Delivery

The capsule producer is the specification. Port it, and pay off the two debts
it could not.

1. `nova_gameplay`: publish the resolved mix. `drive_sfx_voices` writes a
   `VoiceMix { level, gain, emitter, capped }` component on every `SfxVoice`
   entity each frame, whether or not the entity has a sink. This retires the
   capsule's copy of `place_voice`, which is a re-composition of public parts
   and can drift from the engine silently.
2. `VoiceMix.capped` retires the capsule's second divergence. The voice cap's
   hysteresis constant is private, so an outside producer cannot rank exterior
   loops the way the engine does; a capped voice must export as silent.
3. `nova_debug`: move the writer beside the loop recorder, so `loop_start` and
   `loop_end` open and close the sidecar themselves. The capsule has to pair
   `sfx_start` / `sfx_end` onto the same steps by hand, and a bin that forgets
   one records a silent loop.
4. Keep, unchanged, what the capsule already got right: a per-loop voice
   counter that only goes up (an `Entity` index is reused after despawn, and
   entity-keyed rows splice two unrelated sounds into one clip); clip identity
   through `AssetServer::get_path`; samples copied out beside the sidecar so a
   capture is self-contained; a warning rather than an abort when one sample
   cannot be copied.

## Decisions

- The format is version 1 of the SFX sidecar, unchanged: one JSON object per
  line, a header `{version, loop, fps, frames}` then `{f, v, gain}` rows, with
  `clip` and `looping` on a voice's first row only. The contract and the
  reasons behind each field are in `content-machine:docs/sfx-sidecar.md`. The
  reader is `content-machine:editor/src/content_machine/sfx.py`. Both ends are
  landed and tested, so the engine port must not change the format - it must
  produce the same bytes the capsule does.
- Mono only in version 1. Bevy pans through `SpatialAudioSink` and the sidecar
  carries one gain per frame. Version 2 adds a listener-relative azimuth from
  `VoicePlacement.emitter`; `emitter` is in `VoiceMix` from the start so that
  version needs no further gameplay change.
- Buffer in memory and write once at loop close. `frames` belongs in the header
  and is not known until the loop ends, and the cost is small: 900 frames of 20
  voices is about 1 MB.

## Verification

Named behavior: a recorded loop's sidecar reproduces the gain the mixer
resolved, for every frame each voice was alive.

- An asserted example under `nova_debug`: spawn a listener and two voices at
  different distances, run a short armed loop, then assert the sidecar's header
  frame count matches the recorder's, that the far voice's envelope sits below
  the near one's on every shared frame, and that a voice which moves away has a
  falling envelope.
- A unit test for voice numbering: despawn a voice, spawn another that reuses
  the entity index, and assert the two get different `v` values.
- A differential check against the proven producer: re-record one trailer loop
  with the engine exporter and diff its sidecar against the capsule's. Only the
  capped voices may differ, and each difference must be a voice the cap
  silenced.

## Done when

- `loop_start` and `loop_end` open and close a sidecar by themselves; a bin
  needs no second pair of calls.
- An armed loop writes `<loop>.jsonl` and the samples it names.
- `content-machine:editor/src/content_machine/sfx.py` loads that file with no
  changes to the reader, and a render places the samples.
- A capped exterior loop exports as silent.
- The differential check against the capsule producer passes.
- `content-machine:capture/nova-protocol/<next version>/src/sfx_sidecar.rs` is
  DELETED when that capsule is cut, and its two "divergence" notes go with it.
- The asserted example and the unit tests above pass.
