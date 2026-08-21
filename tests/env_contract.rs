//! Every environment variable the game reads, named once.
//!
//! Nothing type-checks an environment variable: renaming one compiles clean
//! and then fails at load, or - worse - silently stops arming something and
//! the run looks fine while measuring nothing. This test is the type check
//! that does not exist. It lives in the root package because that is the only
//! crate that can see every other one.
//!
//! Two jobs:
//!
//! 1. **Pin the wire names.** A variable is a contract with run scripts, CI
//!    and the dev book, so a rename is a break rather than a refactor and has
//!    to be a deliberate edit here.
//! 2. **Pin the duplicates.** Three shipping crates spell a name that another
//!    crate OWNS, because taking a dependency edge for one string is worse
//!    than repeating it. Every such pair is asserted equal below, so the
//!    drift the duplication invites fails a test instead of arming nothing.
//!
//! Adding a variable without adding it here leaves it undeclared, which is the
//! state this whole set exists to make impossible.

use nova_assets::{
    mod_cache::MOD_CACHE_ROOT_ENV, portal::PORTAL_URL_ENV, storage::CONFIG_ROOT_ENV,
};
use nova_autopilot::prelude::*;
use nova_gameplay::prelude::{HARNESS_ENVS, MUTE_ENV};
use nova_menu::prelude::MENU_BACKDROP_ENV;
use nova_probe::{
    probe_env, CONTRACT_PARAM, FRAMES_PARAM, INVARIANTS_PARAM, LABEL_PARAM, NORENDER_ENV,
    OUT_PARAM, PROBE_ENV, PROBE_MODE_ENV, QUALITY_PARAM, SCENARIO_PARAM, SNAPSHOT_PARAM,
    STEPDIAG_BODIES_PARAM, STEPDIAG_PARAM, TIMELINE_PARAM, WARMUP_PARAM,
};
use nova_scenario::prelude::CAPTURE_DIR_ENV;

/// The harness: what drives a run and where its pictures go. Owned by
/// `nova_autopilot`.
#[test]
fn the_harness_variables_are_the_documented_ones() {
    assert_eq!(AUTOPILOT_ENV, "NOVA_AUTOPILOT");
    assert_eq!(DEADLINE_ENV, "NOVA_AUTOPILOT_DEADLINE");
    assert_eq!(CAPTURE_ENV, "NOVA_CAPTURE");
    assert_eq!(CAPTURE_DIR_ENV, "NOVA_CAPTURE_DIR");
}

/// Measurement: every knob is `NOVA_PROBE_*`, computed from one prefix.
#[test]
fn the_measurement_variables_all_carry_the_probe_prefix() {
    assert_eq!(PROBE_ENV, "NOVA_PROBE");
    assert_eq!(PROBE_MODE_ENV, "NOVA_PROBE_MODE");
    assert_eq!(nova_core::RENDER_DIAG_ENV, "NOVA_PROBE_RENDER_DIAG");

    // The computed half: the host pushes these into a child and the child
    // reads them back, so both sides have to agree on the spelling.
    for (param, expected) in [
        (WARMUP_PARAM, "NOVA_PROBE_WARMUP"),
        (FRAMES_PARAM, "NOVA_PROBE_FRAMES"),
        (OUT_PARAM, "NOVA_PROBE_OUT"),
        (LABEL_PARAM, "NOVA_PROBE_LABEL"),
        (SCENARIO_PARAM, "NOVA_PROBE_SCENARIO"),
        (QUALITY_PARAM, "NOVA_PROBE_QUALITY"),
        (TIMELINE_PARAM, "NOVA_PROBE_TIMELINE"),
        (INVARIANTS_PARAM, "NOVA_PROBE_INVARIANTS"),
        (CONTRACT_PARAM, "NOVA_PROBE_CONTRACT"),
        (SNAPSHOT_PARAM, "NOVA_PROBE_SNAPSHOT"),
        (STEPDIAG_PARAM, "NOVA_PROBE_STEPDIAG"),
        (STEPDIAG_BODIES_PARAM, "NOVA_PROBE_STEPDIAG_BODIES"),
    ] {
        assert_eq!(probe_env(param), expected);
    }
}

/// Outputs-off: one variable per device, one flag each on the game binary.
#[test]
fn the_outputs_off_pair_is_norender_and_mute() {
    assert_eq!(NORENDER_ENV, "NOVA_NORENDER");
    assert_eq!(MUTE_ENV, "NOVA_MUTE");
}

/// Modding and the settings store. `NOVA_CONFIG_ROOT` is deliberately NOT in
/// the modding family: it is the settings store root.
#[test]
fn the_modding_variables_carry_their_own_prefix_and_the_config_root_does_not() {
    assert_eq!(MOD_CACHE_ROOT_ENV, "NOVA_MODDING_CACHE_ROOT");
    assert_eq!(PORTAL_URL_ENV, "NOVA_MODDING_PORTAL_URL");
    assert_eq!(CONFIG_ROOT_ENV, "NOVA_CONFIG_ROOT");
}

/// The menu's capture pin, which belongs to the menu rather than to the
/// harness that usually sets it.
#[test]
fn the_menu_backdrop_pin_is_the_documented_one() {
    assert_eq!(MENU_BACKDROP_ENV, "NOVA_MENU_BACKDROP");
}

