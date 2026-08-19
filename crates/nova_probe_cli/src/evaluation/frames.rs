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
    pub use super::{frames_json, read_frames, FrameRead, SMOOTH_FPS, SMOOTH_FRAME_MS};
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
}
