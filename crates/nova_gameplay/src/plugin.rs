use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::prelude::*;

use crate::{bevy_common_systems, prelude::*};

/// A system set that will contain all the systems related to the spaceship plugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpaceshipSystems {
    First,
    Last,
}

#[derive(Default, Clone, Debug)]
pub struct NovaGameplayPlugin {
    pub render: bool,
}

impl Plugin for NovaGameplayPlugin {
    fn build(&self, app: &mut App) {
        // We need to enable the physics plugins to have access to RigidBody and other components.
        // We will also disable gravity for this example, since we are in space, duh.
        app.add_plugins(PhysicsPlugins::default().with_collision_hooks::<TurretProjectileHooks>());
        app.add_plugins(PhysicsPickingPlugin);
        app.insert_resource(Gravity::ZERO);

        // Random number generator
        app.add_plugins(EntropyPlugin::<WyRand>::default());

        // FIXME: For now we disable particle effects on wasm because it's not working
        #[cfg(not(target_family = "wasm"))]
        app.add_plugins(bevy_hanabi::HanabiPlugin);

        // Bevy Common Systems - WASD Camera
        app.add_plugins(bevy_common_systems::prelude::WASDCameraPlugin);
        app.add_plugins(bevy_common_systems::prelude::WASDCameraControllerPlugin);
        // Chase Camera Plugin to have a 3rd person camera following the spaceship
        app.add_plugins(bevy_common_systems::prelude::ChaseCameraPlugin);
        // Bevy Common Systems - Rendering
        app.add_plugins(bevy_common_systems::prelude::SkyboxPlugin);
        app.add_plugins(bevy_common_systems::prelude::PostProcessingDefaultPlugin);
        // Point Rotation Plugin to convert linear movement to a target rotation
        app.add_plugins(bevy_common_systems::prelude::PointRotationPlugin);
        // for debug to have a random orbiting object
        app.add_plugins(bevy_common_systems::prelude::SphereRandomOrbitPlugin);
        // Rotation Plugin for the turret facing direction
        app.add_plugins(bevy_common_systems::prelude::SmoothLookRotationPlugin);
        // Sphere Orbit Plugin
        app.add_plugins(bevy_common_systems::prelude::SphereOrbitPlugin);
        app.add_plugins(bevy_common_systems::prelude::DirectionalSphereOrbitPlugin);
        // Other helper plugins
        app.add_plugins(bevy_common_systems::prelude::TempEntityPlugin);
        app.add_plugins(bevy_common_systems::prelude::DespawnEntityPlugin);
        app.add_plugins(bevy_common_systems::prelude::ExplodeMeshPlugin);
        // Core Mechanics
        app.add_plugins(bevy_common_systems::prelude::PDControllerPlugin);
        app.add_plugins(bevy_common_systems::prelude::CollisionImpactPlugin);
        app.add_plugins(bevy_common_systems::prelude::HealthPlugin);

        // UI Plugins
        app.add_plugins(bevy_common_systems::prelude::StatusBarPlugin);

        // Core Plugins for simulation
        app.add_plugins(crate::input::SpaceshipInputPlugin);
        app.add_plugins(crate::sections::SpaceshipSectionPlugin {
            render: self.render,
        });
        app.add_plugins(crate::hud::NovaHudPlugin);
        app.add_plugins(crate::camera_controller::SpaceshipCameraControllerPlugin);
        app.add_plugins(crate::damage::DamagePlugin);

        // Diagnostics
        if !app.is_plugin_added::<bevy::diagnostic::FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
        }

        // Configure system Sets
        app.configure_sets(
            Update,
            (
                SpaceshipSystems::First,
                SpaceshipInputSystems,
                SpaceshipSectionSystems,
                NovaHudSystems,
                NovaCameraSystems,
                SpaceshipSystems::Last,
            )
                .chain(),
        );

        app.configure_sets(
            FixedUpdate,
            (
                SpaceshipSystems::First,
                SpaceshipInputSystems,
                SpaceshipSectionSystems,
                NovaHudSystems,
                NovaCameraSystems,
                SpaceshipSystems::Last,
            )
                .chain(),
        );
    }
}