/// Every `NOVA_*` name the game reads or writes, and the crate that declares
/// it. The ROSTER, in the sense `catalog_drift` uses the word: the scan below
/// walks the source and fails on anything not listed here.
///
/// `examples/` is deliberately absent - a range keeps its literals, because a
/// probe run goes red when one drifts, which is detection shipped code does
/// not have (`CONVENTIONS.md` Nova rule 5).
const ROSTER: &[&str] = &[
    // Harness - nova_autopilot.
    "NOVA_AUTOPILOT",
    "NOVA_AUTOPILOT_DEADLINE",
    "NOVA_CAPTURE",
    "NOVA_CAPTURE_DIR",
    // Measurement - nova_probe, all computed from one prefix.
    "NOVA_PROBE",
    "NOVA_PROBE_MODE",
    "NOVA_PROBE_RENDER_DIAG",
    "NOVA_PROBE_WARMUP",
    "NOVA_PROBE_FRAMES",
    "NOVA_PROBE_OUT",
    "NOVA_PROBE_LABEL",
    "NOVA_PROBE_RES",
    "NOVA_PROBE_RENDER_SCALE",
    "NOVA_PROBE_MAX_DELTA",
    "NOVA_PROBE_PRESENT",
    "NOVA_PROBE_QUALITY",
    "NOVA_PROBE_SCENARIO",
    "NOVA_PROBE_SHA",
    "NOVA_PROBE_HOST",
    "NOVA_PROBE_CENSUS_FRAME",
    "NOVA_PROBE_FRAMECOST_FRAMES",
    "NOVA_PROBE_TIMELINE",
    "NOVA_PROBE_INVARIANTS",
    "NOVA_PROBE_CONTRACT",
    "NOVA_PROBE_SNAPSHOT",
    "NOVA_PROBE_SNAPSHOT_FRAMES",
    "NOVA_PROBE_STEPDIAG",
    "NOVA_PROBE_STEPDIAG_BODIES",
    // The probe host's own test re-exec marker.
    "NOVA_PROBE_SANDBOX_RESOLVER_CHILD",
    // Outputs off - nova_core and nova_gameplay.
    "NOVA_NORENDER",
    "NOVA_MUTE",
    // Modding - nova_assets.
    "NOVA_MODDING_CACHE_ROOT",
    "NOVA_MODDING_PORTAL_URL",
    // The settings store root - nova_assets, deliberately not modding.
    "NOVA_CONFIG_ROOT",
    // The menu's backdrop pin - nova_menu.
    "NOVA_MENU_BACKDROP",
];

/// Walk `crates/` and `src/` and fail on any `NOVA_*` string the roster above
/// does not name.
///
/// This is the half that catches an ADDITION. The assertions elsewhere in this
/// file pin the names that exist; nothing but a scan can notice a new variable
/// nobody declared, which is exactly how `NOVA_SUBSTEPS` came to live inside a
/// shipped gameplay plugin with a `panic!` in it.
#[test]
fn no_source_file_names_a_variable_the_roster_does_not() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found: Vec<(String, String)> = Vec::new();
    for dir in ["crates", "src", "tests"] {
        collect_nova_names(&root.join(dir), &mut found);
    }
    assert!(
        !found.is_empty(),
        "the scan found no NOVA_* names at all - it is walking the wrong tree"
    );
    let unlisted: Vec<&(String, String)> = found
        .iter()
        .filter(|(name, _)| !ROSTER.contains(&name.as_str()))
        .collect();
    assert!(
        unlisted.is_empty(),
        "these environment variables are not on the roster in \
         tests/env_contract.rs - declare them there, or they exist only in \
         whichever run script happens to set them: {unlisted:#?}"
    );
}

/// Every `"NOVA_..."` string literal under `dir`, with the file that holds it.
///
/// Deliberately a plain string scan rather than a parse: a name reaches the
/// environment as a literal however it is spelled, and the point is to see all
/// of them. `NOVA_OS_*` is filtered out - those are `const Color` and layout
/// values in `nova_os_ui`, not variables, and they outnumber the real set.
fn collect_nova_names(dir: &std::path::Path, found: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_nova_names(&path, found);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (index, _) in text.match_indices("\"NOVA_") {
            let rest = &text[index + 1..];
            let Some(end) = rest.find('"') else { continue };
            let name = &rest[..end];
            if !name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                continue;
            }
            // `NOVA_` with nothing after it is this file's own search pattern
            // reading itself, not a variable.
            if name == "NOVA_" || name.starts_with("NOVA_OS_") {
                continue;
            }
            found.push((name.to_string(), path.display().to_string()));
        }
    }
}

/// The duplicates, each asserted against the crate that OWNS the name.
///
/// These exist because the alternative is a dependency edge from a shipping
/// crate to a dev-tooling one, or from the host harness to the whole asset
/// stack, for a single string. The duplication is the deliberate choice; this
/// test is what makes it safe.
#[test]
fn every_re_spelled_name_matches_the_crate_that_owns_it() {
    assert_eq!(
        HARNESS_ENVS,
        [AUTOPILOT_ENV, CAPTURE_ENV],
        "nova_gameplay decides a run is muted from the harness variables \
         nova_autopilot owns"
    );
    assert_eq!(
        CAPTURE_DIR_ENV,
        nova_autopilot::capture::CAPTURE_DIR_ENV,
        "the scenario Screenshot action stages under the same directory \
         nova_autopilot's capture path does"
    );
    assert_eq!(
        nova_probe_cli::native::profile_sandbox::SANDBOXED_VARS[0],
        MOD_CACHE_ROOT_ENV,
        "the probe sandbox redirects the mod cache nova_assets owns"
    );
}
