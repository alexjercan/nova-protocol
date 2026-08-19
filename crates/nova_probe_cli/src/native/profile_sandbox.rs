//! The PROFILE SANDBOX for probe's native child runs.
//!
//! A probe run measures a COMMIT, so it must not depend on the operator's
//! desktop profile. Left alone, a spawned example inherits probe's environment
//! and resolves its persistent state through the `dirs` crate:
//!
//! - `dirs::data_dir()/nova-protocol` - the downloaded-mod cache and its
//!   `installed.mods.ron` index (`nova_assets::mod_cache`), also reachable via
//!   the crate's own `NOVA_MOD_CACHE_ROOT` override;
//! - `dirs::config_dir()/nova-protocol` - `enabled_mods.ron`
//!   (`nova_assets::mod_prefs`) and `settings.ron`
//!   (`nova_menu::settings_store`).
//!
//! A mod cached in a structure an older commit cannot parse, or a saved
//! enabled-mod set, then fails (or silently changes) a run for reasons that
//! have nothing to do with the commit under measurement.
//!
//! So every native child run is pointed at an EMPTY, probe-owned profile
//! under its own run directory:
//!
//! ```text
//! probe-runs/<commit>/<example>/profile/mods     NOVA_MOD_CACHE_ROOT
//! probe-runs/<commit>/<example>/profile/data     XDG_DATA_HOME
//! probe-runs/<commit>/<example>/profile/config   XDG_CONFIG_HOME
//! ```
//!
//! Per run by construction (the sandbox is a child of the run dir), and
//! visible next to the report rather than hidden in a temp dir. Shipped
//! content is untouched: `assets/` and `assets/mods.catalog.ron` load exactly
//! as they do for a player - only the USER's saved state is swapped out.
//!
//! ## Overriding it
//!
//! Each variable is PRESERVED per-variable when the operator already exports
//! it: probe does not push its own value for a variable that is already set,
//! and [`prepare`] announces the ones it left alone. That keeps the
//! deliberate "probe my real installed mods" run possible:
//!
//! ```text
//! NOVA_MOD_CACHE_ROOT=~/.local/share/nova-protocol cargo run --features debug probe run system_player_path
//! ```
//!
//! ## Deliberate non-goals
//!
//! - `XDG_CACHE_HOME` is NOT redirected: the wgpu/mesa shader cache lives
//!   there, and throwing it away every run would make FPS numbers
//!   incomparable. It holds no Nova profile state.
//! - `HOME` is NOT redirected - too blunt, and the XDG pair already covers
//!   what `dirs` resolves on Linux.
//! - Cross-platform reach: `XDG_DATA_HOME`/`XDG_CONFIG_HOME` are how `dirs`
//!   resolves on LINUX. macOS `dirs` uses `~/Library/...` and ignores XDG, so
//!   there the sandbox rests on `NOVA_MOD_CACHE_ROOT` alone. Probe already
//!   requires Xvfb, so Linux is the supported host.
//! - The web pass (`--platform web`) is out of scope: it runs in a browser
//!   profile and reads no Nova profile state.

/// Glob-import surface for the child run's profile sandbox.
pub mod prelude {
    pub use super::{env, env_with, inherited, prepare, root, SANDBOXED_VARS};
}

use std::path::{Path, PathBuf};

/// The environment variables the sandbox redirects, in the order [`env()`]
/// emits them.
pub const SANDBOXED_VARS: [&str; 3] = ["NOVA_MOD_CACHE_ROOT", "XDG_DATA_HOME", "XDG_CONFIG_HOME"];

/// The sandbox root for a run directory: `<run_dir>/profile`.
pub fn root(run_dir: &Path) -> PathBuf {
    run_dir.join("profile")
}

/// Every sandboxed variable and the path it points at, whether or not the
/// operator has overridden it.
fn all_paths(run_dir: &Path) -> Vec<(&'static str, PathBuf)> {
    let root = root(run_dir);
    vec![
        ("NOVA_MOD_CACHE_ROOT", root.join("mods")),
        ("XDG_DATA_HOME", root.join("data")),
        ("XDG_CONFIG_HOME", root.join("config")),
    ]
}

