//! Record what a WebM loop HEARD: the SFX sidecar `<loop>.jsonl` beside
//! `<loop>.webm`, the samples it names, and the stereo PCM that becomes the
//! WebM's Opus track.
//!
//! An armed run pins the clock to one tick per rendered frame and runs far
//! slower than real time, so there is no live signal to record. Instead, each
//! [`LoopCaptureFrame`] samples the [`VoiceMix`] the engine resolved for every
//! [`SfxVoice`] that frame, and [`LoopCaptureEnded`] renders the loop offline
//! from the loaded sample bytes.
//!
//! The sidecar is version 1 of the format in `content-machine:docs/sfx-sidecar.md`:
//! a header `{version, loop, fps, frames}`, then `{f, v, gain}` rows with
//! `clip` and `looping` on a voice's first row only. The gain is the pre-master
//! sink gain, because a capture run is muted at the master. Speed and the
//! per-ear pan are not in version 1; they drive only the offline render.
//!
//! The Opus track is not the sidecar's mix. It renders each voice through its
//! [`VoiceMix::channel_gains`], so a positional voice pans as the engine pans
//! it, and then attenuates the whole loop once if its peak is over
//! [`PEAK_CEILING_DBFS`]. It never boosts a quiet loop. The ceiling holds for
//! the staged PCM only; the Opus encode can overshoot it on decode.

use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Component, Path},
    process::{Command, Stdio},
};

use bevy::prelude::*;
use nova_autopilot::loops::{
    LoopCaptureEnded, LoopCaptureFrame, LoopCaptureStarted, LOOP_AUDIO_SAMPLE_RATE,
};
use nova_gameplay::prelude::{SfxVoice, VoiceMix};

/// The version of the sidecar format this writes.
const SIDECAR_VERSION: u32 = 1;

/// The highest peak the staged PCM may reach. The sum of many voices at their
/// engine gains goes over full scale, and Opus would carry that as clipping on
/// any 16-bit decode.
const PEAK_CEILING_DBFS: f32 = -1.0;

/// Decode any sample to stereo. `pan` duplicates a mono sample to both
/// channels at unity and keeps a stereo sample as it is; ffmpeg's default
/// `-ac 2` upmix would lower a mono sample by 3 dB.
const DECODE_PAN: &str = "pan=stereo|FL=FL+FC|FR=FR+FC";

/// Decode a spatial voice's sample to the sum of its channels in both ears.
/// Rodio 0.22.2's `ChannelVolume` discards its divide by the channel count,
/// so the spatial sink plays a mono sample at unity and a stereo one as
/// `FL + FR`.
const DOWNMIX_PAN: &str = "pan=stereo|FL=FL+FR+FC|FR=FL+FR+FC";

/// Opens, samples and finalizes the SFX sidecar of every WebM loop. Added by
/// [`DebugPlugin`](crate::DebugPlugin); a Bevy-only autopilot app without it
/// keeps writing silent WebM.
pub(crate) struct SfxSidecarPlugin;

impl Plugin for SfxSidecarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SfxSidecar>();
        app.add_observer(open_sidecar);
        app.add_observer(sample_sidecar);
        app.add_observer(close_sidecar);
    }
}

/// The open sidecar, or nothing between loops.
#[derive(Resource, Default)]
struct SfxSidecar {
    open: Option<OpenSidecar>,
}

struct OpenSidecar {
    name: String,
    fps: u32,
    /// Frames sampled so far.
    frames: u32,
    /// Voice number per entity. An entity index is reused after a despawn, so
    /// `v` comes from [`Self::voices`]' length, which only goes up.
    numbers: HashMap<Entity, usize>,
    /// Every voice heard, indexed by its number.
    voices: Vec<VoiceTrack>,
    rows: Vec<String>,
    /// The first voice this sidecar could not record. It vetoes the loop.
    failure: Option<String>,
}

