use std::{env, process::Command};

fn main() {
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap();
    let debug_enabled = env::var("CARGO_FEATURE_DEBUG").is_ok();

    let version = if debug_enabled {
        // A source tarball has no .git (and a build container may have no git
        // binary at all). The revision is a debug convenience, not a build
        // input, so fall back rather than failing the whole build.
        let hash = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|hash| hash.trim().to_string())
            .filter(|hash| !hash.is_empty())
            .unwrap_or_else(|| "unknown".to_string());
        format!("{pkg_version}+{hash}")
    } else {
        pkg_version
    };

    println!("cargo:rustc-env=APP_VERSION={}", version);
}
