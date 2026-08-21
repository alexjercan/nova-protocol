//! Cross-process proof for the probe profile sandbox.
//!
//! The bug is about what a SPAWNED child reads, so the rig spawns one: this
//! test binary re-executes itself with a marker env var, and the child leg
//! (`resolver_child`) calls the very resolvers the game uses -
//! `nova_assets::mod_cache::read_index`, `nova_assets::mod_prefs`, and the
//! `dirs` config dir `nova_menu::settings_store` resolves through - then
//! prints what they found.
//!
//! Every test therefore carries its own fail-first proof: the same child, same
//! poisoned profile, run WITHOUT the sandbox env reads the poison. That leg is
//! the reproduction, and it stays in the rig so the fix cannot silently rot
//! into a test that would pass either way.
//!
//! The poisoned profile stands in for the operator's real one via `HOME` (the
//! desktop default: `dirs` falls back to `$HOME/.local/share` and
//! `$HOME/.config` when XDG is unset), so the rig never touches the real
//! `~/.local/share/nova-protocol`.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use nova_probe_cli::native::profile_sandbox;

/// Set on the child; absent in the parent leg, which is what keeps
/// `resolver_child` a no-op during a normal `cargo test` run.
const CHILD_MARKER: &str = "NOVA_PROBE_SANDBOX_RESOLVER_CHILD";

/// The mod id written into the poisoned profile. A run that prints it read
/// state it had no business reading.
const POISON_ID: &str = "poison-mod-from-the-operator-profile";

/// The child leg: report what the real resolvers resolve to. A no-op unless
/// [`CHILD_MARKER`] is set, so it costs a normal test run nothing.
#[test]
fn resolver_child() {
    if std::env::var_os(CHILD_MARKER).is_none() {
        return;
    }
    // The downloaded-mod index: `<data_root>/installed.mods.ron`, where
    // data_root is $NOVA_MODDING_CACHE_ROOT or dirs::data_dir()/nova-protocol.
    println!("INDEX={:?}", nova_assets::mod_cache::read_index());
    // The enabled-mod set: dirs::config_dir()/nova-protocol/enabled_mods.ron.
    println!("ENABLED={:?}", nova_assets::mod_prefs::load_enabled_ids());
    // The dirs the stores are built on. settings.ron (nova_menu's private
    // settings_store) resolves through this same config dir, so pinning it
    // pins the settings store too.
    println!("DATA_DIR={}", show(dirs::data_dir()));
    println!("CONFIG_DIR={}", show(dirs::config_dir()));
}

fn show(path: Option<PathBuf>) -> String {
    path.map(|p| p.display().to_string())
        .unwrap_or_else(|| "<none>".into())
}

/// Re-execute this test binary's child leg with `env` layered on top of a
/// cleared profile environment, and return its stdout.
fn run_child(env: &[(String, String)]) -> String {
    let exe = std::env::current_exe().expect("test binary path");
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", "resolver_child", "--nocapture"])
        .env(CHILD_MARKER, "1");
    // Whatever THIS process inherited must not decide the outcome: each leg
    // states its own profile environment in full.
    for var in profile_sandbox::SANDBOXED_VARS {
        cmd.env_remove(var);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    let output = cmd.output().expect("spawn the resolver child");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "resolver child failed: {}\n{stdout}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout
}

/// A stand-in for the operator's profile: a `HOME` holding a downloaded-mod
/// index and an enabled-mods preference the run must not see.
fn poisoned_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("temp home");
    let data = home.path().join(".local/share/nova-protocol");
    let config = home.path().join(".config/nova-protocol");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        data.join("installed.mods.ron"),
        format!("[(id:\"{POISON_ID}\",version:\"9.9.9\",bundle:\"poison.bundle.ron\")]"),
    )
    .unwrap();
    std::fs::write(
        config.join("enabled_mods.ron"),
        format!("[\"{POISON_ID}\"]"),
    )
    .unwrap();
    std::fs::write(
        config.join("settings.ron"),
        "(master_volume:0.123,nova_os_bright_detent:0)",
    )
    .unwrap();
    home
}

