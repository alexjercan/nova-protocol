//! Frame-time statistics and the run schema: [`FrameStats`], the per-run
//! metadata ([`RunMeta`]), and the CSV/JSON writers + parsers both the capture
//! harness and the report/probe consumers share, so the schema is defined once.
//!
//! Four CSV schema versions exist:
//!
//! - **v1** ([`CSV_HEADER_V1`]): the numeric columns only. The v0.7.0 baseline
//!   sweeps are v1 and must keep parsing - the reader accepts it and fills
//!   [`RunMeta::unknown`].
//! - **v2** ([`CSV_HEADER_V2`]): v1 plus the run-metadata columns
//!   (backend, adapter, resolution, quality, git_sha, host), so a results
//!   file is self-describing instead of leaning on its directory name.
//!   Rows parse with `profile = "unknown"`.
//! - **v3** ([`CSV_HEADER_V3`]): v2 plus the build `profile` column (`dev` or
//!   `release`) - dev-profile numbers are not baselines, and the report
//!   labels them.
//! - **v4** ([`CSV_HEADER`]): v3 plus the window's CLUSTER SHAPE
//!   ([`cluster_shape`]) - the evidence a host-side reader needs to tell a
//!   display period from a steady workload. Rows parse with no shape.

/// Glob-import surface for the frame-time wire format.
pub mod prelude {
    pub use super::{
        append_frametime_row, cluster_shape, parse_capture_abort_line, parse_fixed_steps_line,
        parse_frametime_csv, parse_summary_line, CaptureAbort, FixedStepBucket, FixedStepStats,
        FrameStats, PerfRun, RunMeta, CSV_HEADER, CSV_HEADER_V1, CSV_HEADER_V2, CSV_HEADER_V3,
        REFRESH_CAPPED, REFRESH_CAP_AGREEMENT, REFRESH_CAP_BAND, REFRESH_CAP_MIN_MS,
        REFRESH_CAP_SHARE,
    };
}

/// Token naming the finding that a capture window collapsed onto the DISPLAY's
/// refresh period instead of measuring the game: on the capture's suspicion
/// line, and as the repeat gate's verdict when the siblings confirm it.
///
/// The two halves have to speak one word for it, because neither can reach the
/// finding alone - the capture is the only side that knows the run promised not
/// to block on refresh, and the repeat set is the only side holding enough
/// captures to tell a period (a CONSTANT) from a workload that happens to be
/// steady.
pub const REFRESH_CAPPED: &str = "refresh_capped";

/// Half-width of the band around the median that [`cluster_shape`] counts
/// samples in.
pub const REFRESH_CAP_BAND: f64 = 0.05;

/// Half-width within which two captures' cluster medians count as the SAME
/// period, and so as evidence that a display produced both.
///
/// Deliberately far tighter than [`REFRESH_CAP_BAND`], because the two measure
/// different things. That band spans the SCATTER inside one window, which is
/// wide: a capped window is not a flat line, and one held a minimum 23% under
/// its period. This one spans the drift of a period ACROSS windows, which is a
/// crystal-derived constant and does not drift at all - the 165 Hz captures
/// this mechanism was built from agree to under a tenth of a percent.
///
/// Sized against the false positive that motivated the discriminator rather
/// than against the phenomenon alone: three windows of one steady workload read
/// 20.727 / 21.130 / 22.713 ms, which a 5% band came within 2.5 points of
/// convicting. Every value between a real display's agreement and that spread
/// works; this one sits an order of magnitude clear of both ends.
pub const REFRESH_CAP_AGREEMENT: f64 = 0.01;

/// Clustering at or above this share of the window makes a capture a
/// refresh-cap suspect.
///
/// MEASURED, not chosen: on a 165 Hz output, `Fifo` captures of the empty
/// gallery read 0.76 and 0.79 (and mean_fps 165.0 to three figures), while 34
/// `Immediate` captures across ship counts 0 and 1 spread 0.03-0.44. The
/// threshold sits in the gap with roughly equal headroom on both sides. A
/// capped window is NOT a flat line - its own minimum ran 23% under the period
/// - so a stricter share would miss the real thing while still passing every
/// synthetic one.
///
/// The gap is not permanent, which is why crossing it is a SUSPICION and never
/// a verdict: an optimisation makes frames steadier, so a threshold on
/// steadiness alone preferentially accuses the faster arm of an A/B.
pub const REFRESH_CAP_SHARE: f64 = 0.60;

/// Below this cluster median nothing can be a refresh period: no display
/// refreshes above 250 Hz, so a window this tight and this fast is a scene that
/// is genuinely cheap and genuinely steady.
///
/// Both halves apply it. The capture uses it to stay quiet, the repeat gate to
/// keep a fast steady set out of the suspect pool - a floor only one side
/// honoured would let the other convict on evidence the first threw away.
pub const REFRESH_CAP_MIN_MS: f64 = 4.0;

/// The frame time a capture window CLUSTERED on, and the share of its frames
/// inside `+/- REFRESH_CAP_BAND` of that value.
///
/// A workload produces a distribution: the empty gallery's own p99 is 1.6x its
/// median. A period produces a spike, because every frame waits for the same
/// clock edge. Share rather than a percentile ratio because a capped run can
/// still contain a handful of honest fast frames (one measured window held a
/// 5.0 ms minimum under a 16.7 ms cap) and a ratio built on min or max reads
/// those as spread.
pub fn cluster_shape(samples: &[f64]) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2];
    if median <= 0.0 {
        return (median, 0.0);
    }
    let inside = sorted
        .iter()
        .filter(|ms| (*ms - median).abs() / median <= REFRESH_CAP_BAND)
        .count();
    (median, inside as f64 / sorted.len() as f64)
}

/// A capture that REFUSED its window, scraped back out of the run log.
///
/// It has no CSV row and no JSON file by construction - that is what refusing
/// means - so the log line is the whole record, and the host half reads it
/// there. A run carrying one of these measured nothing, however plausible the
/// rows beside it look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureAbort {
    /// The capture's label (`wfc_arena#3`).
    pub label: String,
    /// Why the window was refused (`simulation_stopped`).
    pub reason: String,
    /// The phase it was refused in (`warmup` or `capture`).
    pub phase: String,
    /// How far into that phase the refusal landed, in frames.
    pub frame: usize,
    /// The window that was asked for, `(warmup, frames)`.
    pub window: (u32, u32),
}

/// Parse the capture's abort line (`nova perf: label=<l> ABORTED ...`).
/// `None` when the line is not one.
pub fn parse_capture_abort_line(line: &str) -> Option<CaptureAbort> {
    let rest = line.split("nova perf: label=").nth(1)?;
    let mut tokens = rest.split_whitespace();
    let label = tokens.next()?.to_string();
    if tokens.next()? != "ABORTED" {
        return None;
    }
    let mut reason = None;
    let mut phase = None;
    let mut frame = None;
    let mut warmup = None;
    let mut frames = None;
    for token in tokens {
        let Some((key, value)) = token.split_once('=') else {
            break;
        };
        match key {
            "reason" => reason = Some(value.to_string()),
            "phase" => phase = Some(value.to_string()),
            "frame" => frame = value.parse().ok(),
            "warmup" => warmup = value.parse().ok(),
            "frames" => frames = value.parse().ok(),
            _ => break,
        }
    }
    Some(CaptureAbort {
        label,
        reason: reason?,
        phase: phase?,
        frame: frame?,
        window: (warmup?, frames?),
    })
}

