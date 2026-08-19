//! The last stage of the pipeline: what [`crate::evaluation`] decided, made
//! readable. One self-contained file per run ([`html`]) plus the index over
//! many runs ([`aggregate`]) - inline CSS and inline SVG, no external assets,
//! so a report opens offline.
//!
//! This module root holds the pieces both renderers share (styles, the
//! frame-time chart and table) over parsed [`PerfRun`]s; the standalone FPS
//! renderer they once served retired with the perf_report bin.
//!
//! Renderer identity: schema-v2 rows carry their own metadata (backend,
//! adapter, git SHA - see [`nova_probe::stats::RunMeta`]), which this renderer
//! prefers; v1 rows (the v0.7.0 baseline) fall back to the results
//! directory's name, the old convention (`gpu` / `sw` / `xgpu` / `web`).

pub mod aggregate;
pub mod html;

use std::collections::HashMap;

use nova_probe::prelude::*;

use crate::evaluation::frames::prelude::*;

/// Glob-import surface for both renderers.
pub mod prelude {
    pub use super::{aggregate::prelude::*, html::prelude::*};
}

pub use prelude::*;

/// The renderer string shown for one run: its own metadata when known
/// (schema v2), else the dir-derived fallback (v1 rows).
fn run_renderer(run: &PerfRun, fallback: &str) -> String {
    if run.meta.backend != "unknown" {
        run.meta.backend.clone()
    } else {
        fallback.to_string()
    }
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
pub(crate) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// The FRAME READ that leads the Performance section: for each capture, the
/// worst frame, the mean, and what each comes to in FPS, with the under-60
/// rows called out in words and in colour.
///
/// It is the first thing in the section on purpose. A reader wants one number
/// and one word - "23 FPS, BAD" - and the percentile table below is what they
/// open next if that word surprised them. Nothing here is a gate: probe
/// reports frame cost and a human judges it (see
/// [`crate::evaluation::frames`]).
pub(crate) fn render_frame_read(runs: &[PerfRun]) -> String {
    let reads = read_frames(runs);
    if reads.is_empty() {
        return String::new();
    }
    let mut html = String::from("<div class=\"frameread\">\n");
    for read in &reads {
        html.push_str(&format!(
            "<div class=\"call {}\">\
             <span class=\"who\">{}</span>\
             <span class=\"headline\">{:.0} FPS worst frame &#183; {:.0} FPS mean</span>\
             <span class=\"say\">{}</span>\
             <span class=\"detail\">worst frame {:.1} ms, mean {:.1} ms \
             (60 fps is {:.1} ms)</span></div>\n",
            read.class(),
            escape(&read.label),
            read.worst_fps,
            read.mean_fps,
            read.verdict(),
            read.worst_ms,
            read.mean_ms,
            SMOOTH_FRAME_MS,
        ));
    }
    html.push_str(
        "<p class=\"note\">Reported, not graded. No check passes or fails on a \
         frame-time number - whether this scene is fast enough on this machine \
         is the reader's call.</p>\n</div>\n",
    );
    html
}

/// The REPEAT GATE table, when the run captured a repeat set: one row per
/// capture with its mean, median and worst frame, and whether the gate admitted
/// it. Empty for a single capture - one capture is not a set, and the read
/// above already covers it.
///
/// The point of the table is that the reader sees WHICH captures the reported
/// tail came from. A gate whose workings are hidden is a gate nobody can argue
/// with.
pub(crate) fn render_repeat_gate(runs: &[PerfRun]) -> String {
    let reads = read_repeats(runs);
    if reads.is_empty() {
        return String::new();
    }
    let mut html = String::new();
    for read in &reads {
        html.push_str(&format!(
            "<h3>Repeat gate: {}</h3>\n<p class=\"note\">{} of {} captures admitted, \
             band &plusmn;{:.0}% on mean and median around {:.1} ms / {:.1} ms. ",
            escape(&read.label),
            read.admitted(),
            read.captures.len(),
            REPEAT_GATE_TOLERANCE * 100.0,
            read.reference_mean_ms,
            read.reference_median_ms,
        ));
        match (read.p99_ms, read.p99_spread) {
            (Some(p99), Some(spread)) => html.push_str(&format!(
                "p99 across the admitted captures: <strong>{p99:.1} ms</strong> - \
                 the median of a group spanning {:.0}% of that. The median is the \
                 number to compare between two sets; the span is how wide the \
                 captures under it were.</p>\n",
                spread * 100.0
            )),
            // Two very different causes, and the report may not guess which:
            // a busy machine, or a scene whose own load is not repeatable.
            _ => html.push_str(
                "The gate admitted nothing, so this set reports no tail. The \
                 captures did not measure the same thing - either the machine \
                 moved under them, or this scene's own load is not repeatable \
                 and no number of repeats will fix it.</p>\n",
            ),
        }
        html.push_str("<table>\n<thead>\n<tr>");
        for head in ["Repeat", "Mean", "Median", "p99", "Worst", "Gate"] {
            html.push_str(&format!("<th>{head}</th>"));
        }
        html.push_str("</tr>\n</thead>\n<tbody>\n");
        for capture in &read.captures {
            html.push_str(&format!(
                "<tr><td>#{}</td><td>{:.2} ms</td><td>{:.2} ms</td><td>{:.2} ms</td>\
                 <td>{:.2} ms</td><td class=\"{}\">{}</td></tr>\n",
                capture.index,
                capture.mean_ms,
                capture.median_ms,
                capture.p99_ms,
                capture.worst_ms,
                // SKIPPED, not failed: a discarded capture is a measurement
                // that did not count, and red would read as a broken run.
                if capture.admitted {
                    "status-pass"
                } else {
                    "status-skipped"
                },
                if capture.admitted {
                    "admitted"
                } else {
                    "discarded - contaminated"
                },
            ));
        }
        html.push_str("</tbody>\n</table>\n");
        if let (Some(worst), Some(spread)) = (read.worst_ms, read.worst_spread) {
            html.push_str(&format!(
                "<p class=\"note\">Slowest single frame across the admitted captures: \
                 {worst:.1} ms (median of a group spanning {:.0}% of that). It is one \
                 sample per capture and behaves like one - read it, do not build a \
                 claim on it.</p>\n",
                spread * 100.0
            ));
        }
    }
    html.push_str(
        "<p class=\"note\">A discarded capture says the machine changed under the \
         measurement, not that the code got worse. Still reported, never graded.</p>\n",
    );
    html
}

/// The REFUSED captures, scraped out of the run log.
///
/// A refused capture wrote no CSV row, so it is invisible to every table built
/// from those rows - which is exactly the shape a reader would otherwise take
/// for a slightly smaller set. It leads the frame section because it says the
/// numbers under it are missing on purpose.
pub(crate) fn render_refused_captures(log: &str) -> String {
    let aborts: Vec<CaptureAbort> = log.lines().filter_map(parse_capture_abort_line).collect();
    if aborts.is_empty() {
        return String::new();
    }
    let mut html = String::from(
        "<h3 class=\"fail\">Refused captures</h3>\n<table>\n<thead>\n<tr>\
         <th>Capture</th><th>Refused in</th><th>After</th><th>Window asked for</th>\
         <th>Reason</th></tr>\n</thead>\n<tbody>\n",
    );
    for abort in &aborts {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{} frame(s)</td><td>{} + {}</td>\
             <td><code>{}</code></td></tr>\n",
            escape(&abort.label),
            escape(&abort.phase),
            abort.frame,
            abort.window.0,
            abort.window.1,
            escape(&abort.reason),
        ));
    }
    html.push_str(
        "</tbody>\n</table>\n<p class=\"note\">These captures met a STOPPED \
         simulation inside their window - the scene reached an end, and the frames \
         after it draw a still picture at a plausible cost. They wrote no \
         statistics and appear in no table below. Bound the window so it closes \
         while the scene is still running, or measure a scene that cannot end \
         inside it.</p>\n",
    );
    html
}

