//! The aggregated multi-run report (task 20260719-210438): one row per
//! probed example, an overall worst-of verdict, and the honesty rules
//! carried up from the per-run report - a row's verdict is never shown
//! without its `measured n/total`, and exclusions are IN the report with
//! reasons (no silent caps).
//!
//! Artifacts, written next to the per-example run dirs (`<base>/<example>/`):
//! - `probe-all.json` - the aggregate manifest (spec, identity, per-row
//!   outcome + duration + exclusions). The gate for `probe report <base>`.
//! - `index.json` - the machine mirror agents read: verdict + measured +
//!   per-check statuses per row.
//! - `index.html` - the human table, linking each row's own report.html.

use crate::report::{escape, STYLE};

/// One example's row in the aggregate: identity from the sweep driver,
/// verdict/measured/checks re-read from the run's own checks.json (probe
/// consumes its own agent surface; a run that died before producing one
/// becomes an ERROR row carrying the message).
#[derive(Debug, Clone, PartialEq)]
pub struct AllRow {
    /// The example's name (its run dir under `<base>/`).
    pub example: String,
    /// The category the example belongs to (from the sweep driver).
    pub category: String,
    /// OK | WARN | FAIL | NO_DATA | ERROR (ERROR: the run never produced
    /// checks.json - build failure, probe error - message in `error`).
    pub verdict: String,
    /// "n/total" from checks.json, or "-" for ERROR rows.
    pub measured: String,
    /// (check name, status) pairs from checks.json, empty for ERROR rows.
    pub checks: Vec<(String, String)>,
    /// Wall-clock duration of the run, in seconds.
    pub duration_secs: u64,
    /// Failure message for an ERROR row (the run never produced checks.json).
    pub error: Option<String>,
}

/// The aggregate manifest - what `probe run <multi-spec>` recorded, and
/// what gates + feeds a `probe report` re-render of the index.
#[derive(Debug, Clone, PartialEq)]
pub struct AllManifest {
    /// The spec as given (`--all`, `ui`, `scenario,hud_range`).
    pub spec: String,
    /// Unix timestamp when the sweep started.
    pub started_unix: u64,
    /// The git SHA the sweep ran against.
    pub git_sha: String,
    /// The host the sweep ran on.
    pub host: String,
    /// (example, reason) pairs deliberately not probed by --all/category
    /// expansion - listed in the report so absence reads as a decision.
    pub excluded: Vec<(String, String)>,
    /// One [`AllRow`] per probed example.
    pub rows: Vec<AllRow>,
}