/// Percentile frame-time statistics over a capture window. Frame times are in
/// milliseconds; the derived FPS figures are `1000 / ms`.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameStats {
    /// Number of frames captured.
    pub frames: usize,
    /// Total wall-clock time of the capture window (ms).
    pub total_ms: f64,
    /// Mean frame time (ms).
    pub mean_ms: f64,
    /// Fastest (smallest) frame time (ms).
    pub min_ms: f64,
    /// Slowest (largest) frame time (ms).
    pub max_ms: f64,
    /// Median frame time (ms).
    pub p50_ms: f64,
    /// 95th-percentile frame time (ms).
    pub p95_ms: f64,
    /// 99th-percentile frame time (ms).
    pub p99_ms: f64,
    /// 99.9th-percentile frame time (ms).
    pub p999_ms: f64,
    /// Average frame rate (`1000 / mean_ms`).
    pub mean_fps: f64,
    /// "1% low" frame rate: the rate of the 99th-percentile-slowest frame
    /// (`1000 / p99_ms`), the standard stutter-floor figure.
    pub one_pct_low_fps: f64,
    /// The frame time the window clustered on ([`cluster_shape`]), recorded
    /// only when the run asked for a presentation mode that promised NOT to
    /// block on refresh. Clustering under vsync is the mode working, so it is
    /// not evidence and is not written.
    pub cluster_median_ms: Option<f64>,
    /// Share of the window inside [`REFRESH_CAP_BAND`] of
    /// [`Self::cluster_median_ms`], recorded under the same condition.
    ///
    /// DATA, not a verdict. Whether a clustered window is the display's period
    /// or a workload that is merely steady cannot be answered from one capture:
    /// a period is a CONSTANT, so it takes siblings to tell them apart, and the
    /// host half that holds the repeat set is where that call is made.
    pub cluster_share: Option<f64>,
}

/// How many fixed steps ran inside each captured frame, and what a frame
/// carrying that many steps cost. JSON-only: it is a diagnostic beside the
/// frame window, not a column of the comparable CSV schema.
///
/// The question it answers is whether a slow frame AMPLIFIES itself. Bevy
/// clamps a frame's virtual delta to `Time<Virtual>::max_delta` and then runs
/// as many fixed steps as the accumulated time allows, so a frame that
/// overruns the timestep hands its overrun to the next frame as extra steps.
/// A bucket table that stays flat says the fixed loop is not the amplifier; a
/// rising `mean_frame_ms` against `steps` says it is.
#[derive(Debug, Clone, PartialEq)]
pub struct FixedStepStats {
    /// Frames the counts cover (the capture window).
    pub frames: usize,
    /// Fixed steps run over the whole window.
    pub total_steps: u64,
    /// Fewest steps any single frame ran.
    pub min_steps: u32,
    /// Most steps any single frame ran - the amplification ceiling actually
    /// reached, against the `max_delta / timestep` one that is merely allowed.
    pub max_steps: u32,
    /// Steps per frame, averaged over the window.
    pub mean_steps: f64,
    /// One entry per observed step count, ascending: how many frames ran that
    /// many steps, and what those frames cost on average.
    pub buckets: Vec<FixedStepBucket>,
}

/// One row of [`FixedStepStats::buckets`].
#[derive(Debug, Clone, PartialEq)]
pub struct FixedStepBucket {
    /// Fixed steps this bucket's frames ran.
    pub steps: u32,
    /// Frames that ran exactly [`Self::steps`] steps.
    pub frames: usize,
    /// Mean wall-clock cost of those frames (ms).
    pub mean_frame_ms: f64,
    /// Slowest of those frames (ms).
    pub max_frame_ms: f64,
}

impl FixedStepStats {
    /// Pair each frame time with the step count recorded for it. `None` when
    /// there is nothing to summarize or the two series disagree in length -
    /// a mismatch means the counts were not drained per frame, and a summary
    /// built from it would be fiction.
    pub fn from_frames(frame_ms: &[f64], steps: &[u32]) -> Option<Self> {
        if frame_ms.is_empty() || frame_ms.len() != steps.len() {
            return None;
        }
        let mut counts: std::collections::BTreeMap<u32, (usize, f64, f64)> =
            std::collections::BTreeMap::new();
        for (ms, count) in frame_ms.iter().zip(steps) {
            let entry = counts.entry(*count).or_insert((0, 0.0, f64::MIN));
            entry.0 += 1;
            entry.1 += ms;
            entry.2 = entry.2.max(*ms);
        }
        let total_steps: u64 = steps.iter().map(|s| u64::from(*s)).sum();
        Some(Self {
            frames: frame_ms.len(),
            total_steps,
            min_steps: *steps.iter().min().expect("non-empty"),
            max_steps: *steps.iter().max().expect("non-empty"),
            mean_steps: total_steps as f64 / frame_ms.len() as f64,
            buckets: counts
                .into_iter()
                .map(
                    |(steps, (frames, total_ms, max_frame_ms))| FixedStepBucket {
                        steps,
                        frames,
                        mean_frame_ms: total_ms / frames as f64,
                        max_frame_ms,
                    },
                )
                .collect(),
        })
    }

    /// A greppable one-line summary, same `nova perf:` scrape prefix as the
    /// frame line so a log-only channel (the web capture) can carry it too.
    pub(crate) fn summary_line(&self, label: &str) -> String {
        let buckets = self
            .buckets
            .iter()
            .map(|b| format!("{}:{}@{:.1}ms", b.steps, b.frames, b.mean_frame_ms))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "nova perf: label={label} fixed_steps min={} max={} mean={:.3} total={} \
             buckets={buckets}",
            self.min_steps, self.max_steps, self.mean_steps, self.total_steps,
        )
    }

    /// The JSON object body (no leading key), hand-formatted like the rest of
    /// this dev-only crate's writers.
    fn to_json(&self) -> String {
        let buckets = self
            .buckets
            .iter()
            .map(|b| {
                format!(
                    "{{\"steps\": {}, \"frames\": {}, \"mean_frame_ms\": {:.4}, \
                     \"max_frame_ms\": {:.4}}}",
                    b.steps, b.frames, b.mean_frame_ms, b.max_frame_ms
                )
            })
            .collect::<Vec<_>>()
            .join(",\n      ");
        format!(
            "{{\n    \"frames\": {},\n    \"total_steps\": {},\n    \"min_steps\": {},\n    \
             \"max_steps\": {},\n    \"mean_steps\": {:.4},\n    \"buckets\": [\n      {}\n    ]\n  }}",
            self.frames, self.total_steps, self.min_steps, self.max_steps, self.mean_steps, buckets,
        )
    }

    /// Frames that ran NO fixed step. In a scene slower than the timestep that
    /// means the simulation was stopped - a pause, a result screen, a menu -
    /// and a capture window carrying them did not measure one scene. In a scene
    /// FASTER than the timestep it is ordinary and says nothing, which is why
    /// this is a number to read beside [`Self::mean_steps`], never a flag.
    pub fn stopped_frames(&self) -> usize {
        self.buckets
            .iter()
            .find(|b| b.steps == 0)
            .map_or(0, |b| b.frames)
    }

    /// Frames that ran the most steps the window ever ran, and what they cost.
    /// When that count is `max_delta / timestep` those frames are AT the clamp:
    /// they discarded real time the world never simulated.
    pub fn at_ceiling(&self) -> Option<&FixedStepBucket> {
        // By VALUE, not by position: the writer emits buckets ascending, but
        // a scraped line is whatever the log held.
        self.buckets.iter().max_by_key(|b| b.steps)
    }
}

