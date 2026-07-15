//! The DEEP publish gate for portal mods (task 20260715-142900): every bundle
//! under the repo-root `webmods/` must load through the REAL modding loaders -
//! manifest, every content file, every config tree - to a recursive `Loaded`.
//! The portal generator (`nova_portal_gen`) deliberately validates only what a
//! manifest gate can (it is engine-free so the deploy job stays fast); THIS
//! test, running where CI already runs tests, is the "does the content
//! actually load" half of publishing.
//!
//! Native tests may list directories (the wasm restriction is why the GAME
//! never scans; a test host can).

use std::time::{Duration, Instant};

use bevy::{
    asset::{AssetPlugin, RecursiveDependencyLoadState},
    prelude::*,
};
use nova_modding::prelude::{BundleAsset, NovaModdingPlugin};

/// Load every `webmods/<mod>/<mod's>.bundle.ron` through the real loaders and
/// require recursive `Loaded` for each.
#[test]
fn every_webmods_bundle_loads_recursively() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            // Tests run with the crate root as cwd; webmods lives at the repo root.
            file_path: "../../webmods".to_string(),
            ..default()
        },
    ));
    app.add_plugins(NovaModdingPlugin);
    let asset_server = app.world().resource::<AssetServer>().clone();

    let mut bundles = Vec::new();
    for entry in std::fs::read_dir("../../webmods").expect("webmods/ exists at the repo root") {
        let dir = entry.expect("readable entry").path();
        if !dir.is_dir() {
            continue;
        }
        let id = dir.file_name().unwrap().to_string_lossy().to_string();
        for file in std::fs::read_dir(&dir).expect("readable mod dir") {
            let path = file.expect("readable entry").path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".bundle.ron"))
            {
                let rel = format!("{id}/{}", path.file_name().unwrap().to_string_lossy());
                bundles.push((rel.clone(), asset_server.load::<BundleAsset>(rel)));
            }
        }
    }
    assert!(
        !bundles.is_empty(),
        "webmods/ must contain at least one publishable mod"
    );

    let deadline = Instant::now() + Duration::from_secs(60);
    for (name, handle) in &bundles {
        loop {
            app.update();
            match asset_server.get_recursive_dependency_load_state(handle.id().untyped()) {
                Some(RecursiveDependencyLoadState::Loaded) => break,
                Some(RecursiveDependencyLoadState::Failed(err)) => {
                    panic!("webmods bundle '{name}' failed to load: {err}")
                }
                _ => {}
            }
            assert!(Instant::now() < deadline, "timed out loading '{name}'");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
