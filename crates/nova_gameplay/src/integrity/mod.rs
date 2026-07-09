//! Nova's integrity wiring.
//!
//! The generic destruction pipeline (the graph model, blast volume, collision/blast damage,
//! disable/destroy/leaf/chain logic) now lives in `bevy_common_systems::integrity`
//! ([`IntegrityPlugin`](bevy_common_systems::prelude::IntegrityPlugin)). What stays here is
//! the nova-specific glue on either end of its seam:
//!
//! - [`glue`] builds the integrity graph from the ship section grid and rolls section health
//!   up to the ship root;
//! - [`explode`] reacts to `IntegrityDestroyMarker` (the generic "destroyed" signal) to slice
//!   meshes, spawn debris, and fire the nova `OnDestroyedEvent`.
//!
//! [`NovaIntegrityPlugin`] bundles the generic core with these two, so the rest of nova adds
//! one plugin as before.

use bevy::prelude::*;
use bevy_common_systems::prelude::IntegrityPlugin;

pub mod explode;
pub mod glue;
#[cfg(test)]
pub(crate) mod test_support;

pub mod prelude {
    pub use super::{explode::prelude::*, NovaIntegrityPlugin};
}

/// Nova's integrity plugin: the generic `bevy_common_systems` destruction core plus nova's
/// section-graph glue and explosion reaction.
pub struct NovaIntegrityPlugin;

impl Plugin for NovaIntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("NovaIntegrityPlugin: build");

        // The generic destruction pipeline.
        app.add_plugins(IntegrityPlugin);

        // Section-specific graph construction, section disable, and aggregate ship health.
        app.add_plugins(glue::IntegrityGluePlugin);

        // Nova's reaction to destruction: mesh slice, debris, OnDestroyedEvent.
        app.add_plugins(explode::ExplodablePlugin);
    }
}
