//! Frame-time statistics and the run schema: [`FrameStats`], the per-run
//! metadata ([`RunMeta`]), and the CSV/JSON writers + parsers both the capture
//! harness and the `perf_report` bin share, so the schema is defined once.
//!
//! Two CSV schema versions exist:
//!
//! - **v1** ([`CSV_HEADER_V1`]): the numeric columns only. The v0.7.0 baseline
//!   sweeps (`tasks/20260716-123551/perf-results/`) are v1 and must keep
//!   parsing - the reader accepts it and fills [`RunMeta::unknown`].
//! - **v2** ([`CSV_HEADER`]): v1 plus the run-metadata columns
//!   (backend, adapter, resolution, quality, git_sha, host), so a results
//!   file is self-describing instead of leaning on its directory name.

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
        }
    }

    /// True when every field is still `"unknown"` (i.e. v1 data).
    pub fn is_unknown(&self) -> bool {
        self == &Self::unknown()
    }

    /// The metadata columns in [`CSV_HEADER`] order, comma-sanitized.
    pub(crate) fn csv_cells(&self) -> [String; 6] {
        [
            csv_safe(&self.backend),
            csv_safe(&self.adapter),
            csv_safe(&self.resolution),
            csv_safe(&self.quality),
            csv_safe(&self.git_sha),
            csv_safe(&self.host),
        ]
    }
}

/// Make a metadata value safe as a bare CSV cell: commas and line breaks
/// become spaces (adapter names are free-form vendor strings).
pub(crate) fn csv_safe(value: &str) -> String {
    value.replace([',', '\n', '\r'], " ").trim().to_string()
}

/// One captured run: its label, percentile stats, and run metadata. The unit
/// the aggregated `frametime.csv` stores one per row and the `perf_report`
/// bin renders one per table row.
#[derive(Debug, Clone, PartialEq)]
pub struct PerfRun {
    /// The run's label (e.g. `broadside-high`), as written by the capture.
    pub label: String,
    /// The percentile frame-time statistics for the run.
    pub stats: FrameStats,
    /// The run metadata ([`RunMeta::unknown`] for v1 data).
    pub meta: RunMeta,
}

/// Header row for the aggregated CSV, schema v2 (numeric columns + run
/// metadata), written when a new file is created. Public so a reader can
/// validate a file against the exact column contract the writer emits.
pub const CSV_HEADER: &str = "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,\
     mean_fps,one_pct_low_fps,backend,adapter,resolution,quality,git_sha,host\n";

/// The pre-metadata schema v1 header. Still accepted by the parser so the
/// v0.7.0 baseline results keep loading (their rows parse with
/// [`RunMeta::unknown`]).
pub const CSV_HEADER_V1: &str =
    "label,frames,mean_ms,min_ms,max_ms,p50_ms,p95_ms,p99_ms,p999_ms,mean_fps,one_pct_low_fps\n";

/// Column counts for the two schema versions (label + numerics [+ meta]).
const V1_COLS: usize = 11;
const V2_COLS: usize = 17;

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
        }
    }

    /// A compact, greppable one-line summary. The `nova perf:` prefix is a
    /// scrape contract (`scripts/perf-web.sh` greps it out of the browser
    /// console log) - do not rename it without updating the scrapers.
    pub(crate) fn summary_line(&self, label: &str) -> String {
        format!(
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
        )
    }

    /// Render as a pretty JSON object (hand-formatted to avoid a serde dep in
    /// this dev-only crate). Schema v2: the metadata fields follow the
    /// numeric ones.
    pub(crate) fn to_json(&self, label: &str, meta: &RunMeta) -> String {
        format!(
            "{{\n  \"label\": \"{}\",\n  \"frames\": {},\n  \"total_ms\": {:.3},\n  \
             \"mean_ms\": {:.4},\n  \"min_ms\": {:.4},\n  \"max_ms\": {:.4},\n  \
             \"p50_ms\": {:.4},\n  \"p95_ms\": {:.4},\n  \"p99_ms\": {:.4},\n  \
             \"p999_ms\": {:.4},\n  \"mean_fps\": {:.2},\n  \"one_pct_low_fps\": {:.2},\n  \
             \"backend\": \"{}\",\n  \"adapter\": \"{}\",\n  \"resolution\": \"{}\",\n  \
             \"quality\": \"{}\",\n  \"git_sha\": \"{}\",\n  \"host\": \"{}\"\n}}\n",
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
        )
    }

    /// One CSV data row (no header), schema v2: matches [`CSV_HEADER`].
    pub(crate) fn to_csv_row(&self, label: &str, meta: &RunMeta) -> String {
        let cells = meta.csv_cells();
        format!(
            "{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.2},{:.2},{},{},{},{},{},{}\n",
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
    /// [`FrameStats::to_csv_row`]. Accepts a v1 row (11 columns; metadata
    /// becomes [`RunMeta::unknown`]) or a v2 row (17 columns). The CSV omits
    /// `total_ms` (JSON-only), so it is reconstructed exactly as
    /// `mean_ms * frames` (mean is defined as `total / frames`). Returns
    /// `None` on any other column count or a numeric field that does not
    /// parse, so a truncated or foreign file is rejected rather than
    /// silently mis-read.
    pub fn from_csv_row(row: &str) -> Option<Self> {
        let cols: Vec<&str> = row.split(',').collect();
        if cols.len() != V1_COLS && cols.len() != V2_COLS {
            return None;
        }
        let label = cols[0].to_string();
        let frames: usize = cols[1].trim().parse().ok()?;
        let mean_ms: f64 = cols[2].trim().parse().ok()?;
        let min_ms: f64 = cols[3].trim().parse().ok()?;
        let max_ms: f64 = cols[4].trim().parse().ok()?;
        let p50_ms: f64 = cols[5].trim().parse().ok()?;
        let p95_ms: f64 = cols[6].trim().parse().ok()?;
        let p99_ms: f64 = cols[7].trim().parse().ok()?;
        let p999_ms: f64 = cols[8].trim().parse().ok()?;
        let mean_fps: f64 = cols[9].trim().parse().ok()?;
        let one_pct_low_fps: f64 = cols[10].trim().parse().ok()?;
        let meta = if cols.len() == V2_COLS {
            RunMeta {
                backend: cols[11].trim().to_string(),
                adapter: cols[12].trim().to_string(),
                resolution: cols[13].trim().to_string(),
                quality: cols[14].trim().to_string(),
                git_sha: cols[15].trim().to_string(),
                host: cols[16].trim().to_string(),
            }
        } else {
            RunMeta::unknown()
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
            },
            meta,
        })
    }
}

