//! `perf_report` - turn a `perf-baseline.sh` results directory into one
//! self-contained HTML report (inline CSS + inline SVG, no external assets, so
//! it opens offline and attaches to a task or PR).
//!
//! Input is a results dir holding the aggregated `frametime.csv` that the
//! capture harness writes (one row per `<scene>-<preset>` run; see
//! [`nova_perf::CSV_HEADER`]). Output is a single `.html` file with, per run:
//! frame count and capture window, mean and p50/p95/p99/p999/max frame times,
//! the derived mean / 1%-low FPS, a horizontal bar chart (mean bar + a p99
//! marker, all runs on one common scale), and - when a `--baseline` dir is
//! given - the percentage delta of every run against the baseline row of the
//! same label, so a regression is obvious at a glance.
//!
//! ```text
//! cargo run -p nova_perf --bin perf_report -- <results-dir> \
//!   [--baseline <baseline-dir>] [-o <output.html>]
//! ```
//!
//! Defaults: the report is written to `<results-dir>/report.html`. The renderer
//! label shown in the header is the results dir's own name (the sweep script
//! names its out dirs `gpu` / `sw` / `xgpu` / `web`), since the per-frame JSON
//! schema does not record the renderer itself.
//!
//! This is pure reporting over the capture harness's existing output - it never
//! touches the capture path.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::ExitCode,
};

use nova_perf::{parse_frametime_csv, PerfRun};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match Options::parse(&args) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("perf_report: {message}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let runs = match read_runs(&opts.results_dir) {
        Ok(runs) => runs,
        Err(message) => {
            eprintln!("perf_report: {message}");
            return ExitCode::FAILURE;
        }
    };
    if runs.is_empty() {
        eprintln!(
            "perf_report: no runs found in {} (frametime.csv had only a header)",
            opts.results_dir.display()
        );
        return ExitCode::FAILURE;
    }

    let baseline = match &opts.baseline_dir {
        None => None,
        Some(dir) => match read_runs(dir) {
            Ok(base) => Some((dir.clone(), base)),
            Err(message) => {
                eprintln!("perf_report: baseline: {message}");
                return ExitCode::FAILURE;
            }
        },
    };

    let output = opts
        .output
        .clone()
        .unwrap_or_else(|| opts.results_dir.join("report.html"));

    let html = render_report(&opts.results_dir, &runs, baseline.as_ref());
    if let Err(error) = std::fs::write(&output, &html) {
        eprintln!("perf_report: could not write {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    println!(
        "perf_report: wrote {} ({} runs, {} bytes)",
        output.display(),
        runs.len(),
        html.len()
    );
    ExitCode::SUCCESS
}

const USAGE: &str = "usage: perf_report <results-dir> [--baseline <dir>] [-o <output.html>]";

/// Parsed command line.
struct Options {
    results_dir: PathBuf,
    baseline_dir: Option<PathBuf>,
    output: Option<PathBuf>,
}

impl Options {
    /// Parse `<results-dir> [--baseline <dir>] [-o|--output <file>]`. Exactly
    /// one positional (the results dir) is required.
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut results_dir: Option<PathBuf> = None;
        let mut baseline_dir: Option<PathBuf> = None;
        let mut output: Option<PathBuf> = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => return Err("help".to_string()),
                "--baseline" => {
                    let value = iter.next().ok_or("--baseline needs a directory")?;
                    baseline_dir = Some(PathBuf::from(value));
                }
                "-o" | "--output" => {
                    let value = iter.next().ok_or("-o needs a file path")?;
                    output = Some(PathBuf::from(value));
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown flag {other}"));
                }
                other => {
                    if results_dir.replace(PathBuf::from(other)).is_some() {
                        return Err("only one results dir may be given".to_string());
                    }
                }
            }
        }
        Ok(Self {
            results_dir: results_dir.ok_or("a results dir is required")?,
            baseline_dir,
            output,
        })
    }
}