/// Parse the capture's fixed-step summary line (`nova perf: label=<l>
/// fixed_steps ...`) back into stats. The report scrapes it out of the run log
/// rather than re-reading the per-run JSON, for the same reason the web capture
/// scrapes the frame line: the log is the one channel every run has.
/// Returns `(label, FixedStepStats)`; `None` when the line is not one.
pub fn parse_fixed_steps_line(line: &str) -> Option<(String, FixedStepStats)> {
    let rest = line.split("nova perf: label=").nth(1)?;
    let mut tokens = rest.split_whitespace();
    let label = tokens.next()?.to_string();
    if tokens.next()? != "fixed_steps" {
        return None;
    }
    let mut min_steps = None;
    let mut max_steps = None;
    let mut mean_steps = None;
    let mut total_steps = None;
    let mut buckets = Vec::new();
    for token in tokens {
        let Some((key, value)) = token.split_once('=') else {
            break;
        };
        match key {
            "min" => min_steps = value.parse().ok(),
            "max" => max_steps = value.parse().ok(),
            "mean" => mean_steps = value.parse().ok(),
            "total" => total_steps = value.parse().ok(),
            "buckets" => {
                for item in value.split(',') {
                    let (steps, rest) = item.split_once(':')?;
                    let (frames, ms) = rest.split_once('@')?;
                    buckets.push(FixedStepBucket {
                        steps: steps.parse().ok()?,
                        frames: frames.parse().ok()?,
                        mean_frame_ms: ms.trim_end_matches("ms").parse().ok()?,
                        // The line carries the bucket MEAN only; the per-run
                        // JSON is where a max lives. Reporting the mean twice
                        // would invent a number.
                        max_frame_ms: f64::NAN,
                    });
                }
            }
            _ => break,
        }
    }
    Some((
        label,
        FixedStepStats {
            frames: buckets.iter().map(|b| b.frames).sum(),
            total_steps: total_steps?,
            min_steps: min_steps?,
            max_steps: max_steps?,
            mean_steps: mean_steps?,
            buckets,
        },
    ))
}

/// Per-run metadata recorded alongside the stats (schema v2), so a results
/// file names its own renderer/config instead of leaning on the directory
/// name it happens to sit in. Every field is a plain string; absent knowledge
/// is the literal `"unknown"` (see [`RunMeta::unknown`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RunMeta {
    /// wgpu backend (`vulkan`, `metal`, `dx12`, `gl`, `webgpu`, ...).
    pub backend: String,
    /// GPU adapter name (e.g. `NVIDIA GeForce RTX 3060 Ti`).
    pub adapter: String,
    /// Forced window resolution, `WxH` (the capture request, e.g. `1280x720`).
    pub resolution: String,
    /// Graphics preset the run was asked for (`low`/`medium`/`high`, or
    /// `default` when the run kept the app default).
    pub quality: String,
    /// Short git SHA of the measured tree, or `unknown` outside a repo.
    pub git_sha: String,
    /// Host tag (env override, `/etc/hostname`, or `browser` on wasm).
    pub host: String,
    /// Build profile of the CAPTURE binary: `dev` or `release`, detected via
    /// `cfg!(debug_assertions)` at capture time (schema v3). Dev-profile
    /// numbers are NOT baselines - the report labels them so fps-everywhere
    /// wiring cannot invite apples-to-oranges
    /// deltas. Pre-v3 rows parse as `unknown`.
    pub profile: String,
}

impl RunMeta {
    /// The all-`unknown` metadata: what a v1 CSV row (pre-metadata schema)
    /// parses to, and the safe default when a source cannot be resolved.
    pub fn unknown() -> Self {
        let unknown = || "unknown".to_string();
        Self {
            backend: unknown(),
            adapter: unknown(),
            resolution: unknown(),
            quality: unknown(),
            git_sha: unknown(),
            host: unknown(),
            profile: unknown(),
        }
    }

    /// True when every field is still `"unknown"` (i.e. v1 data).
    pub fn is_unknown(&self) -> bool {
        self == &Self::unknown()
    }

    /// The metadata columns in [`CSV_HEADER`] order, comma-sanitized.
    pub(crate) fn csv_cells(&self) -> [String; 7] {
        [
            csv_safe(&self.backend),
            csv_safe(&self.adapter),
            csv_safe(&self.resolution),
            csv_safe(&self.quality),
            csv_safe(&self.git_sha),
            csv_safe(&self.host),
            csv_safe(&self.profile),
        ]
    }
}

/// Make a metadata value safe as a bare CSV cell: commas and line breaks
/// become spaces (adapter names are free-form vendor strings).
pub(crate) fn csv_safe(value: &str) -> String {
    value.replace([',', '\n', '\r'], " ").trim().to_string()
}

/// One captured run: its label, percentile stats, and run metadata. The unit
/// the aggregated `frametime.csv` stores one per row and the run report
/// renders one per table row.
#[derive(Debug, Clone, PartialEq)]
pub struct PerfRun {
    /// The run's label (e.g. `broadside-high`), as written by the capture.
    pub label: String,
    /// The percentile frame-time statistics for the run.
    pub stats: FrameStats,
    /// The run metadata ([`RunMeta::unknown`] for v1 data).
    pub meta: RunMeta,
}

/// Header row for the aggregated CSV, schema v4 (numeric columns + run
/// metadata + build profile + cluster shape), written when a new file is
/// created. Public so a reader can validate a file against the exact column
/// contract the writer emits. The two cluster cells are EMPTY when the run
/// asked for a mode that may block on refresh.
pub const CSV_HEADER: &str = "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,\
     mean_fps,one_pct_low_fps,backend,adapter,resolution,quality,git_sha,host,profile,\
     cluster_ms,cluster_share\n";

/// The schema v3 header (no cluster shape). Still accepted by the parser; its
/// rows parse with no shape, which reads as unmeasured rather than as absent
/// clustering.
pub const CSV_HEADER_V3: &str = "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,\
     mean_fps,one_pct_low_fps,backend,adapter,resolution,quality,git_sha,host,profile\n";

/// The schema v2 header (metadata without the build profile). Still
/// accepted by the parser; its rows parse with `profile = "unknown"`.
pub const CSV_HEADER_V2: &str = "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,\
     mean_fps,one_pct_low_fps,backend,adapter,resolution,quality,git_sha,host\n";

