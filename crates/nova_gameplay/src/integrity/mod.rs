//! Nova's destruction pipeline: the health store, the structural graph, and what
//! nova does when a node dies.
//!
//! - [`health`] is the hit-point store. Nova owns it because the typed-damage
//!   layer must be the one subtracting - see that module's docs.
//! - [`components`] is the graph a destructible structure describes itself with.
//! - [`core`] drives the generic lifecycle over that graph: ram damage in,
//!   disable at zero, destroy leaves, prune, cascade, [`IntegrityDestroyMarker`]
//!   out.
//! - ship-specific adapters declare their graphs and can promote direct
//!   depletion to immediate destruction at any graph degree;
//! - [`explode`] reacts to the destroy marker: slice meshes, spawn debris, fire
//!   `OnDestroyedEvent`.
//! - [`neutralize`] calls a ship combat-dead once its weapons are gone OR its
//!   flight computer is.
//!
//! [`NovaIntegrityPlugin`] bundles the five generic gameplay modules.
//!
//! [`IntegrityDestroyMarker`]: components::IntegrityDestroyMarker

use bevy::prelude::*;

pub mod components;
pub mod core;
pub mod erosion;
pub mod explode;
pub mod health;
pub mod neutralize;

/// Every integrity submodule's prelude plus `NovaIntegrityPlugin`.
pub mod prelude {
    pub use super::{
        components::prelude::*, core::prelude::*, erosion::prelude::*, explode::prelude::*,
        health::prelude::*, neutralize::prelude::*, NovaIntegrityPlugin,
    };
}

/// Nova's generic integrity plugin: health, graph lifecycle, explosion reaction,
/// and combat-death detection. Structure owners publish their own graphs.
pub struct NovaIntegrityPlugin;

impl Plugin for NovaIntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaIntegrityPlugin: build");

        // The hit-point store the core spends, and the destruction pipeline.
        app.add_plugins(health::NovaHealthPlugin);
        app.add_plugins(core::IntegrityCorePlugin);

        // How far gone a body LOOKS, read off the health above. Owns the
        // number only; what it means visually is each damage effect's business.
        app.add_plugins(erosion::DamageLevelPlugin);

        // Nova's reaction to destruction: mesh slice, debris, OnDestroyedEvent.
        app.add_plugins(explode::ExplodablePlugin);

        // Combat-death detection: weapons + thrusters all gone -> neutralized.
        app.add_plugins(neutralize::NeutralizePlugin);
    }
}