/// Read and parse `<dir>/frametime.csv` into runs, mapping every failure to a
/// message that names the file.
fn read_runs(dir: &Path) -> Result<Vec<PerfRun>, String> {
    let csv_path = dir.join("frametime.csv");
    let contents = std::fs::read_to_string(&csv_path)
        .map_err(|error| format!("could not read {}: {error}", csv_path.display()))?;
    parse_frametime_csv(&contents).map_err(|error| format!("{}: {error}", csv_path.display()))
}

// ---- rendering ------------------------------------------------------------

/// The renderer shown in the header: the results dir's own name, since the
/// per-run schema does not record the renderer. Falls back to the full path
/// when the dir has no file name (e.g. `.`).
fn renderer_label(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty() && name != ".")
        .unwrap_or_else(|| dir.display().to_string())
}

/// Split a run label into `(scene, preset)`. The sweep names runs
/// `<scene>-<preset>` where preset is one of the graphics tiers; when the
/// suffix is not a known tier the whole label is the scene and the preset is
/// blank (so custom labels still render).
fn split_label(label: &str) -> (String, String) {
    if let Some((scene, preset)) = label.rsplit_once('-') {
        if matches!(preset, "high" | "medium" | "low") {
            return (scene.to_string(), preset.to_string());
        }
    }
    (label.to_string(), String::new())
}

/// Escape the five characters that matter for HTML text/attribute content.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Render the whole self-contained HTML document.
fn render_report(
    results_dir: &Path,
    runs: &[PerfRun],
    baseline: Option<&(PathBuf, Vec<PerfRun>)>,
) -> String {
    let renderer = renderer_label(results_dir);
    let baseline_map: HashMap<&str, &PerfRun> = baseline
        .map(|(_, base)| base.iter().map(|run| (run.label.as_str(), run)).collect())
        .unwrap_or_default();

    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!(
        "<title>nova perf report - {}</title>\n",
        escape(&renderer)
    ));
    html.push_str(STYLE);
    html.push_str("</head>\n<body>\n");

    html.push_str("<h1>Frame-time report</h1>\n");
    html.push_str("<p class=\"meta\">Source <code>");
    html.push_str(&escape(&results_dir.display().to_string()));
    html.push_str(&format!(
        "</code> &middot; renderer <strong>{}</strong> &middot; {} run{}",
        escape(&renderer),
        runs.len(),
        if runs.len() == 1 { "" } else { "s" }
    ));
    if let Some((dir, _)) = baseline {
        html.push_str(&format!(
            " &middot; baseline <code>{}</code>",
            escape(&dir.display().to_string())
        ));
    }
    html.push_str("</p>\n");
    html.push_str(
        "<p class=\"note\">Frame times in milliseconds; lower is better. \
         The budget line marks 16.6&nbsp;ms (60&nbsp;fps).</p>\n",
    );

    html.push_str("<h2>Mean frame time per run</h2>\n");
    html.push_str(&render_chart(runs));

    html.push_str("<h2>Runs</h2>\n");
    html.push_str(&render_table(runs, &baseline_map, baseline.is_some()));

    html.push_str(
        "<footer>Generated by <code>nova_perf perf_report</code> \
         over the capture harness's <code>frametime.csv</code>.</footer>\n",
    );
    html.push_str("</body>\n</html>\n");
    html
}

