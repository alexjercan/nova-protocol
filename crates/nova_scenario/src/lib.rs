pub mod actions;
pub mod events;
pub mod filters;
pub mod lint;
pub mod loader;
pub mod objects;
pub mod render_scale;
pub mod variables;
pub mod world;

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
