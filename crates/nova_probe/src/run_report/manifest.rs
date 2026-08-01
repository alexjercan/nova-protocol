//! The run manifest (`probe-run.json`): what was executed, with what
//! outcome, producing which artifacts. The report treats it as the run
//! identity, and `probe report` refuses dirs without one.

/// The manifest `probe run` writes (`probe-run.json`): what was executed,
/// with what outcome, producing which artifacts. The report treats it as
/// the run's identity - `process_exit` reads it, skip details use its
/// `armed` flags to distinguish "not armed" from "armed but the example is
/// not wired", and `probe report` refuses dirs without one.
#[derive(Debug, Clone, PartialEq)]
pub struct RunManifest {
    /// The example that was run.
    pub example: String,
    /// Unix seconds when the run started.
    pub started_unix: u64,
    /// Short git SHA, same resolver as the capture metadata.
    pub git_sha: String,
    /// Full git SHA for mapping commit-keyed probe-runs folders back to the
    /// exact code revision.
    pub full_git_sha: String,
    /// The host the run executed on.
    pub host: String,
    /// Which capture surfaces probe armed (timeline/invariants always; fps
    /// only with --fps).
    pub armed_timeline: bool,
    /// Whether the invariants capture surface was armed.
    pub armed_invariants: bool,
    /// Whether the fps capture surface was armed (only with `--fps`).
    pub armed_fps: bool,
    /// Set when `--fps` was requested but this example is configured
    /// fps-exempt (`[package.metadata.nova_probe] fps_exempt`): the reason,
    /// rendered in place of the frame-time section so a missing capture reads
    /// as "not a perf target" instead of a failure. `None` for a normal run.
    pub fps_exempt: Option<String>,
    /// Per-pass outcomes, in execution order.
    pub passes: Vec<PassRecord>,
}

/// One executed pass and how it ended.
#[derive(Debug, Clone, PartialEq)]
pub struct PassRecord {
    /// `clean`, `profiled`, `samply`.
    pub name: String,
    /// The child exited successfully (false also when timed out).
    pub success: bool,
    /// The child was killed by the runner's timeout.
    pub timed_out: bool,
}

/// The run's identity pair (short git SHA, host tag) via the same
/// resolvers the capture metadata uses - for the probe bin's manifest.
pub fn run_identity() -> (String, String) {
    (
        crate::capture::resolve_git_sha(),
        crate::capture::resolve_host(),
    )
}

impl RunManifest {
    /// Serialize for `probe-run.json`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "example": self.example,
            "started_unix": self.started_unix,
            "git_sha": self.git_sha,
            "full_git_sha": self.full_git_sha,
            "host": self.host,
            "armed": {
                "timeline": self.armed_timeline,
                "invariants": self.armed_invariants,
                "fps": self.armed_fps,
            },
            "fps_exempt": self.fps_exempt,
            "passes": self.passes.iter().map(|p| serde_json::json!({
                "name": p.name, "success": p.success, "timed_out": p.timed_out,
            })).collect::<Vec<_>>(),
        })
    }

    /// Parse `probe-run.json`. Loud on malformed content - a corrupt
    /// manifest must not read as "no manifest".
    pub fn from_json(contents: &str) -> Result<Self, String> {
        let v: serde_json::Value =
            serde_json::from_str(contents).map_err(|e| format!("probe-run.json: {e}"))?;
        let s = |k: &str| -> Result<String, String> {
            v.get(k)
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("probe-run.json: missing {k}"))
        };
        let armed = |k: &str| v["armed"].get(k).and_then(|x| x.as_bool()).unwrap_or(false);
        let passes = v
            .get("passes")
            .and_then(|p| p.as_array())
            .ok_or("probe-run.json: missing passes")?
            .iter()
            .map(|p| {
                Ok(PassRecord {
                    name: p
                        .get("name")
                        .and_then(|x| x.as_str())
                        .ok_or("probe-run.json: pass missing name")?
                        .to_string(),
                    success: p.get("success").and_then(|x| x.as_bool()).unwrap_or(false),
                    timed_out: p
                        .get("timed_out")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            example: s("example")?,
            started_unix: v.get("started_unix").and_then(|x| x.as_u64()).unwrap_or(0),
            git_sha: s("git_sha")?,
            full_git_sha: v
                .get("full_git_sha")
                .and_then(|x| x.as_str())
                .unwrap_or_else(|| v.get("git_sha").and_then(|x| x.as_str()).unwrap_or(""))
                .to_string(),
            host: s("host")?,
            armed_timeline: armed("timeline"),
            armed_invariants: armed("invariants"),
            armed_fps: armed("fps"),
            fps_exempt: v
                .get("fps_exempt")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            passes,
        })
    }
}
