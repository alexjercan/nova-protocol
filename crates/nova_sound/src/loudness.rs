//! Clip loudness: every sound file measured once, when it loads, on one rule.
//!
//! # The rule
//!
//! A clip's loudness is the ITU-R BS.1770 integrated loudness of the clip
//! LOOPED end to end until it lasts at least [`LOOP_SECONDS`]: K-weighting,
//! 400 ms blocks every 100 ms, the -70 LUFS absolute gate and the -10 LU
//! relative gate. Looping is what makes the rule hold for every length: a 20 ms
//! click has no 400 ms block of its own, and padding it with silence would
//! measure the silence. Looped, every block is the clip, so the result is the
//! clip's mean K-weighted level - the same number a loop plays at.
//!
//! The per-file gain to [`LOUDNESS_TARGET_LUFS`] is stored in
//! [`SoundLoudness`], keyed by the asset id. Every category sound plays through
//! it, so a [`SoundCategoryType`](super::SoundCategoryType) level is relative
//! to the target, whatever level the file was recorded at. A category voice
//! waits for its clip's measure before it starts.
//!
//! A clip that cannot be decoded, or that measures below
//! [`SILENT_BELOW_LUFS`], is an authoring error: content lint refuses it, and a
//! load logs it as an error.

use std::{
    collections::{HashMap, VecDeque},
    fmt,
    io::Cursor,
};

use bevy::{
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use rodio::Source;

/// The loudness every clip is normalized to.
pub const LOUDNESS_TARGET_LUFS: f32 = -23.0;

/// A clip quieter than this is silent: an authoring error, not a quiet sound.
pub const SILENT_BELOW_LUFS: f32 = -60.0;

/// How long the looped clip lasts at least, in seconds. Ten 400 ms blocks of
/// whole periods, so the partial last period weighs little.
pub const LOOP_SECONDS: f64 = 4.0;

/// The file extensions the content lint measures as sound. They are the ones
/// bevy's audio loader takes; a format the build cannot decode fails the
/// measure, which is the same refusal playback would give.
pub const SOUND_EXTENSIONS: [&str; 6] = ["wav", "ogg", "oga", "spx", "flac", "mp3"];

/// The BS.1770 block length and step, in seconds.
const BLOCK_SECONDS: f64 = 0.4;
const STEP_SECONDS: f64 = 0.1;
/// The absolute gate, and the relative gate below the abs-gated mean.
const ABSOLUTE_GATE_LUFS: f64 = -70.0;
const RELATIVE_GATE_LU: f64 = 10.0;

/// Why a clip has no loudness.
#[derive(Debug, Clone, PartialEq)]
pub enum LoudnessError {
    /// The bytes are not audio this build can decode.
    Undecodable(String),
    /// The clip decodes to no samples.
    Empty,
    /// The clip is silent: no block passes the absolute gate, or the gated
    /// loudness is below [`SILENT_BELOW_LUFS`].
    Silent {
        /// The gated loudness, when any block passed the absolute gate.
        lufs: Option<f32>,
    },
}

impl fmt::Display for LoudnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undecodable(reason) => write!(f, "cannot be decoded: {reason}"),
            Self::Empty => write!(f, "decodes to no samples"),
            Self::Silent { lufs: Some(lufs) } => write!(
                f,
                "is silent: {lufs:.1} LUFS, below the {SILENT_BELOW_LUFS} LUFS floor"
            ),
            Self::Silent { lufs: None } => write!(f, "is silent: no sound above -70 LUFS"),
        }
    }
}

impl std::error::Error for LoudnessError {}

/// Decode a sound file and measure its clip loudness, in LUFS.
///
/// The decode streams through [`clip_loudness`], so a four minute track costs
/// what a click costs in memory. Takes the bytes by value: the decoder reads
/// them in place, and the engine hands it the loaded asset's shared buffer.
pub fn measure_clip(bytes: impl AsRef<[u8]> + Send + Sync + 'static) -> Result<f32, LoudnessError> {
    let decoder = rodio::Decoder::new(Cursor::new(bytes))
        .map_err(|error| LoudnessError::Undecodable(error.to_string()))?;
    let channels = decoder.channels().get();
    let sample_rate = decoder.sample_rate().get();
    clip_loudness(decoder, channels, sample_rate)
}

