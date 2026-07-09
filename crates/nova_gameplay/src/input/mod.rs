use bevy::prelude::*;

pub mod ai;
pub mod player;
pub mod targeting;

pub mod prelude {
    pub use super::{
        ai::prelude::*, player::prelude::*, targeting::prelude::*, SpaceshipInputPlugin,
        SpaceshipInputSystems,
    };
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpaceshipInputSystems;

pub struct SpaceshipInputPlugin;

impl Plugin for SpaceshipInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipInputPlugin: build");

        app.add_plugins(player::SpaceshipPlayerInputPlugin);
        app.add_plugins(targeting::SpaceshipTargetingPlugin);
        app.add_plugins(ai::SpaceshipAIInputPlugin);
    }
}
