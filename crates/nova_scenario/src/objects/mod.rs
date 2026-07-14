pub mod area;
pub mod asteroid;
pub mod beacon;
pub mod binding_input;
pub mod salvage;
pub mod spaceship;

pub mod prelude {
    pub use super::{
        area::prelude::*, asteroid::prelude::*, beacon::prelude::*, binding_input::prelude::*,
        salvage::prelude::*, spaceship::prelude::*, ScenarioObjectsPlugin,
    };
}

use bevy::prelude::*;

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
