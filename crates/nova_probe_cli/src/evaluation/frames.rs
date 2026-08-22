//! The FRAME READ: what a capture cost, put in front of a reader.
//!
//! Nothing here grades. Whether a frame rate is good enough is a judgement
//! about a scene, a machine and a build, and the only thing qualified to make
//! it is the person looking at the report - so this module answers "how slow
//! was the worst frame, what does that come to in FPS, and is it under 60",
//! and stops.
//!
//! There used to be a table of per-example millisecond budgets here that
//! FAILED a run. It baked example names into library code and turned a reading
//! into a verdict; a number a script asserts is a number nobody looks at.

use nova_probe::prelude::*;

/// Glob-import surface for the frame read.
pub mod prelude {
    pub use super::{
        fixed_steps_json, frames_json, read_frames, read_repeats, repeat_label, repeats_json,
        split_repeat_label, FrameRead, RefreshCap, RepeatCapture, RepeatRead,
        REPEAT_GATE_TOLERANCE, SMOOTH_FPS, SMOOTH_FRAME_MS,
    };
}

/// The line the report draws: 60 frames a second, the rate the game is meant
/// to hold. It is a READING AID, never a gate.
pub const SMOOTH_FPS: f64 = 60.0;

/// [`SMOOTH_FPS`] as a frame time in milliseconds, so a reader comparing a
/// `max_ms` against the line does not have to do the division.
pub const SMOOTH_FRAME_MS: f64 = 1000.0 / SMOOTH_FPS;

/// One capture row, read for a human.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameRead {
    /// The `frametime.csv` row label.
    pub label: String,
    /// The single slowest frame of the window, milliseconds.
    pub worst_ms: f64,
    /// The mean frame time over the window, milliseconds.
    pub mean_ms: f64,
    /// What [`Self::worst_ms`] comes to in frames per second - the rate the
    /// run dropped to at its worst moment, which is what a stutter feels like.
    pub worst_fps: f64,
    /// What [`Self::mean_ms`] comes to in frames per second.
    pub mean_fps: f64,
}

impl FrameRead {
    /// Whether the AVERAGE frame missed the 60 fps line.
    pub fn mean_is_slow(&self) -> bool {
        self.mean_fps < SMOOTH_FPS
    }

    /// Whether the WORST frame missed the 60 fps line. Nearly every run trips
    /// this, which is the point: a tail is normal and a reader wants to see
    /// how deep it went, not be told it happened.
    pub fn worst_is_slow(&self) -> bool {
        self.worst_fps < SMOOTH_FPS
    }

    /// The one-line call, in the words a reader wants at a glance.
    pub fn verdict(&self) -> &'static str {
        if self.mean_is_slow() {
            "FPS IS UNDER 60 - BAD"
        } else if self.worst_is_slow() {
            "60 FPS ON AVERAGE, BUT THE WORST FRAME IS UNDER 60"
        } else {
            "60 FPS OR BETTER, WORST FRAME INCLUDED - GOOD"
        }
    }

    /// CSS class token for the verdict, so the HTML and this file agree on
    /// which reading is which colour.
    pub fn class(&self) -> &'static str {
        if self.mean_is_slow() {
            "bad"
        } else if self.worst_is_slow() {
            "mixed"
        } else {
            "good"
        }
    }
}

/// Frames per second for a frame time in milliseconds. A zero or negative
/// frame time is not a measurement, so it reads as zero rather than infinity.
fn fps_of(ms: f64) -> f64 {
    if ms > 0.0 {
        1000.0 / ms
    } else {
        0.0
    }
}

/// Read every capture row. Ordered worst frame first: the row a reader has to
/// look at is the slow one, and a sweep can carry a dozen.
pub fn read_frames(runs: &[PerfRun]) -> Vec<FrameRead> {
    let mut reads: Vec<FrameRead> = runs
        .iter()
        .map(|run| FrameRead {
            label: run.label.clone(),
            worst_ms: run.stats.max_ms,
            mean_ms: run.stats.mean_ms,
            worst_fps: fps_of(run.stats.max_ms),
            mean_fps: fps_of(run.stats.mean_ms),
        })
        .collect();
    reads.sort_by(|a, b| b.worst_ms.total_cmp(&a.worst_ms));
    reads
}

/// The `checks.json` mirror of the frame read.
///
/// It sits BESIDE the checks rather than among them, because it is not one:
/// an agent reading this file must not fold it into a verdict. The `graded`
/// field says so in the payload itself, for a consumer that only reads keys.
pub fn frames_json(runs: Option<&Vec<PerfRun>>) -> serde_json::Value {
    let reads = runs.map(|runs| read_frames(runs)).unwrap_or_default();
    serde_json::json!({
        "graded": false,
        "smooth_fps": SMOOTH_FPS,
        "smooth_frame_ms": SMOOTH_FRAME_MS,
        "note": "reported for a reader, never graded - no check passes or fails \
                 on a frame-time number",
        "captures": reads.iter().map(|read| serde_json::json!({
            "label": read.label,
            "worst_ms": read.worst_ms,
            "worst_fps": read.worst_fps,
            "mean_ms": read.mean_ms,
            "mean_fps": read.mean_fps,
            "mean_under_60_fps": read.mean_is_slow(),
            "worst_under_60_fps": read.worst_is_slow(),
            "read": read.verdict(),
        })).collect::<Vec<_>>(),
    })
}