/// The sandbox environment for a child run under `run_dir`, minus any
/// variable the operator already exports (see the module doc).
pub fn env(run_dir: &Path) -> Vec<(String, String)> {
    env_with(run_dir, |var| std::env::var_os(var).is_some())
}

/// [`env()`] with the "is this variable already set?" lookup injected, so the
/// override policy is testable without mutating the process environment.
pub fn env_with(run_dir: &Path, is_inherited: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    all_paths(run_dir)
        .into_iter()
        .filter(|(var, _)| !is_inherited(var))
        .map(|(var, path)| (var.to_string(), path.display().to_string()))
        .collect()
}

/// The sandboxed variables the operator has already exported - the ones
/// [`env()`] leaves alone.
pub fn inherited() -> Vec<&'static str> {
    SANDBOXED_VARS
        .into_iter()
        .filter(|var| std::env::var_os(var).is_some())
        .collect()
}

/// Clear and recreate the sandbox directories for `run_dir`, and announce any
/// variable the operator's environment owns.
///
/// The wipe matters: re-running probe into the SAME run directory must start
/// from an empty profile too, or the second run inherits the first one's
/// state. The whole `<run_dir>/profile` tree is probe-owned, so it is safe to
/// remove wholesale. Best-effort: a directory probe cannot create is not worth
/// failing a run over (the child creates what it writes, and the point of the
/// sandbox is what it does NOT read).
pub fn prepare(run_dir: &Path) {
    let root = root(run_dir);
    if root.exists() {
        if let Err(e) = std::fs::remove_dir_all(&root) {
            eprintln!(
                "probe: profile sandbox: could not clear {}: {e}",
                root.display()
            );
        }
    }
    for (_, path) in all_paths(run_dir) {
        if let Err(e) = std::fs::create_dir_all(&path) {
            eprintln!(
                "probe: profile sandbox: could not create {}: {e}",
                path.display()
            );
        }
    }
    for var in inherited() {
        eprintln!("probe: profile sandbox: {var} inherited from the environment");
    }
    eprintln!(
        "probe: profile sandbox: {} (mod cache, data, config)",
        root.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(env: &[(String, String)], key: &str) -> Option<String> {
        env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn profile_sandbox_points_every_store_under_the_run_dir() {
        let env = env_with(Path::new("/repo/probe-runs/abc/playable"), |_| false);
        assert_eq!(
            get(&env, "NOVA_MOD_CACHE_ROOT").as_deref(),
            Some("/repo/probe-runs/abc/playable/profile/mods")
        );
        assert_eq!(
            get(&env, "XDG_DATA_HOME").as_deref(),
            Some("/repo/probe-runs/abc/playable/profile/data"),
            "the dirs crate's data_dir - mod cache + installed.mods.ron"
        );
        assert_eq!(
            get(&env, "XDG_CONFIG_HOME").as_deref(),
            Some("/repo/probe-runs/abc/playable/profile/config"),
            "the dirs crate's config_dir - enabled_mods.ron + settings.ron"
        );
        assert_eq!(env.len(), SANDBOXED_VARS.len(), "no other variable moves");
    }

    #[test]
    fn profile_sandbox_is_per_run() {
        let a = env_with(Path::new("/repo/probe-runs/abc/playable"), |_| false);
        let b = env_with(Path::new("/repo/probe-runs/abc/scenario"), |_| false);
        assert_ne!(
            get(&a, "NOVA_MOD_CACHE_ROOT"),
            get(&b, "NOVA_MOD_CACHE_ROOT"),
            "two runs never share downloaded-mod state"
        );
    }

    #[test]
    fn profile_sandbox_preserves_operator_overrides() {
        let run_dir = Path::new("/repo/probe-runs/abc/playable");
        // One override: that variable is left to the operator, the rest of
        // the sandbox still applies.
        let env = env_with(run_dir, |var| var == "NOVA_MOD_CACHE_ROOT");
        assert_eq!(
            get(&env, "NOVA_MOD_CACHE_ROOT"),
            None,
            "probe must not overwrite a deliberate mod-cache override"
        );
        assert!(get(&env, "XDG_DATA_HOME").is_some());
        assert!(get(&env, "XDG_CONFIG_HOME").is_some());

        // All three: probe pushes nothing, the run is fully the operator's.
        assert!(env_with(run_dir, |_| true).is_empty());
    }
}