/// Horizontal bar chart: one row per run, bar length = mean frame time, a tick
/// at p99, all runs on one common scale (the largest p99/max across runs), plus
/// a dashed 16.6 ms budget line. Pure inline SVG - no script, no external lib.
fn render_chart(runs: &[PerfRun]) -> String {
    const LABEL_W: f64 = 200.0;
    const BAR_W: f64 = 460.0;
    const VALUE_W: f64 = 80.0;
    const ROW_H: f64 = 26.0;
    const TOP: f64 = 12.0;
    let width = LABEL_W + BAR_W + VALUE_W;
    let height = TOP + ROW_H * runs.len() as f64 + 12.0;

    // Scale to the largest value any bar/tick can reach so nothing clips.
    let scale = runs
        .iter()
        .map(|run| run.stats.p99_ms.max(run.stats.mean_ms))
        .fold(1.0_f64, f64::max);
    let x_of = |ms: f64| LABEL_W + (ms / scale) * BAR_W;

    let mut svg = format!(
        "<svg class=\"chart\" viewBox=\"0 0 {width:.0} {height:.0}\" \
         role=\"img\" aria-label=\"mean frame time per run\">\n"
    );

    // 16.6 ms budget line, if it falls within the plotted range.
    let budget_ms = 16.6;
    if budget_ms <= scale {
        let x = x_of(budget_ms);
        svg.push_str(&format!(
            "<line class=\"budget\" x1=\"{x:.1}\" y1=\"{:.1}\" x2=\"{x:.1}\" y2=\"{:.1}\"/>\n",
            TOP - 4.0,
            height - 10.0
        ));
    }

    for (i, run) in runs.iter().enumerate() {
        let y = TOP + ROW_H * i as f64;
        let bar_len = x_of(run.stats.mean_ms) - LABEL_W;
        let over_budget = run.stats.mean_ms > budget_ms;
        let bar_class = if over_budget { "bar over" } else { "bar" };
        svg.push_str(&format!(
            "<text class=\"rowlabel\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            LABEL_W - 6.0,
            y + ROW_H * 0.62,
            escape(&run.label)
        ));
        svg.push_str(&format!(
            "<rect class=\"track\" x=\"{LABEL_W:.1}\" y=\"{:.1}\" width=\"{BAR_W:.1}\" height=\"{:.1}\"/>\n",
            y + 4.0,
            ROW_H - 10.0
        ));
        svg.push_str(&format!(
            "<rect class=\"{bar_class}\" x=\"{LABEL_W:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>\n",
            y + 4.0,
            bar_len.max(0.0),
            ROW_H - 10.0
        ));
        // p99 tick.
        let p99_x = x_of(run.stats.p99_ms);
        svg.push_str(&format!(
            "<line class=\"p99\" x1=\"{p99_x:.1}\" y1=\"{:.1}\" x2=\"{p99_x:.1}\" y2=\"{:.1}\"/>\n",
            y + 1.0,
            y + ROW_H - 5.0
        ));
        svg.push_str(&format!(
            "<text class=\"value\" x=\"{:.1}\" y=\"{:.1}\">{:.1} ms</text>\n",
            LABEL_W + BAR_W + 6.0,
            y + ROW_H * 0.62,
            run.stats.mean_ms
        ));
    }
    svg.push_str(
        "<text class=\"legend\" x=\"0\" y=\"0\">\
         <tspan>bar = mean</tspan> <tspan>| tick = p99</tspan> <tspan>-- = 60fps budget</tspan>\
         </text>\n",
    );
    svg.push_str("</svg>\n");
    svg
}

/// The per-run table. When `has_baseline`, two delta columns (mean, p99) show
/// the percentage change against the baseline row of the same label; a missing
/// baseline row renders as an em dash.
fn render_table(
    runs: &[PerfRun],
    baseline: &HashMap<&str, &PerfRun>,
    has_baseline: bool,
) -> String {
    let mut table = String::from("<table>\n<thead>\n<tr>");
    for head in [
        "Scene", "Preset", "Frames", "Window", "Mean", "p50", "p95", "p99", "p999", "Max",
        "Mean FPS", "1% low",
    ] {
        table.push_str(&format!("<th>{head}</th>"));
    }
    if has_baseline {
        table.push_str("<th>&Delta; mean</th><th>&Delta; p99</th>");
    }
    table.push_str("</tr>\n</thead>\n<tbody>\n");

    for run in runs {
        let (scene, preset) = split_label(&run.label);
        let s = &run.stats;
        table.push_str("<tr>");
        table.push_str(&format!("<td class=\"scene\">{}</td>", escape(&scene)));
        table.push_str(&format!("<td>{}</td>", escape(&preset)));
        table.push_str(&format!("<td class=\"num\">{}</td>", s.frames));
        table.push_str(&format!(
            "<td class=\"num\">{:.1} s</td>",
            s.total_ms / 1000.0
        ));
        for value in [s.mean_ms, s.p50_ms, s.p95_ms, s.p99_ms, s.p999_ms, s.max_ms] {
            table.push_str(&format!("<td class=\"num\">{value:.2}</td>"));
        }
        table.push_str(&format!("<td class=\"num\">{:.1}</td>", s.mean_fps));
        table.push_str(&format!("<td class=\"num\">{:.1}</td>", s.one_pct_low_fps));
        if has_baseline {
            let base = baseline.get(run.label.as_str());
            table.push_str(&delta_cell(base.map(|b| b.stats.mean_ms), s.mean_ms));
            table.push_str(&delta_cell(base.map(|b| b.stats.p99_ms), s.p99_ms));
        }
        table.push_str("</tr>\n");
    }
    table.push_str("</tbody>\n</table>\n");
    table
}