/// The `checks.json` mirror of the fixed-step record, scraped from the run
/// log. Beside the checks, `graded: false`, like every other frame reading.
///
/// `stopped_frames` is the one an automated reader should look at: in a scene
/// slower than the fixed timestep it means the simulation was halted inside
/// the capture window, and that window did not measure one scene.
pub fn fixed_steps_json(log: Option<&String>) -> serde_json::Value {
    let reads: Vec<(String, FixedStepStats)> = log
        .map(|log| log.lines().filter_map(parse_fixed_steps_line).collect())
        .unwrap_or_default();
    serde_json::json!({
        "graded": false,
        "note": "how many fixed steps ran inside each captured frame. Frames with \
                 none mean the simulation was stopped inside the window; frames at \
                 the top of the range are where Time<Virtual>::max_delta is \
                 discarding time. Reported, never graded.",
        "captures": reads.iter().map(|(label, steps)| serde_json::json!({
            "label": label,
            "mean_steps": steps.mean_steps,
            "min_steps": steps.min_steps,
            "max_steps": steps.max_steps,
            "total_steps": steps.total_steps,
            "stopped_frames": steps.stopped_frames(),
            "frames_at_max": steps.at_ceiling().map_or(0, |b| b.frames),
            "mean_ms_at_max": steps.at_ceiling().map(|b| b.mean_frame_ms),
        })).collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// The REPEAT GATE.
// ---------------------------------------------------------------------------

/// Separator between a repeat set's base label and its index (`wfc_arena#3`).
/// A capture label is free-form, so the character has to be one nothing else
/// uses; `-` is already the sweep's preset separator.
const REPEAT_SEPARATOR: char = '#';

/// How far a repeat's mean and median may sit from the set's reference before
/// the repeat is discarded, as a fraction.
///
/// MEASURED, not chosen, and it is deliberately loose. Eight captures of an
/// unchanged shipped chapter on a settled reference host spread their means
/// 34% and their medians 29%; a band tighter than about 12% therefore admits
/// fewer than half the set, and the survivors are arbitrary enough that the
/// reported tail gets NOISIER, not quieter. At this width the band still
/// throws out the contamination it exists for - a capture taken while the box
/// was still busy measured 3.9x the settled value - while admitting seven
/// captures out of eight of an honest set.
///
/// Re-derive it on a different host. It is a property of the machine, not of
/// the code.
pub const REPEAT_GATE_TOLERANCE: f64 = 0.20;

/// The label a repeat capture writes: `<base>#<n>`, one-based.
pub fn repeat_label(base: &str, repeat: u32) -> String {
    format!("{base}{REPEAT_SEPARATOR}{repeat}")
}

/// Split a capture label back into `(base, repeat index)`. A label with no
/// separator - every non-repeat capture - reads as its own base with no index.
pub fn split_repeat_label(label: &str) -> (&str, Option<u32>) {
    match label.rsplit_once(REPEAT_SEPARATOR) {
        Some((base, index)) => match index.parse() {
            Ok(index) => (base, Some(index)),
            Err(_) => (label, None),
        },
        None => (label, None),
    }
}

/// One capture of a repeat set, with the gate's call on it.
#[derive(Debug, Clone, PartialEq)]
pub struct RepeatCapture {
    /// The repeat index parsed out of the label (one-based).
    pub index: u32,
    /// Mean frame time over the capture window (ms).
    pub mean_ms: f64,
    /// Median frame time over the capture window (ms).
    pub median_ms: f64,
    /// The 99th-percentile frame time (ms) - the 1%-low tail.
    pub p99_ms: f64,
    /// The single slowest frame of the capture (ms).
    pub worst_ms: f64,
    /// The frame time this window clustered on, as the capture measured it.
    /// `None` when the run made no no-vsync promise to measure it against, or
    /// when the row predates the schema that carries it.
    pub cluster_median_ms: Option<f64>,
    /// Share of the window inside [`REFRESH_CAP_BAND`] of
    /// [`Self::cluster_median_ms`], under the same condition.
    pub cluster_share: Option<f64>,
    /// Whether both stable statistics sat inside the band. Also false for
    /// every capture of a set the discriminator called [`RefreshCap::Capped`]:
    /// those windows measured the display, so none of them counts.
    pub admitted: bool,
}

/// What a repeat set's cluster shapes say about whether the DISPLAY, rather
/// than the game, decided the frame time.
///
/// The discriminator is that a refresh period is a CONSTANT. One clustered
/// window is ambiguous - an optimisation makes frames steadier, so steadiness
/// alone accuses the faster arm of an A/B - but a SET of them is not: captures
/// of one display agree on the period to a fraction of a percent, and a steady
/// workload does not. Three windows refused as capped on one Xvfb reported
/// cluster medians of 20.727 / 21.130 / 22.713 ms, which no single display
/// produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshCap {
    /// No capture carried a cluster shape: pre-schema-v4 rows, or a run that
    /// asked for a mode allowed to block on refresh. Unmeasured, not clean.
    Unmeasured,
    /// Shapes were measured and none cleared both bars ([`REFRESH_CAP_SHARE`]
    /// and [`REFRESH_CAP_MIN_MS`]).
    NotSuspected,
    /// Exactly one capture is a suspect, so there is no sibling to compare its
    /// period against. Suspected and UNVERIFIABLE - the evidence to decide is
    /// missing, and inventing a call either way is worse than saying so.
    Unverifiable,
    /// Two or more suspects whose cluster medians DISAGREE. A period does not
    /// drift, so this is a workload that is merely steady, and the captures
    /// are gated on their statistics like any others.
    Workload,
    /// Two or more suspects agreeing on one period within
    /// [`REFRESH_CAP_AGREEMENT`]. The set measured the display.
    Capped,
}

impl RefreshCap {
    /// The token `checks.json` carries, sharing [`REFRESH_CAPPED`] with the
    /// capture's suspicion line so both halves name the finding once.
    pub fn token(self) -> &'static str {
        match self {
            Self::Unmeasured => "unmeasured",
            Self::NotSuspected => "not_suspected",
            Self::Unverifiable => "unverifiable",
            Self::Workload => "workload",
            Self::Capped => REFRESH_CAPPED,
        }
    }
}

/// A repeat set read through the gate.
///
/// The design in one line: the mean and the median decide whether a capture
/// COUNTS, and only then is its worst frame read. A worst frame moves by tens
/// of percent between two captures of an unchanged scene, so it cannot police
/// itself; the mean and the median are the stable pair, so they can.
///
/// The gate is only as good as the SUBJECT. On a scene whose own load depends
/// on how fast the machine ran it - a live fight in a frame-count-bounded
/// window - the mean is not stable either, the band discards honest captures,
/// and the set correctly reports that it measured nothing. That reading is the
/// point: a subject the gate empties is a subject that cannot be a baseline.
///
/// The set is also the only place a refresh cap can be RULED ON - see
/// [`RefreshCap`] - and a set that measured the display empties for that
/// reason instead.
///
/// Nothing here grades. A discarded capture is a statement about the
/// measurement, never about the code under it.
#[derive(Debug, Clone, PartialEq)]
pub struct RepeatRead {
    /// The shared base label (`wfc_arena` for `wfc_arena#1..#n`).
    pub label: String,
    /// The reference the band is centred on: the MEDIAN of the captures' own
    /// means, so the reference does not move until half the set is bad.
    pub reference_mean_ms: f64,
    /// The same, for the captures' medians.
    pub reference_median_ms: f64,
    /// Every capture in file order, gated.
    pub captures: Vec<RepeatCapture>,
    /// The tail a CLAIM should be made on: the median of the admitted
    /// captures' p99. `None` when the gate admitted nothing.
    ///
    /// p99 rather than the slowest single frame, because the slowest single
    /// frame is one sample and behaves like one: over eight settled captures
    /// of an unchanged scene it spread 148% of its median against p99's 87%,
    /// and the smallest improvement the two can resolve is 46% against 27%.
    /// It is still a tail - the ninth-worst frame of a 900-frame window - so
    /// it is still a stutter and not an average.
    pub p99_ms: Option<f64>,
    /// Spread of the admitted p99 values, as a fraction of [`Self::p99_ms`].
    pub p99_spread: Option<f64>,
    /// The single slowest admitted frame's median, reported BESIDE the p99 as
    /// a diagnostic. Read it to see how deep the deepest frame went; do not
    /// build a claim on a change in it that the spread beside it covers.
    pub worst_ms: Option<f64>,
    /// Spread of the admitted worst frames, as a fraction of
    /// [`Self::worst_ms`].
    pub worst_spread: Option<f64>,
    /// Whether the set measured the display's refresh period instead of the
    /// game - the call the capture cannot make on its own.
    pub refresh_cap: RefreshCap,
}

impl RepeatRead {
    /// Captures the gate admitted.
    pub fn admitted(&self) -> usize {
        self.captures.iter().filter(|c| c.admitted).count()
    }

    /// Captures the gate did not admit.
    pub fn discarded(&self) -> usize {
        self.captures.len() - self.admitted()
    }
}

/// Rule on whether the set measured a display period. See [`RefreshCap`] for
/// why only a SET can.
///
/// A suspect is a capture that clustered at [`REFRESH_CAP_SHARE`] or above on
/// a period at least [`REFRESH_CAP_MIN_MS`] long - the same two bars the
/// capture's own suspicion line applies, so the two halves cannot convict on
/// evidence the other discarded.
///
/// Agreement is "every suspect within [`REFRESH_CAP_AGREEMENT`] of the
/// suspects' median", the same reference-and-band shape the repeat gate itself
/// uses. One disagreeing member is enough to make the verdict
/// [`RefreshCap::Workload`], and that asymmetry is deliberate: the failure this
/// replaced was biased toward refusing, so the residual error is pointed at
/// admitting.
fn refresh_cap(captures: &[RepeatCapture]) -> RefreshCap {
    let measured: Vec<(f64, f64)> = captures
        .iter()
        .filter_map(|c| c.cluster_median_ms.zip(c.cluster_share))
        .collect();
    if measured.is_empty() {
        return RefreshCap::Unmeasured;
    }
    let suspects: Vec<f64> = measured
        .iter()
        .filter(|(period, share)| *share >= REFRESH_CAP_SHARE && *period >= REFRESH_CAP_MIN_MS)
        .map(|(period, _)| *period)
        .collect();
    match suspects.len() {
        0 => RefreshCap::NotSuspected,
        1 => RefreshCap::Unverifiable,
        _ => {
            let Some(reference) = median(&suspects) else {
                return RefreshCap::Unverifiable;
            };
            if suspects
                .iter()
                .all(|period| within(*period, reference, REFRESH_CAP_AGREEMENT))
            {
                RefreshCap::Capped
            } else {
                RefreshCap::Workload
            }
        }
    }
}

/// The median of a slice, by the same nearest-rank rule the frame percentiles
/// use (a real observed value, never an interpolation). `None` when empty.
fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[(sorted.len() - 1) / 2])
}

