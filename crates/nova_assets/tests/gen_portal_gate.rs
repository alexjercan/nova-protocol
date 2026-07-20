//! Publish-gate coverage for the PRODUCTION portal generator, `scripts/gen-portal.py`.
//!
//! This test drives the real Python tool the deploy workflow and the preview
//! script both run, over synthetic fixtures, and asserts the manifest-level
//! PUBLISH gates each reject (non-zero exit) or accept (exit 0) exactly as the
//! now-deleted `nova_portal_gen` crate's `tests/generate.rs` did (task
//! 20260720-230924). The Rust crate's `tests/generate.rs` was the ONLY committed
//! exercise of these gates; re-homing them here onto the production tool keeps
//! that coverage after the crate is gone
//! (`deleted-content-tests-carry-engine-coverage`).
//!
//! The rejection cases are TABLE-DRIVEN off a single fixture builder so each new
//! case is one row. Positives (real webmods/ publishes, a synthetic mod
//! publishes, determinism) are individual tests.
//!
//! The script needs `python3` on PATH and is invoked from the repo root. We
//! resolve the script and repo paths from `CARGO_MANIFEST_DIR`
//! (`.../crates/nova_assets` -> `../..`) and pass ABSOLUTE `--source/--shipped/
//! --out` so cwd never matters. If `python3` is absent (an unusual local
//! runner), every test self-skips with a printed note rather than hard-failing;
//! CI (ci.yaml on ubuntu-latest, deploy runs gen-portal.py) always has python3.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Repo root: `.../crates/nova_assets` -> `../..`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root resolves from CARGO_MANIFEST_DIR")
}

fn script_path() -> PathBuf {
    repo_root().join("scripts/gen-portal.py")
}

fn shipped_catalog() -> PathBuf {
    repo_root().join("assets/mods.catalog.ron")
}

/// `true` (with a printed note) when `python3` is not runnable, so the caller can
/// self-skip. Matches the repo's env-dependent self-skip convention: an unusual
/// local runner without python3 does not hard-fail, but CI always has it.
fn python3_missing() -> bool {
    match Command::new("python3").arg("--version").output() {
        Ok(out) if out.status.success() => false,
        _ => {
            // CI must NEVER silently skip the only committed gate coverage: if
            // python3 is missing under CI, fail loudly. A local runner without
            // python3 still self-skips (the repo's env-dependent convention).
            assert!(
                std::env::var_os("CI").is_none(),
                "python3 is not runnable but CI is set - the gen-portal.py gate \
                 coverage must not silently skip in CI"
            );
            eprintln!("SKIP: python3 not runnable on PATH; skipping gen-portal.py gate tests");
            true
        }
    }
}

/// Run `python3 scripts/gen-portal.py --source <src> [--shipped <cat>] --out <out>`
/// with ABSOLUTE paths, from the repo root. Returns the process output.
fn run_gen(source: &Path, shipped: Option<&Path>, out: &Path) -> std::process::Output {
    let mut cmd = Command::new("python3");
    cmd.arg(script_path())
        .arg("--source")
        .arg(source)
        .arg("--out")
        .arg(out)
        .current_dir(repo_root());
    if let Some(shipped) = shipped {
        cmd.arg("--shipped").arg(shipped);
    }
    cmd.output().expect("gen-portal.py runs")
}

// ---------------------------------------------------------------------------
// Fixture builders (mirror generate.rs's synthetic_mod / synthetic_source).
// ---------------------------------------------------------------------------

const VALID_CONTENT: &str = "[]";

fn valid_bundle(version: &str) -> String {
    format!(
        r#"(content: ["mod.content.ron"], meta: (name: "M", description: "d", author: "a", version: "{version}"))"#
    )
}

/// One mod in a source tree: `(id, bundle_body, extra_files)`.
type Mod<'a> = (&'a str, &'a str, &'a [(&'a str, &'a str)]);

