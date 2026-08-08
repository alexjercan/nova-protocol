//! Nova's destruction pipeline: the health store, the structural graph, and what
//! nova does when a node dies.
//!
//! - [`health`] is the hit-point store. Nova owns it because the typed-damage
//!   layer must be the one subtracting - see that module's docs.
//! - [`components`] is the graph a destructible structure describes itself with.
//! - [`core`] drives the lifecycle over that graph: ram damage in, disable at
//!   zero, destroy leaves, prune, cascade, [`IntegrityDestroyMarker`] out.
//! - [`glue`] builds the graph from the ship section grid and rolls section
//!   health up to the ship root.
//! - [`explode`] reacts to the destroy marker: slice meshes, spawn debris, fire
//!   `OnDestroyedEvent`.
//! - [`neutralize`] calls a ship combat-dead once its weapons and thrusters are
//!   gone.
//!
//! [`NovaIntegrityPlugin`] bundles all six.
//!
//! [`IntegrityDestroyMarker`]: components::IntegrityDestroyMarker

use bevy::prelude::*;

pub mod components;
pub mod core;
pub mod explode;
pub mod glue;
pub mod health;
pub mod neutralize;
#[cfg(test)]
pub(crate) mod test_support;

/// Every integrity submodule's prelude plus `NovaIntegrityPlugin`.
pub mod prelude {
    pub use super::{
        components::prelude::*, core::prelude::*, explode::prelude::*, health::prelude::*,
        neutralize::prelude::*, NovaIntegrityPlugin,
    };
}

/// Nova's integrity plugin: the health store, the destruction core, and nova's
/// section-graph glue, explosion reaction and combat-death detection.
pub struct NovaIntegrityPlugin;

impl Plugin for NovaIntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaIntegrityPlugin: build");

        // The hit-point store the core spends, and the destruction pipeline.
        app.add_plugins(health::NovaHealthPlugin);
        app.add_plugins(core::IntegrityCorePlugin);

        // Section-specific graph construction, section disable, and aggregate ship health.
        app.add_plugins(glue::IntegrityGluePlugin);

        // Nova's reaction to destruction: mesh slice, debris, OnDestroyedEvent.
        app.add_plugins(explode::ExplodablePlugin);

        // Combat-death detection: weapons + thrusters all gone -> neutralized.
        app.add_plugins(neutralize::NeutralizePlugin);
    }
}
