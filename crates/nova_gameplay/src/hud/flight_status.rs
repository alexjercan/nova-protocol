//! A one-line text readout of the flight computer: assist mode, actual speed,
//! and (in assisted mode) the commanded speed. The mode toggle would be
//! invisible without it. Anything richer belongs to the weapons-HUD tasks.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::flight::prelude::*;

pub mod prelude {
    pub use super::{
        flight_status_hud, FlightStatusHudConfig, FlightStatusHudMarker, FlightStatusHudPlugin,
        FlightStatusHudTargetEntity,
    };
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct FlightStatusHudMarker;

/// The ship whose flight state this readout shows.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct FlightStatusHudTargetEntity(pub Entity);

#[derive(Clone, Debug)]
pub struct FlightStatusHudConfig {
    pub target: Entity,
}

/// UI bundle for the readout: a small fixed text node in the lower-left,
/// clear of the velocity sphere and the health bar.
pub fn flight_status_hud(config: FlightStatusHudConfig) -> impl Bundle {
    debug!("flight_status_hud: config {:?}", config);

    (
        Name::new("FlightStatusHUD"),
        FlightStatusHudMarker,
        FlightStatusHudTargetEntity(config.target),
        Text::new(""),
        TextFont::from_font_size(14.0),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
    )
}

#[derive(Default)]
pub struct FlightStatusHudPlugin;

impl Plugin for FlightStatusHudPlugin {
    fn build(&self, app: &mut App) {
        debug!("FlightStatusHudPlugin: build");

        app.add_systems(
            Update,
            update_flight_status_text.in_set(super::NovaHudSystems),
        );
    }
}

fn update_flight_status_text(
    mut q_hud: Query<(&FlightStatusHudTargetEntity, &mut Text), With<FlightStatusHudMarker>>,
    q_ship: Query<(&FlightAssistMode, &FlightCommand, &LinearVelocity)>,
) {
    for (target, mut text) in &mut q_hud {
        let Ok((mode, command, velocity)) = q_ship.get(**target) else {
            // The ship can die a frame before the HUD is despawned.
            continue;
        };

        **text = crate::flight::flight_status_line(
            *mode,
            velocity.length(),
            command.velocity.map(|v| v.length()),
        );
    }
}
