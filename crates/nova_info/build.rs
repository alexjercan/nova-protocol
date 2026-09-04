use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap();
    let debug_enabled = env::var("CARGO_FEATURE_DEBUG").is_ok();

    let version = if debug_enabled {
        track_git_head();
        format!("{pkg_version}+{}", short_head())
    } else {
        pkg_version
    };

    println!("cargo:rustc-env=APP_VERSION={}", version);
}

/// The short commit hash, or `unknown` when it cannot be read.
///
/// A source tarball has no `.git` (and a build container may have no git binary
/// at all). The revision is a debug convenience, not a build input, so fall
/// back rather than failing the whole build.
fn short_head() -> String {
    git(["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string())
}

/// Names the files that change when HEAD moves, so a new commit rebuilds the
/// hash into the binary.
///
/// Emitting any `rerun-if-changed` replaces cargo's default rule - rerun when a
/// packaged file changes - and no packaged file moves per commit, so without
/// this the status bar would show whichever revision happened to be checked out
/// on the first build forever.
fn track_git_head() {
    let mut paths = vec![git_path("HEAD"), git_path("packed-refs")];
    if let Some(head_ref) = git(["symbolic-ref", "--quiet", "HEAD"]) {
        paths.push(git_path(&head_ref));
    }

    for path in paths.into_iter().flatten() {
        // A path git names but that does not exist yet (an unpacked ref, a
        // missing packed-refs) reads as dirty to cargo, which would rerun this
        // script on every build.
        if path.exists() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

/// Resolves a path inside the git directory, honouring worktrees and
/// `GIT_DIR`. `None` when this is not a checkout.
fn git_path(relative: &str) -> Option<PathBuf> {
    git(["rev-parse", "--git-path", relative]).map(PathBuf::from)
}

/// Runs git in the crate directory and returns trimmed stdout, or `None` when
/// git is absent, fails, or says nothing.
fn git<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}
