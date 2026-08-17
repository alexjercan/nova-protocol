//! The things a scenario can place in the world - asteroids, beacons, lights,
//! salvage, spaceships, trigger areas - one module each.
//!
//! Each submodule owns its authored config, its spawn bundle and its plugin;
//! [`ScenarioObjectsPlugin`] adds them all and is the only registration point.
//!
//! Touch this module when adding a new kind of authored world object.

/// Anchor scenario object: an invisible authored point publishing a
/// deterministic gravity well (camera framing, orbit targets) with no body.
pub mod anchor;
/// Scenario trigger areas: sensor volumes that fire `OnEnter`/`OnExit` events.
pub mod area;
/// Asteroid scenario object: noise-generated rocks that can act as gravity wells.
pub mod asteroid;
/// The signed field behind a carvable rock, and the remesh that follows a hit.
pub mod asteroid_carve;
pub mod beacon;
pub mod binding_input;
/// Light scenario object: the authored directional and point lights a scene
/// lights itself with.
pub mod light;
pub mod modification;
pub mod salvage;
/// The ship CONTENT kind: a hull authored once and spawned by id.
pub mod ship;
/// Spaceship scenario object: player/AI ships built from a section list.
pub mod spaceship;

/// Every scenario object submodule's prelude plus `ScenarioObjectsPlugin`.
pub mod prelude {
    pub use super::{
        anchor::prelude::*, area::prelude::*, asteroid::prelude::*, asteroid_carve::prelude::*,
        beacon::prelude::*, binding_input::prelude::*, light::prelude::*, modification::prelude::*,
        salvage::prelude::*, ship::prelude::*, spaceship::prelude::*, ScenarioObjectsPlugin,
    };
}

use bevy::prelude::*;

/// Aggregates the scenario-object plugins (asteroid, spaceship, area, beacon,
/// salvage crate, light) into one group. `render` is threaded to the
/// render-bearing members so headless tools can spawn objects without their
/// visuals.
/// Adds each object type's own plugin (see [`asteroid::AsteroidPlugin`],
/// [`spaceship::SpaceshipPlugin`], [`area::ScenarioAreaPlugin`],
/// [`beacon::BeaconPlugin`], [`salvage::SalvageCratePlugin`],
/// [`light::LightPlugin`]) at build time.
pub struct ScenarioObjectsPlugin {
    /// Whether the render-bearing object plugins spawn their visuals (false for headless tools).
    pub render: bool,
}

impl Plugin for ScenarioObjectsPlugin {
    fn build(&self, app: &mut App) {
        debug!("ScenarioObjectsPlugin: build");

        app.add_plugins(anchor::AnchorPlugin);
        app.add_plugins(asteroid::AsteroidPlugin {
            render: self.render,
        });
        app.add_plugins(asteroid_carve::AsteroidCarvePlugin {
            render: self.render,
        });
        app.add_plugins(spaceship::SpaceshipPlugin);
        app.add_plugins(area::ScenarioAreaPlugin);
        app.add_plugins(beacon::BeaconPlugin {
            render: self.render,
        });
        app.add_plugins(salvage::SalvageCratePlugin {
            render: self.render,
        });
        app.add_plugins(light::LightPlugin {
            render: self.render,
        });
    }
}