/// One voice's envelope, for the offline render.
struct VoiceTrack {
    clip: String,
    handle: Handle<AudioSource>,
    looping: bool,
    /// The voice plays through a spatial sink, which applies its channel gains
    /// to the sum of the sample's channels, so its sample decodes through
    /// [`DOWNMIX_PAN`].
    ///
    /// A positional voice with no listener or no placed source publishes flat
    /// `[level; 2]`, while its sink still pans from its last emitter pose. The
    /// render does not model that pose, so such a voice renders centred.
    downmix: bool,
    frames: Vec<VoiceFrame>,
}

#[derive(Clone, Copy, Debug)]
struct VoiceFrame {
    frame: u32,
    /// Rendered `[left, right]` gain: [`VoiceMix::channel_gains`].
    channels: [f32; 2],
    speed: f32,
    /// The voice cap or a frozen sim paused the voice's sink, so the playhead
    /// holds too.
    held: bool,
}

impl OpenSidecar {
    fn new(name: &str, fps: u32) -> Self {
        Self {
            name: name.to_string(),
            fps,
            frames: 0,
            numbers: HashMap::new(),
            voices: Vec::new(),
            rows: Vec::new(),
            failure: None,
        }
    }

    /// Append one row per voice for video frame `frame`. `clip_of` resolves a
    /// new voice's asset path.
    fn sample<'a>(
        &mut self,
        frame: u32,
        voices: impl IntoIterator<Item = (Entity, &'a SfxVoice, &'a VoiceMix)>,
        clip_of: impl Fn(&Handle<AudioSource>) -> Option<String>,
    ) {
        for (entity, voice, mix) in voices {
            let held = mix.capped || mix.paused;
            let gain = if held { 0.0 } else { mix.gain };
            let (number, row) = match self.numbers.get(&entity) {
                Some(&number) => (
                    number,
                    format!(r#"{{"f":{frame},"v":{number},"gain":{gain:.5}}}"#),
                ),
                None => {
                    let Some(clip) = clip_of(&voice.handle) else {
                        // Version 1 names every voice by its asset path, and
                        // dropping one would ship a mix missing a sound.
                        self.failure.get_or_insert_with(|| {
                            format!("a voice on frame {frame} has no asset path")
                        });
                        continue;
                    };
                    // The path is also where the sample is copied, beside the
                    // sidecar. Anything but plain names could write outside
                    // the capture directory.
                    if !is_confined(&clip) {
                        self.failure.get_or_insert_with(|| {
                            format!("a voice on frame {frame} names unsafe sample path `{clip}`")
                        });
                        continue;
                    }
                    let number = self.voices.len();
                    self.numbers.insert(entity, number);
                    let row = format!(
                        r#"{{"f":{frame},"v":{number},"clip":{},"gain":{gain:.5},"looping":{}}}"#,
                        json_string(&clip),
                        voice.looping
                    );
                    self.voices.push(VoiceTrack {
                        clip,
                        handle: voice.handle.clone(),
                        looping: voice.looping,
                        downmix: voice.route.is_positional(),
                        frames: Vec::new(),
                    });
                    (number, row)
                }
            };
            self.voices[number].frames.push(VoiceFrame {
                frame,
                channels: mix.channel_gains,
                speed: mix.speed,
                held,
            });
            self.rows.push(row);
        }
        self.frames += 1;
    }

    /// The whole version 1 file: header, then rows in frame order.
    fn jsonl(&self) -> String {
        let mut text = format!(
            r#"{{"version":{SIDECAR_VERSION},"loop":{},"fps":{},"frames":{}}}"#,
            json_string(&self.name),
            self.fps,
            self.frames
        );
        text.push('\n');
        for row in &self.rows {
            text.push_str(row);
            text.push('\n');
        }
        text
    }
}

fn open_sidecar(started: On<LoopCaptureStarted>, mut sidecar: ResMut<SfxSidecar>) {
    sidecar.open = Some(OpenSidecar::new(&started.name, started.fps));
}