/// Build a synthetic multi-mod source tree, returning (source, out) tempdirs.
fn synthetic_source(mods: &[Mod<'_>]) -> (tempfile::TempDir, tempfile::TempDir) {
    let source = tempfile::tempdir().expect("source tempdir");
    for (id, bundle_body, extra_files) in mods {
        let mod_dir = source.path().join(id);
        fs::create_dir_all(&mod_dir).expect("mod dir");
        fs::write(mod_dir.join(format!("{id}.bundle.ron")), bundle_body).expect("bundle");
        for (path, body) in *extra_files {
            let p = mod_dir.join(path);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).expect("parents");
            }
            fs::write(p, body).expect("extra file");
        }
    }
    (source, tempfile::tempdir().expect("out tempdir"))
}

/// Single-mod convenience wrapper.
fn synthetic_mod(
    id: &str,
    bundle_body: &str,
    extra_files: &[(&str, &str)],
) -> (tempfile::TempDir, tempfile::TempDir) {
    synthetic_source(&[(id, bundle_body, extra_files)])
}

// An `art` mod shipping one texture resource - the dependency in the dep:// cases.
const ART_BUNDLE: &str = r#"(content: ["mod.content.ron"], resources: ["textures/sky.png"], meta: (name: "Art", version: "0.1.0"))"#;
const ART_FILES: &[(&str, &str)] = &[("mod.content.ron", "[]"), ("textures/sky.png", "bytes")];

// ---------------------------------------------------------------------------
// Rejection cases, table-driven. Each builds a source and asserts a NON-ZERO
// exit. A `shipped` flag decides whether the real shipped catalog is passed
// (only the id-collision and dep://base cases need it).
// ---------------------------------------------------------------------------

/// How a rejection fixture is built. Boxed builder so each row can differ.
struct Reject {
    name: &'static str,
    with_shipped: bool,
    /// A substring the rejection's stderr must contain, so a gate that starts
    /// rejecting for the WRONG reason is caught (not just any non-zero exit).
    /// Ported from the deleted generate.rs's `err.0.contains(...)` checks.
    stderr_contains: &'static str,
    build: fn() -> (tempfile::TempDir, tempfile::TempDir),
}

