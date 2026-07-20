//! The human-facing content-lint report (task 20260718-152240): the unified
//! `content lint` gathers every finding - reference/geometry checks
//! (`nova_scenario::lint`), combat-balance/fairness findings
//! (`nova_assets::balance`), and flight-rig input overlaps - into one
//! [`ContentReport`] that names, for each finding, the mod, the file it lives
//! in, the offending element, a short explanation and a suggested fix. A modder
//! debugging a multi-file bundle gets a document instead of a wall of stdout
//! lines. Markdown is the diffable baseline; HTML matches the perf report's
//! styling for a browsable view.
//!
//! The report is BUILT by `crate::lint_walk::{collect_tree, collect_target}`
//! (they own the content walk and its file provenance); this module owns the
//! data model and the two renderers.

use std::fmt::Write as _;

/// A finding's severity. Errors fail the lint (broken references, canonical
/// violations, spawned-dead balance, stale acks); warnings are authoring smells
/// that still load but misbehave (close-spawn reinforcements, input overlaps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warn,
}

impl Severity {
    fn tag(self) -> &'static str {
        match self {
            Severity::Error => "ERROR",
            Severity::Warn => "WARN",
        }
    }
}

/// Which checker raised a finding - the report groups by mod then severity, and
/// prints the category so a reader knows which tool to reach for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// `nova_scenario::lint`: unknown prototypes, dangling chains, unseated
    /// mounts, resource-ref and canonical-scheme violations, section geometry.
    Reference,
    /// `nova_assets::balance`: static combat fairness (spawned-dead / close-spawn).
    Balance,
    /// A content `input_mapping` section reusing a key the always-on flight rig
    /// already binds (`consume_input: false`), silently double-driving flight.
    InputOverlap,
}

impl Category {
    fn label(self) -> &'static str {
        match self {
            Category::Reference => "reference/geometry",
            Category::Balance => "balance",
            Category::InputOverlap => "input-overlap",
        }
    }
}

/// One finding, located: which mod, which file (relative to the mod dir), which
/// element, what is wrong, and how to fix it.
#[derive(Debug, Clone)]
pub struct Finding {
    pub bundle: String,
    /// The content file the finding is about, relative to the mod directory.
    /// `None` when the element could not be traced to a source file (the walk
    /// still names the mod and element).
    pub file: Option<String>,
    pub severity: Severity,
    pub category: Category,
    /// The addressable element: the scenario id, section id, or
    /// `scenario > section` the finding is about.
    pub element: String,
    /// The full explanation of what is wrong (the checker's own message).
    pub message: String,
    /// A short suggested fix, when one adds value beyond the message.
    pub suggestion: Option<String>,
}

/// A balance finding an author acknowledged in `balance_acks.ron` - reported as
/// context (not a problem) so a reader sees the exception and who owns it.
#[derive(Debug, Clone)]
pub struct AckedFinding {
    pub bundle: String,
    pub file: Option<String>,
    pub element: String,
    pub message: String,
    pub ack_task: String,
    pub ack_reason: String,
}

/// The whole report: the scope, the counts, every finding, and the
/// acknowledged balance exceptions.
#[derive(Debug, Clone, Default)]
pub struct ContentReport {
    /// The `--target` mod id, or `None` for a whole-tree lint.
    pub target: Option<String>,
    /// Every mod the report covers, in walk order.
    pub bundles: Vec<String>,
    /// How many combat scenarios the balance audit graded.
    pub scenarios_audited: usize,
    pub findings: Vec<Finding>,
    pub acked: Vec<AckedFinding>,
}

