//! Integration tests for the portal generator: the REAL `webmods/` tree must
//! publish cleanly (with verifiable hashes and deterministic output), and each
//! manifest-gate validation must fail with its own clear error on a synthetic
//! bad source. Tests run with the crate root as cwd; the repo root is `../..`.

use std::{fs, path::Path};

use nova_mod_format::PortalCatalog;
use sha2::{Digest, Sha256};

const WEBMODS: &str = "../../webmods";
const SHIPPED: &str = "../../assets/mods.catalog.ron";

/// Publish the real webmods/ tree and verify the whole contract: the catalog
/// parses back, every listed file was copied under `<id>/<version>/` with a
/// size and sha256 that RECOMPUTE from the copied bytes, the bundle entry point
/// is in the file list, and totals add up.
#[test]
fn real_webmods_publish_and_hashes_verify() {
    let out = tempfile::tempdir().expect("tempdir");
    let catalog =
        nova_portal_gen::generate(Path::new(WEBMODS), Some(Path::new(SHIPPED)), out.path())
            .expect("the real webmods tree must publish");

    let json = fs::read_to_string(out.path().join("catalog.json")).expect("catalog.json written");
    let parsed: PortalCatalog = serde_json::from_str(&json).expect("catalog.json parses");
    assert_eq!(
        parsed.schema_version,
        nova_mod_format::PORTAL_SCHEMA_VERSION
    );
    assert!(
        parsed.entries.iter().any(|e| e.id == "gauntlet"),
        "the first portal mod is published"
    );
    // Deterministic ORDER is asserted directly (the byte-identity test alone
    // would only catch map-ordered serialization probabilistically at today's
    // entry counts - review R1.3).
    assert!(
        parsed.entries.windows(2).all(|w| w[0].id < w[1].id),
        "entries are sorted by id"
    );
    for entry in &parsed.entries {
        assert!(
            entry.files.windows(2).all(|w| w[0].path < w[1].path),
            "files are sorted by path for {}",
            entry.id
        );
    }

    for entry in &parsed.entries {
        assert!(!entry.version.is_empty());
        assert!(
            entry.files.iter().any(|f| f.path == entry.bundle),
            "the bundle entry point is part of the file list"
        );
        let mut total = 0;
        for file in &entry.files {
            let copied = out
                .path()
                .join(&entry.id)
                .join(&entry.version)
                .join(&file.path);
            let bytes = fs::read(&copied)
                .unwrap_or_else(|e| panic!("listed file {} was not copied: {e}", file.path));
            assert_eq!(
                bytes.len() as u64,
                file.size,
                "size matches for {}",
                file.path
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                file.sha256,
                "sha256 recomputes for {}",
                file.path
            );
            total += file.size;
        }
        assert_eq!(
            total, entry.total_size,
            "total_size adds up for {}",
            entry.id
        );
    }

    // The returned catalog is the written one.
    assert_eq!(catalog.entries.len(), parsed.entries.len());
}

/// Generating twice yields byte-identical catalog.json - the
/// verify-generator-stability-before-commit-diff lesson made this a hard
/// requirement before any workflow relies on the output.
#[test]
fn generation_is_deterministic() {
    let out_a = tempfile::tempdir().expect("tempdir");
    let out_b = tempfile::tempdir().expect("tempdir");
    nova_portal_gen::generate(Path::new(WEBMODS), Some(Path::new(SHIPPED)), out_a.path())
        .expect("first run");
    nova_portal_gen::generate(Path::new(WEBMODS), Some(Path::new(SHIPPED)), out_b.path())
        .expect("second run");
    let a = fs::read(out_a.path().join("catalog.json")).expect("catalog a");
    let b = fs::read(out_b.path().join("catalog.json")).expect("catalog b");
    assert_eq!(
        a, b,
        "two runs over the same source must write identical bytes"
    );
}

/// Build a synthetic single-mod source tree and return (source_dir, out_dir).
fn synthetic_mod(
    id: &str,
    bundle_body: &str,
    extra_files: &[(&str, &str)],
) -> (tempfile::TempDir, tempfile::TempDir) {
    let source = tempfile::tempdir().expect("source tempdir");
    let mod_dir = source.path().join(id);
    fs::create_dir_all(&mod_dir).expect("mod dir");
    fs::write(mod_dir.join(format!("{id}.bundle.ron")), bundle_body).expect("bundle");
    for (path, body) in extra_files {
        let p = mod_dir.join(path);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).expect("parents");
        }
        fs::write(p, body).expect("extra file");
    }
    (source, tempfile::tempdir().expect("out tempdir"))
}

const VALID_CONTENT: &str = "[]";

