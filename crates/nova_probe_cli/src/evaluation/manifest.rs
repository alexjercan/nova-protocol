//! The run manifest (`probe-run.json`): what was executed, with what
//! outcome, producing which artifacts. The report treats it as the run
//! identity, and `probe report` refuses dirs without one.

/// Glob-import surface for the run manifest.
pub mod prelude {
    pub use super::{run_identity, PassRecord, RunManifest};
}

use nova_probe::prelude::*;

/// The manifest `probe run` writes (`probe-run.json`): what was executed,
/// with what outcome, producing which artifacts. The report treats it as
/// the run's identity, `process_exit` reads its pass records, its `armed`
/// flags are probe's half of the coverage handshake (the example's half is
/// its own `probe-contract.json`), and `probe report` refuses dirs without
/// one.
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
    /// Whether probe armed the timeline capture surface.
    pub armed_timeline: bool,
    /// Whether the invariants capture surface was armed.
    pub armed_invariants: bool,
    /// Whether the frame-time capture surface was armed.
    pub armed_fps: bool,
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
    (resolve_git_sha(), resolve_host())
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
            "passes": self.passes.iter().map(|p| serde_json::json!({
                "name": p.name, "success": p.success, "timed_out": p.timed_out,
            })).collect::<Vec<_>>(),
        })
    }

    /// Parse `probe-run.json`. Every field `to_json` writes is required:
    /// probe is the only writer, so an incomplete manifest is corruption,
    /// and a corrupt manifest must not read as "no manifest".
    pub fn from_json(contents: &str) -> Result<Self, String> {
        let v: serde_json::Value =
            serde_json::from_str(contents).map_err(|e| format!("probe-run.json: {e}"))?;
        let s = |k: &str| -> Result<String, String> {
            v.get(k)
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .ok_or_else(|| format!("probe-run.json: missing {k}"))
        };
        let u = |k: &str| -> Result<u64, String> {
            v.get(k)
                .and_then(|x| x.as_u64())
                .ok_or_else(|| format!("probe-run.json: missing {k}"))
        };
        let armed = |k: &str| -> Result<bool, String> {
            v.get("armed")
                .and_then(|a| a.get(k))
                .and_then(|x| x.as_bool())
                .ok_or_else(|| format!("probe-run.json: missing armed.{k}"))
        };
        let flag = |p: &serde_json::Value, k: &str| -> Result<bool, String> {
            p.get(k)
                .and_then(|x| x.as_bool())
                .ok_or_else(|| format!("probe-run.json: pass missing {k}"))
        };
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
                    success: flag(p, "success")?,
                    timed_out: flag(p, "timed_out")?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            example: s("example")?,
            started_unix: u("started_unix")?,
            git_sha: s("git_sha")?,
            full_git_sha: s("full_git_sha")?,
            host: s("host")?,
            armed_timeline: armed("timeline")?,
            armed_invariants: armed("invariants")?,
            armed_fps: armed("fps")?,
            passes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_round_trips_through_json() {
        let manifest = RunManifest {
            example: "playable".into(),
            started_unix: 1789000123,
            git_sha: "abc123".into(),
            full_git_sha: "abc123def".into(),
            host: "devbox".into(),
            armed_timeline: true,
            armed_invariants: true,
            armed_fps: true,
            passes: vec![
                PassRecord {
                    name: "clean".into(),
                    success: false,
                    timed_out: true,
                },
                PassRecord {
                    name: "profiled".into(),
                    success: true,
                    timed_out: false,
                },
            ],
        };
        let parsed = RunManifest::from_json(&manifest.to_json().to_string()).expect("round-trips");
        assert_eq!(parsed, manifest);
    }

    /// Probe writes every field, so an absent one is corruption and the
    /// error has to name it. `armed.fps` and `timed_out` are the sharp
    /// cases: their old defaults matched the fixture's own values, so a
    /// silently completed manifest was indistinguishable from a real one.
    #[test]
    fn an_incomplete_manifest_fails_naming_the_missing_field() {
        let full = || crate::evaluation::fixtures::manifest_ok().to_json();

        for key in [
            "example",
            "started_unix",
            "git_sha",
            "full_git_sha",
            "host",
            "passes",
        ] {
            let mut json = full();
            json.as_object_mut().unwrap().remove(key);
            assert_eq!(
                RunManifest::from_json(&json.to_string()).unwrap_err(),
                format!("probe-run.json: missing {key}"),
            );
        }

        for key in ["timeline", "invariants", "fps"] {
            let mut json = full();
            json["armed"].as_object_mut().unwrap().remove(key);
            assert_eq!(
                RunManifest::from_json(&json.to_string()).unwrap_err(),
                format!("probe-run.json: missing armed.{key}"),
            );
        }

        for key in ["name", "success", "timed_out"] {
            let mut json = full();
            json["passes"][0].as_object_mut().unwrap().remove(key);
            assert_eq!(
                RunManifest::from_json(&json.to_string()).unwrap_err(),
                format!("probe-run.json: pass missing {key}"),
            );
        }
    }
}