/// Whether `value` sits inside `tolerance` of `reference`. A non-positive
/// reference is not a measurement, so nothing is inside it.
fn within(value: f64, reference: f64, tolerance: f64) -> bool {
    reference > 0.0 && ((value - reference) / reference).abs() <= tolerance
}

/// The pair a repeat set reports for one tail statistic: the MEDIAN across the
/// admitted captures, and how far those captures spread as a fraction of it.
/// `(None, None)` when the gate admitted nothing, or when the median is not a
/// positive number to take a fraction of.
fn summarize(
    captures: &[RepeatCapture],
    pick: fn(&RepeatCapture) -> f64,
) -> (Option<f64>, Option<f64>) {
    let admitted: Vec<f64> = captures.iter().filter(|c| c.admitted).map(pick).collect();
    let value = median(&admitted);
    let spread = value.filter(|v| *v > 0.0).map(|v| {
        let lo = admitted.iter().copied().fold(f64::MAX, f64::min);
        let hi = admitted.iter().copied().fold(f64::MIN, f64::max);
        (hi - lo) / v
    });
    (value, spread)
}

/// Read every repeat set in `runs`. Fewer than two captures is not a set and
/// produces no row - `read_frames` already covers a single capture, and a
/// one-member set would admit itself by construction and report a tail with no
/// spread, which is the false confidence this whole mechanism replaces.
pub fn read_repeats(runs: &[PerfRun]) -> Vec<RepeatRead> {
    let mut order: Vec<&str> = Vec::new();
    let mut groups: std::collections::HashMap<&str, Vec<(u32, &PerfRun)>> =
        std::collections::HashMap::new();
    for run in runs {
        let (base, Some(index)) = split_repeat_label(&run.label) else {
            continue;
        };
        if !groups.contains_key(base) {
            order.push(base);
        }
        groups.entry(base).or_default().push((index, run));
    }

    order
        .into_iter()
        .filter_map(|base| {
            let mut group = groups.remove(base)?;
            if group.len() < 2 {
                return None;
            }
            group.sort_by_key(|(index, _)| *index);
            let means: Vec<f64> = group.iter().map(|(_, run)| run.stats.mean_ms).collect();
            let medians: Vec<f64> = group.iter().map(|(_, run)| run.stats.p50_ms).collect();
            let reference_mean_ms = median(&means)?;
            let reference_median_ms = median(&medians)?;
            let mut captures: Vec<RepeatCapture> = group
                .iter()
                .map(|(index, run)| RepeatCapture {
                    index: *index,
                    mean_ms: run.stats.mean_ms,
                    median_ms: run.stats.p50_ms,
                    p99_ms: run.stats.p99_ms,
                    worst_ms: run.stats.max_ms,
                    cluster_median_ms: run.stats.cluster_median_ms,
                    cluster_share: run.stats.cluster_share,
                    admitted: within(run.stats.mean_ms, reference_mean_ms, REPEAT_GATE_TOLERANCE)
                        && within(run.stats.p50_ms, reference_median_ms, REPEAT_GATE_TOLERANCE),
                })
                .collect();
            // The whole set, not the suspects alone: once the display is
            // pacing the swap chain, no window of the set measured the game,
            // including the ones that happened to spread.
            let refresh_cap = refresh_cap(&captures);
            if refresh_cap == RefreshCap::Capped {
                for capture in &mut captures {
                    capture.admitted = false;
                }
            }
            let (p99_ms, p99_spread) = summarize(&captures, |c| c.p99_ms);
            let (worst_ms, worst_spread) = summarize(&captures, |c| c.worst_ms);
            Some(RepeatRead {
                label: base.to_string(),
                reference_mean_ms,
                reference_median_ms,
                captures,
                p99_ms,
                p99_spread,
                worst_ms,
                worst_spread,
                refresh_cap,
            })
        })
        .collect()
}

