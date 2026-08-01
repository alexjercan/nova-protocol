//! Physics-level integration tests for the flight layer.
//!
//! A real avian world with the real PD controller, controller-section glue,
//! and thruster impulse system, so these cover the whole diegetic pipeline:
//! autopilot -> rotation command -> PD torque -> hull swings -> aligned ->
//! spooled burn -> impulse -> velocity. No external forces anywhere.

mod control;
mod goto;
mod manual;
mod orbit;
mod rcs;
mod stop;
mod support;
mod telemetry;

use bevy::prelude::*;

use super::insert_flight_control;
use crate::prelude::*;

#[test]
fn player_marker_receives_flight_intent() {
    let mut app = App::new();
    app.add_observer(insert_flight_control);

    let ship = app
        .world_mut()
        .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
        .id();
    app.update();

    assert!(app.world().get::<FlightIntent>(ship).is_some());
    assert!(
        app.world().get::<Autopilot>(ship).is_none(),
        "no maneuver engaged at spawn"
    );
}