/// The FIXED-STEP read, scraped out of the run log.
///
/// One row per capture: how many fixed steps ran inside a frame, how many
/// frames ran NONE (the simulation was stopped - a pause, a result screen, a
/// menu), and how many sat on the window's step ceiling and what those cost.
///
/// It is here because a capture window that contains a stopped simulation, or
/// that spends frames pinned at `Time<Virtual>`'s clamp, is not measuring one
/// scene - and no percentile above says so. Reported, never graded.
pub(crate) fn render_fixed_steps(log: &str) -> String {
    let reads: Vec<(String, FixedStepStats)> =
        log.lines().filter_map(parse_fixed_steps_line).collect();
    if reads.is_empty() {
        return String::new();
    }
    let mut html = String::from("<h3>Fixed steps per frame</h3>\n<table>\n<thead>\n<tr>");
    for head in [
        "Capture",
        "Steps/frame",
        "Range",
        "Frames with no step",
        "Frames at the top",
        "Their cost",
    ] {
        html.push_str(&format!("<th>{head}</th>"));
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for (label, steps) in &reads {
        let top = steps.at_ceiling();
        html.push_str(&format!(
            "<tr><td>{}</td><td>{:.2}</td><td>{} - {}</td><td>{}</td>\
             <td>{}</td><td>{}</td></tr>\n",
            escape(label),
            steps.mean_steps,
            steps.min_steps,
            steps.max_steps,
            steps.stopped_frames(),
            top.map_or(0, |b| b.frames),
            top.map_or_else(|| "-".into(), |b| format!("{:.1} ms", b.mean_frame_ms)),
        ));
    }
    html.push_str(
        "</tbody>\n</table>\n<p class=\"note\">A frame runs as many fixed steps as \
         the accumulated virtual time allows, capped by <code>Time&lt;Virtual&gt;\
         ::max_delta</code>. Frames with NO step mean the simulation was stopped \
         inside the window - in a scene slower than the timestep that window did \
         not measure one scene. Frames at the top of the range are where the clamp \
         is discarding time the world never simulates. Reported, not graded.</p>\n",
    );
    html
}

/// Horizontal bar chart: one row per run, bar length = mean frame time, a tick
/// at p99, all runs on one common scale (the largest p99/max across runs), plus
/// a dashed 16.6 ms budget line. Pure inline SVG - no script, no external lib.
pub(crate) fn render_chart(runs: &[PerfRun]) -> String {
    const LABEL_W: f64 = 200.0;
    const BAR_W: f64 = 460.0;
    const VALUE_W: f64 = 80.0;
    const ROW_H: f64 = 26.0;
    const TOP: f64 = 12.0;
    let width = LABEL_W + BAR_W + VALUE_W;
    // A dedicated legend row below the bars (a text at y=0 clips above the
    // viewBox - SVG anchors text at its BASELINE).
    const LEGEND_H: f64 = 22.0;
    let height = TOP + ROW_H * runs.len() as f64 + LEGEND_H;

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

    // The 60 fps line, if it falls within the plotted range.
    let smooth_ms = SMOOTH_FRAME_MS;
    if smooth_ms <= scale {
        let x = x_of(smooth_ms);
        svg.push_str(&format!(
            "<line class=\"smooth\" x1=\"{x:.1}\" y1=\"{:.1}\" x2=\"{x:.1}\" y2=\"{:.1}\"/>\n",
            TOP - 4.0,
            height - 10.0
        ));
    }

    for (i, run) in runs.iter().enumerate() {
        let y = TOP + ROW_H * i as f64;
        let bar_len = x_of(run.stats.mean_ms) - LABEL_W;
        let slow = run.stats.mean_ms > smooth_ms;
        let bar_class = if slow { "bar over" } else { "bar" };
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
    // One plain text run: tspan spacing via CSS margins does not exist in
    // SVG, so the separators live in the string itself.
    svg.push_str(&format!(
        "<text class=\"legend\" x=\"{LABEL_W:.0}\" y=\"{:.1}\">\
         bar = mean&#160;&#160;|&#160;&#160;tick = p99&#160;&#160;|&#160;&#160;dashed = 60 fps</text>\n",
        height - 7.0
    ));
    svg.push_str("</svg>\n");
    svg
}

/// The per-run table. The Renderer column shows each run's own metadata
/// (v2) or the dir-derived fallback (v1). When `has_baseline`, two delta
/// columns (mean, p99) show the percentage change against the baseline row of
/// the same label; a missing baseline row renders as an em dash.
pub(crate) fn render_table(
    runs: &[PerfRun],
    fallback_renderer: &str,
    baseline: &HashMap<&str, &PerfRun>,
    has_baseline: bool,
) -> String {
    let mut table = String::from("<table>\n<thead>\n<tr>");
    for head in [
        "Scene", "Preset", "Renderer", "Frames", "Window", "Mean", "p50", "p95", "p99", "p999",
        "Max", "Mean FPS", "1% low",
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
        // The adapter name rides as a hover title so the cell stays narrow.
        // The build-profile badge (schema v3) flags dev rows - dev numbers
        // are NOT baselines; unknown (pre-v3 rows) shows nothing.
        let profile_badge = match run.meta.profile.as_str() {
            "dev" => " <span class=\"profile dev\" title=\"dev build - not a baseline\">dev</span>",
            "release" => " <span class=\"profile release\">release</span>",
            _ => "",
        };
        table.push_str(&format!(
            "<td title=\"{}\">{}{profile_badge}</td>",
            escape(&run.meta.adapter),
            escape(&run_renderer(run, fallback_renderer))
        ));
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

pub(crate) const STYLE: &str = r#"<style>
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body {
  font: 15px/1.5 -apple-system, "Segoe UI", Roboto, sans-serif;
  max-width: 900px; margin: 2rem auto; padding: 0 1rem;
  color: #1a1a1a; background: #fafafa;
}
h1 { font-size: 1.6rem; margin-bottom: 0.2rem; }
h2 { font-size: 1.15rem; margin-top: 2rem; border-bottom: 1px solid #ddd; padding-bottom: 0.3rem; }
h3 { font-size: 1rem; margin-top: 1.4rem; margin-bottom: 0.2rem; }
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
.chart .smooth { stroke: #087f23; stroke-width: 1.2; stroke-dasharray: 3 3; }
.frameread { margin: 0.6rem 0 1rem; }
.frameread .call {
  display: grid; grid-template-columns: minmax(9rem, auto) 1fr; gap: 0.1rem 0.8rem;
  padding: 0.7rem 0.9rem; border-radius: 6px; margin-bottom: 0.5rem;
  border-left: 5px solid currentColor;
}
.frameread .who { grid-row: span 3; align-self: center; font-weight: 600; font-size: 0.95rem; }
.frameread .headline { font-size: 1.35rem; font-weight: 700; font-variant-numeric: tabular-nums; }
.frameread .say { font-weight: 700; letter-spacing: 0.02em; font-size: 0.9rem; }
.frameread .detail { font-size: 0.85rem; opacity: 0.75; font-variant-numeric: tabular-nums; }
.frameread .call.good { background: #e3f4e6; color: #0b6623; }
.frameread .call.mixed { background: #fff3d6; color: #7a5b00; }
.frameread .call.bad { background: #fbe3e4; color: #8f1013; }
.chart .legend { font-size: 11px; fill: #777; }
footer { margin-top: 2.5rem; color: #888; font-size: 0.85rem; border-top: 1px solid #ddd; padding-top: 0.6rem; }
.banner { padding: 0.8rem 1rem; border-radius: 6px; font-weight: 600; margin: 1rem 0; }
.banner.ok { background: #e3f4e6; color: #0b6623; }
.banner.warn { background: #fff3d6; color: #7a5b00; }
.banner.fail { background: #fbe3e4; color: #8f1013; }
.banner .confirm { display: block; font-weight: 400; font-size: 0.85rem; margin-top: 0.3rem; }
td.status-pass { color: #087f23; font-weight: 600; }
td.status-warn { color: #b8860b; font-weight: 600; }
td.status-fail { color: #b00020; font-weight: 600; }
td.status-skipped { color: #999; }
td.status-na { color: #999; font-style: italic; }
td.status-unknown { color: #999; }
.profile { font-size: 0.75em; padding: 0.05rem 0.3rem; border-radius: 3px; }
.profile.dev { background: #fff3d6; color: #7a5b00; }
.profile.release { background: #e3f4e6; color: #0b6623; }
details { margin: 0.6rem 0; }
details summary { cursor: pointer; color: #555; }
.checklist li { margin: 0.3rem 0; }
.oknok { font-weight: 700; margin-top: 0.8rem; }
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

#[cfg(test)]
mod tests {
    use super::*;

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

    /// One capture, with the two numbers the frame read is made of.
    fn run(label: &str, mean_ms: f64, max_ms: f64) -> PerfRun {
        PerfRun {
            label: label.into(),
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
                mean_fps: 1000.0 / mean_ms,
                one_pct_low_fps: 500.0 / mean_ms,
            },
            meta: RunMeta::unknown(),
        }
    }

    /// The reader's headline: the FPS, the word, and the fact that neither is
    /// a gate. A slow row is flagged `bad` so the colour matches the words.
    #[test]
    fn the_frame_read_leads_with_fps_and_says_it_grades_nothing() {
        let html = render_frame_read(&[run("slow_scene", 40.0, 200.0)]);
        assert!(html.contains("class=\"call bad\""), "{html}");
        assert!(html.contains("FPS IS UNDER 60 - BAD"), "{html}");
        assert!(html.contains("25 FPS mean"), "{html}");
        assert!(html.contains("5 FPS worst frame"), "{html}");
        assert!(html.contains("Reported, not graded"), "{html}");
    }

    #[test]
    fn a_capture_holding_the_line_reads_good() {
        let html = render_frame_read(&[run("quick", 8.0, 12.0)]);
        assert!(html.contains("class=\"call good\""), "{html}");
        assert!(html.contains("GOOD"), "{html}");
        assert!(!html.contains("BAD"), "{html}");
    }

    /// Nothing to read is nothing rendered - an empty box would say a capture
    /// happened and read as fine.
    #[test]
    fn no_capture_renders_no_frame_read() {
        assert!(render_frame_read(&[]).is_empty());
    }

    /// The gate has to SHOW its working: which captures it kept, which it threw
    /// out, and how wide the number it reports actually is.
    #[test]
    fn the_repeat_gate_table_shows_every_capture_and_the_spread() {
        let mut runs = vec![
            run("s#1", 26.0, 44.0),
            run("s#2", 26.0, 58.0),
            run("s#3", 60.0, 300.0),
        ];
        // `run` ties p50 and p99 to the mean; the gate reads all of them.
        for r in &mut runs {
            r.stats.p50_ms = r.stats.mean_ms;
            r.stats.p99_ms = r.stats.max_ms * 0.8;
        }
        let html = render_repeat_gate(&runs);
        assert!(html.contains("Repeat gate: s"), "{html}");
        assert!(html.contains("2 of 3 captures admitted"), "{html}");
        assert!(html.contains("discarded - contaminated"), "{html}");
        // The discarded row reads SKIPPED, never failed.
        assert!(html.contains("class=\"status-skipped\""), "{html}");
        assert!(!html.contains("status-fail"), "{html}");
        // The headline is the median of the admitted p99 (35.2, 46.4), not
        // the 240 ms the discarded capture carried.
        assert!(html.contains("<strong>35.2 ms</strong>"), "{html}");
        // The slowest single frame rides beside it, marked as one sample.
        assert!(html.contains("44.0 ms (median"), "{html}");
        assert!(html.contains("do not build a claim on it"), "{html}");
        assert!(html.contains("never graded"), "{html}");
        // A lone capture is not a set and renders nothing at all.
        assert!(render_repeat_gate(&[run("s", 26.0, 44.0)]).is_empty());
    }

    /// The reading that caught a capture window measuring a paused result
    /// screen: frames that ran no fixed step at all.
    #[test]
    fn the_fixed_step_table_shows_stopped_frames_and_the_ceiling() {
        let log = "\
2026-08-19T15:00:00Z INFO nova_probe: nova perf: label=wfc_arena#5 fixed_steps \
min=0 max=16 mean=5.419 total=4877 buckets=0:165@69.7ms,4:187@67.2ms,16:34@354.1ms
2026-08-19T15:00:00Z INFO nova_probe: unrelated line";
        let html = render_fixed_steps(log);
        assert!(html.contains("Fixed steps per frame"), "{html}");
        assert!(html.contains("wfc_arena#5"), "{html}");
        assert!(html.contains("<td>0 - 16</td>"), "{html}");
        assert!(html.contains("<td>165</td>"), "165 stopped frames: {html}");
        assert!(html.contains("<td>34</td>"), "34 at the ceiling: {html}");
        assert!(html.contains("354.1 ms"), "{html}");
        assert!(html.contains("not graded"), "{html}");
        // A log with no capture line renders nothing rather than an empty box.
        assert!(render_fixed_steps("nothing to see").is_empty());
    }

    /// A refused capture has no row anywhere else, so the report has to say so
    /// on its own or the set just reads as smaller.
    #[test]
    fn a_refused_capture_gets_its_own_table() {
        let log = "\
2026-08-19T15:00:00Z ERROR nova_probe: nova perf: label=wfc_arena#2 ABORTED \
reason=simulation_stopped phase=capture frame=345 warmup=60 frames=360 - stopped
2026-08-19T15:00:00Z INFO nova_probe: unrelated line";
        let html = render_refused_captures(log);
        assert!(html.contains("Refused captures"), "{html}");
        assert!(html.contains("wfc_arena#2"), "{html}");
        assert!(html.contains("345 frame(s)"), "{html}");
        assert!(html.contains("60 + 360"), "{html}");
        assert!(html.contains("simulation_stopped"), "{html}");
        // A clean run renders nothing rather than an empty box.
        assert!(render_refused_captures("nothing to see").is_empty());
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
}
