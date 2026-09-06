//! Where a run writes: the repo root, the commit it measures, and the run
//! directory layout `bench-runs/<sha>/<scenario>/<agent>-<n>/` - the same
//! shape as `probe-runs`, so release-over-release comparison is a directory
//! diff.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// The repo root, derived from this crate's manifest dir at compile time
/// (crates/nova_bench -> ../..). A dev tool run via cargo from the repo.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// The short sha of `HEAD`, or `unknown` outside a checkout.
pub fn short_sha(root: &Path) -> String {
    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// The first free `<base>/<label>-<n>`, n from 1. Pure over the filesystem
/// probe passed in, so the numbering is testable without a disk.
pub fn numbered_dir(base: &Path, label: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    (1..)
        .map(|n| base.join(format!("{label}-{n}")))
        .find(|candidate| !exists(candidate))
        .expect("an unbounded range always has a free slot")
}

/// The run directory for one play: the explicit `--out`, or
/// `<root>/bench-runs/<sha>/<scenario>/<agent>-<n>`.
pub fn run_dir(root: &Path, explicit: Option<&Path>, scenario: &str, agent: &str) -> PathBuf {
    if let Some(dir) = explicit {
        return dir.to_path_buf();
    }
    let base = root.join("bench-runs").join(short_sha(root)).join(scenario);
    numbered_dir(&base, agent, |candidate| candidate.exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_run_dir_takes_the_first_free_number() {
        let base = Path::new("/runs/tutorial");
        let taken = [
            PathBuf::from("/runs/tutorial/pi-1"),
            PathBuf::from("/runs/tutorial/pi-2"),
        ];
        let dir = numbered_dir(base, "pi", |candidate| {
            taken.contains(&candidate.to_path_buf())
        });
        assert_eq!(dir, PathBuf::from("/runs/tutorial/pi-3"));
        assert_eq!(
            numbered_dir(base, "baseline", |_| false),
            PathBuf::from("/runs/tutorial/baseline-1")
        );
    }

    #[test]
    fn an_explicit_out_wins() {
        let dir = run_dir(Path::new("/repo"), Some(Path::new("/tmp/here")), "x", "y");
        assert_eq!(dir, PathBuf::from("/tmp/here"));
    }
}
