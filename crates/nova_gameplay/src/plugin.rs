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
        // Fail fast on the one external dependency this plugin does NOT add itself.
        // NovaGameplayPlugin brings its own physics, particles, rng and
        // bevy_common_systems plugins, but the spaceship input system is built on
        // bevy_enhanced_input, which the host app must add first (AppBuilder does). If it
        // is missing, panic here at startup with a clear message rather than failing
        // obscurely later when input contexts are registered.
        assert!(
            app.is_plugin_added::<bevy_enhanced_input::EnhancedInputPlugin>(),
            "NovaGameplayPlugin requires bevy_enhanced_input::EnhancedInputPlugin to be \
             added first; the spaceship input system depends on it. Add EnhancedInputPlugin \
             before NovaGameplayPlugin (AppBuilder does this for you)."
        );

        // We need to enable the physics plugins to have access to RigidBody and other components.
        // We will also disable gravity for this example, since we are in space, duh.
        app.add_plugins(PhysicsPlugins::default().with_collision_hooks::<ProjectileHooks>());
        app.add_plugins(PhysicsPickingPlugin);
        app.insert_resource(Gravity::ZERO);

        // The mode the main menu hands off to Playing with; defaults to Sandbox so
        // menu-less apps (all the examples) keep the pre-menu editor behavior.
        app.init_resource::<crate::GameMode>();
        app.register_type::<crate::GameMode>();

        configure_pause_gating(app);

        // Random number generator
        app.add_plugins(EntropyPlugin::<WyRand>::default());

        // FIXME(20260706-162908): For now we disable particle effects on wasm because it's not working
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
        app.add_plugins(crate::integrity::NovaIntegrityPlugin);
        app.add_plugins(crate::flight::NovaFlightPlugin);
        app.add_plugins(crate::gravity::NovaGravityPlugin);
        app.add_plugins(crate::relations::NovaRelationsPlugin);
        app.add_plugins(crate::audio::NovaAudioPlugin);
        app.add_plugins(crate::juice::NovaJuicePlugin);

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

/// While the pause overlay is up nothing may fly or fire: gate the spaceship
/// sets on Unpaused. Run conditions from separate configure_sets calls
/// compose (AND), so this stacks with the editor's Scenario-state gate. The
/// clocks are also paused (nova_menu), but input actions do not consume
/// time - without this gate a held trigger would still spawn projectiles
/// into the frozen world. Factored out so the test below exercises the
/// production wiring (review R1.3).
pub(crate) fn configure_pause_gating(app: &mut App) {
    app.configure_sets(
        Update,
        (SpaceshipInputSystems, SpaceshipSectionSystems)
            .run_if(in_state(crate::PauseStates::Unpaused)),
    );
    app.configure_sets(
        FixedUpdate,
        SpaceshipSectionSystems.run_if(in_state(crate::PauseStates::Unpaused)),
    );
}

#[cfg(test)]
mod pause_gating_tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    #[derive(Resource, Default)]
    struct Ticks(u32);

    /// Review R1.3: the production pause gating must actually stop systems in
    /// the spaceship sets. Probe runs while Unpaused, freezes while Paused,
    /// resumes after (delivery-guarded on both edges).
    #[test]
    fn spaceship_sets_freeze_while_paused() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<crate::PauseStates>();
        app.init_resource::<Ticks>();
        configure_pause_gating(&mut app);
        app.add_systems(
            Update,
            (|mut ticks: ResMut<Ticks>| ticks.0 += 1).in_set(SpaceshipInputSystems),
        );

        app.update();
        assert_eq!(app.world().resource::<Ticks>().0, 1, "runs while Unpaused");

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Paused);
        app.update();
        app.update();
        // StateTransition runs before Update, so the freeze takes effect the
        // same frame the state is set.
        assert_eq!(app.world().resource::<Ticks>().0, 1, "frozen while Paused");

        app.world_mut()
            .resource_mut::<NextState<crate::PauseStates>>()
            .set(crate::PauseStates::Unpaused);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<Ticks>().0,
            3,
            "resumes after unpause"
        );
    }
}
