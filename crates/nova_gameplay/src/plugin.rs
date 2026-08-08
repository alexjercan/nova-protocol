//! The gameplay composition root: [`NovaGameplayPlugin`] adds every shared
//! gameplay plugin (integrity, damage, gravity, relations, audio, juice,
//! transform, lifetime, mesh, settings) plus the third-party plugins the whole
//! game builds on (avian3d physics with [`ProjectileHooks`] collision hooks,
//! `bevy_hanabi` particles, and `bevy_rand` entropy). Every gameplay layer it
//! composes is nova's own. It also declares the top-level [`SpaceshipSystems`]
//! brackets; `nova_ship` chains its per-subsystem sets inside them.
//!
//! See the architecture wiki for how this crate sits between `nova_core`
//! (wiring) and its neighbors.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::prelude::*;

use crate::prelude::*;

/// Glob-import surface: `use nova_gameplay::plugin::prelude::*`.
pub mod prelude {
    pub use super::{NovaGameplayPlugin, SpaceshipSystems};
}

/// Top-level ordering brackets for gameplay: every per-subsystem set
/// (`nova_ship`'s `SpaceshipInputSystems`, `SpaceshipSectionSystems`, and the
/// rest) chains between [`First`](SpaceshipSystems::First) and
/// [`Last`](SpaceshipSystems::Last) in both `Update` and `FixedUpdate`. Use
/// these to run a system strictly before or after all of gameplay in a frame.
/// Declared here rather than in `nova_ship` so a ship-less app (the editor's
/// scenery, a headless harness) can still bracket against gameplay.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpaceshipSystems {
    /// Runs before every gameplay subsystem set in the frame.
    First,
    /// Runs after every gameplay subsystem set in the frame.
    Last,
}

/// Composes the shared gameplay layer into one `App`. Add `nova_ship`'s
/// `NovaShipPlugin` after it for a flyable, fightable ship; `AppBuilder` wires
/// both for the full game and the examples add them directly.
#[derive(Default, Clone, Debug)]
pub struct NovaGameplayPlugin {
    /// Whether the render-side plugins are added: the `nova_ui` wiring and
    /// hanabi particles. `false` for headless / harness runs that only need the
    /// simulation. The HUD (`nova_hud`) and the ship (`nova_ship`) are peer
    /// crates, so whoever adds them applies the same gate.
    pub render: bool,
}

impl Plugin for NovaGameplayPlugin {
    fn build(&self, app: &mut App) {
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

        // Random number generator
        app.add_plugins(EntropyPlugin::<WyRand>::default());

        // Hanabi particles run on every target: native, and wasm via the WebGPU
        // backend (compute shaders; see nova_core's wasm webgpu feature).
        if self.render {
            app.add_plugins(bevy_hanabi::HanabiPlugin);
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
        // UI Plugins. The menu and editor want the same app-global UI wiring;
        // whoever gets there first adds it.
        if self.render && !app.is_plugin_added::<nova_ui::NovaUiPlugin>() {
            app.add_plugins(nova_ui::NovaUiPlugin);
        }

        // Core Plugins for simulation
        app.add_plugins(crate::integrity::NovaIntegrityPlugin);
        app.add_plugins(crate::damage::NovaDamagePlugin);
        app.add_plugins(crate::gravity::NovaGravityPlugin);
        app.add_plugins(crate::relations::NovaRelationsPlugin);
        app.add_plugins(crate::audio::NovaAudioPlugin);
        app.add_plugins(crate::juice::NovaJuicePlugin);
        app.add_plugins(crate::settings::NovaSettingsPlugin);

        // Diagnostics
        if !app.is_plugin_added::<bevy::diagnostic::FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
        }
    }
}
