pub mod area;
pub mod asteroid;
pub mod beacon;
pub mod binding_input;
pub mod modification;
pub mod salvage;
pub mod spaceship;

pub mod prelude {
    pub use super::{
        area::prelude::*, asteroid::prelude::*, beacon::prelude::*, binding_input::prelude::*,
        modification::prelude::*, salvage::prelude::*, spaceship::prelude::*,
        ScenarioObjectsPlugin,
    };
}

use bevy::prelude::*;

/// Aggregates the scenario-object plugins (asteroid, spaceship, area, beacon,
/// salvage crate) into one group. `render` is threaded to the render-bearing
/// members so headless tools can spawn objects without their visuals.
/// Adds each object type's own plugin (see [`asteroid::AsteroidPlugin`],
/// [`spaceship::SpaceshipPlugin`], [`area::ScenarioAreaPlugin`],
/// [`beacon::BeaconPlugin`], [`salvage::SalvageCratePlugin`]) at build time.
pub struct ScenarioObjectsPlugin {
    pub render: bool,
}

impl Plugin for ScenarioObjectsPlugin {
    fn build(&self, app: &mut App) {
        debug!("ScenarioObjectsPlugin: build");

        app.add_plugins(asteroid::AsteroidPlugin {
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
    }
}
