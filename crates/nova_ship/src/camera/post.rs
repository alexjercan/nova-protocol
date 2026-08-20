//! A plugin that applies default post processing settings to 3D cameras.
//!
//! This plugin automatically enables tonemapping and bloom on any `Camera3d`
//! entity marked with the `PostProcessingCamera` component. It is meant to
//! provide a simple and visually appealing default look without requiring
//! any manual setup.
//!
//! The current defaults are:
//! - Tonemapping::TonyMcMapface
//! - Bloom::NATURAL
//!
//! Example usage:
//!
//! ```rust
//! # use bevy::prelude::*;
//! # use nova_ship::prelude::{PostProcessingCamera, PostProcessingDefaultPlugin};
//! # fn demo(mut commands: Commands) {
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugins(PostProcessingDefaultPlugin);
//!
//! // Cameras marked with PostProcessingCamera receive bloom and tonemapping:
//! commands.spawn((Camera3d::default(), PostProcessingCamera));
//! # }
//! ```
//!
//! If you want different defaults or more control over post processing,
//! consider writing your own plugin or inserting the components manually.

use bevy::{core_pipeline::tonemapping::Tonemapping, post_process::bloom::Bloom, prelude::*};

/// Glob-import surface for the post-processing camera defaults.
pub mod prelude {
    pub use super::{PostProcessingCamera, PostProcessingDefaultPlugin};
}

/// Post Processing Camera settings.
#[derive(Component, Clone, Debug, Reflect)]
pub struct PostProcessingCamera;

/// A plugin that applies default post processing settings.
///
/// When a `Camera3d` is added to an entity, this plugin automatically inserts
/// `Tonemapping::TonyMcMapface` and `Bloom::NATURAL`.
pub struct PostProcessingDefaultPlugin;

impl Plugin for PostProcessingDefaultPlugin {
    fn build(&self, app: &mut App) {
        trace!("PostProcessingDefaultPlugin: build");

        app.add_observer(setup_post_processing_camera);
    }
}

fn setup_post_processing_camera(
    insert: On<Insert, PostProcessingCamera>,
    mut commands: Commands,
    q_camera: Query<&PostProcessingCamera, With<Camera3d>>,
) {
    let entity = insert.entity;
    trace!("setup_post_processing_camera: entity {:?}", entity);

    let Ok(_) = q_camera.get(entity) else {
        error!(
            "setup_post_processing_camera: entity {:?} not found in q_camera",
            entity
        );
        return;
    };

    commands
        .entity(entity)
        .insert((Tonemapping::TonyMcMapface, Bloom::NATURAL));
}