/// The clip loudness of interleaved samples, in LUFS (see the module docs).
///
/// One pass over the samples. Memory is the open 400 ms block sums and one
/// mean square per closed block, plus a copy of the clip while it is shorter
/// than [`LOOP_SECONDS`]: a short clip is fed again from that copy until the
/// loop is long enough. A trailing partial frame is dropped.
pub fn clip_loudness(
    samples: impl IntoIterator<Item = f32>,
    channels: u16,
    sample_rate: u32,
) -> Result<f32, LoudnessError> {
    if sample_rate == 0 {
        return Err(LoudnessError::Empty);
    }
    let channels = usize::from(channels.max(1));
    let rate = f64::from(sample_rate);
    let loop_frames = (LOOP_SECONDS * rate).ceil() as usize;
    let mut meter = BlockMeter::new(rate, channels);
    let mut short = Vec::new();
    let mut frame = Vec::with_capacity(channels);
    let mut frames = 0;
    for sample in samples {
        frame.push(sample);
        if frame.len() < channels {
            continue;
        }
        meter.feed(&frame);
        if frames < loop_frames {
            short.extend_from_slice(&frame);
        } else if frames == loop_frames {
            short = Vec::new();
        }
        frames += 1;
        frame.clear();
    }
    if frames == 0 {
        return Err(LoudnessError::Empty);
    }
    // One filter state per channel runs across the joins, as it would on a
    // playing loop.
    let repeats = loop_frames.div_ceil(frames).max(1);
    for _ in 1..repeats {
        for frame in short.chunks_exact(channels) {
            meter.feed(frame);
        }
    }
    gated_loudness(&meter.blocks)
}

/// The BS.1770 gates over the mean squares of the 400 ms blocks.
fn gated_loudness(blocks: &[f64]) -> Result<f32, LoudnessError> {
    let absolute: Vec<f64> = blocks
        .iter()
        .copied()
        .filter(|z| block_lufs(*z) > ABSOLUTE_GATE_LUFS)
        .collect();
    if absolute.is_empty() {
        return Err(LoudnessError::Silent { lufs: None });
    }
    let relative_gate = block_lufs(mean(&absolute)) - RELATIVE_GATE_LU;
    let gated: Vec<f64> = absolute
        .into_iter()
        .filter(|z| block_lufs(*z) > relative_gate)
        .collect();
    let lufs = block_lufs(mean(&gated)) as f32;
    if lufs < SILENT_BELOW_LUFS {
        return Err(LoudnessError::Silent { lufs: Some(lufs) });
    }
    Ok(lufs)
}

/// The K-weighted energy of a frame stream, summed into 400 ms blocks that
/// start every 100 ms.
///
/// A block opens on every step and closes once it holds a block of frames.
/// A block the stream ends inside is never closed, so it does not count.
struct BlockMeter {
    filters: Vec<KWeighting>,
    channels: usize,
    block: usize,
    step: usize,
    /// Frames fed so far.
    position: usize,
    /// The start frame and energy sum of every open block, oldest first.
    open: VecDeque<(usize, f64)>,
    /// The mean square of every closed block, in order.
    blocks: Vec<f64>,
}

impl BlockMeter {
    fn new(rate: f64, channels: usize) -> Self {
        let block = ((BLOCK_SECONDS * rate).round() as usize).max(1);
        let step = ((STEP_SECONDS * rate).round() as usize).max(1);
        Self {
            filters: vec![KWeighting::new(rate); channels],
            channels,
            block,
            step,
            position: 0,
            open: VecDeque::with_capacity(block.div_ceil(step)),
            blocks: Vec::new(),
        }
    }

