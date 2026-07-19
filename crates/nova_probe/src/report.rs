//! HTML report rendering over a results directory: turns parsed [`PerfRun`]s
//! into one self-contained `report.html` (inline CSS + inline SVG, no external
//! assets, so it opens offline). These are the SHARED pieces (styles, the
//! frame-time chart and table) the unified run report composes; the
//! standalone FPS renderer they once served retired with the perf_report
//! bin (consolidation task 20260719-174603).
//!
//! Renderer identity: schema-v2 rows carry their own metadata (backend,
//! adapter, git SHA - see [`crate::stats::RunMeta`]), which this renderer
//! prefers; v1 rows (the v0.7.0 baseline) fall back to the results
//! directory's name, the old convention (`gpu` / `sw` / `xgpu` / `web`).

use std::collections::HashMap;

use crate::stats::PerfRun;

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
.banner { padding: 0.8rem 1rem; border-radius: 6px; font-weight: 600; margin: 1rem 0; }
.banner.ok { background: #e3f4e6; color: #0b6623; }
.banner.warn { background: #fff3d6; color: #7a5b00; }
.banner.fail { background: #fbe3e4; color: #8f1013; }
.banner .confirm { display: block; font-weight: 400; font-size: 0.85rem; margin-top: 0.3rem; }
td.status-pass { color: #087f23; font-weight: 600; }
td.status-warn { color: #b8860b; font-weight: 600; }
td.status-fail { color: #b00020; font-weight: 600; }
td.status-skipped { color: #999; }
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