fn reject_cases() -> Vec<Reject> {
    vec![
        Reject {
            name: "id-collision-with-shipped",
            with_shipped: true, // shipped catalog installs 'example'
            stderr_contains: "collides with a SHIPPED catalog id",
            build: || {
                synthetic_mod(
                    "example",
                    &valid_bundle("0.1.0"),
                    &[("mod.content.ron", VALID_CONTENT)],
                )
            },
        },
        Reject {
            name: "missing-content-file",
            with_shipped: false,
            stderr_contains: "is not a file inside",
            build: || synthetic_mod("ok-mod", &valid_bundle("0.1.0"), &[]),
        },
        Reject {
            name: "escaping-content-path",
            with_shipped: false,
            stderr_contains: "is not a file inside",
            build: || {
                // The escaped target EXISTS next to the mod dir (existence-based
                // validation would pass); membership-based validation rejects it.
                let bundle =
                    r#"(content: ["../outside.content.ron"], meta: (name: "M", version: "0.1.0"))"#;
                let (source, out) = synthetic_mod("ok-mod", bundle, &[]);
                fs::write(source.path().join("outside.content.ron"), VALID_CONTENT)
                    .expect("outside file");
                (source, out)
            },
        },
        Reject {
            name: "missing-resource-file",
            with_shipped: false,
            stderr_contains: "listed resource file",
            build: || {
                let bundle = r#"(content: ["mod.content.ron"], resources: ["textures/rock.png"], meta: (name: "M", version: "0.1.0"))"#;
                synthetic_mod("ok-mod", bundle, &[("mod.content.ron", VALID_CONTENT)])
            },
        },
        Reject {
            name: "content-ref-to-undeclared-resource",
            with_shipped: false,
            stderr_contains: "references undeclared mod resource",
            build: || {
                let bundle =
                    r#"(content: ["mod.content.ron"], meta: (name: "M", version: "0.1.0"))"#;
                synthetic_mod(
                    "ok-mod",
                    bundle,
                    &[("mod.content.ron", r#"["self://textures/missing.png"]"#)],
                )
            },
        },
        Reject {
            name: "empty-name",
            with_shipped: false,
            stderr_contains: "meta.name is required to publish",
            build: || {
                let bundle =
                    r#"(content: ["mod.content.ron"], meta: (name: "", version: "0.1.0"))"#;
                synthetic_mod("ok-mod", bundle, &[("mod.content.ron", VALID_CONTENT)])
            },
        },
        Reject {
            name: "empty-source",
            with_shipped: false,
            stderr_contains: "refusing to publish an empty portal",
            build: || {
                let source = tempfile::tempdir().expect("source tempdir");
                let out = tempfile::tempdir().expect("out tempdir");
                (source, out)
            },
        },
        Reject {
            name: "empty-version",
            with_shipped: false,
            stderr_contains: "meta.version is required to publish",
            build: || {
                synthetic_mod(
                    "ok-mod",
                    &valid_bundle(""),
                    &[("mod.content.ron", VALID_CONTENT)],
                )
            },
        },
        Reject {
            name: "invalid-id",
            with_shipped: false,
            stderr_contains: "is invalid: use lowercase ascii",
            build: || {
                synthetic_mod(
                    "Bad_Mod",
                    &valid_bundle("0.1.0"),
                    &[("mod.content.ron", VALID_CONTENT)],
                )
            },
        },
        Reject {
            name: "missing-bundle-manifest",
            with_shipped: false,
            stderr_contains: "no *.bundle.ron at the mod root",
            build: || {
                let source = tempfile::tempdir().expect("source tempdir");
                fs::create_dir_all(source.path().join("no-bundle")).expect("mod dir");
                let out = tempfile::tempdir().expect("out tempdir");
                (source, out)
            },
        },
        Reject {
            name: "unresolvable-dependency",
            with_shipped: false,
            stderr_contains: "is neither a portal mod",
            build: || {
                let bundle = r#"(content: ["mod.content.ron"], meta: (name: "M", version: "0.1.0", dependencies: ["nonexistent-mod"]))"#;
                synthetic_mod("ok-mod", bundle, &[("mod.content.ron", VALID_CONTENT)])
            },
        },
        Reject {
            name: "dep-ref-to-non-declared-dependency",
            with_shipped: false,
            stderr_contains: "is not a declared dependency",
            build: || {
                // consumer references art but does NOT declare it as a dependency.
                let consumer =
                    r#"(content: ["mod.content.ron"], meta: (name: "Consumer", version: "0.1.0"))"#;
                synthetic_source(&[
                    ("art", ART_BUNDLE, ART_FILES),
                    (
                        "consumer",
                        consumer,
                        &[("mod.content.ron", r#"["dep://art/textures/sky.png"]"#)],
                    ),
                ])
            },
        },
        Reject {
            name: "dep-ref-to-undeclared-resource-of-dependency",
            with_shipped: false,
            stderr_contains: "of dependency",
            build: || {
                let consumer = r#"(content: ["mod.content.ron"], meta: (name: "Consumer", version: "0.1.0", dependencies: ["art"]))"#;
                synthetic_source(&[
                    ("art", ART_BUNDLE, ART_FILES),
                    (
                        "consumer",
                        consumer,
                        // art declares textures/sky.png, not textures/missing.png.
                        &[("mod.content.ron", r#"["dep://art/textures/missing.png"]"#)],
                    ),
                ])
            },
        },
        Reject {
            name: "malformed-dep-ref",
            with_shipped: false,
            stderr_contains: "malformed dependency resource ref",
            build: || {
                let consumer = r#"(content: ["mod.content.ron"], meta: (name: "Consumer", version: "0.1.0", dependencies: ["art"]))"#;
                synthetic_source(&[
                    ("art", ART_BUNDLE, ART_FILES),
                    (
                        "consumer",
                        consumer,
                        &[("mod.content.ron", r#"["dep://art"]"#)],
                    ),
                ])
            },
        },
        Reject {
            name: "bare-asset-ref",
            with_shipped: false,
            stderr_contains: "with no scheme",
            build: || {
                let bundle =
                    r#"(content: ["mod.content.ron"], meta: (name: "M", version: "0.1.0"))"#;
                synthetic_mod(
                    "ok-mod",
                    bundle,
                    &[("mod.content.ron", r#"["textures/cubemap.png"]"#)],
                )
            },
        },
    ]
}

#[test]
fn rejection_gates_all_fail_nonzero() {
    if python3_missing() {
        return;
    }
    let shipped = shipped_catalog();
    for case in reject_cases() {
        let (source, out) = (case.build)();
        let shipped_arg = case.with_shipped.then(|| shipped.as_path());
        let output = run_gen(source.path(), shipped_arg, out.path());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "gate '{}' must reject (non-zero exit); got success. stderr:\n{stderr}",
            case.name,
        );
        assert!(
            stderr.contains(case.stderr_contains),
            "gate '{}' rejected, but for the wrong reason: expected stderr to contain '{}'.\nstderr:\n{stderr}",
            case.name,
            case.stderr_contains,
        );
    }
}

// ---------------------------------------------------------------------------
// Positives.
// ---------------------------------------------------------------------------

/// The REAL webmods/ tree publishes: exit 0 and catalog.json names both mods.
#[test]
fn real_webmods_publishes_and_lists_both_mods() {
    if python3_missing() {
        return;
    }
    let webmods = repo_root().join("webmods");
    let out = tempfile::tempdir().expect("out tempdir");
    let output = run_gen(&webmods, Some(&shipped_catalog()), out.path());
    assert!(
        output.status.success(),
        "the real webmods tree must publish. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = fs::read_to_string(out.path().join("catalog.json")).expect("catalog.json written");
    assert!(
        json.contains("\"gauntlet\""),
        "catalog names gauntlet:\n{json}"
    );
    assert!(
        json.contains("\"the-ledger\""),
        "catalog names the-ledger:\n{json}"
    );
}

/// A well-formed synthetic mod publishes (exit 0, catalog lists it). Proves the
/// fixture rig is itself valid, so the rejection rows fail for their OWN reason.
#[test]
fn synthetic_valid_mod_publishes() {
    if python3_missing() {
        return;
    }
    let (source, out) = synthetic_mod(
        "ok-mod",
        &valid_bundle("0.1.0"),
        &[("mod.content.ron", VALID_CONTENT)],
    );
    let output = run_gen(source.path(), None, out.path());
    assert!(
        output.status.success(),
        "a valid synthetic mod must publish. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = fs::read_to_string(out.path().join("catalog.json")).expect("catalog.json");
    assert!(json.contains("\"ok-mod\""), "catalog lists ok-mod:\n{json}");
}

/// Two runs over the same source produce byte-identical portal trees
/// (the verify-generator-stability-before-commit-diff requirement).
#[test]
fn generation_is_deterministic() {
    if python3_missing() {
        return;
    }
    let webmods = repo_root().join("webmods");
    let out_a = tempfile::tempdir().expect("out a");
    let out_b = tempfile::tempdir().expect("out b");
    assert!(
        run_gen(&webmods, Some(&shipped_catalog()), out_a.path())
            .status
            .success(),
        "first run publishes"
    );
    assert!(
        run_gen(&webmods, Some(&shipped_catalog()), out_b.path())
            .status
            .success(),
        "second run publishes"
    );
    // Compare the full trees byte-for-byte: same relative paths, same bytes.
    let files_a = collect_tree(out_a.path());
    let files_b = collect_tree(out_b.path());
    assert_eq!(
        files_a.keys().collect::<Vec<_>>(),
        files_b.keys().collect::<Vec<_>>(),
        "two runs produce the same file set"
    );
    for (rel, bytes_a) in &files_a {
        assert_eq!(
            bytes_a,
            files_b.get(rel).expect("same file set"),
            "byte-identical for {rel}"
        );
    }
}

/// Every file under `root`, keyed by forward-slash relative path, with contents.
fn collect_tree(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, acc: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("readable dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(root, &path, acc);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                acc.insert(rel, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut acc = std::collections::BTreeMap::new();
    walk(root, root, &mut acc);
    acc
}