fn home_env(home: &Path) -> Vec<(String, String)> {
    vec![("HOME".into(), home.path_string())]
}

/// Small helper so the env vectors read as plain strings.
trait PathString {
    fn path_string(&self) -> String;
}
impl PathString for Path {
    fn path_string(&self) -> String {
        self.display().to_string()
    }
}

/// The sandbox as probe applies it, with the override skip forced off: a leg
/// must not silently lose a variable because the test host exports it (the
/// skip itself is pinned by `profile_sandbox_preserves_operator_overrides`).
fn sandbox_env(run_dir: &Path) -> Vec<(String, String)> {
    profile_sandbox::env_with(run_dir, |_| false)
}

fn line<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .unwrap_or_else(|| panic!("no {key} line in child output:\n{stdout}"))
}

#[test]
fn child_run_resolves_mod_cache_inside_the_sandbox() {
    let home = poisoned_home();
    let run_dir = tempfile::tempdir().expect("run dir");
    profile_sandbox::prepare(run_dir.path());

    // Fail-first: unsandboxed, the child reads the operator's installed-mod
    // index - exactly the state that can falsify an older-commit run.
    let bug = run_child(&home_env(home.path()));
    assert!(
        line(&bug, "INDEX=").contains(POISON_ID),
        "the reproduction must read the poisoned index, got: {}",
        line(&bug, "INDEX=")
    );

    // Sandboxed: the index resolves inside this run's own empty profile.
    let mut fixed = home_env(home.path());
    fixed.extend(sandbox_env(run_dir.path()));
    let fixed = run_child(&fixed);
    assert_eq!(
        line(&fixed, "INDEX="),
        "None",
        "no downloaded-mod index in a fresh run profile"
    );
    assert!(
        line(&fixed, "DATA_DIR=").starts_with(&run_dir.path().join("profile/data").path_string()),
        "data dir must sit under the run dir, got: {}",
        line(&fixed, "DATA_DIR=")
    );

    // Isolated lever: NOVA_MODDING_CACHE_ROOT alone already moves the index (the
    // belt for commits older than the XDG isolation), while the config-side
    // store still reads the poison - so the two levers are proven separately,
    // not by one all-or-nothing env blob.
    let mut cache_only = home_env(home.path());
    cache_only.push((
        "NOVA_MODDING_CACHE_ROOT".into(),
        run_dir.path().join("profile/mods").path_string(),
    ));
    let cache_only = run_child(&cache_only);
    assert_eq!(line(&cache_only, "INDEX="), "None");
    assert!(
        line(&cache_only, "ENABLED=").contains(POISON_ID),
        "control: the mod-cache override does NOT cover the config stores"
    );
}

#[test]
fn child_run_resolves_prefs_and_settings_inside_the_sandbox() {
    let home = poisoned_home();
    let run_dir = tempfile::tempdir().expect("run dir");
    profile_sandbox::prepare(run_dir.path());

    // Fail-first: unsandboxed, the child reads the operator's enabled mods.
    let bug = run_child(&home_env(home.path()));
    assert!(
        line(&bug, "ENABLED=").contains(POISON_ID),
        "the reproduction must read the poisoned enabled-mods store, got: {}",
        line(&bug, "ENABLED=")
    );
    assert!(
        line(&bug, "CONFIG_DIR=").starts_with(&home.path().path_string()),
        "and settings.ron would come from the same dir"
    );

    // Sandboxed: prefs are absent and the config dir - which settings.ron
    // resolves through too - sits under the run dir.
    let mut fixed = home_env(home.path());
    fixed.extend(sandbox_env(run_dir.path()));
    let fixed = run_child(&fixed);
    assert_eq!(
        line(&fixed, "ENABLED="),
        "None",
        "a fresh run profile has no saved enabled-mod set"
    );
    let config_dir = line(&fixed, "CONFIG_DIR=");
    assert!(
        config_dir.starts_with(&run_dir.path().join("profile/config").path_string()),
        "config dir must sit under the run dir, got: {config_dir}"
    );
    assert!(
        !Path::new(config_dir)
            .join("nova-protocol/settings.ron")
            .exists(),
        "the operator's saved settings are not visible from the sandbox"
    );
}
