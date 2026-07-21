//! `nova_scenario` is the scenario and modding ENGINE: it turns authored
//! content (RON scenarios and their mod bundles) into running missions.
//! `NovaScenarioPlugin` wires it up, and the modules are the vocabulary a
//! scenario is built from - `events` (what happened), `filters` (which
//! entities and conditions), `actions` (what to do), `variables` (scenario
//! state), `objects` (spawnable scenario entities), `world` (the
//! `NovaEventWorld` holding live scenario state), `loader` (parse + register
//! bundles), `render_scale` (the Low-preset resolution lever), and `lint` (the
//! author-time content checks the `content` CLI runs). This crate is the
//! runtime; the authoring grammar is documented in the scenario-system wiki.
#![warn(missing_docs)]

/// What a handler does when it fires: the action config vocabulary.
pub mod actions;
/// What a handler reacts to: the [`events::EventConfig`] trigger enum.
pub mod events;
/// Which entities and conditions gate a handler: the filter config vocabulary.
pub mod filters;
pub mod lint;
/// Parse, register, and load/unload scenario bundles at runtime.
pub mod loader;
/// Spawnable scenario entities (asteroids, ships, beacons, salvage crates).
pub mod objects;
pub mod render_scale;
/// Typed scenario variables and the small expression tree over them.
pub mod variables;
/// The [`world::NovaEventWorld`] resource holding live scenario state.
pub mod world;

/// Glob-import surface: `use nova_scenario::prelude::*` re-exports the public
/// API of the scenario engine (the module preludes plus [`NovaScenarioPlugin`]).
pub mod prelude {
    pub use super::{
        actions::prelude::*, events::prelude::*, filters::prelude::*, lint::prelude::*,
        loader::prelude::*, objects::prelude::*, render_scale::RenderScalePlugin,
        variables::prelude::*, world::NovaEventWorld, NovaScenarioPlugin,
    };
}

use bevy::prelude::*;
use bevy_common_systems::prelude::*;

/// A plugin that handles Game Events.
pub struct NovaScenarioPlugin {
    /// Whether a window/GPU is present; gates the render-scale lever, which is
    /// skipped on a headless rig with no scenario view to downscale.
    pub render: bool,
}

impl Plugin for NovaScenarioPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaEventsPlugin: build");

        app.add_plugins(GameEventsPlugin::<world::NovaEventWorld>::default());
        app.add_plugins(loader::ScenarioLoaderPlugin);
        app.add_plugins(objects::ScenarioObjectsPlugin {
            render: self.render,
        });

        // The render-scale lever only means anything with a window/GPU; a
        // headless rig (render == false) has no scenario view to downscale.
        if self.render {
            app.add_plugins(render_scale::RenderScalePlugin);
        }
    }
}
