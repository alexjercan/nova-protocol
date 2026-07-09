//! Shared harness for the physics-level integrity tests in `plugin.rs` and `glue.rs`.
//!
//! The avian-free core of the pipeline is unit-tested directly in those modules (driving
//! `ConnectedTo` / `HealthApplyDamage` by hand). These helpers cover the other half: the
//! physics-driven *inputs* - collision/blast damage and `build_integrity_relations`' graph
//! construction - which need a real avian world to produce `ColliderOf` links and
//! `ComputedMass`.

use core::time::Duration;

use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use bevy_common_systems::prelude::*;

use super::NovaIntegrityPlugin;

/// A headless avian app wired with the full integrity pipeline.
///
/// Mirrors avian's own test harness (`MinimalPlugins` + `TransformPlugin` + `AssetPlugin` +
/// `MeshPlugin` + `PhysicsPlugins`); `MeshPlugin` is required because nova enables avian's
/// `collider-from-mesh` feature, whose collider backend reads `AssetEvent<Mesh>` and panics
/// on a `Messages<AssetEvent<Mesh>>` that was never initialized. A fixed manual timestep
/// makes stepping deterministic, and gravity is zeroed so a body stays exactly where the
/// test puts it.
pub(crate) fn integrity_physics_app() -> App {
    let mut app = unfinished_integrity_physics_app();
    app.finish();
    app
}

/// The same harness without `finish()`, for tests that must add further
/// plugins first (the flight tests add the bcs PD controller); the caller
/// finishes the app itself.
pub(crate) fn unfinished_integrity_physics_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        bevy::asset::AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        PhysicsPlugins::default(),
        HealthPlugin,
        NovaIntegrityPlugin,
    ));
    app.insert_resource(Gravity(Vec3::ZERO));
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app
}

/// Step the app enough times for avian to link colliders (`ColliderOf`) and finalize masses
/// (`ComputedMass`). A single update is not enough - mass is computed over the first few
/// steps, and reading it too early yields `NaN`.
pub(crate) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}
