//! Shared rigs for the player-input tests: a world with the flight rig's
//! actions bound as in the real rig, and a flyable ship on it.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::*;

use super::flight_rig::{
    flight_input_rig, AutopilotGotoInput, AutopilotOffInput, AutopilotOrbitInput,
    AutopilotStopInput,
};
use crate::{input::bindings::flight_bindings, prelude::*};

/// Spawn the REAL flight rig, bound from the REAL defaults. Every rig test
/// goes through here so none of them can drift from what the game ships.
pub(crate) fn spawn_flight_rig(app: &mut App) -> Entity {
    let bindings = InputBindings::from_actions(flight_bindings());
    app.world_mut().spawn(flight_input_rig(&bindings)).id()
}

/// A world with the flight rig's four autopilot actions bound as in
/// the real rig, plus the resources the resolver reads.
pub(super) fn hint_world() -> World {
    let mut world = World::new();
    world.init_resource::<FlightVerbHints>();
    world.spawn((
        Action::<AutopilotStopInput>::new(),
        bindings![KeyCode::KeyX, GamepadButton::East],
    ));
    world.spawn((
        Action::<AutopilotGotoInput>::new(),
        bindings![KeyCode::KeyG, GamepadButton::North],
    ));
    world.spawn((
        Action::<AutopilotOrbitInput>::new(),
        bindings![KeyCode::KeyO, GamepadButton::South],
    ));
    world.spawn((
        Action::<AutopilotOffInput>::new(),
        bindings![KeyCode::KeyZ, GamepadButton::West],
    ));
    world
}

/// A flyable player ship: live controller (with PD, all verbs granted) +
/// live thruster. Mirrors the production `controller_section` bundle, which
/// carries NO [`WithheldVerbs`] by default (an absent component grants every
/// verb); tests that withhold a verb insert a `WithheldVerbs` on the
/// returned controller.
pub(super) fn spawn_flyable_ship(world: &mut World) -> (Entity, Entity) {
    let ship = world.spawn((PlayerSpaceshipMarker, targeting_state())).id();
    let controller = world
        .spawn((
            ChildOf(ship),
            ControllerSectionMarker,
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 40.0,
            },
        ))
        .id();
    world.spawn((ChildOf(ship), ThrusterSectionMarker));
    (ship, controller)
}
