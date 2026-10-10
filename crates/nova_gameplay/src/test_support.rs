//! The crate's headless avian test app, behind the `test-support` feature.
//!
//! The avian-free halves of nova's pipelines are unit-tested directly in their
//! own modules (driving `ConnectedTo` / `HealthApplyDamage` by hand). These
//! helpers cover the other half - anything whose inputs are physics-driven
//! (collision and blast damage, ship graph publication, the flight controller,
//! the turret and torpedo sections) and
//! so needs a real avian world to produce `ColliderOf` links and
//! `ComputedMass`.
//!
//! Behind a FEATURE, not `#[cfg(test)]`: a `cfg(test)` module is invisible
//! across a crate boundary, and the crates split out of `nova_gameplay` test
//! against this same harness. The feature keeps avian's test wiring out of
//! every release build, which a plain `pub` would not.

use core::time::Duration;

use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};

use crate::integrity::NovaIntegrityPlugin;

/// A headless avian app wired with the full integrity pipeline.
///
/// Mirrors avian's own test harness (`MinimalPlugins` + `TransformPlugin` + `AssetPlugin` +
/// `MeshPlugin` + `PhysicsPlugins`); `MeshPlugin` is required because nova enables avian's
/// `collider-from-mesh` feature, whose collider backend reads `AssetEvent<Mesh>` and panics
/// on a `Messages<AssetEvent<Mesh>>` that was never initialized. A fixed manual timestep
/// makes stepping deterministic, and gravity is zeroed so a body stays exactly where the
/// test puts it.
pub fn integrity_physics_app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.finish();
    app
}

/// The same harness without `finish()`, for tests that must add further
/// plugins first (the flight tests add the bcs PD controller); the caller
/// finishes the app itself.
pub fn unfinished_integrity_physics_app() -> App {
    unfinished_integrity_physics_app_with(PhysicsPlugins::default())
}

/// The same harness with a caller-supplied physics plugin group, for tests
/// that need a non-default physics setup (the projectile-hook tests register
/// `with_collision_hooks`, as `NovaGameplayPlugin` does).
pub fn unfinished_integrity_physics_app_with(physics: impl PluginGroup) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        bevy::asset::AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
    ));
    app.add_plugins(physics);
    // NovaIntegrityPlugin brings the health store with it.
    app.add_plugins(NovaIntegrityPlugin);
    app.insert_resource(Gravity(Vec3::ZERO));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app
}

/// Step the app enough times for avian to link colliders (`ColliderOf`) and finalize masses
/// (`ComputedMass`). A single update is not enough - mass is computed over the first few
/// steps, and reading it too early yields `NaN`.
pub fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// An item catalog with the base game's ten items and their base values, for
/// tests that move stock without merging content. Mirrors the base content
/// builder (`nova_authoring`'s `base_content::items`); a test that reads a
/// mass or a price reads it through this catalog, never as a literal.
pub fn test_items() -> crate::inventory::GameItems {
    use crate::inventory::{ItemCategoryType::*, *};
    let item = |id: &str, name: &str, category, mass_g, ask_cr, bid_cr| ItemDesign {
        id: ItemDesignId::from(id),
        name: name.to_string(),
        about: format!("{name} for tests."),
        category,
        mass_g,
        ask_cr,
        bid_cr,
    };
    GameItems::new([
        item(ITEM_HULL_PLATE, "Hull plate", Repair, 10_000, 40, 30),
        item(ITEM_PDC_ROUND, "PDC round", Ammo, 200, 4, 3),
        item(ITEM_RAIL_SLUG, "Rail slug", Ammo, 20_000, 40, 30),
        item(ITEM_TORPEDO, "Torpedo", Ammo, 150_000, 400, 300),
        item(ITEM_STONE_ORE, "Stone ore", Raw, 10_000, 4, 3),
        item(ITEM_IRON_ORE, "Iron ore", Raw, 10_000, 16, 12),
        item(ITEM_WATER_ICE, "Water ice", Raw, 10_000, 12, 9),
        item(ITEM_CARBON_ORE, "Carbon ore", Raw, 10_000, 8, 6),
        item(ITEM_RATIONS, "Rations", Food, 2_000, 8, 6),
        item(
            ITEM_SALVAGED_PARTS,
            "Salvaged parts",
            Parts,
            25_000,
            120,
            90,
        ),
    ])
}
