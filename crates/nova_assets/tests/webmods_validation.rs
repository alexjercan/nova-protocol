//! The DEEP publish gate for portal mods (task 20260715-142900): every bundle
//! under the repo-root `webmods/` must load through the REAL modding loaders -
//! manifest, every content file, every config tree - to a recursive `Loaded`.
//! The portal generator (`scripts/gen-portal.py`) deliberately validates only
//! what a manifest gate can (it is engine-free so the deploy job stays fast); THIS
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
use nova_modding::prelude::{BundleAsset, Content, NovaModdingPlugin};

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

/// The Ledger's campaign mapping lists its six chapters in play order, every
/// member resolves to a real scenario in the bundle, and every member past the
/// visible entry (`ch1`) is `hidden` - so the picker's collapsible "The Ledger"
/// header lists the whole arc and the hidden chapters are reachable ONLY through
/// it (task 20260724-220842). Reads the committed content files directly, the
/// contract a mod author authored.
#[test]
fn the_ledger_campaign_lists_its_chapters_in_order() {
    let dir = "../../webmods/the-ledger";
    let read_content = |file: &str| -> Vec<Content> {
        let path = format!("{dir}/{file}");
        ron::de::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}")),
        )
        .unwrap_or_else(|e| panic!("parse {path}: {e}"))
    };

    // The campaign mapping.
    let campaign = read_content("ledger_campaign.content.ron")
        .into_iter()
        .find_map(|c| match c {
            Content::Campaign(cfg) => Some(cfg),
            _ => None,
        })
        .expect("ledger_campaign.content.ron declares a Campaign");
    assert_eq!(campaign.id, "the_ledger");
    assert_eq!(campaign.name, "The Ledger");
    assert_eq!(
        campaign.scenarios,
        vec![
            "ledger_ch1_dead_weight",
            "ledger_ch2_claim_jumpers",
            "ledger_ch2b_the_heavies",
            "ledger_ch3_quiet_channel",
            "ledger_ch4_the_buyer",
            "ledger_ch5_the_raid",
        ],
        "chapters listed in play order (chapter two's two acts contiguous)"
    );

    // Every chapter scenario the bundle ships, by id -> hidden.
    let mut hidden_by_id = std::collections::HashMap::new();
    for file in [
        "ledger_ch1.content.ron",
        "ledger_ch2.content.ron",
        "ledger_ch2b.content.ron",
        "ledger_ch3.content.ron",
        "ledger_ch4.content.ron",
        "ledger_ch5_the_raid.content.ron",
    ] {
        for item in read_content(file) {
            if let Content::Scenario(s) = item {
                hidden_by_id.insert(s.id.clone(), s.hidden);
            }
        }
    }

    // Every member resolves, and only the ch1 entry is visible in the flat picker.
    for member in &campaign.scenarios {
        let hidden = hidden_by_id
            .get(member)
            .unwrap_or_else(|| panic!("campaign member '{member}' has no scenario in the bundle"));
        if member == "ledger_ch1_dead_weight" {
            assert!(!hidden, "chapter one is the visible campaign entry");
        } else {
            assert!(
                hidden,
                "'{member}' is a hidden chapter, reachable via the header"
            );
        }
    }
}