impl ContentReport {
    pub fn error_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    pub fn warn_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warn)
            .count()
    }

    /// A stable heading describing the report's scope.
    fn scope(&self) -> String {
        match &self.target {
            Some(target) => format!("mod `{target}`"),
            None => format!("{} mod(s)", self.bundles.len()),
        }
    }

    /// Findings grouped by bundle (in `self.bundles` order), each bundle's
    /// findings sorted Error-before-Warn then by file/element, so the report is
    /// deterministic and diffable.
    fn by_bundle(&self) -> Vec<(&str, Vec<&Finding>)> {
        let mut out = Vec::new();
        for bundle in &self.bundles {
            let mut group: Vec<&Finding> = self
                .findings
                .iter()
                .filter(|f| &f.bundle == bundle)
                .collect();
            group.sort_by(|a, b| {
                (a.severity == Severity::Warn)
                    .cmp(&(b.severity == Severity::Warn))
                    .then_with(|| a.file.cmp(&b.file))
                    .then_with(|| a.element.cmp(&b.element))
                    .then_with(|| a.message.cmp(&b.message))
            });
            if !group.is_empty() {
                out.push((bundle.as_str(), group));
            }
        }
        out
    }

    /// The Markdown report - the diffable, pasteable baseline.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        let _ = writeln!(md, "# Content lint report - {}", self.scope());
        let _ = writeln!(md);
        let _ = writeln!(
            md,
            "- {} error(s), {} warning(s) across {} finding(s)",
            self.error_count(),
            self.warn_count(),
            self.findings.len()
        );
        let _ = writeln!(
            md,
            "- {} combat scenario(s) balance-audited, {} acknowledged exception(s)",
            self.scenarios_audited,
            self.acked.len()
        );
        let _ = writeln!(md);

        if self.findings.is_empty() {
            let _ = writeln!(md, "No findings - the content is clean. ✅");
        } else {
            for (bundle, group) in self.by_bundle() {
                let _ = writeln!(md, "## {bundle}");
                let _ = writeln!(md);
                for f in group {
                    let file = f.file.as_deref().unwrap_or("(unknown file)");
                    let _ = writeln!(
                        md,
                        "- **{}** [{}] `{}` - `{}`",
                        f.severity.tag(),
                        f.category.label(),
                        file,
                        f.element
                    );
                    let _ = writeln!(md, "  - {}", f.message);
                    if let Some(fix) = &f.suggestion {
                        let _ = writeln!(md, "  - fix: {fix}");
                    }
                }
                let _ = writeln!(md);
            }
        }

        if !self.acked.is_empty() {
            let _ = writeln!(md, "## Acknowledged balance exceptions");
            let _ = writeln!(md);
            for a in &self.acked {
                let file = a.file.as_deref().unwrap_or("(unknown file)");
                let _ = writeln!(
                    md,
                    "- [{}] `{}` `{}` - {} (acked by {}: {})",
                    a.bundle, file, a.element, a.message, a.ack_task, a.ack_reason
                );
            }
            let _ = writeln!(md);
        }

        md
    }

    /// The HTML report - the same content as [`to_markdown`], styled to match
    /// the perf report (`nova_probe`) so the two dev-tool reports look alike.
    pub fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
        let _ = write!(
            html,
            "<title>Content lint - {}</title>\n",
            escape(&self.scope())
        );
        html.push_str(STYLE);
        html.push_str("</head>\n<body>\n");

        let _ = write!(
            html,
            "<h1>Content lint report</h1>\n<p class=\"meta\">{}</p>\n",
            escape(&self.scope())
        );
        let banner = if self.error_count() > 0 {
            "fail"
        } else if self.warn_count() > 0 {
            "warn"
        } else {
            "ok"
        };
        let _ = write!(
            html,
            "<div class=\"banner {banner}\">{} error(s), {} warning(s) across {} finding(s)\
             <span class=\"confirm\">{} combat scenario(s) balance-audited, {} acknowledged \
             exception(s)</span></div>\n",
            self.error_count(),
            self.warn_count(),
            self.findings.len(),
            self.scenarios_audited,
            self.acked.len(),
        );

        if self.findings.is_empty() {
            html.push_str("<p class=\"oknok\">No findings - the content is clean.</p>\n");
        } else {
            for (bundle, group) in self.by_bundle() {
                let _ = write!(html, "<h2>{}</h2>\n<table>\n", escape(bundle));
                html.push_str(
                    "<tr><th>severity</th><th>category</th><th>file</th><th>element</th>\
                     <th>finding</th></tr>\n",
                );
                for f in group {
                    let cls = match f.severity {
                        Severity::Error => "status-fail",
                        Severity::Warn => "status-warn",
                    };
                    let file = f.file.as_deref().unwrap_or("(unknown file)");
                    let mut finding_cell = escape(&f.message);
                    if let Some(fix) = &f.suggestion {
                        let _ = write!(
                            finding_cell,
                            "<br><span class=\"note\">fix: {}</span>",
                            escape(fix)
                        );
                    }
                    let _ = write!(
                        html,
                        "<tr><td class=\"{cls}\">{}</td><td>{}</td><td><code>{}</code></td>\
                         <td><code>{}</code></td><td>{}</td></tr>\n",
                        f.severity.tag(),
                        f.category.label(),
                        escape(file),
                        escape(&f.element),
                        finding_cell,
                    );
                }
                html.push_str("</table>\n");
            }
        }

        if !self.acked.is_empty() {
            html.push_str("<h2>Acknowledged balance exceptions</h2>\n<table>\n");
            html.push_str(
                "<tr><th>mod</th><th>file</th><th>element</th><th>finding</th><th>acked by</th></tr>\n",
            );
            for a in &self.acked {
                let file = a.file.as_deref().unwrap_or("(unknown file)");
                let _ = write!(
                    html,
                    "<tr><td>{}</td><td><code>{}</code></td><td><code>{}</code></td><td>{}</td>\
                     <td class=\"note\">{}: {}</td></tr>\n",
                    escape(&a.bundle),
                    escape(file),
                    escape(&a.element),
                    escape(&a.message),
                    escape(&a.ack_task),
                    escape(&a.ack_reason),
                );
            }
            html.push_str("</table>\n");
        }

        html.push_str("</body>\n</html>\n");
        html
    }
}