fn sample_sidecar(
    frame: On<LoopCaptureFrame>,
    mut sidecar: ResMut<SfxSidecar>,
    assets: Res<AssetServer>,
    q_voices: Query<(Entity, &SfxVoice, &VoiceMix)>,
) {
    let Some(open) = sidecar.open.as_mut() else {
        return;
    };
    open.sample(frame.frame, &q_voices, |handle| {
        assets
            .get_path(handle.id())
            .map(|path| path.path().to_string_lossy().replace('\\', "/"))
    });
}

/// Stage the PCM, copy the samples, then publish the JSONL. Any failure except
/// a sample copy vetoes the loop. The encode runs after this, so an ffmpeg mux
/// failure there still leaves the complete sidecar and its samples on disk.
fn close_sidecar(
    mut ended: On<LoopCaptureEnded>,
    mut sidecar: ResMut<SfxSidecar>,
    sources: Res<Assets<AudioSource>>,
) {
    let result = match sidecar.open.take() {
        Some(open) => finish(&open, ended.event(), &sources),
        None => Err("the SFX sidecar never opened".to_string()),
    };
    if let Err(failure) = result {
        ended.event_mut().failure = Some(format!("sfx sidecar: {failure}"));
    }
}

fn finish(
    open: &OpenSidecar,
    ended: &LoopCaptureEnded,
    sources: &Assets<AudioSource>,
) -> Result<(), String> {
    if let Some(failure) = &open.failure {
        return Err(failure.clone());
    }
    if open.name != ended.name || open.frames != ended.frames {
        return Err(format!(
            "sampled `{}` for {} frames, but `{}` recorded {}",
            open.name, open.frames, ended.name, ended.frames
        ));
    }
    let mut bytes: HashMap<&str, &[u8]> = HashMap::new();
    for voice in &open.voices {
        let source = sources
            .get(&voice.handle)
            .ok_or_else(|| format!("`{}` has no loaded sample bytes to render", voice.clip))?;
        bytes.insert(&voice.clip, &source.bytes);
    }
    // A sample that both a flat and a spatial voice play decodes twice.
    let mut clips = HashMap::new();
    for voice in &open.voices {
        let key = (voice.clip.as_str(), voice.downmix);
        if clips.contains_key(&key) {
            continue;
        }
        let decoded = decode_clip(bytes[key.0], voice.downmix)
            .map_err(|error| format!("cannot decode `{}`: {error}", voice.clip))?;
        clips.insert(key, decoded);
    }
    let mut pcm = mix_voices(&open.voices, &clips, open.fps, open.frames);
    let (peak, scale) = attenuate_peak(&mut pcm);
    let pcm: Vec<u8> = pcm.iter().flat_map(|sample| sample.to_le_bytes()).collect();
    fs::write(&ended.audio_path, pcm)
        .map_err(|error| format!("cannot write {}: {error}", ended.audio_path.display()))?;

    let directory = ended.sidecar_path.parent().unwrap_or(Path::new(""));
    let copied = bytes
        .iter()
        .filter(|(clip, data)| copy_sample(directory, clip, data))
        .count();

    let partial = ended.sidecar_path.with_extension("jsonl.part");
    fs::create_dir_all(directory)
        .and_then(|()| fs::write(&partial, open.jsonl()))
        .and_then(|()| fs::rename(&partial, &ended.sidecar_path))
        .map_err(|error| {
            let _ = fs::remove_file(&partial);
            format!("cannot write {}: {error}", ended.sidecar_path.display())
        })?;
    let peak = if peak > 0.0 {
        format!("{:.2} dBFS", 20.0 * peak.log10())
    } else {
        "silent".to_string()
    };
    info!(
        "sfx sidecar: `{}` recorded {} frames, {} voices, {copied}/{} samples; \
         audio peak {peak}, attenuated {:.2} dB",
        open.name,
        open.frames,
        open.voices.len(),
        bytes.len(),
        20.0 * scale.recip().log10()
    );
    Ok(())
}

