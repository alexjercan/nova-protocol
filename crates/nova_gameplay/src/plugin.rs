//! The gameplay composition root: [`NovaGameplayPlugin`] adds every gameplay
//! subsystem plugin (input, sections, hud, camera, integrity, damage, flight,
//! gravity, relations, audio, juice, settings) plus the third-party plugins
//! they depend on (avian3d physics with [`ProjectileHooks`] collision hooks,
//! `bevy_hanabi` particles, and `bevy_rand` entropy). Every gameplay layer it
//! composes is nova's own. It also pins the top-level [`SpaceshipSystems`] set ordering that
//! the per-subsystem sets chain inside.
//!
//! The one dependency it does NOT add itself is
//! `bevy_enhanced_input::EnhancedInputPlugin` (the spaceship input contexts are
//! built on it); the host app must add that first and this plugin asserts it at
//! `build` time. See the architecture wiki for how this crate sits between
//! `nova_core` (wiring) and its neighbors.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::prelude::*;

use crate::prelude::*;

/// Top-level ordering brackets for gameplay: every per-subsystem set
/// ([`SpaceshipInputSystems`], [`SpaceshipSectionSystems`], and the rest)
/// chains between [`First`](SpaceshipSystems::First) and
/// [`Last`](SpaceshipSystems::Last) in both `Update` and `FixedUpdate`. Use
/// these to run a system strictly before or after all of gameplay in a frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpaceshipSystems {
    /// Runs before every gameplay subsystem set in the frame.
    First,
    /// Runs after every gameplay subsystem set in the frame.
    Last,
}

/// Composes all of gameplay into one `App`. Add this (after
/// `EnhancedInputPlugin`) to get a flyable, fightable ship; `AppBuilder` wires
/// it for the full game and the examples add it directly.
#[derive(Default, Clone, Debug)]
pub struct NovaGameplayPlugin {
    /// Whether the render-side plugins are added: section meshes, the HUD and
    /// the rest of the `nova_ui` wiring, hanabi particles, the skybox and
    /// post-processing. `false` for headless / harness runs that only need the
    /// simulation.
    pub render: bool,
}

impl Plugin for NovaGameplayPlugin {
    fn build(&self, app: &mut App) {
        // Fail fast on the one external dependency this plugin does NOT add itself.
        // NovaGameplayPlugin brings its own physics, particles and rng
        // plugins, but the spaceship input system is built on
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

        // Mission state, not HUD state: the scenario loader writes it whether or
        // not anything renders it, so it is owned here rather than by the
        // render-gated HUD.
        app.init_resource::<crate::objectives::GameObjectives>();

        configure_pause_gating(app);

        // Random number generator
        app.add_plugins(EntropyPlugin::<WyRand>::default());

        // Hanabi particles run on every target: native, and wasm via the WebGPU
        // backend (compute shaders; see nova_core's wasm webgpu feature).
        if self.render {
            app.add_plugins(bevy_hanabi::HanabiPlugin);
        }

        // WASD free-fly camera and the input controller over it
        app.add_plugins(crate::camera::wasd::WASDCameraPlugin);
        app.add_plugins(crate::camera::wasd_controller::WASDCameraControllerPlugin);
        // Chase Camera Plugin to have a 3rd person camera following the spaceship
        app.add_plugins(crate::camera::chase::ChaseCameraPlugin);
        // Skybox and post-processing
        if self.render {
            app.add_plugins(crate::camera::skybox::SkyboxPlugin);
            app.add_plugins(crate::camera::post::PostProcessingDefaultPlugin);
        }
        // Point Rotation Plugin to convert linear movement to a target rotation
        app.add_plugins(crate::transform::prelude::PointRotationPlugin);
        // for debug to have a random orbiting object
        app.add_plugins(crate::transform::prelude::SphereRandomOrbitPlugin);
        // Rotation Plugin for the turret facing direction
        app.add_plugins(crate::transform::prelude::SmoothLookRotationPlugin);
        // Sphere Orbit Plugin
        app.add_plugins(crate::transform::prelude::SphereOrbitPlugin);
        app.add_plugins(crate::transform::prelude::DirectionalSphereOrbitPlugin);
        // Other helper plugins
        app.add_plugins(crate::lifetime::TempEntityPlugin);
        app.add_plugins(crate::lifetime::DespawnEntityPlugin);
        app.add_plugins(crate::mesh::prelude::ExplodeMeshPlugin);
        // Core Mechanics
        app.add_plugins(crate::physics::prelude::PDControllerPlugin);

        // UI Plugins. The menu and editor want the same app-global UI wiring;
        // whoever gets there first adds it.
        if self.render && !app.is_plugin_added::<nova_ui::NovaUiPlugin>() {
            app.add_plugins(nova_ui::NovaUiPlugin);
        }

        // Core Plugins for simulation
        app.add_plugins(crate::input::SpaceshipInputPlugin);
        app.add_plugins(crate::sections::SpaceshipSectionPlugin {
            render: self.render,
        });
        if self.render {
            app.add_plugins(crate::hud::NovaHudPlugin);
        }
        app.add_plugins(crate::camera::SpaceshipCameraControllerPlugin);
        app.add_plugins(crate::integrity::NovaIntegrityPlugin);
        app.add_plugins(crate::damage::NovaDamagePlugin);
        app.add_plugins(crate::flight::NovaFlightPlugin);
        app.add_plugins(crate::gravity::NovaGravityPlugin);
        app.add_plugins(crate::relations::NovaRelationsPlugin);
        app.add_plugins(crate::audio::NovaAudioPlugin);
        app.add_plugins(crate::juice::NovaJuicePlugin);
        app.add_plugins(crate::settings::NovaSettingsPlugin);

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
/// sets on Unpaused. Run conditions from separate configure_sets calls compose
/// (AND), so this stacks with nova_scenario's scenario_is_live gate. The clocks
/// are also paused (nova_menu), but input actions do not consume time - without
/// this gate a held trigger would still spawn projectiles into the frozen
/// world. Factored out so the test below exercises the production wiring.
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

    /// The production pause gating must actually stop systems in
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
