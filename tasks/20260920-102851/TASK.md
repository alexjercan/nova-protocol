# Export an SFX sidecar from a capture loop

- STATUS: CLOSED
- PRIORITY: 60
- TAGS: v0.15.0,capture,audio,tooling

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
- Land it in the version after v0.14.0.

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
   `VoiceMix` component on every `SfxVoice` entity each frame, whether or not
   the entity has a sink (fields in the 2026-09-24 decision below). This
   retires the capsule's copy of `place_voice`, which is a re-composition of
   public parts and can drift from the engine silently.
2. `VoiceMix.capped` retires the capsule's second divergence. The voice cap's
   hysteresis constant is private, so an outside producer cannot rank exterior
   loops the way the engine does; a capped voice must export as silent.
3. `nova_debug`: observe the loop recorder's lifecycle events, so `loop_start`
   and `loop_end` open and close the sidecar themselves. The capsule has to pair
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
  landed and tested, so the engine port must preserve the reader's v1 format.
  The v0.14.0 producer is tied to its frozen release, not a byte-for-byte
  acceptance oracle for a different game build.
- Mono only in the version 1 sidecar. Bevy pans through `SpatialAudioSink` and
  the sidecar carries one gain per frame. The embedded WebM track is stereo;
  see the 2026-09-24 decision. Version 2 adds a listener-relative azimuth from
  `VoicePlacement.emitter`; `emitter` is in `VoiceMix` from the start so that
  version needs no further gameplay change.
- Buffer in memory and write once at loop close. `frames` belongs in the header
  and is not known until the loop ends, and the cost is small: 900 frames of 20
  voices is about 1 MB.

## Decision: Nova v0.15 owns the sounded WebM (2026-09-24)

Owner decision. The normal `<loop>.webm` carries VP9 video and an Opus track;
there is no second preview file. The sidecar stays, unchanged at version 1, as
editorial data. content-machine v0.14.0 is frozen and gets no compatibility
edit.

Implementation boundary:

- `nova_gameplay` publishes
  `VoiceMix { level, gain, channel_gains, emitter, speed, capped, paused }`
  on every `SfxVoice` each frame, through `set_if_neq`. `gain` and
  `channel_gains` are pre-master. `channel_gains` is `[gain; 2]` for a flat
  voice and `gain` times `pan_gains(bearing)` for a positional one. `paused`
  marks a World-bus voice that a frozen sim (`PauseStates`) holds.
- `nova_autopilot` stays Bevy-only. A WebM loop triggers
  `LoopCaptureStarted`, one `LoopCaptureFrame` immediately before each frame
  request, and `LoopCaptureEnded`. Sheets trigger none. An observer may stage
  stereo 44.1 kHz `f32le` PCM at `audio_path`, which the encode muxes as Opus
  at 48 kHz, or veto the loop through `failure`, which aborts before the
  encode. Without an adapter the WebM stays silent.
- `nova_debug` owns the adapter through `DebugPlugin`, on host targets only
  (the render runs ffmpeg from a thread). It samples `VoiceMix` on each
  `LoopCaptureFrame`, then at `LoopCaptureEnded` renders the PCM offline from
  the loaded sample bytes (ffmpeg decode, per-frame channel gains and speed,
  capped or paused frames silent with a held playhead and gain 0 rows,
  one-shots play out and hold their last row, loops end with their rows),
  copies the samples (a copy failure warns; any other failure, including a
  voice with no asset path, a clip path that is not plain relative names, a
  clip path inside the reserved `.loop-frames` staging subtree, or an existing
  symlink, non-directory parent or uninspectable entry on a clip's destination
  under the capture directory, vetoes the loop before any write; a link
  created during the copy or a hardlinked sample file is not detected), and
  publishes `<loop>.jsonl` last through a rename. Speed is internal to the
  render and never enters the sidecar. Loop names and clip paths are
  JSON-escaped; an ordinary name or path keeps its bytes.
- The encode runs after the sidecar is published, so an ffmpeg mux failure
  fails the run but leaves the complete sidecar and samples.

## Decision: exact stereo and over-only peak attenuation (2026-09-24)

Owner decision, after review found the first sounded WebM decoded at
+10.07 dBFS: the render applied the pan-compensated sink gain to both
channels without the pan itself.

- The sidecar `gain` stays the pre-master sink gain. It is the editorial
  contract in `content-machine:docs/sfx-sidecar.md` and must match the frozen
  capsule producer.
- The embedded WebM uses exact stereo. A positional voice plays the sum of
  its sample's channels at `channel_gains`, as rodio 0.22.2's spatial sink
  does: its `ChannelVolume` discards the divide by the channel count. ffmpeg
  decodes a spatial voice's sample with `pan=stereo|FL=FL+FR+FC|FR=FL+FR+FC`,
  so mono stays at unity and stereo becomes `FL + FR`. A flat voice scales its
  own channels. Decodes are keyed by clip and downmix, so a sample that a flat
  and a spatial voice both play decodes twice at loop close. Shipped samples
  are mono, so the sum changes no shipped mix.