impl AllManifest {
    /// Serialize the manifest to the `probe-all.json` JSON shape.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "spec": self.spec,
            "started_unix": self.started_unix,
            "git_sha": self.git_sha,
            "host": self.host,
            "excluded": self.excluded.iter().map(|(example, reason)| {
                serde_json::json!({ "example": example, "reason": reason })
            }).collect::<Vec<_>>(),
            "rows": self.rows.iter().map(|row| serde_json::json!({
                "example": row.example,
                "category": row.category,
                "verdict": row.verdict,
                "measured": row.measured,
                "checks": row.checks.iter().map(|(name, status)| {
                    serde_json::json!({ "name": name, "status": status })
                }).collect::<Vec<_>>(),
                "duration_secs": row.duration_secs,
                "error": row.error,
            })).collect::<Vec<_>>(),
            "generated_by": "nova_probe aggregate",
        })
    }

    /// Parse an [`AllManifest`] back from the `probe-all.json` JSON shape.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, String> {
        let str_field = |v: &serde_json::Value, key: &str| -> Result<String, String> {
            v.get(key)
                .and_then(|f| f.as_str())
                .map(String::from)
                .ok_or_else(|| format!("probe-all.json: missing {key}"))
        };
        let rows = value
            .get("rows")
            .and_then(|r| r.as_array())
            .ok_or("probe-all.json: missing rows")?
            .iter()
            .map(|row| {
                Ok(AllRow {
                    example: str_field(row, "example")?,
                    category: str_field(row, "category")?,
                    verdict: str_field(row, "verdict")?,
                    measured: str_field(row, "measured")?,
                    checks: row
                        .get("checks")
                        .and_then(|c| c.as_array())
                        .map(|checks| {
                            checks
                                .iter()
                                .filter_map(|c| {
                                    Some((
                                        c.get("name")?.as_str()?.to_string(),
                                        c.get("status")?.as_str()?.to_string(),
                                    ))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    duration_secs: row
                        .get("duration_secs")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(0),
                    error: row.get("error").and_then(|e| e.as_str()).map(String::from),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(AllManifest {
            spec: str_field(value, "spec")?,
            started_unix: value
                .get("started_unix")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            git_sha: str_field(value, "git_sha")?,
            host: str_field(value, "host")?,
            excluded: value
                .get("excluded")
                .and_then(|e| e.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| {
                            Some((
                                entry.get("example")?.as_str()?.to_string(),
                                entry.get("reason")?.as_str()?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            rows,
        })
    }
}

/// Verdict severity for the worst-of aggregation. NO_DATA ranks between
/// WARN and FAIL on purpose: an example that produced zero evidence is a
/// failure of the evaluation, not a soft skip (the per-run exit code
/// already treats it as one). Unrecognized verdicts rank as FAIL -
/// fail-closed.
pub fn verdict_severity(verdict: &str) -> u8 {
    match verdict {
        "OK" => 0,
        "WARN" => 1,
        "NO_DATA" => 2,
        _ => 3, // FAIL, ERROR, anything unrecognized
    }
}

/// The aggregate verdict: the WORST row. An empty run is NO_DATA.
pub fn overall_verdict(rows: &[AllRow]) -> &'static str {
    let worst = rows
        .iter()
        .map(|row| verdict_severity(&row.verdict))
        .max()
        .unwrap_or(2);
    match worst {
        0 => "OK",
        1 => "WARN",
        2 => "NO_DATA",
        _ => "FAIL",
    }
}

/// The check columns every row renders, in per-run report order. A run
/// missing a check (older checks.json) renders "-" instead of shifting
/// columns.
const CHECK_COLUMNS: &[&str] = &[
    "process_exit",
    "run_completed",
    "reached_playing",
    "invariants_held",
    "fps_within_baseline",
    "log_clean",
];

/// The machine mirror (index.json): everything an agent needs to answer
/// "does every feature still work" from one file.
pub fn index_json(manifest: &AllManifest) -> serde_json::Value {
    let mut value = manifest.to_json();
    value["overall"] = serde_json::json!(overall_verdict(&manifest.rows));
    value["generated_by"] = serde_json::json!("nova_probe aggregate index");
    value
}

/// Render index.html: verdict banner, the status table, the exclusions.
/// Same shell + classes as the per-run report (report::STYLE).
pub fn render_index(manifest: &AllManifest) -> String {
    let overall = overall_verdict(&manifest.rows);
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!(
        "<title>nova probe aggregate - {overall}</title>\n"
    ));
    html.push_str(STYLE);
    html.push_str("</head>\n<body>\n<h1>Probe aggregate</h1>\n");
    html.push_str(&format!(
        "<p class=\"meta\">spec <code>{}</code> | git {} | host {}</p>\n",
        escape(&manifest.spec),
        escape(&manifest.git_sha),
        escape(&manifest.host),
    ));

    let totals = |verdict: &str| {
        manifest
            .rows
            .iter()
            .filter(|row| row.verdict == verdict)
            .count()
    };
    let banner_class = match overall {
        "OK" => "ok",
        "FAIL" => "fail",
        _ => "warn",
    };
    html.push_str(&format!(
        "<div class=\"banner {banner_class}\">Aggregate verdict: {overall} \
         ({} example(s): {} OK, {} WARN, {} FAIL, {} NO_DATA, {} ERROR)\
         <span class=\"confirm\">The verdict is the WORST row. A row's verdict only \
         means what its measured column covers - SKIPPED checks were NOT measured, \
         never \"held\". Open a row's report for the evidence.</span></div>\n",
        manifest.rows.len(),
        totals("OK"),
        totals("WARN"),
        totals("FAIL"),
        totals("NO_DATA"),
        totals("ERROR"),
    ));

    html.push_str(
        "<table>\n<thead><tr><th>example</th><th>category</th><th>verdict</th>\
         <th>measured</th>",
    );
    for column in CHECK_COLUMNS {
        html.push_str(&format!("<th>{}</th>", column.replace('_', " ")));
    }
    html.push_str("<th>duration</th></tr></thead>\n<tbody>\n");
    for row in &manifest.rows {
        let verdict_class = match row.verdict.as_str() {
            "OK" => "pass",
            "WARN" => "warn",
            _ => "fail", // FAIL, NO_DATA, ERROR: needs attention
        };
        html.push_str(&format!(
            "<tr><td class=\"scene\"><a href=\"{0}/report.html\">{0}</a></td><td>{1}</td>\
             <td class=\"status-{2}\">{3}</td><td>{4}</td>",
            escape(&row.example),
            escape(&row.category),
            verdict_class,
            row.verdict,
            escape(&row.measured),
        ));
        for column in CHECK_COLUMNS {
            let status = row
                .checks
                .iter()
                .find(|(name, _)| name == column)
                .map(|(_, status)| status.as_str())
                .unwrap_or("-");
            html.push_str(&format!(
                "<td class=\"status-{}\">{}</td>",
                status.to_lowercase(),
                if status == "SKIPPED" { "skip" } else { status }
            ));
        }
        html.push_str(&format!("<td>{}s</td></tr>\n", row.duration_secs));
        if let Some(error) = &row.error {
            html.push_str(&format!(
                "<tr><td></td><td colspan=\"10\" class=\"status-fail\">{}</td></tr>\n",
                escape(error)
            ));
        }
    }
    html.push_str("</tbody></table>\n");

    if !manifest.excluded.is_empty() {
        html.push_str("<h2>Not probed (deliberately)</h2>\n<ul>\n");
        for (example, reason) in &manifest.excluded {
            html.push_str(&format!(
                "<li><code>{}</code> - {}</li>\n",
                escape(example),
                escape(reason)
            ));
        }
        html.push_str("</ul>\n");
    }
    html.push_str(
        "<footer>nova_probe aggregate - per-example evidence lives in each row's \
         own run dir.</footer>\n</body>\n</html>\n",
    );
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(example: &str, verdict: &str) -> AllRow {
        AllRow {
            example: example.into(),
            category: "gameplay".into(),
            verdict: verdict.into(),
            measured: "5/6".into(),
            checks: vec![("process_exit".into(), "PASS".into())],
            duration_secs: 42,
            error: None,
        }
    }

    #[test]
    fn overall_is_the_worst_row_and_empty_is_no_data() {
        assert_eq!(overall_verdict(&[row("a", "OK"), row("b", "OK")]), "OK");
        assert_eq!(overall_verdict(&[row("a", "OK"), row("b", "WARN")]), "WARN");
        assert_eq!(
            overall_verdict(&[row("a", "WARN"), row("b", "NO_DATA")]),
            "NO_DATA"
        );
        assert_eq!(overall_verdict(&[row("a", "OK"), row("b", "FAIL")]), "FAIL");
        assert_eq!(
            overall_verdict(&[row("a", "OK"), row("b", "ERROR")]),
            "FAIL",
            "unrecognized verdicts fail closed"
        );
        assert_eq!(overall_verdict(&[]), "NO_DATA");
    }

    #[test]
    fn manifest_roundtrips_through_json() {
        let manifest = AllManifest {
            spec: "--all".into(),
            started_unix: 1_700_000_000,
            git_sha: "abc1234".into(),
            host: "workstation".into(),
            excluded: vec![("render_scale_shot".into(), "real-GPU capture".into())],
            rows: vec![AllRow {
                error: Some("build failed".into()),
                ..row("scenario", "ERROR")
            }],
        };
        let parsed = AllManifest::from_json(&manifest.to_json()).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn index_render_carries_the_honesty_surface() {
        let manifest = AllManifest {
            spec: "ui".into(),
            started_unix: 0,
            git_sha: "abc".into(),
            host: "h".into(),
            excluded: vec![("render_scale_shot".into(), "needs a real GPU".into())],
            rows: vec![row("editor", "OK"), row("hud_range", "FAIL")],
        };
        let html = render_index(&manifest);
        assert!(
            html.contains("Aggregate verdict: FAIL"),
            "worst-of verdict in the banner"
        );
        assert!(html.contains("5/6"), "measured column present");
        assert!(
            html.contains("hud_range/report.html"),
            "rows link their reports"
        );
        assert!(
            html.contains("Not probed (deliberately)") && html.contains("needs a real GPU"),
            "exclusions listed with reasons"
        );
        assert!(
            html.contains("never \"held\""),
            "the SKIPPED honesty note is on the page"
        );
    }
}