/// Neutralize HTML metacharacters in text that goes into the report body.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The report shell styling - a trimmed copy of the perf report's palette
/// (`nova_probe::report::STYLE`) so the two dev-tool reports read as a set. Kept
/// local rather than shared to avoid a nova_assets -> nova_probe dependency.
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
table { border-collapse: collapse; width: 100%; margin-top: 0.5rem; }
th, td { padding: 0.35rem 0.55rem; border-bottom: 1px solid #e2e2e2; text-align: left; vertical-align: top; }
th { font-weight: 600; color: #333; border-bottom: 2px solid #ccc; }
.oknok { font-weight: 700; margin-top: 0.8rem; }
.banner { padding: 0.8rem 1rem; border-radius: 6px; font-weight: 600; margin: 1rem 0; }
.banner.ok { background: #e3f4e6; color: #0b6623; }
.banner.warn { background: #fff3d6; color: #7a5b00; }
.banner.fail { background: #fbe3e4; color: #8f1013; }
.banner .confirm { display: block; font-weight: 400; font-size: 0.85rem; margin-top: 0.3rem; }
td.status-warn { color: #b8860b; font-weight: 600; }
td.status-fail { color: #b00020; font-weight: 600; }
@media (prefers-color-scheme: dark) {
  body { color: #e6e6e6; background: #16181c; }
  h2 { border-color: #333; }
  .meta { color: #aaa; } .note { color: #999; }
  code { background: #2a2d33; }
  th { color: #ddd; border-color: #444; } th, td { border-color: #2a2d33; }
}
</style>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(sev: Severity, cat: Category, file: &str, element: &str, msg: &str) -> Finding {
        Finding {
            bundle: "the-ledger".to_string(),
            file: Some(file.to_string()),
            severity: sev,
            category: cat,
            element: element.to_string(),
            message: msg.to_string(),
            suggestion: Some("do the thing".to_string()),
        }
    }

    #[test]
    fn clean_report_is_not_empty() {
        let report = ContentReport {
            target: Some("example".to_string()),
            bundles: vec!["example".to_string()],
            scenarios_audited: 3,
            ..Default::default()
        };
        let md = report.to_markdown();
        assert!(md.contains("clean"), "clean report says so: {md}");
        assert!(md.contains("mod `example`"));
        // A clean report is a document, not an empty file (DoD).
        assert!(md.lines().count() > 3);
    }

    #[test]
    fn markdown_pins_severity_file_element_and_fix() {
        let report = ContentReport {
            target: None,
            bundles: vec!["the-ledger".to_string()],
            scenarios_audited: 1,
            findings: vec![
                finding(
                    Severity::Warn,
                    Category::InputOverlap,
                    "ledger_ch1.content.ron",
                    "chapter_one > guns",
                    "binds Space",
                ),
                finding(
                    Severity::Error,
                    Category::Reference,
                    "ledger_ch2.content.ron",
                    "chapter_two",
                    "unknown prototype 'imaginary_hull'",
                ),
            ],
            acked: vec![],
        };
        let md = report.to_markdown();
        // Error is listed before Warn within the bundle.
        let err_at = md.find("ERROR").expect("error present");
        let warn_at = md.find("WARN").expect("warn present");
        assert!(err_at < warn_at, "errors sort before warnings:\n{md}");
        assert!(md.contains("ledger_ch2.content.ron"));
        assert!(md.contains("chapter_two"));
        assert!(md.contains("imaginary_hull"));
        assert!(md.contains("fix: do the thing"));
    }

    #[test]
    fn html_escapes_and_banners() {
        let mut report = ContentReport {
            bundles: vec!["the-ledger".to_string()],
            ..Default::default()
        };
        report.findings.push(finding(
            Severity::Error,
            Category::Reference,
            "a.ron",
            "s",
            "bad <tag> & ref",
        ));
        let html = report.to_html();
        assert!(html.contains("banner fail"), "an error banners fail");
        assert!(
            html.contains("bad &lt;tag&gt; &amp; ref"),
            "escaped: {html}"
        );
        assert!(!html.contains("bad <tag>"));
    }
}
