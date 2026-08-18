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
//! - [`explode`] reacts to the destroy marker: detach the dead body from its
//!   parent so it tumbles away whole, and fire `OnDestroyedEvent`.
//! - [`neutralize`] calls a ship combat-dead once its weapons are gone OR its
//!   flight computer is.
//! - [`erosion`] reads health as a level, for effects that grade a whole body.
//! - [`carve`] remembers where the hits landed, for effects that change a
//!   body's shape.
//! - [`spew`] throws the dust a carve knocked off; [`chunk`] puts the PIECES
//!   into the world as bodies of their own.
//!
//! [`NovaIntegrityPlugin`] bundles the generic gameplay modules.
//!
//! [`IntegrityDestroyMarker`]: components::IntegrityDestroyMarker

use bevy::prelude::*;

pub mod carve;
pub mod chunk;
pub mod components;
pub mod core;
pub mod erosion;
pub mod explode;
pub mod health;
pub mod neutralize;
pub mod spew;

/// Every integrity submodule's prelude plus `NovaIntegrityPlugin`.
pub mod prelude {
    pub use super::{
        carve::prelude::*, chunk::prelude::*, components::prelude::*, core::prelude::*,
        erosion::prelude::*, explode::prelude::*, health::prelude::*, neutralize::prelude::*,
        spew::prelude::*, NovaIntegrityPlugin,
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

        // The two damage READINGS, and neither is a look of its own. The level
        // says how far gone a body is (grading whole-body effects: cracks,
        // sparks); the marks say where it was hit (driving anything that has to
        // change a body's shape). See `carve` for why one number could never
        // do both.
        app.add_plugins(erosion::DamageLevelPlugin);
        app.add_plugins(carve::DamageMarksPlugin);

        // What a carve took off, seen leaving. A crater that opened in silence
        // reads as a rendering glitch rather than as a hit. Dust from `spew`,
        // and real pieces from `chunk` once there is enough material to be
        // worth simulating.
        app.add_plugins(spew::CarveSpewPlugin);
        app.add_plugins(chunk::CarvedChunkPlugin);

        // Nova's reaction to destruction: detach the dead body, OnDestroyedEvent.
        app.add_plugins(explode::ExplodablePlugin);

        // Combat-death detection: weapons + thrusters all gone -> neutralized.
        app.add_plugins(neutralize::NeutralizePlugin);
    }
}