fn valid_bundle(version: &str) -> String {
    format!(
        r#"(content: ["mod.content.ron"], meta: (name: "M", description: "d", author: "a", version: "{version}"))"#
    )
}

/// A well-formed synthetic mod publishes (the fixture builder itself is valid,
/// so the failure cases below fail for their OWN reason, not a broken rig).
#[test]
fn synthetic_valid_mod_publishes() {
    let (source, out) = synthetic_mod(
        "ok-mod",
        &valid_bundle("0.1.0"),
        &[("mod.content.ron", VALID_CONTENT)],
    );
    let catalog =
        nova_portal_gen::generate(source.path(), None, out.path()).expect("valid mod publishes");
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].id, "ok-mod");
    assert_eq!(catalog.entries[0].version, "0.1.0");
}

#[test]
fn id_colliding_with_shipped_catalog_is_rejected() {
    let (source, out) = synthetic_mod(
        "demo", // shipped catalog installs 'demo'
        &valid_bundle("0.1.0"),
        &[("mod.content.ron", VALID_CONTENT)],
    );
    let err = nova_portal_gen::generate(source.path(), Some(Path::new(SHIPPED)), out.path())
        .expect_err("shipped-id collision must fail");
    assert!(err.0.contains("collides"), "got: {err}");
}

#[test]
fn missing_content_file_is_rejected() {
    let (source, out) = synthetic_mod("ok-mod", &valid_bundle("0.1.0"), &[]);
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("missing content file must fail");
    assert!(err.0.contains("not a file inside"), "got: {err}");
}

/// R1.1 regression: a content path that ESCAPES the mod directory must be
/// rejected even when the escaped file exists in the source tree - a plain
/// existence check accepted it while the portal never served it (broken mod
/// published with exit 0).
#[test]
fn escaping_content_path_is_rejected() {
    let bundle = r#"(content: ["../outside.content.ron"], meta: (name: "M", version: "0.1.0"))"#;
    let (source, out) = synthetic_mod("ok-mod", bundle, &[]);
    // The escaped target EXISTS (next to the mod dir), making existence-based
    // validation pass; membership-based validation must still reject it.
    fs::write(source.path().join("outside.content.ron"), VALID_CONTENT).expect("outside file");
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("escaping content path must fail");
    assert!(err.0.contains("not a file inside"), "got: {err}");
}

#[test]
fn empty_name_is_rejected() {
    let bundle = r#"(content: ["mod.content.ron"], meta: (name: "", version: "0.1.0"))"#;
    let (source, out) = synthetic_mod("ok-mod", bundle, &[("mod.content.ron", VALID_CONTENT)]);
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("empty name must fail");
    assert!(err.0.contains("name"), "got: {err}");
}

/// R1.4 regression: zero mods found means a broken invocation (wrong --source,
/// bad checkout), never a silently-published empty portal.
#[test]
fn empty_source_is_rejected() {
    let source = tempfile::tempdir().expect("source tempdir");
    let out = tempfile::tempdir().expect("out tempdir");
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("an empty source must fail");
    assert!(err.0.contains("no mods found"), "got: {err}");
}

#[test]
fn empty_version_is_rejected() {
    let (source, out) = synthetic_mod(
        "ok-mod",
        &valid_bundle(""),
        &[("mod.content.ron", VALID_CONTENT)],
    );
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("empty version must fail");
    assert!(err.0.contains("version"), "got: {err}");
}

#[test]
fn invalid_id_is_rejected() {
    let (source, out) = synthetic_mod(
        "Bad_Mod",
        &valid_bundle("0.1.0"),
        &[("mod.content.ron", VALID_CONTENT)],
    );
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("uppercase/underscore id must fail");
    assert!(err.0.contains("invalid"), "got: {err}");
}

#[test]
fn missing_bundle_manifest_is_rejected() {
    let source = tempfile::tempdir().expect("source tempdir");
    fs::create_dir_all(source.path().join("no-bundle")).expect("mod dir");
    let out = tempfile::tempdir().expect("out tempdir");
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("bundle-less mod must fail");
    assert!(err.0.contains("no *.bundle.ron"), "got: {err}");
}

#[test]
fn unresolvable_dependency_is_rejected() {
    let bundle = r#"(content: ["mod.content.ron"], meta: (name: "M", version: "0.1.0", dependencies: ["nonexistent-mod"]))"#;
    let (source, out) = synthetic_mod("ok-mod", bundle, &[("mod.content.ron", VALID_CONTENT)]);
    let err = nova_portal_gen::generate(source.path(), None, out.path())
        .expect_err("unresolvable dependency must fail");
    assert!(err.0.contains("dependency"), "got: {err}");
}
