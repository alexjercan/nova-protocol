//! End-to-end test for the `.meta` generator: point it at a temp asset tree
//! and assert it writes correct default sidecars, preserves existing ones, and
//! skips extensions with no loader. Runs headless and GPU-free (no RenderPlugin,
//! no `App::run`), exactly like the deploy hook.

use std::{fs, path::Path};

use bevy::asset::AssetServer;
use nova_meta_gen::{build_app, generate, Outcome, Summary};

fn read_meta(dir: &Path, rel: &str) -> String {
    fs::read_to_string(dir.join(format!("{rel}.meta")))
        .unwrap_or_else(|e| panic!("expected {rel}.meta to exist: {e}"))
}

#[test]
fn generates_default_metas_for_every_loader() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();

    // One asset per loader family the game ships. Contents are irrelevant:
    // `default_meta()` reads only the loader's `Settings::default()`, never the
    // asset bytes.
    let assets = [
        ("textures/ship.png", "not-a-real-png"),
        ("gltf/hull.glb", "not-a-real-glb"),
        ("shaders/ring.wgsl", "// wgsl"),
        ("audio/blip.wav", "not-a-real-wav"),
        ("scenarios/level.content.ron", "[]"),
        ("mods/base.bundle.ron", "()"),
        ("mods.catalog.ron", "[]"),
        // No loader claims `.md`; must be skipped, not errored.
        ("wiki/page.md", "# hello"),
    ];
    for (rel, body) in assets {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    // A pre-existing hand-authored sidecar (mirrors the cubemap metas) that must
    // be preserved verbatim, never overwritten.
    fs::write(root.join("textures/skybox.png"), "not-a-real-png").unwrap();
    let sentinel = "SENTINEL-DO-NOT-OVERWRITE";
    fs::write(root.join("textures/skybox.png.meta"), sentinel).unwrap();

    let dir = root.to_str().unwrap();
    let app = build_app(dir);
    let server = app.world().resource::<AssetServer>().clone();

    let mut outcomes = Vec::new();
    let summary = generate(&server, dir, |rel, outcome| {
        outcomes.push((rel.display().to_string(), outcome.clone()));
    })
    .expect("generation should not error");

    // 7 real assets get fresh metas; the .md is skipped; the skybox already has one.
    assert_eq!(
        summary,
        Summary {
            written: 7,
            already_exists: 1,
            no_loader: 1,
        },
        "unexpected summary: {summary:?} ({outcomes:?})"
    );

    // Each sidecar names the loader picked by extension, with a Load action.
    let cases = [
        ("textures/ship.png", "ImageLoader"),
        ("gltf/hull.glb", "GltfLoader"),
        ("shaders/ring.wgsl", "ShaderLoader"),
        ("audio/blip.wav", "AudioLoader"),
        ("scenarios/level.content.ron", "ContentAssetLoader"),
        ("mods/base.bundle.ron", "BundleAssetLoader"),
        ("mods.catalog.ron", "CatalogLoader"),
    ];
    for (rel, loader) in cases {
        let meta = read_meta(root, rel);
        assert!(
            meta.contains("Load("),
            "{rel}.meta should be a Load action, got:\n{meta}"
        );
        assert!(
            meta.contains(loader),
            "{rel}.meta should name {loader}, got:\n{meta}"
        );
    }

    // The .md file has no loader, so no sidecar is written.
    assert!(
        !root.join("wiki/page.md.meta").exists(),
        "no loader claims .md; no sidecar should be written"
    );

    // The hand-authored sidecar is untouched.
    assert_eq!(
        fs::read_to_string(root.join("textures/skybox.png.meta")).unwrap(),
        sentinel,
        "existing meta must not be overwritten"
    );

    // Re-running is idempotent: everything now already exists.
    let app2 = build_app(dir);
    let server2 = app2.world().resource::<AssetServer>().clone();
    let again = generate(&server2, dir, |_, _| {}).expect("second pass");
    assert_eq!(again.written, 0, "second pass must write nothing new");
    assert_eq!(again.already_exists, 8, "all 8 real assets now have metas");

    // Sanity: the generated pngs really are re-parseable as minimal meta (the
    // exact thing the web server would now serve instead of an HTML 404 page).
    let generated = read_meta(root, "textures/ship.png");
    assert!(generated.contains("meta_format_version"));
    let _ = &outcomes; // silence unused in the happy path
    assert!(matches!(
        outcomes.iter().find(|(p, _)| p == "wiki/page.md"),
        Some((_, Outcome::NoLoader))
    ));
}
