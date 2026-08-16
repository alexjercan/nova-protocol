//! The ship and how it is flown. `sections` is the modular hull the player
//! builds (hull, thruster, turret, torpedo, controller, their ammo and damage
//! tint), `input` is everything that commands one (player rigs, the AI pilot
//! and gunner, radar and component targeting), `flight` is the diegetic
//! controller and its autopilot verbs, `camera` is the rigs that watch it,
//! `physics` is the PD attitude controller the hull steers with, and
//! `ship_audio` is the soundtrack those five produce.
//!
//! [`NovaShipPlugin`] composes all of it and owns the [`SpaceshipSystems`]
//! ordering brackets, so the seam runs `nova_ship -> nova_gameplay` and never
//! the reverse: the shared gameplay layer below (integrity, damage, gravity,
//! the SFX engine, the markers this crate tags entities with) knows nothing
//! about a ship. Add it after `NovaGameplayPlugin`; `AppBuilder` does.
#![warn(missing_docs)]

use bevy::prelude::*;
use nova_gameplay::prelude::*;

pub mod camera;
pub mod flight;
pub mod input;
pub mod physics;
pub mod sections;
// Private: the soundtrack is wired by `NovaShipPlugin` and has no consumer
// outside this crate, so nothing about it is public API.
mod ship_audio;

/// Glob-import surface: `use nova_ship::prelude::*` re-exports the public API
/// of this crate's submodules.
pub mod prelude {
    // Re-export BY NAME, never by glob over another crate's prelude. This one
    // deliberately does NOT re-export `nova_gameplay`'s - a consumer that
    // needs both imports both, the precedent `nova_hud` set.
    pub use super::{
        camera::prelude::*, flight::prelude::*, input::prelude::*, physics::prelude::*,
        sections::prelude::*, NovaShipPlugin,
    };
}

/// Composes the ship: sections, input, flight, the camera rigs, the PD
/// controller and the ship's soundtrack. Add it after `NovaGameplayPlugin`
/// (which brings the physics, entropy and SFX engine it builds on); the
/// examples add both directly and `AppBuilder` wires them for the full game.
#[derive(Default, Clone, Debug)]
pub struct NovaShipPlugin {
    /// Whether the render-side plugins are added: section meshes, the skybox
    /// and post-processing. `false` for headless / harness runs that only need
    /// the simulation. Mirrors `NovaGameplayPlugin::render`.
    pub render: bool,
}

impl Plugin for NovaShipPlugin {
    fn build(&self, app: &mut App) {
        // Fail fast on the one external dependency this plugin does NOT add
        // itself. The spaceship input contexts are built on
        // bevy_enhanced_input, which the host app must add first (AppBuilder
        // does). If it is missing, panic here at startup with a clear message
        // rather than failing obscurely later when input contexts are
        // registered.
        assert!(
            app.is_plugin_added::<bevy_enhanced_input::EnhancedInputPlugin>(),
            "NovaShipPlugin requires bevy_enhanced_input::EnhancedInputPlugin to be \
             added first; the spaceship input system depends on it. Add EnhancedInputPlugin \
             before NovaShipPlugin (AppBuilder does this for you)."
        );

        // WASD free-fly camera and the input controller over it
        app.add_plugins(camera::wasd::WASDCameraPlugin);
        app.add_plugins(camera::wasd_controller::WASDCameraControllerPlugin);
        // Chase Camera Plugin to have a 3rd person camera following the spaceship
        app.add_plugins(camera::chase::ChaseCameraPlugin);
        // Skybox and post-processing
        if self.render {
            app.add_plugins(camera::skybox::SkyboxPlugin);
            app.add_plugins(camera::post::PostProcessingDefaultPlugin);
        }

        // Core Mechanics
        app.add_plugins(physics::prelude::PDControllerPlugin);

        // Core Plugins for simulation
        app.add_plugins(input::SpaceshipInputPlugin {
            render: self.render,
        });
        app.add_plugins(sections::SpaceshipSectionPlugin {
            render: self.render,
        });
        app.add_plugins(camera::SpaceshipCameraControllerPlugin);
        app.add_plugins(flight::NovaFlightPlugin);
        app.add_plugins(ship_audio::ShipAudioPlugin);

        // Configure system Sets
        app.configure_sets(
            Update,
            (
                SpaceshipSystems::First,
                input::SpaceshipInputSystems,
                sections::SpaceshipSectionSystems,
                camera::NovaCameraSystems,
                SpaceshipSystems::Last,
            )
                .chain(),
        );

        app.configure_sets(
            FixedUpdate,
            (
                SpaceshipSystems::First,
                input::SpaceshipInputSystems,
                sections::SpaceshipSectionSystems,
                camera::NovaCameraSystems,
                SpaceshipSystems::Last,
            )
                .chain(),
        );

        configure_pause_gating(app);
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
        (
            input::SpaceshipInputSystems,
            sections::SpaceshipSectionSystems,
        )
            .run_if(in_state(PauseStates::Unpaused)),
    );
    app.configure_sets(
        FixedUpdate,
        (
            input::SpaceshipInputSystems,
            sections::SpaceshipSectionSystems,
        )
            .run_if(in_state(PauseStates::Unpaused)),
    );
}