/// A delta table cell: `(current - baseline) / baseline` as a signed percent.
/// Frame time is lower-is-better, so a positive delta (slower) is flagged
/// `worse` and a negative one `better`. No baseline value (missing row) renders
/// as an em dash.
fn delta_cell(baseline: Option<f64>, current: f64) -> String {
    let Some(base) = baseline.filter(|b| *b != 0.0) else {
        return "<td class=\"num delta none\">&mdash;</td>".to_string();
    };
    let pct = (current - base) / base * 100.0;
    let class = if pct > 0.5 {
        "worse"
    } else if pct < -0.5 {
        "better"
    } else {
        "flat"
    };
    format!("<td class=\"num delta {class}\">{pct:+.1}%</td>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn split_label_extracts_scene_and_preset() {
        assert_eq!(
            split_label("asteroid_field-high"),
            ("asteroid_field".to_string(), "high".to_string())
        );
        assert_eq!(
            split_label("broadside-low"),
            ("broadside".to_string(), "low".to_string())
        );
        // A non-tier suffix stays part of the scene, preset blank.
        assert_eq!(
            split_label("broadside-combat"),
            ("broadside-combat".to_string(), String::new())
        );
        assert_eq!(split_label("scene"), ("scene".to_string(), String::new()));
    }

    #[test]
    fn escape_neutralizes_html_metacharacters() {
        assert_eq!(escape("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&#39;");
    }

    #[test]
    fn delta_cell_classifies_by_direction() {
        // Lower ms is better: current above baseline is worse.
        assert!(delta_cell(Some(100.0), 110.0).contains("worse"));
        assert!(delta_cell(Some(100.0), 110.0).contains("+10.0%"));
        assert!(delta_cell(Some(100.0), 90.0).contains("better"));
        assert!(delta_cell(Some(100.0), 90.0).contains("-10.0%"));
        assert!(delta_cell(Some(100.0), 100.1).contains("flat"));
        // No baseline (or zero baseline) is an em dash, not a divide-by-zero.
        assert!(delta_cell(None, 90.0).contains("&mdash;"));
        assert!(delta_cell(Some(0.0), 90.0).contains("&mdash;"));
    }

    #[test]
    fn read_runs_parses_the_fixture_dir() {
        let runs = read_runs(&fixture("mini")).expect("fixture parses");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].label, "asteroid_field-high");
        assert!((runs[0].stats.mean_ms - 126.5503).abs() < 1e-9);
    }

    #[test]
    fn render_report_is_self_contained_and_shows_every_run() {
        let dir = fixture("mini");
        let runs = read_runs(&dir).expect("fixture parses");
        let html = render_report(&dir, &runs, None);

        // Self-contained: a real HTML doc with inlined CSS and inline SVG, no
        // external stylesheet/script references.
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<style>"));
        assert!(html.contains("<svg"));
        assert!(!html.contains("<link"));
        assert!(!html.contains("src=\"http"));
        assert!(!html.contains("<script"));

        // Every run's label and a signature number appears.
        for run in &runs {
            assert!(html.contains(&run.label), "missing {}", run.label);
        }
        assert!(html.contains("126.55")); // asteroid_field-high mean
        assert!(html.contains("166.71")); // broadside-high p99 (rounded)

        // All fixture runs are well over the 16.6 ms budget -> flagged bars.
        assert!(html.contains("bar over"));
        // No baseline -> no delta columns.
        assert!(!html.contains("&Delta;"));
    }

    #[test]
    fn render_report_with_baseline_shows_signed_deltas() {
        let dir = fixture("mini");
        let base_dir = fixture("mini-baseline");
        let runs = read_runs(&dir).expect("fixture parses");
        let base = read_runs(&base_dir).expect("baseline parses");
        let html = render_report(&dir, &runs, Some(&(base_dir.clone(), base)));

        // Delta columns exist.
        assert!(html.contains("&Delta; mean"));
        // asteroid_field-high: 126.55 vs 120.0 baseline -> slower -> worse.
        assert!(html.contains("worse"));
        // asteroid_field-low: 117.87 vs 130.0 baseline -> faster -> better.
        assert!(html.contains("better"));
        // broadside-high has no baseline row -> an em-dash delta cell.
        assert!(html.contains("delta none"));
    }

    #[test]
    fn options_parse_positional_and_flags() {
        let opts = Options::parse(&[
            "results/".to_string(),
            "--baseline".to_string(),
            "base/".to_string(),
            "-o".to_string(),
            "out.html".to_string(),
        ])
        .expect("valid args");
        assert_eq!(opts.results_dir, PathBuf::from("results/"));
        assert_eq!(opts.baseline_dir, Some(PathBuf::from("base/")));
        assert_eq!(opts.output, Some(PathBuf::from("out.html")));

        assert!(Options::parse(&[]).is_err()); // results dir required
        assert!(Options::parse(&["a".to_string(), "b".to_string()]).is_err()); // two positionals
        assert!(Options::parse(&["--nope".to_string()]).is_err()); // unknown flag
    }
}