- After the sum, if the absolute PCM peak is over -1 dBFS, one loop-wide
  factor puts it at -1 dBFS before the Opus encode. A quieter loop is never
  boosted. Opus can overshoot the ceiling on decode, so the ceiling is not a
  decoded-peak guarantee. The log names the
  original peak and the attenuation.

Armed `loop_torpedo_blast` on lavapipe, 2026-09-24: VP9 and Opus, 96 video
frames and a 96-frame header, 3.2 s of stereo audio. The decoded peak is
-9.6 dBFS (L -9.6, R -9.9), with no 0 dB histogram bin, so the run applied no
attenuation. The first sounded WebM peaked at +10.07 dBFS, with 7708 samples at
0 dB. Only unit tests cover the attenuation path.

Frozen reader, 2026-09-24: `content-machine:editor/src/content_machine/sfx.py`,
with no change, read that run's `torpedo-blast.jsonl` from a `/tmp` media root
whose `sfx/` links to the capture directory. `load_sidecar` accepted the
header (96 frames at 30 fps) and 3 voices, all audible. `resolve_sample`
resolved all 3 copied samples, and `read_samples` plus `enveloped` placed all
3. The run predates the sum change. Its samples are mono, and both pan filters
decode them to identical bytes, so its PCM is unchanged.

The byte-for-byte producer differential is not applicable across versions:
`content-machine:capture/nova-protocol/v0.14.0/Cargo.toml` pins the v0.14.0
tag, while Nova's built-in capture path is for the later build. Changing the
frozen capsule to share a build would change the comparison's reference. The
unchanged content-machine v1 reader accepted Nova's 96-frame sidecar and all
three copied samples (see the frozen-reader proof above). The v0.14.0 producer
remains with its frozen capsule; a future v0.15.0 capsule must use Nova's
built-in capture path rather than copy that producer.

## Verification

Named behavior: a recorded loop's sidecar reproduces the gain the mixer
resolved, for every frame each voice was alive.

- A unit test in `nova_gameplay` audio: spawn a listener and two voices at
  different distances, then assert the far voice's published gain sits below
  the near one's, that a positional voice publishes the spatial sink's per-ear
  factors, and that a voice which moves away has a falling gain. The sidecar
  header's frame count is checked against the recorder's at loop close, which
  vetoes the loop on a mismatch.
- A unit test in `nova_gameplay` audio: spawn a listener and
  `MAX_EXTERIOR_LOOP_VOICES + 1` exterior loops at distinct distances, run
  the start and drive systems, and assert that only the farthest publishes
  `capped`. The `nova_debug` row test writes a capped row at gain 0, so the
  two together prove that a capped exterior loop exports as silent.
- Unit tests in `nova_debug`: exact version 1 row bytes, per-ear channel use
  with a spatial voice reading its summed decode, over-only PCM peak
  attenuation, unsafe sample paths, unsafe sample destinations, and JSON
  escaping. The summing pan filter itself needs ffmpeg, which the CI test job
  does not install, so a disposable ffmpeg run proves it: mono 0.25 decodes to
  0.25 in both ears, and stereo 0.25/0.125 decodes to 0.375 in both ears.
- An armed `loop_torpedo_blast` run: VP9 and Opus in one WebM, and the header
  frame count equal to the encoded frames. The -1 dBFS ceiling applies to the
  staged PCM before Opus; the decoded proof peak is -9.6 dBFS, which proves no
  attenuated loop's decoded peak.
- A unit test for voice numbering: despawn a voice, spawn another that reuses
  the entity index, and assert the two get different `v` values.
- Reader compatibility, not cross-version producer byte equality: the frozen
  content-machine v1 reader loads the engine sidecar and resolves its samples.
  The game tests assert mixer gain, cap state, frame parity, and v1 row bytes.

## Done when

- `loop_start` and `loop_end` open and close a sidecar by themselves; a bin
  needs no second pair of calls.
- An armed loop writes `<loop>.jsonl` and the samples it names.
- `content-machine:editor/src/content_machine/sfx.py` loads that file with no
  changes to the reader, and a render places the samples.
- A capped exterior loop exports as silent.
- The unchanged content-machine v1 reader accepts the sidecar and samples.
- The frozen v0.14.0 capsule remains unchanged. No separate sidecar producer
  is required or planned for a future v0.15.0 capsule.
- The unit tests and the armed run above pass.

## Scheduling

Pulled onto the v0.15.0 board at priority 60 on 2026-09-21 by owner decision.
The work is small and already proven outside the engine, and the capture and
content loops need the sidecar before they can rebuild a mix from engine
numbers. Epic: `20260921-231507`.