/// Whether `clip` is a non-empty relative path of plain names, with no root,
/// prefix, `.` or `..`.
fn is_confined(clip: &str) -> bool {
    !clip.is_empty()
        && Path::new(clip)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// `text` as a JSON string literal. Only `"`, `\` and control characters are
/// escaped, so an ordinary name or path keeps its exact bytes.
fn json_string(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            control if u32::from(control) < 0x20 => {
                literal.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => literal.push(other),
        }
    }
    literal.push('"');
    literal
}

/// Write one sample next to the sidecar at its asset-relative path, so a
/// capture is self-contained. An existing file is overwritten, because the
/// asset may have changed since an earlier capture. A failure WARNS: a missing
/// sample costs one editor row, while aborting would cost the whole capture
/// run.
fn copy_sample(directory: &Path, clip: &str, bytes: &[u8]) -> bool {
    let destination = directory.join(clip);
    let written = destination
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&destination, bytes));
    if let Err(error) = written {
        warn!(
            "sfx sidecar: cannot copy `{clip}` to {}: {error}",
            destination.display()
        );
        return false;
    }
    true
}

/// Decode one sample's bytes to stereo frames at [`LOOP_AUDIO_SAMPLE_RATE`],
/// through [`DOWNMIX_PAN`] for a spatial voice and [`DECODE_PAN`] otherwise.
fn decode_clip(bytes: &[u8], downmix: bool) -> Result<Vec<[f32; 2]>, String> {
    let pan = if downmix { DOWNMIX_PAN } else { DECODE_PAN };
    let mut child = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-i", "pipe:0", "-af"])
        .arg(format!("{pan},aresample={LOOP_AUDIO_SAMPLE_RATE}"))
        .args(["-f", "f32le", "pipe:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not run `ffmpeg`: {error} (is ffmpeg installed?)"))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let input = bytes.to_vec();
    // Written from a thread: ffmpeg fills its stdout pipe while it reads, so a
    // blocking write here would deadlock on a large sample.
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = child
        .wait_with_output()
        .map_err(|error| format!("ffmpeg did not finish: {error}"))?;
    let written = writer.join().expect("the stdin writer does not panic");
    if !output.status.success() {
        return Err(format!(
            "ffmpeg exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    written.map_err(|error| format!("could not feed ffmpeg: {error}"))?;
    if output.stdout.len() % 8 != 0 {
        return Err(format!(
            "ffmpeg returned {} bytes, not whole stereo f32 frames",
            output.stdout.len()
        ));
    }
    Ok(output
        .stdout
        .chunks_exact(8)
        .map(|frame| {
            let channel =
                |at: usize| f32::from_le_bytes(frame[at..at + 4].try_into().expect("4 bytes"));
            [channel(0), channel(4)]
        })
        .collect())
}

/// First PCM frame of video frame `frame`. Rounded up, so the staged track is
/// never shorter than the video.
fn frame_start(frame: u32, fps: u32) -> usize {
    let start = (u64::from(frame) * u64::from(LOOP_AUDIO_SAMPLE_RATE)).div_ceil(u64::from(fps));
    usize::try_from(start).expect("a loop's PCM length fits in memory")
}

/// Render every voice into interleaved stereo PCM covering `frames` video
/// frames at `fps`.
///
/// Each frame applies its row's channel gains and speed to the whole frame, as
/// the engine writes the sink once per frame. Each voice reads the decode of
/// its clip keyed by [`VoiceTrack::downmix`]. A held frame is silent and holds
/// the playhead. A one-shot plays its whole sample and holds its last row after
/// that row; a loop ends with its last row.
fn mix_voices(
    voices: &[VoiceTrack],
    clips: &HashMap<(&str, bool), Vec<[f32; 2]>>,
    fps: u32,
    frames: u32,
) -> Vec<f32> {
    let mut pcm = vec![0.0; frame_start(frames, fps) * 2];
    for voice in voices {
        let (Some(first), Some(last)) = (voice.frames.first(), voice.frames.last()) else {
            continue;
        };
        let sample = &clips[&(voice.clip.as_str(), voice.downmix)];
        if sample.is_empty() {
            continue;
        }
        let end = if voice.looping {
            last.frame + 1
        } else {
            frames
        };
        let mut rows = voice.frames.iter().peekable();
        let mut last_row = *first;
        let mut playhead = 0.0_f64;
        'frames: for frame in first.frame..end {
            let row = match rows.next_if(|row| row.frame == frame) {
                Some(row) => {
                    last_row = *row;
                    last_row
                }
                // A gap inside the envelope is silence at the last speed.
                None if frame < last.frame => VoiceFrame {
                    channels: [0.0; 2],
                    held: false,
                    ..last_row
                },
                None => last_row,
            };
            if row.held {
                continue;
            }
            for out in frame_start(frame, fps)..frame_start(frame + 1, fps) {
                if !voice.looping && playhead >= sample.len() as f64 {
                    break 'frames;
                }
                let [left, right] = sample_at(sample, playhead, voice.looping);
                pcm[out * 2] += left * row.channels[0];
                pcm[out * 2 + 1] += right * row.channels[1];
                playhead += f64::from(row.speed);
                if voice.looping {
                    playhead %= sample.len() as f64;
                }
            }
        }
    }
    pcm
}

/// Scale `pcm` once so its absolute peak sits at [`PEAK_CEILING_DBFS`] when it
/// is over it, and leave it untouched otherwise. Returns the original peak and
/// the applied linear scale.
fn attenuate_peak(pcm: &mut [f32]) -> (f32, f32) {
    let peak = pcm
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let ceiling = 10.0_f32.powf(PEAK_CEILING_DBFS / 20.0);
    if peak <= ceiling {
        return (peak, 1.0);
    }
    // Divide first: `peak / peak` is exactly 1, so no sample rounds past the
    // ceiling, which `sample * (ceiling / peak)` can do by one ulp.
    for sample in pcm.iter_mut() {
        *sample = *sample / peak * ceiling;
    }
    (peak, ceiling / peak)
}

/// The sample at a fractional `position`, linearly interpolated. A loop wraps
/// to its start; a one-shot fades to silence past its end.
fn sample_at(sample: &[[f32; 2]], position: f64, looping: bool) -> [f32; 2] {
    let index = position as usize;
    let fraction = (position - index as f64) as f32;
    let next = if index + 1 < sample.len() {
        sample[index + 1]
    } else if looping {
        sample[0]
    } else {
        [0.0; 2]
    };
    let current = sample[index];
    [
        current[0] + (next[0] - current[0]) * fraction,
        current[1] + (next[1] - current[1]) * fraction,
    ]
}

#[cfg(test)]
mod tests {
    use bevy::ecs::entity::EntityIndex;
    use nova_gameplay::prelude::AudioRoute;

    use super::*;

    fn mix(gain: f32, capped: bool) -> VoiceMix {
        VoiceMix {
            level: gain,
            gain,
            channel_gains: [gain; 2],
            emitter: None,
            speed: 1.25,
            capped,
            paused: false,
        }
    }

    fn clip_path(_: &Handle<AudioSource>) -> Option<String> {
        Some("sounds/thruster.wav".to_string())
    }

    /// Version 1 is the editor's contract: exact header and row bytes, clip
    /// and looping on the first row only, capped and paused rows silent, and no
    /// speed.
    #[test]
    fn the_sidecar_writes_version_one_rows_exactly() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let voice = SfxVoice::looping(Handle::default(), AudioRoute::Exterior);
        let mut open = OpenSidecar::new("strike", 30);

        open.sample(0, [(entity, &voice, &mix(0.123_456, false))], clip_path);
        open.sample(1, [(entity, &voice, &mix(0.5, true))], clip_path);
        let paused = VoiceMix {
            paused: true,
            ..mix(0.5, false)
        };
        open.sample(2, [(entity, &voice, &paused)], clip_path);

        assert_eq!(
            open.jsonl(),
            concat!(
                r#"{"version":1,"loop":"strike","fps":30,"frames":3}"#,
                "\n",
                r#"{"f":0,"v":0,"clip":"sounds/thruster.wav","gain":0.12346,"looping":true}"#,
                "\n",
                r#"{"f":1,"v":0,"gain":0.00000}"#,
                "\n",
                r#"{"f":2,"v":0,"gain":0.00000}"#,
                "\n",
            )
        );
    }

    /// A despawned voice's entity index comes back for the next sound, and the
    /// two must never splice into one clip.
    #[test]
    fn voice_numbers_only_go_up_across_entity_index_reuse() {
        let voice = SfxVoice::one_shot(Handle::default(), AudioRoute::Interface);
        let mut open = OpenSidecar::new("reuse", 30);

        let first = Entity::from_index(EntityIndex::from_raw_u32(7).unwrap());
        let second =
            Entity::from_index_and_generation(first.index(), first.generation().after_versions(1));
        open.sample(0, [(first, &voice, &mix(1.0, false))], clip_path);
        open.sample(1, [(second, &voice, &mix(1.0, false))], clip_path);

        assert!(open.rows[0].contains(r#""v":0,"clip""#), "{}", open.rows[0]);
        assert!(open.rows[1].contains(r#""v":1,"clip""#), "{}", open.rows[1]);
    }

    /// A sample path is also where its copy is written, so a path that could
    /// leave the capture directory vetoes the loop and writes no row.
    #[test]
    fn a_sample_path_outside_the_capture_directory_vetoes_the_loop() {
        let voice = SfxVoice::one_shot(Handle::default(), AudioRoute::Interface);
        for unsafe_path in [
            "../escape.wav",
            "/etc/escape.wav",
            "sounds/../x.wav",
            "./x.wav",
        ] {
            let mut open = OpenSidecar::new("escape", 30);
            let entity = Entity::from_index(EntityIndex::from_raw_u32(1).unwrap());
            open.sample(0, [(entity, &voice, &mix(1.0, false))], |_| {
                Some(unsafe_path.to_string())
            });
            assert!(open.rows.is_empty(), "{unsafe_path} wrote a row");
            assert!(
                open.failure
                    .as_deref()
                    .is_some_and(|failure| failure.contains(unsafe_path)),
                "{unsafe_path} must veto: {:?}",
                open.failure
            );
        }
    }

    /// A quote, backslash or control character in a loop name or clip path
    /// stays a valid JSON string.
    #[test]
    fn loop_names_and_clip_paths_are_json_escaped() {
        let voice = SfxVoice::one_shot(Handle::default(), AudioRoute::Interface);
        let mut open = OpenSidecar::new("say \"hi\"\\\n", 30);
        let entity = Entity::from_index(EntityIndex::from_raw_u32(1).unwrap());
        open.sample(0, [(entity, &voice, &mix(1.0, false))], |_| {
            Some("sounds/a\"b.wav".to_string())
        });

        assert_eq!(
            open.jsonl(),
            concat!(
                r#"{"version":1,"loop":"say \"hi\"\\\u000a","fps":30,"frames":1}"#,
                "\n",
                r#"{"f":0,"v":0,"clip":"sounds/a\"b.wav","gain":1.00000,"looping":false}"#,
                "\n",
            )
        );
    }

    /// 11025 fps puts exactly four PCM frames in each video frame.
    const FPS: u32 = LOOP_AUDIO_SAMPLE_RATE / 4;

    fn ramp(len: usize) -> HashMap<(&'static str, bool), Vec<[f32; 2]>> {
        let sample = (0..len).map(|k| [k as f32, -(k as f32)]).collect();
        HashMap::from([(("ramp", false), sample)])
    }

    /// A flat voice with `(frame, gain, speed, held)` rows.
    fn track(looping: bool, rows: &[(u32, f32, f32, bool)]) -> VoiceTrack {
        VoiceTrack {
            clip: "ramp".to_string(),
            handle: Handle::default(),
            looping,
            downmix: false,
            frames: rows
                .iter()
                .map(|&(frame, gain, speed, held)| VoiceFrame {
                    frame,
                    channels: [gain; 2],
                    speed,
                    held,
                })
                .collect(),
        }
    }

    fn left(pcm: &[f32]) -> Vec<f32> {
        pcm.iter().step_by(2).copied().collect()
    }

    /// A held frame is silent and holds the playhead, speed advances it, and
    /// a loop stops with its last row. Stereo stays as decoded.
    #[test]
    fn a_loop_pauses_while_held_follows_its_speed_and_ends_with_its_rows() {
        let loop_voice = track(
            true,
            &[
                (0, 1.0, 1.0, false),
                (1, 0.0, 1.0, true),
                (2, 0.5, 2.0, false),
            ],
        );

        let pcm = mix_voices(&[loop_voice], &ramp(64), FPS, 4);

        assert_eq!(pcm.len(), 4 * 4 * 2, "exactly the video's duration");
        assert_eq!(
            left(&pcm),
            [
                0.0, 1.0, 2.0, 3.0, // frame 0 at full gain
                0.0, 0.0, 0.0, 0.0, // held: silent, playhead paused
                2.0, 3.0, 4.0, 5.0, // resumes at 4, double speed, half gain
                0.0, 0.0, 0.0, 0.0, // the loop ended with its rows
            ]
        );
        assert_eq!(pcm[3], -1.0, "the right channel is the decoded one");
    }

    /// A one-shot plays its whole sample at the gain and speed of its last
    /// row, however few rows its entity lived for.
    #[test]
    fn a_one_shot_holds_its_last_row_until_its_sample_ends() {
        let one_shot = track(false, &[(1, 1.0, 1.0, false), (2, 0.5, 1.0, false)]);

        let pcm = mix_voices(&[one_shot], &ramp(10), FPS, 5);

        assert_eq!(
            left(&pcm),
            [
                0.0, 0.0, 0.0, 0.0, // before it fired
                0.0, 1.0, 2.0, 3.0, // first row
                2.0, 2.5, 3.0, 3.5, // last row
                4.0, 4.5, 0.0, 0.0, // held past the rows until the sample ends
                0.0, 0.0, 0.0, 0.0,
            ]
        );
    }

    /// Each voice scales each channel of its decode by that ear's gain, and a
    /// spatial voice reads its clip's summed decode.
    #[test]
    fn each_voice_reaches_each_ear_at_its_resolved_channel_gain() {
        let clips = HashMap::from([
            (("ramp", false), vec![[0.8, 0.2]; 64]),
            (("ramp", true), vec![[1.0, 1.0]; 64]),
        ]);
        let voice = |downmix: bool| VoiceTrack {
            downmix,
            frames: vec![VoiceFrame {
                frame: 0,
                channels: [0.5, 0.25],
                speed: 1.0,
                held: false,
            }],
            ..track(true, &[])
        };

        let flat = mix_voices(&[voice(false)], &clips, FPS, 1);
        let spatial = mix_voices(&[voice(true)], &clips, FPS, 1);

        assert_eq!(
            flat[..2],
            [0.8 * 0.5, 0.2 * 0.25],
            "flat keeps its channels"
        );
        assert_eq!(
            spatial[..2],
            [1.0 * 0.5, 1.0 * 0.25],
            "spatial pans the summed decode"
        );
    }

    /// A loop over the ceiling is scaled once, keeping its shape, so its peak
    /// lands on the ceiling; a loop under it is never boosted.
    #[test]
    fn only_a_loop_over_the_peak_ceiling_is_attenuated() {
        let ceiling = 10.0_f32.powf(PEAK_CEILING_DBFS / 20.0);
        let mut loud = vec![0.5, -3.0, 1.5, 0.0];

        let (peak, scale) = attenuate_peak(&mut loud);

        assert_eq!(peak, 3.0);
        assert_eq!(scale, ceiling / 3.0);
        assert_eq!(loud, [0.5 / 3.0 * ceiling, -ceiling, 0.5 * ceiling, 0.0]);

        let mut quiet = vec![0.25, -0.5, 0.75];
        assert_eq!(attenuate_peak(&mut quiet), (0.75, 1.0));
        assert_eq!(quiet, [0.25, -0.5, 0.75], "a quiet loop is unchanged");
    }
}