/// Parse a whole aggregated `frametime.csv` (header + one row per run) into a
/// list of runs, preserving file order. The first line must match
/// [`CSV_HEADER`] (v2) or [`CSV_HEADER_V1`] (trimmed) or the file is rejected
/// as not-a-frametime-CSV; every data row must then carry that version's
/// column count. Blank lines are skipped and any row that fails to parse is
/// an error naming its line, so a corrupt sweep is caught instead of silently
/// dropping runs. Shared by the `perf_report` bin and its tests so the schema
/// lives in one place.
pub fn parse_frametime_csv(contents: &str) -> Result<Vec<PerfRun>, String> {
    let mut lines = contents.lines();
    let header = lines.next().ok_or("empty CSV (no header)")?;
    let expected_cols = if header.trim() == CSV_HEADER.trim() {
        V2_COLS
    } else if header.trim() == CSV_HEADER_V1.trim() {
        V1_COLS
    } else {
        return Err(format!(
            "unexpected CSV header\n  expected: {}\n  or (v1):  {}\n  found:    {}",
            CSV_HEADER.trim(),
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
        }
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
    fn v2_row_write_then_read_round_trips_stats_and_meta() {
        // Forward (to_csv_row) then back (from_csv_row) preserves every field
        // the CSV carries, metadata included. total_ms is CSV-omitted, so
        // compare the rest.
        let original = FrameStats::from_samples(&[8.0, 12.0, 10.0, 40.0, 9.5, 11.0, 10.5]);
        let meta = some_meta();
        let row = original.to_csv_row("shakedown_run-low", &meta);
        let run = PerfRun::from_csv_row(row.trim()).expect("v2 round-trips");
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
        assert_eq!(row.trim().split(',').count(), 17, "row: {row}");
        let run = PerfRun::from_csv_row(row.trim()).expect("sanitized row parses");
        assert_eq!(run.meta.adapter, "Intel  Inc. UHD Graphics   770");
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
    fn parse_frametime_csv_reads_a_v2_file_with_meta() {
        let stats = FrameStats::from_samples(&[10.0, 12.0, 11.0]);
        let csv = format!(
            "{}{}",
            CSV_HEADER,
            stats.to_csv_row("scene-high", &some_meta())
        );
        let runs = parse_frametime_csv(&csv).expect("v2 file parses");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].meta, some_meta());
    }

    #[test]
    fn parse_frametime_csv_rejects_a_row_width_mismatch() {
        // A v2 header promises 17 columns; an 11-column (v1-shaped) row under
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
    fn json_carries_the_meta_fields() {
        let stats = FrameStats::from_samples(&[10.0; 3]);
        let json = stats.to_json("scene", &some_meta());
        assert!(json.contains("\"backend\": \"vulkan\""), "{json}");
        assert!(json.contains("\"git_sha\": \"f4bfb3af\""), "{json}");
        assert!(json.contains("\"adapter\": \"NVIDIA GeForce RTX 3060 Ti\""));
    }
}
