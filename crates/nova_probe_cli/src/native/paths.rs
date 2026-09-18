//! Repo, output and baseline paths: where a run writes and which previous
//! commit dir it compares against.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// The repo root, derived from this crate's manifest dir at compile time
/// (crates/nova_probe -> ../..). A dev tool run via cargo from the repo.
pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// Resolve an example's baseline against a baseline root: the run dir
/// `<root>/<example>` when that holds a `frametime.csv`. Missing examples
/// skip the comparison rather than erroring.
pub(crate) fn baseline_for(root: &Path, example: &str) -> Option<PathBuf> {
    let dir = root.join(example);
    dir.join("frametime.csv").is_file().then_some(dir)
}

pub(crate) fn default_output_base(root: &Path, explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(|| root.join("probe-runs"))
}

pub(crate) fn default_output_root(
    root: &Path,
    explicit: Option<PathBuf>,
    short_sha: &str,
) -> PathBuf {
    default_output_base(root, explicit).join(short_sha)
}

fn is_hash_dir_name(name: &str) -> bool {
    (7..=40).contains(&name.len()) && name.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub(crate) fn discover_baseline_root(
    base: &Path,
    current_sha: &str,
    history: &[String],
) -> Option<PathBuf> {
    for sha in history {
        if sha == current_sha || !is_hash_dir_name(sha) {
            continue;
        }
        let dir = base.join(sha);
        if dir.is_dir() {
            return Some(dir);
        }
    }
    None
}

pub(crate) fn git_history_short(root: &Path) -> Vec<String> {
    Command::new("git")
        .current_dir(root)
        .args(["rev-list", "--abbrev-commit", "--max-count=256", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| {
            stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn resolve_full_git_sha(root: &Path) -> String {
    Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::fixtures::s;

    #[test]
    fn baseline_for_resolves_present_and_skips_missing() {
        // A baseline root resolves per example: the example's dir when it
        // holds a frametime.csv, else None (skip, not error).
        let base = std::env::temp_dir().join(format!("nova_probe_gb_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("playable")).unwrap();
        std::fs::write(base.join("playable").join("frametime.csv"), "x").unwrap();
        // dir exists but no csv - still a miss.
        std::fs::create_dir_all(base.join("scenario")).unwrap();
        // A csv at the root itself is no example's baseline: a root IS a
        // commit dir, and every run under it lives in its own example dir.
        std::fs::write(base.join("frametime.csv"), "x").unwrap();

        assert_eq!(baseline_for(&base, "playable"), Some(base.join("playable")));
        assert_eq!(baseline_for(&base, "scenario"), None);
        assert_eq!(baseline_for(&base, "missing"), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn default_output_root_is_keyed_by_short_commit() {
        let root = Path::new("/repo");
        assert_eq!(
            default_output_root(root, None, "61675034"),
            PathBuf::from("/repo/probe-runs/61675034")
        );
        assert_eq!(
            default_output_root(root, Some(PathBuf::from("custom/out")), "61675034"),
            PathBuf::from("custom/out/61675034"),
            "explicit --out is the storage base, not the exact run dir"
        );
    }

    #[test]
    fn baseline_discovery_picks_nearest_ancestor_and_ignores_compat_dirs() {
        let base = std::env::temp_dir().join(format!("nova_probe_baseline_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("before")).unwrap();
        std::fs::create_dir_all(base.join("playable")).unwrap();
        std::fs::create_dir_all(base.join("aaaa1111")).unwrap();
        std::fs::create_dir_all(base.join("bbbb2222")).unwrap();
        std::fs::create_dir_all(base.join("cccc3333")).unwrap();

        let history = s(&["cccc3333", "bbbb2222", "aaaa1111"]);
        assert_eq!(
            discover_baseline_root(&base, "dddd4444", &history),
            Some(base.join("cccc3333"))
        );
        let history = s(&["bbbb2222", "aaaa1111"]);
        assert_eq!(
            discover_baseline_root(&base, "dddd4444", &history),
            Some(base.join("bbbb2222"))
        );
        let history = s(&["missing", "playable", "before"]);
        assert_eq!(discover_baseline_root(&base, "dddd4444", &history), None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_old_run_root_without_a_commit_dir_is_not_a_baseline() {
        // Explicit --baseline and auto-discovery share one resolver, so a root
        // that holds <example>/frametime.csv but no commit dir is a baseline
        // for neither. Only a hash dir named by history resolves.
        let base =
            std::env::temp_dir().join(format!("nova_probe_baseline_root_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("playable")).unwrap();
        std::fs::write(base.join("playable").join("frametime.csv"), "x").unwrap();

        let history = s(&["61675034"]);
        assert_eq!(discover_baseline_root(&base, "61675034", &history), None);

        std::fs::create_dir_all(base.join("aaaa1111").join("playable")).unwrap();
        let history = s(&["61675034", "aaaa1111"]);
        assert_eq!(
            discover_baseline_root(&base, "61675034", &history),
            Some(base.join("aaaa1111"))
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