#[cfg(test)]
mod pause_gating_tests {
    use std::time::Duration;

    use bevy::{
        state::app::StatesPlugin,
        time::{TimePlugin, TimeUpdateStrategy},
    };

    use super::*;

    #[derive(Resource, Default)]
    struct Ticks(u32);

    #[derive(Resource, Default)]
    struct FixedTicks(u32);

    /// The production pause gating must actually stop systems in
    /// the spaceship sets. Probe runs while Unpaused, freezes while Paused,
    /// resumes after (delivery-guarded on both edges).
    #[test]
    fn spaceship_sets_freeze_while_paused() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<PauseStates>();
        app.init_resource::<Ticks>();
        configure_pause_gating(&mut app);
        app.add_systems(
            Update,
            (|mut ticks: ResMut<Ticks>| ticks.0 += 1).in_set(input::SpaceshipInputSystems),
        );

        app.update();
        assert_eq!(app.world().resource::<Ticks>().0, 1, "runs while Unpaused");

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        app.update();
        // StateTransition runs before Update, so the freeze takes effect the
        // same frame the state is set.
        assert_eq!(app.world().resource::<Ticks>().0, 1, "frozen while Paused");

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<Ticks>().0,
            3,
            "resumes after unpause"
        );
    }

    /// The FixedUpdate half of the gate carries `update_fire_cadence`, so a
    /// paused AI ship must not advance its burst clock. Asserted separately
    /// from the Update probe above: the two schedules are configured by two
    /// `configure_sets` calls, and only this one covers the fixed clock.
    #[test]
    fn spaceship_sets_freeze_in_fixed_update_while_paused() {
        let mut app = App::new();
        app.add_plugins((TimePlugin, StatesPlugin));
        app.init_state::<PauseStates>();
        app.init_resource::<FixedTicks>();
        // One fixed step per update, so every phase below advances the clock.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 64.0,
        )));
        configure_pause_gating(&mut app);
        app.add_systems(
            FixedUpdate,
            (|mut ticks: ResMut<FixedTicks>| ticks.0 += 1).in_set(input::SpaceshipInputSystems),
        );

        // The first update lands a zero delta, so warm up before measuring.
        app.update();
        app.update();
        let unpaused = app.world().resource::<FixedTicks>().0;
        assert!(unpaused > 0, "runs in FixedUpdate while Unpaused");

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Paused);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<FixedTicks>().0,
            unpaused,
            "frozen in FixedUpdate while Paused"
        );

        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Unpaused);
        app.update();
        app.update();
        assert!(
            app.world().resource::<FixedTicks>().0 > unpaused,
            "resumes in FixedUpdate after unpause"
        );
    }
}