/// The `checks.json` mirror of the repeat gate. Beside the checks, never among
/// them, for the same reason [`frames_json`] is: a discarded capture is not a
/// failure, and `graded: false` says so to a consumer that only reads keys.
pub fn repeats_json(runs: Option<&Vec<PerfRun>>) -> serde_json::Value {
    let reads = runs.map(|runs| read_repeats(runs)).unwrap_or_default();
    serde_json::json!({
        "graded": false,
        "tolerance": REPEAT_GATE_TOLERANCE,
        "cluster_share_threshold": REFRESH_CAP_SHARE,
        "cluster_band": REFRESH_CAP_BAND,
        "cluster_agreement": REFRESH_CAP_AGREEMENT,
        "note": "the mean and median gate which captures COUNT; the tail is read \
                 only across the admitted ones. Make a claim on p99_ms and check it \
                 against p99_spread; worst_ms is a diagnostic beside it. \
                 refresh_cap is the set's call on whether it measured the DISPLAY \
                 rather than the game: a period is a constant, so it takes agreeing \
                 cluster medians across captures, never one clustered window. \
                 Nothing here passes or fails.",
        "sets": reads.iter().map(|read| serde_json::json!({
            "label": read.label,
            "captures": read.captures.len(),
            "admitted": read.admitted(),
            "discarded": read.discarded(),
            "reference_mean_ms": read.reference_mean_ms,
            "reference_median_ms": read.reference_median_ms,
            "p99_ms": read.p99_ms,
            "p99_spread": read.p99_spread,
            "worst_ms": read.worst_ms,
            "worst_spread": read.worst_spread,
            "refresh_cap": read.refresh_cap.token(),
            "repeats": read.captures.iter().map(|capture| serde_json::json!({
                "index": capture.index,
                "mean_ms": capture.mean_ms,
                "median_ms": capture.median_ms,
                "p99_ms": capture.p99_ms,
                "worst_ms": capture.worst_ms,
                "cluster_median_ms": capture.cluster_median_ms,
                "cluster_share": capture.cluster_share,
                "admitted": capture.admitted,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a run with the two numbers the read is made of; everything else
    /// is a plausible constant.
    fn run(label: &str, mean_ms: f64, max_ms: f64) -> PerfRun {
        PerfRun {
            label: label.to_string(),
            stats: FrameStats {
                frames: 900,
                total_ms: mean_ms * 900.0,
                mean_ms,
                min_ms: mean_ms * 0.5,
                max_ms,
                p50_ms: mean_ms,
                p95_ms: mean_ms * 1.5,
                p99_ms: mean_ms * 2.0,
                p999_ms: max_ms,
                mean_fps: fps_of(mean_ms),
                one_pct_low_fps: fps_of(mean_ms * 2.0),
                cluster_median_ms: None,
                cluster_share: None,
            },
            meta: RunMeta::unknown(),
        }
    }

    #[test]
    fn a_smooth_capture_reads_good_and_a_slow_one_reads_bad() {
        let reads = read_frames(&[run("smooth", 8.0, 12.0), run("slow", 40.0, 200.0)]);
        // Worst frame first: the row a reader has to look at leads.
        assert_eq!(reads[0].label, "slow");
        assert!(reads[0].mean_is_slow());
        assert_eq!(reads[0].verdict(), "FPS IS UNDER 60 - BAD");
        assert_eq!(reads[0].class(), "bad");
        assert!((reads[0].mean_fps - 25.0).abs() < 1e-9);
        assert!((reads[0].worst_fps - 5.0).abs() < 1e-9);

        assert_eq!(reads[1].label, "smooth");
        assert!(!reads[1].mean_is_slow());
        assert!(!reads[1].worst_is_slow());
        assert_eq!(reads[1].class(), "good");
    }

    /// The common case: the mean holds the line and one frame did not. It is
    /// its own reading, because calling it BAD would make the flag useless.
    #[test]
    fn a_smooth_mean_with_a_slow_tail_is_its_own_reading() {
        let reads = read_frames(&[run("tail", 10.0, 50.0)]);
        assert!(!reads[0].mean_is_slow());
        assert!(reads[0].worst_is_slow());
        assert_eq!(reads[0].class(), "mixed");
    }

    /// A frame time of zero is not a 60-thousand-fps run.
    #[test]
    fn an_unmeasured_frame_time_reads_zero_fps_not_infinity() {
        let reads = read_frames(&[run("empty", 0.0, 0.0)]);
        assert_eq!(reads[0].mean_fps, 0.0);
        assert_eq!(reads[0].worst_fps, 0.0);
        assert!(reads[0].mean_is_slow());
    }

    /// The json mirror carries the numbers AND says it is not a judgement, so
    /// an agent cannot fold it into a verdict by accident.
    #[test]
    fn the_json_mirror_says_it_is_not_graded() {
        let json = frames_json(Some(&vec![run("one", 20.0, 33.4)]));
        assert_eq!(json["graded"], false);
        assert_eq!(json["smooth_fps"], 60.0);
        assert_eq!(json["captures"][0]["label"], "one");
        assert_eq!(json["captures"][0]["mean_under_60_fps"], true);
        assert_eq!(json["captures"][0]["read"], "FPS IS UNDER 60 - BAD");
    }

    /// A run with no capture at all is an empty list, never a missing key: a
    /// consumer indexing `frames.captures` must not have to guard.
    #[test]
    fn a_run_without_a_capture_reads_an_empty_list() {
        let json = frames_json(None);
        assert_eq!(json["captures"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn a_repeat_label_round_trips() {
        assert_eq!(repeat_label("wfc_arena", 3), "wfc_arena#3");
        assert_eq!(split_repeat_label("wfc_arena#3"), ("wfc_arena", Some(3)));
        // A plain capture is not a one-member repeat set.
        assert_eq!(split_repeat_label("wfc_arena"), ("wfc_arena", None));
        // Nor is a label that merely contains the separator.
        assert_eq!(split_repeat_label("odd#label"), ("odd#label", None));
    }

    /// The whole design in one test: a contaminated capture is thrown out on
    /// its MEAN, and the tail is then read only across what survived.
    #[test]
    fn the_gate_discards_on_the_mean_and_reads_the_tail_on_what_survives() {
        // Four captures of an unchanged scene. #3 met a busy machine: its mean
        // is 30% high, and so are the tails that would otherwise have owned
        // the set.
        let runs = vec![
            repeat("s", 1, 26.0, 25.0, 40.0, 44.0),
            repeat("s", 2, 26.2, 25.1, 52.0, 58.0),
            repeat("s", 3, 34.0, 33.0, 150.0, 190.0),
            repeat("s", 4, 25.8, 24.9, 46.0, 50.0),
        ];
        let reads = read_repeats(&runs);
        assert_eq!(reads.len(), 1);
        let read = &reads[0];
        assert_eq!(read.label, "s");
        assert_eq!(read.admitted(), 3);
        assert_eq!(read.discarded(), 1);
        assert!(!read.captures[2].admitted, "the busy capture is out");
        // The number a claim is made on: the median of the three admitted p99
        // values (40, 46, 52), not the 150 ms the discarded one carried.
        assert_eq!(read.p99_ms, Some(46.0));
        assert!((read.p99_spread.unwrap() - 12.0 / 46.0).abs() < 1e-9);
        // The worst frame rides beside it, same treatment: median of 44, 50, 58.
        assert_eq!(read.worst_ms, Some(50.0));
        assert!((read.worst_spread.unwrap() - 0.28).abs() < 1e-9);
    }

    /// The reference is the MEDIAN of the means, so one bad capture cannot
    /// drag the band over itself and start discarding the good ones.
    #[test]
    fn one_bad_capture_cannot_move_the_reference_onto_itself() {
        let runs = vec![
            repeat("s", 1, 26.0, 26.0, 40.0, 44.0),
            repeat("s", 2, 26.0, 26.0, 42.0, 46.0),
            repeat("s", 3, 100.0, 100.0, 300.0, 400.0),
        ];
        let read = &read_repeats(&runs)[0];
        assert!((read.reference_mean_ms - 26.0).abs() < 1e-9);
        assert_eq!(read.admitted(), 2);
        assert_eq!(read.p99_ms, Some(40.0), "nearest-rank median of 40, 42");
        assert_eq!(read.worst_ms, Some(44.0), "nearest-rank median of 44, 46");
    }

    /// The band is wide on purpose (see [`REPEAT_GATE_TOLERANCE`]): an honest
    /// capture that lands 10% off the reference still counts, and one that
    /// measured a busy machine does not.
    #[test]
    fn the_band_admits_an_honest_spread_and_rejects_a_busy_machine() {
        let runs = vec![
            repeat("s", 1, 30.0, 30.0, 60.0, 90.0),
            repeat("s", 2, 33.0, 33.0, 66.0, 99.0),
            repeat("s", 3, 27.0, 27.0, 54.0, 81.0),
            repeat("s", 4, 117.0, 117.0, 250.0, 400.0),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.admitted(), 3, "+/-10% is inside the band");
        assert!(!read.captures[3].admitted, "3.9x is not");
    }

    /// A single capture is not a repeat set, and the gate must not pretend it
    /// is - a one-member set would admit itself by construction and report a
    /// tail with no spread, which is the false confidence this replaces.
    #[test]
    fn a_plain_capture_is_not_a_repeat_set() {
        assert!(read_repeats(&[run("wfc_arena", 26.0, 44.0)]).is_empty());
        // Nor is a set of exactly one: it would admit itself by construction.
        assert!(read_repeats(&[repeat("wfc_arena", 1, 26.0, 26.0, 40.0, 44.0)]).is_empty());
        assert_eq!(repeats_json(None)["sets"].as_array().unwrap().len(), 0);
    }

    /// A set the gate empties still renders: "nothing here was measurable" is
    /// the finding, and a missing row would read as a missing run.
    #[test]
    fn a_set_the_gate_empties_reports_no_tail() {
        let runs = vec![
            repeat("z", 1, 0.0, 0.0, 0.0, 0.0),
            repeat("z", 2, 0.0, 0.0, 0.0, 0.0),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.admitted(), 0);
        assert_eq!(read.p99_ms, None);
        assert_eq!(read.worst_ms, None);
        assert_eq!(read.worst_spread, None);
    }

    /// The fixed-step mirror carries the stopped-frame count - the reading
    /// that says a window did not measure one scene - and says it grades
    /// nothing.
    #[test]
    fn the_fixed_step_json_carries_the_stopped_frames() {
        let log = "nova perf: label=wfc_arena#5 fixed_steps min=0 max=16 mean=5.419 \
                   total=4877 buckets=0:165@69.7ms,4:187@67.2ms,16:34@354.1ms"
            .to_string();
        let json = fixed_steps_json(Some(&log));
        assert_eq!(json["graded"], false);
        assert_eq!(json["captures"][0]["label"], "wfc_arena#5");
        assert_eq!(json["captures"][0]["stopped_frames"], 165);
        assert_eq!(json["captures"][0]["frames_at_max"], 34);
        assert_eq!(json["captures"][0]["max_steps"], 16);
        // No log is an empty list, never a missing key.
        assert_eq!(
            fixed_steps_json(None)["captures"].as_array().unwrap().len(),
            0
        );
    }

    #[test]
    fn the_repeat_json_says_it_is_not_graded() {
        let runs = vec![
            repeat("s", 1, 26.0, 26.0, 40.0, 44.0),
            repeat("s", 2, 26.0, 26.0, 42.0, 46.0),
        ];
        let json = repeats_json(Some(&runs));
        assert_eq!(json["graded"], false);
        assert_eq!(json["tolerance"], REPEAT_GATE_TOLERANCE);
        assert_eq!(json["sets"][0]["label"], "s");
        assert_eq!(json["sets"][0]["admitted"], 2);
        assert_eq!(json["sets"][0]["p99_ms"], 40.0);
        assert_eq!(json["sets"][0]["repeats"][1]["index"], 2);
        assert_eq!(json["sets"][0]["repeats"][1]["p99_ms"], 42.0);
    }

    /// One capture of a repeat set, with the four numbers the gate reads
    /// chosen independently - the plain `run` helper ties them together.
    fn repeat(
        base: &str,
        index: u32,
        mean_ms: f64,
        median_ms: f64,
        p99_ms: f64,
        max_ms: f64,
    ) -> PerfRun {
        let mut run = run(&repeat_label(base, index), mean_ms, max_ms);
        run.stats.p99_ms = p99_ms;
        run.stats.p50_ms = median_ms;
        run
    }

    /// A capture of a repeat set that also carries the cluster shape the
    /// capture measured, which is the only input the discriminator reads.
    fn clustered(base: &str, index: u32, mean_ms: f64, period_ms: f64, share: f64) -> PerfRun {
        let mut run = repeat(base, index, mean_ms, mean_ms, mean_ms * 1.2, mean_ms * 1.5);
        run.stats.cluster_median_ms = Some(period_ms);
        run.stats.cluster_share = Some(share);
        run
    }

    /// The real thing: every capture agrees on one period, which is what a
    /// display does and a workload does not. The set measured the monitor, so
    /// nothing in it counts and no tail survives to be compared.
    #[test]
    fn captures_agreeing_on_one_period_are_a_refresh_cap_and_the_set_reports_no_tail() {
        // 165 Hz: 6.06 ms, reproduced to the precision a real cap holds.
        let runs = vec![
            clustered("s", 1, 6.06, 6.061, 0.79),
            clustered("s", 2, 6.06, 6.058, 0.76),
            clustered("s", 3, 6.06, 6.063, 0.81),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.refresh_cap, RefreshCap::Capped);
        assert_eq!(read.admitted(), 0);
        assert_eq!(read.discarded(), 3);
        assert_eq!(read.p99_ms, None);
    }

    /// The regression this whole mechanism exists for. The damage-cracks A/B
    /// produced windows at 0.64-0.65 clustering whose cluster medians read
    /// 20.727 / 21.130 / 22.713 ms - 48.3 / 47.3 / 44.0 Hz, which is no
    /// display, and a 9.6% drift, which no refresh period does. The in-app
    /// check refused all six windows of BOTH arms and would have reported "no
    /// change" for a fix worth -8.6% mean.
    #[test]
    fn captures_whose_cluster_medians_disagree_are_a_steady_workload_and_are_admitted() {
        let runs = vec![
            clustered("s", 1, 20.7, 20.727, 0.64),
            clustered("s", 2, 21.1, 21.130, 0.65),
            clustered("s", 3, 22.7, 22.713, 0.64),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.refresh_cap, RefreshCap::Workload);
        assert_eq!(read.admitted(), 3, "a steady scene is still a measurement");
        assert!(read.p99_ms.is_some());
    }

    /// The two bands are not interchangeable, and reusing the cluster band for
    /// agreement is the way this discriminator would fail quietly. These three
    /// medians all sit inside 5% of theirs - a cluster band would convict - and
    /// spread 2.4%, which is 30x the drift a real 165 Hz display shows.
    #[test]
    fn a_period_drifting_more_than_a_clock_can_is_a_workload_however_tight_it_looks() {
        let runs = vec![
            clustered("s", 1, 20.0, 20.000, 0.72),
            clustered("s", 2, 20.5, 20.500, 0.70),
            clustered("s", 3, 20.7, 20.700, 0.71),
        ];
        let read = &read_repeats(&runs)[0];
        assert!(
            (20.000_f64 - 20.500).abs() / 20.500 < REFRESH_CAP_BAND,
            "the cluster band alone would have called this one display",
        );
        assert_eq!(read.refresh_cap, RefreshCap::Workload);
        assert_eq!(read.admitted(), 3);
    }

    /// One suspect and no sibling to check its period against. There is no
    /// discriminator, so the honest reading is UNMEASURED: the set is not
    /// refused (that is the bias this replaced) and not waved through either.
    #[test]
    fn a_lone_suspect_has_no_sibling_to_check_its_period_against() {
        let runs = vec![
            clustered("s", 1, 20.7, 20.727, 0.64),
            clustered("s", 2, 20.9, 20.900, 0.31),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.refresh_cap, RefreshCap::Unverifiable);
        assert_eq!(read.admitted(), 2, "insufficient evidence refuses nothing");
        assert_eq!(
            repeats_json(Some(&runs))["sets"][0]["refresh_cap"],
            "unverifiable"
        );
    }

    /// Shapes measured, none clustered: an ordinary workload spread, and the
    /// discriminator has nothing to say about it.
    #[test]
    fn a_set_that_never_clustered_is_not_even_a_suspect() {
        let runs = vec![
            clustered("s", 1, 20.7, 20.7, 0.11),
            clustered("s", 2, 20.9, 20.9, 0.09),
        ];
        assert_eq!(read_repeats(&runs)[0].refresh_cap, RefreshCap::NotSuspected);
    }

    /// A cheap scene runs steady and agrees with itself run to run, which is
    /// every condition a cap meets except the one that matters: no display
    /// refreshes at 500 Hz. The floor is the whole reason this is not a cap.
    #[test]
    fn a_scene_too_fast_for_any_display_is_not_a_suspect_however_steady_it_is() {
        let runs = vec![
            clustered("s", 1, 2.0, 2.001, 0.88),
            clustered("s", 2, 2.0, 2.003, 0.91),
            clustered("s", 3, 2.0, 1.998, 0.87),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.refresh_cap, RefreshCap::NotSuspected);
        assert_eq!(read.admitted(), 3);
    }

    /// A pre-v4 row, or a run that asked for a mode allowed to block on
    /// refresh, carries no shape. That is UNMEASURED and must not read as
    /// "not clustered" - the same distinction SKIPPED draws in the checks.
    #[test]
    fn a_set_carrying_no_cluster_shape_reads_unmeasured_not_clean() {
        let runs = vec![
            repeat("s", 1, 26.0, 26.0, 40.0, 44.0),
            repeat("s", 2, 26.2, 26.1, 42.0, 46.0),
        ];
        let read = &read_repeats(&runs)[0];
        assert_eq!(read.refresh_cap, RefreshCap::Unmeasured);
        assert_eq!(read.admitted(), 2);
    }

    /// `checks.json` carries the verdict and the evidence under it, so a
    /// reader can re-derive the call instead of trusting the token.
    #[test]
    fn the_json_mirror_carries_the_verdict_and_the_shapes_it_was_made_from() {
        let runs = vec![
            clustered("s", 1, 6.06, 6.061, 0.79),
            clustered("s", 2, 6.06, 6.058, 0.76),
        ];
        let json = repeats_json(Some(&runs));
        assert_eq!(json["graded"], false);
        assert_eq!(json["cluster_share_threshold"], REFRESH_CAP_SHARE);
        assert_eq!(json["cluster_band"], REFRESH_CAP_BAND);
        assert_eq!(json["sets"][0]["refresh_cap"], "refresh_capped");
        assert_eq!(json["sets"][0]["admitted"], 0);
        assert_eq!(json["sets"][0]["repeats"][0]["cluster_share"], 0.79);
        assert_eq!(json["sets"][0]["repeats"][0]["cluster_median_ms"], 6.061);
    }
}