/// The pre-metadata schema v1 header. Still accepted by the parser so the
/// v0.7.0 baseline results keep loading (their rows parse with
/// [`RunMeta::unknown`]).
pub const CSV_HEADER_V1: &str =
    "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\n";

/// Column counts for the four schema versions (label + numerics [+ meta
/// [+ profile [+ cluster shape]]]).
const V1_COLS: usize = 11;
const V2_COLS: usize = 17;
const V3_COLS: usize = 18;
const V4_COLS: usize = 20;

impl FrameStats {
    /// Compute stats from a slice of per-frame times in milliseconds. Pure and
    /// order-independent (it sorts a copy), so it is unit-testable without an
    /// app. Percentiles use the nearest-rank method on the ascending sort, so
    /// `pXX` is a real observed frame time, never an interpolated value.
    pub fn from_samples(samples: &[f64]) -> Self {
        assert!(!samples.is_empty(), "FrameStats needs at least one sample");
        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("frame times are never NaN"));
        let n = sorted.len();
        let total_ms: f64 = sorted.iter().sum();
        let mean_ms = total_ms / n as f64;

        // Nearest-rank: the smallest value at or above the p-th percentile.
        let percentile = |p: f64| -> f64 {
            let rank = (p / 100.0 * n as f64).ceil() as usize;
            let idx = rank.saturating_sub(1).min(n - 1);
            sorted[idx]
        };