const STYLE: &str = r#"<style>
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body {
  font: 15px/1.5 -apple-system, "Segoe UI", Roboto, sans-serif;
  max-width: 900px; margin: 2rem auto; padding: 0 1rem;
  color: #1a1a1a; background: #fafafa;
}
h1 { font-size: 1.6rem; margin-bottom: 0.2rem; }
h2 { font-size: 1.15rem; margin-top: 2rem; border-bottom: 1px solid #ddd; padding-bottom: 0.3rem; }
.meta { color: #555; margin: 0.2rem 0; }
.note { color: #666; font-size: 0.9rem; }
code { background: #eee; padding: 0.05rem 0.3rem; border-radius: 3px; font-size: 0.85em; }
table { border-collapse: collapse; width: 100%; margin-top: 0.5rem; font-variant-numeric: tabular-nums; }
th, td { padding: 0.35rem 0.55rem; border-bottom: 1px solid #e2e2e2; text-align: left; }
th { font-weight: 600; color: #333; border-bottom: 2px solid #ccc; }
td.num { text-align: right; }
td.scene { font-weight: 600; }
td.delta.worse { color: #b00020; }
td.delta.better { color: #087f23; }
td.delta.flat { color: #888; }
td.delta.none { color: #bbb; }
.chart { width: 100%; height: auto; margin-top: 0.5rem; }
.chart .rowlabel { font-size: 12px; text-anchor: end; fill: #333; }
.chart .value { font-size: 12px; fill: #333; }
.chart .track { fill: #ececec; rx: 2; }
.chart .bar { fill: #3576c4; }
.chart .bar.over { fill: #c46a35; }
.chart .p99 { stroke: #1a1a1a; stroke-width: 1.5; }
.chart .budget { stroke: #087f23; stroke-width: 1.2; stroke-dasharray: 3 3; }
.chart .legend { font-size: 11px; fill: #777; }
.chart .legend tspan { margin-right: 8px; }
footer { margin-top: 2.5rem; color: #888; font-size: 0.85rem; border-top: 1px solid #ddd; padding-top: 0.6rem; }
@media (prefers-color-scheme: dark) {
  body { color: #e6e6e6; background: #16181c; }
  h2 { border-color: #333; }
  .meta { color: #aaa; } .note, footer { color: #999; }
  code { background: #2a2d33; }
  th { color: #ddd; border-color: #444; } th, td { border-color: #2a2d33; }
  .chart .rowlabel, .chart .value { fill: #ccc; }
  .chart .track { fill: #2a2d33; }
  .chart .p99 { stroke: #e6e6e6; }
}
</style>
"#;
