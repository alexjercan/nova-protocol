//! What the picker offers, and what it is willing to call wrong.

use bevy::ecs::system::RunSystemOnce;
use nova_modding::prelude::{CatalogEntry, ModEntry, ModMeta};

use super::*;

/// A world holding one installed bundle, enabled, shipping `resources`.
fn installed(id: &str, resources: &[&str]) -> World {
    let mut world = World::new();
    let mut bundles = Assets::<BundleAsset>::default();
    let bundle = bundles.add(BundleAsset {
        content: vec![],
        meta: ModMeta::default(),
        new_game_scenario: None,
        resources: resources.iter().map(|file| (*file).to_string()).collect(),
        resource_base: format!("mods/{id}"),
    });
    let mut catalogs = Assets::<InstalledCatalog>::default();
    catalogs.add(InstalledCatalog {
        entries: vec![CatalogEntry {
            decl: ModEntry {
                id: id.to_string(),
                bundle: format!("mods/{id}/{id}.bundle.ron"),
                base: true,
                hidden: false,
            },
            bundle,
        }],
    });
    world.insert_resource(bundles);
    world.insert_resource(catalogs);
    world.insert_resource(EnabledMods([id.to_string()].into_iter().collect()));
    world
}

/// The list a picker shows is the list the MERGE would accept: the declared
/// files of the enabled bundles, under the `dep://` spelling a saved range
/// writes them by.
#[test]
fn the_picker_offers_what_the_installed_bundles_declare() {
    let mut world = installed("base", &["textures/rock.png", "sounds/hit.wav"]);

    let offered = world
        .run_system_once(|files: AssetIndex| files.offers(AssetSort::Image))
        .expect("the index reads");

    assert_eq!(offered, vec!["dep://base/textures/rock.png".to_string()]);
}

/// A mesh ref addresses a SCENE inside the file. The picker writes the label so
/// a builder does not have to know that a `.glb` alone loads nothing.
#[test]
fn a_picked_model_takes_the_scene_label_a_mesh_ref_needs() {
    let mut world = installed("base", &["gltf/hull-01.glb"]);

    let offered = world
        .run_system_once(|files: AssetIndex| files.offers(AssetSort::Model))
        .expect("the index reads");

    assert_eq!(
        offered,
        vec!["dep://base/gltf/hull-01.glb#Scene0".to_string()]
    );
}

/// A disabled mod's files are not offered: naming one would write a ref the
/// merge leaves literal, which fails at load with nothing said in the editor.
#[test]
fn a_mod_that_is_off_offers_nothing() {
    let mut world = installed("base", &["textures/rock.png"]);
    world.insert_resource(EnabledMods::default());

    let offered = world
        .run_system_once(|files: AssetIndex| files.offers(AssetSort::Image))
        .expect("the index reads");

    assert!(offered.is_empty(), "got {offered:?}");
}

/// A `dep://` ref is the one spelling the panel can judge, and it judges it
/// against the same declared list the picker offered.
#[test]
fn a_dep_ref_the_bundle_does_not_declare_is_wrong() {
    let mut world = installed("base", &["textures/rock.png"]);

    let (declared, undeclared, labelled) = world
        .run_system_once(|files: AssetIndex| {
            (
                files.resolves("dep://base/textures/rock.png"),
                files.resolves("dep://base/textures/gone.png"),
                files.resolves("dep://base/textures/rock.png#Scene0"),
            )
        })
        .expect("the index reads");

    assert!(declared, "the file the bundle declares resolves");
    assert!(!undeclared, "a file it does not declare does not");
    assert!(labelled, "the scene label is not part of the file name");
}

/// What the panel CANNOT judge it leaves alone. `self://` resolves against
/// whichever bundle owns the config - which the base game's own sections do,
/// and marking those wrong would paint a whole panel red over nothing.
#[test]
fn a_ref_the_panel_cannot_resolve_is_left_alone() {
    let mut world = installed("base", &["textures/rock.png"]);

    let (own, bare) = world
        .run_system_once(|files: AssetIndex| {
            (
                files.resolves("self://gltf/hull-01.glb#Scene0"),
                files.resolves("textures/loose.png"),
            )
        })
        .expect("the index reads");

    assert!(own, "a self:// ref is scope-dependent, not wrong");
    assert!(
        bare,
        "a bare path addresses the asset root, which is not indexed"
    );
}