        Self {
            frames: n,
            total_ms,
            mean_ms,
            min_ms: sorted[0],
            max_ms: sorted[n - 1],
            p50_ms: percentile(50.0),
            p95_ms: percentile(95.0),
            p99_ms: percentile(99.0),
            p999_ms: percentile(99.9),
            mean_fps: 1000.0 / mean_ms,
            one_pct_low_fps: 1000.0 / percentile(99.0),
            cluster_median_ms: None,
            cluster_share: None,
        }
    }

    /// Attach the window's [`cluster_shape`]. Separate from
    /// [`Self::from_samples`] because whether the shape is evidence at all
    /// depends on the presentation mode the run asked for, which the samples
    /// do not carry.
    #[must_use]
    pub fn with_cluster(mut self, median_ms: f64, share: f64) -> Self {
        self.cluster_median_ms = Some(median_ms);
        self.cluster_share = Some(share);
        self
    }

    /// A compact, greppable one-line summary. The `nova perf:` prefix is a
    /// scrape contract (`probe run --platform web` greps it out of the browser
    /// console log) - do not rename it without updating the scrapers.
    pub(crate) fn summary_line(&self, label: &str) -> String {
        let mut line = format!(
            "nova perf: label={} frames={} mean={:.3}ms p50={:.3}ms p95={:.3}ms \
             p99={:.3}ms p999={:.3}ms min={:.3}ms max={:.3}ms mean_fps={:.1} 1%low_fps={:.1}",
            label,
            self.frames,
            self.mean_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms,
            self.p999_ms,
            self.min_ms,
            self.max_ms,
            self.mean_fps,
            self.one_pct_low_fps,
        );
        if let (Some(median), Some(share)) = (self.cluster_median_ms, self.cluster_share) {
            line.push_str(&format!(" cluster={median:.3}ms cluster_share={share:.3}"));
        }
        line
    }

    /// Render as a pretty JSON object (hand-formatted to avoid a serde dep in
    /// this dev-only crate). The metadata fields follow the numeric ones, and
    /// the cluster shape follows those - absent, never `null`, when the run
    /// made no no-vsync promise to measure it against.
    pub(crate) fn to_json(
        &self,
        label: &str,
        meta: &RunMeta,
        steps: Option<&FixedStepStats>,
        reload_ms: &[f64],
    ) -> String {
        let steps_field = steps
            .map(|steps| format!(",\n  \"fixed_steps\": {}", steps.to_json()))
            .unwrap_or_default();
        let reload_field = if reload_ms.is_empty() {
            String::new()
        } else {
            format!(
                ",\n  \"reload_ms\": [{}]",
                reload_ms
                    .iter()
                    .map(|ms| format!("{ms:.1}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let cluster_field = match (self.cluster_median_ms, self.cluster_share) {
            (Some(median), Some(share)) => {
                format!(",\n  \"cluster_median_ms\": {median:.4},\n  \"cluster_share\": {share:.4}")
            }
            _ => String::new(),
        };
        format!(
            "{{\n  \"label\": \"{}\",\n  \"frames\": {},\n  \"total_ms\": {:.3},\n  \
             \"mean_ms\": {:.4},\n  \"min_ms\": {:.4},\n  \"max_ms\": {:.4},\n  \
             \"p50_ms\": {:.4},\n  \"p95_ms\": {:.4},\n  \"p99_ms\": {:.4},\n  \
             \"p999_ms\": {:.4},\n  \"mean_fps\": {:.2},\n  \"one_pct_low_fps\": {:.2},\n  \
             \"backend\": \"{}\",\n  \"adapter\": \"{}\",\n  \"resolution\": \"{}\",\n  \
             \"quality\": \"{}\",\n  \"git_sha\": \"{}\",\n  \"host\": \"{}\",\n  \
             \"profile\": \"{}\"{}{}{}\n}}\n",
            json_safe(label),
            self.frames,
            self.total_ms,
            self.mean_ms,
            self.min_ms,
            self.max_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms,
            self.p999_ms,
            self.mean_fps,
            self.one_pct_low_fps,
            json_safe(&meta.backend),
            json_safe(&meta.adapter),
            json_safe(&meta.resolution),
            json_safe(&meta.quality),
            json_safe(&meta.git_sha),
            json_safe(&meta.host),
            json_safe(&meta.profile),
            cluster_field,
            steps_field,
            reload_field,
        )
    }

    /// One CSV data row (no header), schema v4: matches [`CSV_HEADER`]. The
    /// two cluster cells are empty when there is no shape to report.
    pub(crate) fn to_csv_row(&self, label: &str, meta: &RunMeta) -> String {
        let cells = meta.csv_cells();
        let cell = |value: Option<f64>| value.map_or_else(String::new, |v| format!("{v:.4}"));
        format!(
            "{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.2},{:.2},{},{},{},{},{},{},{},\
             {},{}\n",
            csv_safe(label),
            self.frames,
            self.mean_ms,
            self.min_ms,
            self.max_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms,
            self.p999_ms,
            self.mean_fps,
            self.one_pct_low_fps,
            cells[0],
            cells[1],
            cells[2],
            cells[3],
            cells[4],
            cells[5],
            cells[6],
            cell(self.cluster_median_ms),
            cell(self.cluster_share),
        )
    }
}

/// Escape the two characters that matter inside a JSON string literal here
/// (labels and vendor strings never legitimately contain control chars).
fn json_safe(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

impl PerfRun {
    /// Parse one aggregated-CSV data row (no header) - the inverse of
    /// `FrameStats::to_csv_row`. Accepts a v1 row (11 columns; metadata
    /// becomes [`RunMeta::unknown`]), a v2 row (17 columns; profile
    /// `unknown`), a v3 row (18 columns; no cluster shape) or a v4 row (20
    /// columns). The CSV omits `total_ms` (JSON-only), so it is reconstructed
    /// exactly as `mean_ms * frames` (mean is defined as `total / frames`).
    /// Returns `None` on any other column count or a numeric field that does
    /// not parse, so a truncated or foreign file is rejected rather than
    /// silently mis-read.
    pub fn from_csv_row(row: &str) -> Option<Self> {
        let cols: Vec<&str> = row.split(',').collect();
        if !matches!(cols.len(), V1_COLS | V2_COLS | V3_COLS | V4_COLS) {
            return None;
        }
        // "NaN"/"inf" parse as f64 but poison every downstream stat; a row
        // carrying them is corrupt, not data.
        let finite = |s: &str| s.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        let label = cols[0].to_string();
        let frames: usize = cols[1].trim().parse().ok()?;
        let mean_ms: f64 = finite(cols[2])?;
        let min_ms: f64 = finite(cols[3])?;
        let max_ms: f64 = finite(cols[4])?;
        let p50_ms: f64 = finite(cols[5])?;
        let p95_ms: f64 = finite(cols[6])?;
        let p99_ms: f64 = finite(cols[7])?;
        let p999_ms: f64 = finite(cols[8])?;
        let mean_fps: f64 = finite(cols[9])?;
        let one_pct_low_fps: f64 = finite(cols[10])?;
        let meta = if cols.len() >= V2_COLS {
            RunMeta {
                backend: cols[11].trim().to_string(),
                adapter: cols[12].trim().to_string(),
                resolution: cols[13].trim().to_string(),
                quality: cols[14].trim().to_string(),
                git_sha: cols[15].trim().to_string(),
                host: cols[16].trim().to_string(),
                profile: cols
                    .get(17)
                    .map(|cell| cell.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        } else {
            RunMeta::unknown()
        };
        // An EMPTY cell is the writer saying there was nothing to measure the
        // clustering against, so it reads as absent rather than failing the
        // row; a cell that is present and unparseable still fails it.
        let shape = |index: usize| -> Option<Option<f64>> {
            match cols.get(index).map(|cell| cell.trim()) {
                None | Some("") => Some(None),
                Some(cell) => finite(cell).map(Some),
            }
        };
        Some(Self {
            label,
            stats: FrameStats {
                frames,
                total_ms: mean_ms * frames as f64,
                mean_ms,
                min_ms,
                max_ms,
                p50_ms,
                p95_ms,
                p99_ms,
                p999_ms,
                mean_fps,
                one_pct_low_fps,
                cluster_median_ms: shape(18)?,
                cluster_share: shape(19)?,
            },
            meta,
        })
    }
}

/// Parse the capture's greppable summary line (`nova perf: label=...`) back
/// into stats - the WEB capture's only output channel (no filesystem in the
/// browser; the runner scrapes this from the chromium console log).
/// Returns `(label, FrameStats)`; `None` when the line is not a summary.
pub fn parse_summary_line(line: &str) -> Option<(String, FrameStats)> {
    let rest = line.split("nova perf: label=").nth(1)?;
    let mut label = None;
    let mut fields: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for (i, token) in rest.split_whitespace().enumerate() {
        if i == 0 {
            label = Some(token.to_string());
            continue;
        }
        // The line may be embedded in a wrapper that APPENDS text (chromium
        // CONSOLE lines carry %c style arguments after the message): the
        // summary fields are contiguous, so the first non-key=value token
        // ends the record instead of failing the parse.
        let Some((key, value)) = token.split_once('=') else {
            break;
        };
        let value = value.trim_end_matches("ms").trim_end_matches('"');
        match value.parse() {
            Ok(parsed) => fields.insert(key, parsed),
            Err(_) => break,
        };
    }
    let get = |k: &str| fields.get(k).copied();
    let frames = get("frames")? as usize;
    let mean_ms = get("mean")?;
    Some((
        label?,
        FrameStats {
            frames,
            total_ms: mean_ms * frames as f64,
            mean_ms,
            min_ms: get("min")?,
            max_ms: get("max")?,
            p50_ms: get("p50")?,
            p95_ms: get("p95")?,
            p99_ms: get("p99")?,
            p999_ms: get("p999")?,
            mean_fps: get("mean_fps")?,
            one_pct_low_fps: get("1%low_fps")?,
            cluster_median_ms: get("cluster"),
            cluster_share: get("cluster_share"),
        },
    ))
}

/// Append one labeled row (creating the file + v2 header when absent) - the
/// public writer for runners that assemble a frametime.csv from scraped
/// output (the web capture) rather than through the in-app plugin.
pub fn append_frametime_row(
    path: &std::path::Path,
    label: &str,
    stats: &FrameStats,
    meta: &RunMeta,
) -> Result<(), String> {
    use std::io::Write;
    let need_header = !path.exists();
    // Never mix schemas in one file: appending a v4 row under an older
    // header would give every consumer a column-count error at parse time
    // (or worse, silent misreads). Probe's fresh-dir discipline makes this
    // unreachable in practice; a manual NOVA_PROBE_OUT into an old results
    // dir is exactly when it matters.
    if !need_header {
        let existing = std::fs::read_to_string(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let header = existing.lines().next().unwrap_or("");
        if header.trim() != CSV_HEADER.trim() {
            return Err(format!(
                "{} has a pre-v4 header - appending would mix schemas; \
                 move the old file aside (its rows still parse read-only)",
                path.display()
            ));
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?;
    let mut buf = String::new();
    if need_header {
        buf.push_str(CSV_HEADER);
    }
    buf.push_str(&stats.to_csv_row(label, meta));
    file.write_all(buf.as_bytes())
        .map_err(|e| format!("could not append {}: {e}", path.display()))
}

/// Parse a whole aggregated `frametime.csv` (header + one row per run) into a
/// list of runs, preserving file order. The first line must match
/// [`CSV_HEADER`] (v4), [`CSV_HEADER_V3`], [`CSV_HEADER_V2`] or
/// [`CSV_HEADER_V1`] (trimmed) or the file is rejected as
/// not-a-frametime-CSV; every data row must then carry that version's column
/// count. Blank lines are skipped and any row that fails to parse is an error
/// naming its line, so a corrupt sweep is caught instead of silently dropping
/// runs. Shared by every frametime consumer so the schema lives in one place.
pub fn parse_frametime_csv(contents: &str) -> Result<Vec<PerfRun>, String> {
    let mut lines = contents.lines();
    let header = lines.next().ok_or("empty CSV (no header)")?;
    let expected_cols = if header.trim() == CSV_HEADER.trim() {
        V4_COLS
    } else if header.trim() == CSV_HEADER_V3.trim() {
        V3_COLS
    } else if header.trim() == CSV_HEADER_V2.trim() {
        V2_COLS
    } else if header.trim() == CSV_HEADER_V1.trim() {
        V1_COLS
    } else {
        return Err(format!(
            "unexpected CSV header\n  expected: {}\n  or (v3):  {}\n  or (v2):  {}\n  \
             or (v1):  {}\n  found:    {}",
            CSV_HEADER.trim(),
            CSV_HEADER_V3.trim(),
            CSV_HEADER_V2.trim(),
            CSV_HEADER_V1.trim(),
            header.trim()
        ));
    };
    let mut runs = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if line.split(',').count() != expected_cols {
            return Err(format!(
                "CSV row at data line {} has {} columns, header promises {}: {line:?}",
                i + 1,
                line.split(',').count(),
                expected_cols
            ));
        }
        let run = PerfRun::from_csv_row(line)
            .ok_or_else(|| format!("malformed CSV row at data line {}: {line:?}", i + 1))?;
        runs.push(run);
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some_meta() -> RunMeta {
        RunMeta {
            backend: "vulkan".to_string(),
            adapter: "NVIDIA GeForce RTX 3060 Ti".to_string(),
            resolution: "1280x720".to_string(),
            quality: "high".to_string(),
            git_sha: "f4bfb3af".to_string(),
            host: "devbox".to_string(),
            profile: "release".to_string(),
        }
    }

    #[test]
    fn v2_rows_parse_with_unknown_profile() {
        // A v2 file (pre-profile header + 17-column rows) must keep loading;
        // its rows carry profile "unknown", never a guess.
        let csv = format!(
            "{}scene-high,100,10.0,9.0,12.0,10.0,11.0,11.5,12.0,100.0,87.0,\
             vulkan,RTX,1280x720,high,abc123,devbox\n",
            CSV_HEADER_V2
        );
        let runs = parse_frametime_csv(&csv).expect("v2 parses");
        assert_eq!(runs[0].meta.quality, "high");
        assert_eq!(runs[0].meta.profile, "unknown");
    }

    #[test]
    fn v3_rows_parse_with_no_cluster_shape() {
        // A v3 file (pre-cluster header + 18-column rows) must keep loading;
        // its rows carry NO shape, which is unmeasured and not "unclustered".
        let csv = format!(
            "{}scene-high,100,10.0,9.0,12.0,10.0,11.0,11.5,12.0,100.0,87.0,\
             vulkan,RTX,1280x720,high,abc123,devbox,release\n",
            CSV_HEADER_V3
        );
        let runs = parse_frametime_csv(&csv).expect("v3 parses");
        assert_eq!(runs[0].meta.profile, "release");
        assert_eq!(runs[0].stats.cluster_median_ms, None);
        assert_eq!(runs[0].stats.cluster_share, None);
    }

    #[test]
    fn a_window_that_measured_its_cluster_shape_round_trips_through_the_csv() {
        let stats = FrameStats::from_samples(&[10.0; 10]).with_cluster(20.727, 0.64);
        let row = stats.to_csv_row("scene-high", &some_meta());
        assert_eq!(
            row.trim().split(',').count(),
            20,
            "v4 writes the two cluster columns"
        );
        let run = PerfRun::from_csv_row(row.trim()).expect("v4 row parses");
        assert_eq!(run.meta.profile, "release");
        assert!((run.stats.cluster_median_ms.expect("median") - 20.727).abs() < 5e-4);
        assert!((run.stats.cluster_share.expect("share") - 0.64).abs() < 5e-4);
    }

    /// A capture that made no no-vsync promise writes the columns EMPTY, and
    /// an empty cell must read back as unmeasured instead of as zero.
    #[test]
    fn a_window_with_no_cluster_shape_writes_empty_cells_that_read_back_as_absent() {
        let stats = FrameStats::from_samples(&[10.0; 10]);
        let row = stats.to_csv_row("scene-high", &some_meta());
        assert!(row.trim().ends_with("release,,"), "row: {row}");
        let csv = format!("{}{}", CSV_HEADER, row);
        let runs = parse_frametime_csv(&csv).expect("v4 file parses");
        assert_eq!(runs[0].meta, some_meta());
        assert_eq!(runs[0].stats.cluster_median_ms, None);
        assert_eq!(runs[0].stats.cluster_share, None);
    }

    #[test]
    fn stats_on_a_uniform_window_are_exact() {
        // Ten identical 10 ms frames: every percentile is 10 ms, 100 fps.
        let stats = FrameStats::from_samples(&[10.0; 10]);
        assert_eq!(stats.frames, 10);
        assert!((stats.mean_ms - 10.0).abs() < 1e-9);
        assert!((stats.p50_ms - 10.0).abs() < 1e-9);
        assert!((stats.p99_ms - 10.0).abs() < 1e-9);
        assert!((stats.mean_fps - 100.0).abs() < 1e-6);
        assert!((stats.one_pct_low_fps - 100.0).abs() < 1e-6);
    }

    #[test]
    fn percentiles_use_nearest_rank_on_a_known_ramp() {
        // 1..=100 ms. Nearest-rank: p50 -> rank 50 -> 50 ms, p95 -> 95 ms,
        // p99 -> 99 ms, p99.9 -> rank ceil(99.9) = 100 -> 100 ms.
        let samples: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let stats = FrameStats::from_samples(&samples);
        assert_eq!(stats.min_ms, 1.0);
        assert_eq!(stats.max_ms, 100.0);
        assert_eq!(stats.p50_ms, 50.0);
        assert_eq!(stats.p95_ms, 95.0);
        assert_eq!(stats.p99_ms, 99.0);
        assert_eq!(stats.p999_ms, 100.0);
        // 1% low uses the p99 frame (99 ms) -> ~10.1 fps.
        assert!((stats.one_pct_low_fps - 1000.0 / 99.0).abs() < 1e-6);
    }

    #[test]
    fn stats_are_order_independent() {
        let ascending: Vec<f64> = (1..=50).map(|i| i as f64).collect();
        let mut shuffled = ascending.clone();
        shuffled.reverse();
        assert_eq!(
            FrameStats::from_samples(&ascending),
            FrameStats::from_samples(&shuffled)
        );
    }

    #[test]
    fn v1_row_reads_a_known_literal_with_unknown_meta() {
        // A real row from the v0.7.0 sw baseline (broadside-high). Assert the
        // literal values, not just a round-trip, so a shared writer/reader bug
        // cannot pass this (roundtrip-hides-shared-bug). v1 back-compat pin:
        // 11 columns must keep parsing, with all-unknown metadata.
        let row =
            "broadside-high,120,115.0519,82.7471,168.3229,111.4533,140.7148,166.7084,168.3229,8.69,6.00";
        let run = PerfRun::from_csv_row(row).expect("valid v1 row parses");
        assert_eq!(run.label, "broadside-high");
        assert_eq!(run.stats.frames, 120);
        assert!((run.stats.mean_ms - 115.0519).abs() < 1e-9);
        assert!((run.stats.min_ms - 82.7471).abs() < 1e-9);
        assert!((run.stats.max_ms - 168.3229).abs() < 1e-9);
        assert!((run.stats.p99_ms - 166.7084).abs() < 1e-9);
        assert!((run.stats.mean_fps - 8.69).abs() < 1e-9);
        assert!((run.stats.one_pct_low_fps - 6.00).abs() < 1e-9);
        // total_ms is reconstructed as mean * frames (CSV omits it).
        assert!((run.stats.total_ms - 115.0519 * 120.0).abs() < 1e-6);
        assert!(run.meta.is_unknown());
    }

    #[test]
    fn a_written_row_reads_back_with_its_stats_and_meta() {
        // Forward (to_csv_row) then back (from_csv_row) preserves every field
        // the CSV carries, metadata included. total_ms is CSV-omitted, so
        // compare the rest.
        let original = FrameStats::from_samples(&[8.0, 12.0, 10.0, 40.0, 9.5, 11.0, 10.5]);
        let meta = some_meta();
        let row = original.to_csv_row("shakedown_run-low", &meta);
        let run = PerfRun::from_csv_row(row.trim()).expect("the row round-trips");
        assert_eq!(run.label, "shakedown_run-low");
        // The written row has 4-decimal precision, so compare at that scale.
        assert!((run.stats.mean_ms - original.mean_ms).abs() < 5e-4);
        assert!((run.stats.p99_ms - original.p99_ms).abs() < 5e-4);
        assert!((run.stats.max_ms - original.max_ms).abs() < 5e-4);
        assert_eq!(run.stats.frames, original.frames);
        assert_eq!(run.meta, meta);
    }

    #[test]
    fn meta_values_with_commas_are_sanitized_into_one_cell() {
        // A vendor string with commas must not shift the CSV columns.
        let mut meta = some_meta();
        meta.adapter = "Intel, Inc. UHD Graphics,  770".to_string();
        let stats = FrameStats::from_samples(&[10.0; 5]);
        let row = stats.to_csv_row("scene", &meta);
        assert_eq!(row.trim().split(',').count(), 20, "row: {row}");
        let run = PerfRun::from_csv_row(row.trim()).expect("sanitized row parses");
        assert_eq!(run.meta.adapter, "Intel  Inc. UHD Graphics   770");
    }

    #[test]
    fn non_finite_numerics_reject_the_row() {
        let row =
            "broadside-high,120,NaN,82.7471,168.3229,111.4533,140.7148,166.7084,168.3229,8.69,6.00";
        assert!(PerfRun::from_csv_row(row).is_none(), "NaN mean rejected");
        let row = "broadside-high,120,115.0,82.7,inf,111.4,140.7,166.7,168.3,8.69,6.00";
        assert!(PerfRun::from_csv_row(row).is_none(), "inf max rejected");
    }

    #[test]
    fn parse_frametime_csv_rejects_a_foreign_header() {
        let err = parse_frametime_csv("a,b,c\n1,2,3\n").expect_err("foreign header rejected");
        assert!(err.contains("unexpected CSV header"), "{err}");
    }

    #[test]
    fn parse_frametime_csv_reads_a_v1_file_in_order() {
        let csv = format!(
            "{}asteroid_field-high,120,126.5503,96.6889,166.1786,125.4380,152.8573,164.2634,166.1786,7.90,6.09\n\
             broadside-low,120,98.8898,72.3828,133.8965,98.2504,118.7390,133.2727,133.8965,10.11,7.50\n",
            CSV_HEADER_V1
        );
        let runs = parse_frametime_csv(&csv).expect("v1 file parses");
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].label, "asteroid_field-high");
        assert_eq!(runs[1].label, "broadside-low");
        assert!((runs[0].stats.p99_ms - 164.2634).abs() < 1e-9);
        assert!(runs.iter().all(|run| run.meta.is_unknown()));
    }

    #[test]
    fn parse_frametime_csv_rejects_a_row_width_mismatch() {
        // A v4 header promises 20 columns; an 11-column (v1-shaped) row under
        // it is a corrupt file, not a silent meta-default.
        let csv = format!(
            "{}broadside-high,120,115.0519,82.7471,168.3229,111.4533,140.7148,166.7084,168.3229,8.69,6.00\n",
            CSV_HEADER
        );
        let err = parse_frametime_csv(&csv).expect_err("width mismatch rejected");
        assert!(err.contains("header promises"), "{err}");
    }

    #[test]
    fn parse_frametime_csv_errors_on_a_truncated_row() {
        let csv = format!("{}broadside-high,120,115.05\n", CSV_HEADER_V1);
        let err = parse_frametime_csv(&csv).expect_err("short row rejected");
        assert!(err.contains("header promises"), "{err}");
    }

    #[test]
    fn summary_line_round_trips_through_the_real_writer() {
        // The web capture's contract: whatever summary_line prints,
        // parse_summary_line reads back (the scrape is the only channel).
        let stats = FrameStats::from_samples(&[30.0, 35.0, 40.0, 33.0, 31.0]);
        let line = stats.summary_line("asteroid_field-high-web");
        let (label, parsed) = parse_summary_line(&line).expect("summary parses");
        assert_eq!(label, "asteroid_field-high-web");
        assert_eq!(parsed.frames, stats.frames);
        // The line prints 3 decimals; compare at that precision.
        assert!((parsed.mean_ms - stats.mean_ms).abs() < 5e-3);
        assert!((parsed.p99_ms - stats.p99_ms).abs() < 5e-3);
        assert!((parsed.one_pct_low_fps - stats.one_pct_low_fps).abs() < 5e-2);
        // Embedded in a chromium console line with a prefix: still parses.
        let wrapped = format!("[1234:5678:INFO:CONSOLE(1)] {line}");
        assert!(parse_summary_line(&wrapped).is_some());
        assert!(parse_summary_line("unrelated log line").is_none());
        // The REAL chromium CONSOLE format (captured live 2026-07-19):
        // style markers before and TRAILING style arguments after the
        // message - the parser must stop at the junk, not fail.
        let real = r#"[997943:997943:0719/185025.216734:INFO:CONSOLE:1486] "%cINFO%c crates/nova_probe/src/capture.rs:398%c nova perf: label=asteroid_field-high-web frames=600 mean=31.607ms p50=31.300ms p95=44.300ms p99=48.900ms p999=60.800ms min=16.600ms max=60.800ms mean_fps=31.6 1%low_fps=20.4 color: whitesmoke; background: #444 color: gray; font-style: italic color: inherit", source: http://127.0.0.1:42609/perf_web-cd5e76059d930d0f.js (1486)"#;
        let (label, stats) = parse_summary_line(real).expect("real chromium line parses");
        assert_eq!(label, "asteroid_field-high-web");
        assert_eq!(stats.frames, 600);
        assert!((stats.mean_ms - 31.607).abs() < 1e-9);
        assert!((stats.one_pct_low_fps - 20.4).abs() < 1e-9);
    }

    /// The web capture has no filesystem, so the summary line is the only
    /// channel the cluster shape can reach the host on. A line without one
    /// must not invent a zero.
    #[test]
    fn the_summary_line_carries_the_cluster_shape_only_when_there_is_one() {
        let bare = FrameStats::from_samples(&[30.0, 35.0, 40.0]);
        let line = bare.summary_line("scene-web");
        assert!(!line.contains("cluster"), "{line}");
        let (_, parsed) = parse_summary_line(&line).expect("summary parses");
        assert_eq!(parsed.cluster_median_ms, None);
        assert_eq!(parsed.cluster_share, None);

        let shaped = bare.with_cluster(20.727, 0.643);
        let line = shaped.summary_line("scene-web");
        let (_, parsed) = parse_summary_line(&line).expect("summary parses");
        assert!((parsed.cluster_median_ms.expect("median") - 20.727).abs() < 5e-3);
        assert!((parsed.cluster_share.expect("share") - 0.643).abs() < 5e-3);
    }

    #[test]
    fn append_frametime_row_creates_header_then_appends() {
        let dir = std::env::temp_dir().join(format!("nova_probe_append_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("frametime.csv");
        let stats = FrameStats::from_samples(&[10.0; 4]);
        append_frametime_row(&path, "a-high", &stats, &some_meta()).unwrap();
        append_frametime_row(&path, "a-low", &stats, &some_meta()).unwrap();
        let runs = parse_frametime_csv(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].label, "a-high");
        assert_eq!(runs[1].label, "a-low");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn json_carries_the_meta_fields() {
        let stats = FrameStats::from_samples(&[10.0; 3]);
        let json = stats.to_json("scene", &some_meta(), None, &[]);
        assert!(json.contains("\"backend\": \"vulkan\""), "{json}");
        assert!(json.contains("\"git_sha\": \"f4bfb3af\""), "{json}");
        assert!(json.contains("\"adapter\": \"NVIDIA GeForce RTX 3060 Ti\""));
        assert!(
            !json.contains("fixed_steps"),
            "a run with no step counts must not invent the key: {json}"
        );
    }

    #[test]
    fn fixed_steps_bucket_frames_by_step_count() {
        // Four frames: one ran no step, two ran one, one ran three.
        let frame_ms = [4.0, 16.0, 18.0, 60.0];
        let steps = [0, 1, 1, 3];
        let stats = FixedStepStats::from_frames(&frame_ms, &steps).expect("summarizes");
        assert_eq!(stats.frames, 4);
        assert_eq!(stats.total_steps, 5);
        assert_eq!(stats.min_steps, 0);
        assert_eq!(stats.max_steps, 3);
        assert!((stats.mean_steps - 1.25).abs() < 1e-9);
        assert_eq!(stats.buckets.len(), 3, "one row per OBSERVED count");
        assert_eq!(stats.buckets[1].steps, 1);
        assert_eq!(stats.buckets[1].frames, 2);
        assert!((stats.buckets[1].mean_frame_ms - 17.0).abs() < 1e-9);
        assert!((stats.buckets[1].max_frame_ms - 18.0).abs() < 1e-9);
        let json = stats.to_json();
        assert!(json.contains("\"max_steps\": 3"), "{json}");
    }

    /// A count series that does not line up with the frames is not data: the
    /// drain missed a frame, and every bucket built from it would be shifted.
    #[test]
    fn fixed_steps_refuse_a_mismatched_series() {
        assert!(FixedStepStats::from_frames(&[10.0, 11.0], &[1]).is_none());
        assert!(FixedStepStats::from_frames(&[], &[]).is_none());
    }

    #[test]
    fn fixed_step_summary_line_names_the_label_and_the_ceiling() {
        let stats = FixedStepStats::from_frames(&[10.0, 40.0], &[1, 4]).expect("summarizes");
        let line = stats.summary_line("wfc_arena");
        assert!(line.contains("label=wfc_arena"), "{line}");
        assert!(line.contains("fixed_steps min=1 max=4"), "{line}");
        assert!(line.contains("buckets=1:1@10.0ms,4:1@40.0ms"), "{line}");
    }

    /// The scrape contract: whatever the capture prints, the report reads back.
    #[test]
    fn fixed_step_summary_line_round_trips_through_the_real_writer() {
        let frame_ms = [70.0, 70.0, 90.0, 330.0];
        let steps = [0, 4, 6, 16];
        let stats = FixedStepStats::from_frames(&frame_ms, &steps).expect("summarizes");
        let line = stats.summary_line("wfc_arena#5");
        let (label, parsed) = parse_fixed_steps_line(&line).expect("parses");
        assert_eq!(label, "wfc_arena#5");
        assert_eq!(parsed.frames, 4);
        assert_eq!(parsed.total_steps, 26);
        assert_eq!(parsed.min_steps, 0);
        assert_eq!(parsed.max_steps, 16);
        assert!((parsed.mean_steps - 6.5).abs() < 5e-4);
        // A stopped simulation inside the window is the reading that matters.
        assert_eq!(parsed.stopped_frames(), 1);
        let top = parsed.at_ceiling().expect("a top bucket");
        assert_eq!(top.steps, 16);
        assert!((top.mean_frame_ms - 330.0).abs() < 5e-2);

        // Embedded in a log line with a prefix: still parses. The FRAME line is
        // not this line and must not read as one.
        let wrapped = format!("2026-08-19T15:40:00Z INFO nova_probe: {line}");
        assert!(parse_fixed_steps_line(&wrapped).is_some());
        let frame_line = FrameStats::from_samples(&frame_ms).summary_line("wfc_arena#5");
        assert!(parse_fixed_steps_line(&frame_line).is_none());
        assert!(parse_fixed_steps_line("unrelated log line").is_none());
    }

    /// A window with no stopped frame and no clamp reads as exactly that,
    /// rather than as a missing measurement.
    #[test]
    fn a_window_that_never_stopped_reports_zero_stopped_frames() {
        let stats = FixedStepStats::from_frames(&[60.0, 75.0], &[4, 5]).expect("summarizes");
        assert_eq!(stats.stopped_frames(), 0);
        assert_eq!(stats.at_ceiling().map(|b| b.steps), Some(5));
    }

    /// An aborted capture writes no row, so the log line is its only record
    /// and it has to survive a log prefix.
    #[test]
    fn the_abort_line_parses_back_out_of_a_prefixed_log() {
        let line = "2026-08-19T15:40:00Z ERROR nova_probe: nova perf: label=wfc_arena#3 \
                    ABORTED reason=simulation_stopped phase=capture frame=345 warmup=180 \
                    frames=900 - Time<Virtual> was stopped (paused, or running at speed 0)";
        let abort = parse_capture_abort_line(line).expect("parses");
        assert_eq!(abort.label, "wfc_arena#3");
        assert_eq!(abort.reason, "simulation_stopped");
        assert_eq!(abort.phase, "capture");
        assert_eq!(abort.frame, 345);
        assert_eq!(abort.window, (180, 900));

        // Neither of the other two `nova perf: label=` lines reads as one.
        let stats = FrameStats::from_samples(&[60.0, 75.0]);
        assert!(parse_capture_abort_line(&stats.summary_line("s")).is_none());
        let steps = FixedStepStats::from_frames(&[60.0, 75.0], &[4, 5]).expect("summarizes");
        assert!(parse_capture_abort_line(&steps.summary_line("s")).is_none());
        assert!(parse_capture_abort_line("unrelated log line").is_none());
    }
}