    /// Feed one interleaved frame.
    fn feed(&mut self, frame: &[f32]) {
        let mut energy = 0.0;
        for (channel, (sample, filter)) in frame.iter().zip(&mut self.filters).enumerate() {
            let filtered = filter.process(f64::from(*sample));
            energy += channel_weight(channel, self.channels) * filtered * filtered;
        }
        if self.position % self.step == 0 {
            self.open.push_back((self.position, 0.0));
        }
        for (_, sum) in &mut self.open {
            *sum += energy;
        }
        self.position += 1;
        while let Some(&(start, sum)) = self.open.front() {
            if self.position - start < self.block {
                break;
            }
            self.blocks.push(sum / self.block as f64);
            self.open.pop_front();
        }
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// The loudness of a block's weighted mean square.
fn block_lufs(mean_square: f64) -> f64 {
    -0.691 + 10.0 * mean_square.log10()
}

/// The BS.1770 channel weight in the L, R, C, LFE, Ls, Rs order: the LFE
/// counts for nothing and the surrounds for +1.5 dB. Mono and stereo are all
/// ones.
fn channel_weight(channel: usize, channels: usize) -> f64 {
    if channels < 4 {
        return 1.0;
    }
    match channel {
        3 => 0.0,
        4 | 5 => 1.41,
        _ => 1.0,
    }
}

/// The two-stage K-weighting pre-filter (a high shelf, then a high pass), as
/// biquads in transposed direct form II. The coefficients are derived for the
/// sample rate from the analog prototypes BS.1770 tabulates at 48 kHz.
#[derive(Clone, Copy)]
struct KWeighting {
    shelf: Biquad,
    high_pass: Biquad,
}

impl KWeighting {
    fn new(rate: f64) -> Self {
        let shelf = {
            let f0 = 1_681.974_450_955_533;
            let gain_db = 3.999_843_853_973_347;
            let q = 0.707_175_236_955_419_6;
            let k = (std::f64::consts::PI * f0 / rate).tan();
            let vh = 10f64.powf(gain_db / 20.0);
            let vb = vh.powf(0.499_666_774_154_541_6);
            let a0 = 1.0 + k / q + k * k;
            Biquad::new(
                [
                    (vh + vb * k / q + k * k) / a0,
                    2.0 * (k * k - vh) / a0,
                    (vh - vb * k / q + k * k) / a0,
                ],
                [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
            )
        };
        let high_pass = {
            let f0 = 38.135_470_876_024_44;
            let q = 0.500_327_037_323_877_3;
            let k = (std::f64::consts::PI * f0 / rate).tan();
            let a0 = 1.0 + k / q + k * k;
            Biquad::new(
                [1.0, -2.0, 1.0],
                [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
            )
        };
        Self { shelf, high_pass }
    }

    fn process(&mut self, sample: f64) -> f64 {
        self.high_pass.process(self.shelf.process(sample))
    }
}

#[derive(Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn new(b: [f64; 3], a: [f64; 2]) -> Self {
        Self {
            b,
            a,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z1;
        self.z1 = self.b[1] * x - self.a[0] * y + self.z2;
        self.z2 = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// One clip's measured loudness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipLoudness {
    /// The clip loudness, in LUFS.
    pub lufs: f32,
}

impl ClipLoudness {
    /// The gain that brings the clip to [`LOUDNESS_TARGET_LUFS`], in dB.
    pub fn gain_db(self) -> f32 {
        LOUDNESS_TARGET_LUFS - self.lufs
    }

    /// [`Self::gain_db`] as a linear amplitude factor.
    pub fn gain(self) -> f32 {
        10f32.powf(self.gain_db() / 20.0)
    }
}

/// The measured loudness of every loaded [`AudioSource`], keyed by asset id.
///
/// Filled by the engine as each clip's measure finishes, a few frames after
/// it loads, and emptied as it unloads.
#[derive(Resource, Default, Debug)]
pub struct SoundLoudness {
    clips: HashMap<AssetId<AudioSource>, ClipLoudness>,
}

impl SoundLoudness {
    /// The loudness of a loaded clip, or `None` before it loads or when it
    /// failed to measure.
    pub fn get(&self, id: impl Into<AssetId<AudioSource>>) -> Option<ClipLoudness> {
        self.clips.get(&id.into()).copied()
    }

    /// How many clips are measured.
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Whether no clip is measured.
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    /// Record a clip's measure without loading it.
    #[cfg(test)]
    pub(crate) fn insert(&mut self, id: AssetId<AudioSource>, clip: ClipLoudness) {
        self.clips.insert(id, clip);
    }
}

/// A clip's measure, running on the async compute pool.
type MeasureTask = Task<Result<f32, LoudnessError>>;

/// Measure each [`AudioSource`] as it loads (and again when it reloads), and
/// forget it when it unloads.
///
/// The measure runs on the [`AsyncComputeTaskPool`], so a long clip's decode
/// stays off the frame. On wasm that pool is the page's one thread: the
/// decode still lands between two frames, as one stall. A clip enters
/// [`SoundLoudness`] the frame its task is seen done, and a voice of it waits
/// until then. A reload replaces the clip's running task, and an unload
/// drops it, which cancels it.
///
/// A clip that cannot be measured is an authoring error the content lint
/// refuses; one that reaches a load anyway (a mod built without the lint) is
/// logged as an error and left out of [`SoundLoudness`].
///
/// Each clip logs at trace. A load step finishes its clips over a run of
/// consecutive frames, so `burst` collects them and the first frame that
/// measures nothing logs the run as one debug line.
pub(crate) fn measure_loaded_sounds(
    mut events: MessageReader<AssetEvent<AudioSource>>,
    sources: Res<Assets<AudioSource>>,
    asset_server: Option<Res<AssetServer>>,
    mut loudness: ResMut<SoundLoudness>,
    mut pending: Local<HashMap<AssetId<AudioSource>, (String, MeasureTask)>>,
    mut burst: Local<Vec<(String, ClipLoudness)>>,
) {
    for event in events.read() {
        let id = match *event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => id,
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                loudness.clips.remove(&id);
                pending.remove(&id);
                continue;
            }
            AssetEvent::LoadedWithDependencies { .. } => continue,
        };
        let Some(source) = sources.get(id) else {
            continue;
        };
        let name = asset_server
            .as_ref()
            .and_then(|server| server.get_path(id))
            .map_or_else(|| format!("{id}"), |path| path.to_string());
        let bytes = source.bytes.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move { measure_clip(bytes) });
        pending.insert(id, (name, task));
    }

    let before = burst.len();
    let mut done = Vec::new();
    for (id, (name, task)) in pending.iter_mut() {
        let Some(measure) = block_on(poll_once(task)) else {
            continue;
        };
        done.push(*id);
        match measure {
            Ok(lufs) => {
                let clip = ClipLoudness { lufs };
                trace!(
                    "nova audio: `{name}` measures {lufs:.1} LUFS, gain {:+.1} dB to {LOUDNESS_TARGET_LUFS} LUFS",
                    clip.gain_db()
                );
                loudness.clips.insert(*id, clip);
                burst.push((std::mem::take(name), clip));
            }
            Err(error) => {
                loudness.clips.remove(id);
                error!("nova audio: `{name}` {error}");
            }
        }
    }
    for id in done {
        pending.remove(&id);
    }
    if burst.len() == before && !burst.is_empty() {
        debug!("nova audio: {}", burst_summary(&mut burst));
        burst.clear();
    }
}

/// One burst of measured clips as one log line: the count, the gain range, and
/// every clip as `name LUFS/gain` in name order.
fn burst_summary(burst: &mut [(String, ClipLoudness)]) -> String {
    burst.sort_by(|(a, _), (b, _)| a.cmp(b));
    let gains = burst.iter().map(|(_, clip)| clip.gain_db());
    let low = gains.clone().fold(f32::INFINITY, f32::min);
    let high = gains.fold(f32::NEG_INFINITY, f32::max);
    let clips: Vec<String> = burst
        .iter()
        .map(|(name, clip)| format!("{name} {:.1}/{:+.1}", clip.lufs, clip.gain_db()))
        .collect();
    format!(
        "measured {} clip(s), gain {low:+.1}..{high:+.1} dB to {LOUDNESS_TARGET_LUFS} LUFS; \
         LUFS/gain dB: {}",
        burst.len(),
        clips.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// A 1 kHz sine at `amplitude`, `seconds` long, on `channels` identical
    /// channels.
    fn sine(amplitude: f32, seconds: f32, sample_rate: u32, channels: u16) -> Vec<f32> {
        let frames = (seconds * sample_rate as f32) as usize;
        (0..frames)
            .flat_map(|n| {
                let phase = std::f32::consts::TAU * 1000.0 * n as f32 / sample_rate as f32;
                std::iter::repeat_n(amplitude * phase.sin(), usize::from(channels))
            })
            .collect()
    }

    /// A 16-bit PCM WAV file of mono samples.
    fn wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// BS.1770 calibrates K-weighting so a 1 kHz sine on one channel reads its
    /// RMS level in dBFS: 20 log10(amplitude) - 3.01.
    fn expected_lufs(amplitude: f32) -> f32 {
        20.0 * amplitude.log10() - 3.01
    }

    #[test]
    fn a_sine_at_a_known_level_measures_that_loudness() {
        for (sample_rate, amplitude) in [(48_000, 0.1), (44_100, 0.1), (44_100, 0.5)] {
            let lufs = clip_loudness(sine(amplitude, 1.0, sample_rate, 1), 1, sample_rate)
                .expect("a sine is not silent");
            let expected = expected_lufs(amplitude);
            assert!(
                (lufs - expected).abs() < 0.5,
                "{amplitude} at {sample_rate} Hz: {lufs:.2} LUFS, expected {expected:.2}"
            );
        }
    }

    #[test]
    fn a_click_shorter_than_one_block_measures_like_the_long_tone() {
        let long = clip_loudness(sine(0.1, 1.0, 44_100, 1), 1, 44_100).unwrap();
        let click = clip_loudness(sine(0.1, 0.02, 44_100, 1), 1, 44_100).unwrap();
        assert!(
            (long - click).abs() < 0.5,
            "one rule for all lengths: 1 s {long:.2}, 20 ms {click:.2}"
        );
    }

    #[test]
    fn stereo_sums_its_two_channels() {
        let mono = clip_loudness(sine(0.1, 1.0, 48_000, 1), 1, 48_000).unwrap();
        let stereo = clip_loudness(sine(0.1, 1.0, 48_000, 2), 2, 48_000).unwrap();
        assert!(
            (stereo - mono - 3.01).abs() < 0.1,
            "the same tone on both channels is 3 dB louder: {mono:.2} -> {stereo:.2}"
        );
    }

    #[test]
    fn silence_is_an_error_and_not_a_quiet_sound() {
        assert_eq!(
            clip_loudness(vec![0.0; 44_100], 1, 44_100),
            Err(LoudnessError::Silent { lufs: None })
        );
        let faint = sine(0.000_5, 1.0, 44_100, 1);
        assert!(
            matches!(
                clip_loudness(faint, 1, 44_100),
                Err(LoudnessError::Silent { lufs: Some(lufs) }) if lufs < SILENT_BELOW_LUFS
            ),
            "a tone at {:.1} LUFS is below the floor",
            expected_lufs(0.000_5)
        );
        assert_eq!(
            clip_loudness(std::iter::empty(), 1, 44_100),
            Err(LoudnessError::Empty)
        );
    }

    #[test]
    fn a_wav_file_decodes_and_measures() {
        let lufs = measure_clip(wav(&sine(0.1, 0.5, 44_100, 1), 44_100)).unwrap();
        assert!(
            (lufs - expected_lufs(0.1)).abs() < 0.5,
            "the decoded file measures like its samples: {lufs:.2}"
        );
        assert!(matches!(
            measure_clip(b"not a sound file"),
            Err(LoudnessError::Undecodable(_))
        ));
    }

    /// The reference measure: the whole clip decoded into memory, every
    /// looped frame's energy kept, and the blocks read off a prefix sum.
    fn whole_clip_loudness(samples: &[f32], channels: u16, sample_rate: u32) -> f32 {
        let channels = usize::from(channels.max(1));
        let frames = samples.len() / channels;
        let rate = f64::from(sample_rate);
        let loop_frames = (LOOP_SECONDS * rate).ceil() as usize;
        let repeats = loop_frames.div_ceil(frames).max(1);
        let mut filters = vec![KWeighting::new(rate); channels];
        let mut energy = Vec::with_capacity(frames * repeats);
        for _ in 0..repeats {
            for frame in samples.chunks_exact(channels) {
                let mut sum = 0.0;
                for (channel, (sample, filter)) in frame.iter().zip(&mut filters).enumerate() {
                    let filtered = filter.process(f64::from(*sample));
                    sum += channel_weight(channel, channels) * filtered * filtered;
                }
                energy.push(sum);
            }
        }
        let block = (BLOCK_SECONDS * rate).round() as usize;
        let step = (STEP_SECONDS * rate).round() as usize;
        let mut prefix = vec![0.0];
        for value in &energy {
            prefix.push(prefix.last().copied().unwrap_or_default() + value);
        }
        let blocks: Vec<f64> = (0..)
            .map(|index| index * step)
            .take_while(|start| start + block <= energy.len())
            .map(|start| (prefix[start + block] - prefix[start]) / block as f64)
            .collect();
        gated_loudness(&blocks).expect("the reference clip is not silent")
    }

    /// The decode of `bytes`, all of it, with its channel count and rate.
    fn decoded(bytes: Vec<u8>) -> (Vec<f32>, u16, u32) {
        let decoder = rodio::Decoder::new(Cursor::new(bytes)).expect("the file decodes");
        let channels = decoder.channels().get();
        let sample_rate = decoder.sample_rate().get();
        (decoder.collect(), channels, sample_rate)
    }

    #[test]
    fn streaming_loudness_matches_the_whole_clip_measure() {
        let sounds = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/base/sounds");
        let mut measured = 0;
        for entry in std::fs::read_dir(&sounds).expect("the base sounds are readable") {
            let path = entry.expect("a directory entry").path();
            if path.extension().is_none_or(|extension| extension != "wav") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("a base sound is readable");
            let streamed = measure_clip(bytes.clone()).expect("a base sound measures");
            let (samples, channels, sample_rate) = decoded(bytes);
            let reference = whole_clip_loudness(&samples, channels, sample_rate);
            assert!(
                (streamed - reference).abs() < 1e-4,
                "{}: streamed {streamed} LUFS, whole clip {reference} LUFS",
                path.display()
            );
            measured += 1;
        }
        assert!(measured > 0, "no base sound under {}", sounds.display());

        // Past the loop length a clip is measured once, with no copy kept: a
        // stereo tone whose level steps, so the relative gate has work to do.
        let long: Vec<f32> = sine(0.3, 3.0, 48_000, 2)
            .into_iter()
            .chain(sine(0.02, 3.0, 48_000, 2))
            .chain(sine(0.1, 3.0, 48_000, 2))
            .collect();
        let streamed = clip_loudness(long.iter().copied(), 2, 48_000).unwrap();
        let reference = whole_clip_loudness(&long, 2, 48_000);
        assert!(
            (streamed - reference).abs() < 1e-4,
            "a 9 s clip: streamed {streamed} LUFS, whole clip {reference} LUFS"
        );
    }

    #[test]
    fn the_gain_brings_a_clip_to_the_target() {
        let clip = ClipLoudness { lufs: -17.8 };
        assert!((clip.gain_db() - (-5.2)).abs() < 1e-4);
        assert!((clip.gain() - 10f32.powf(-5.2 / 20.0)).abs() < 1e-6);
    }

    #[test]
    fn a_loaded_clip_is_measured_and_an_unloaded_one_forgotten() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<AudioSource>();
        app.init_resource::<SoundLoudness>();
        app.add_systems(Update, measure_loaded_sounds);

        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource {
                bytes: wav(&sine(0.1, 0.5, 44_100, 1), 44_100).into(),
            });
        // The asset event is written in this frame's `PostUpdate` and read in
        // the next frame's `Update`, which starts the measure on the task
        // pool. The clip enters the table on a later frame.
        let clip = (0..10_000)
            .find_map(|_| {
                app.update();
                app.world().resource::<SoundLoudness>().get(&handle)
            })
            .expect("the clip is measured on load");
        assert!((clip.lufs - expected_lufs(0.1)).abs() < 0.5);

        let id = handle.id();
        drop(handle);
        app.update();
        app.update();
        assert!(
            app.world().resource::<SoundLoudness>().get(id).is_none(),
            "an unloaded clip leaves the table"
        );
    }
}
