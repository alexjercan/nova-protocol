//! A neutralized non-player ship brakes toward world rest on its own finite
//! RCS, so a player can close on it and dock by hand.
//!
//! The brake takes the helm when [`NeutralizedMarker`] lands: the observer
//! drops any engaged maneuver and the AI and order writers refuse the hull
//! from then on. The brake needs no engine: RCS is a torque-free push at the
//! centre of mass, so a hull that lost every thruster still brakes. It needs
//! a live flight computer, the `rcs_enabled` capability, delta-v in its own
//! [`RcsBudget`], no dock, and a local well its RCS can out-push. The first
//! of those it lacks ends the brake for good with the reason in
//! [`NeutralizedBrakeType`], and the hull coasts. The player's ship and
//! stations never brake here: the player keeps their own STOP, and a dead
//! station keeps the post nothing reads.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{
    autopilot::{
        local_gravity_accel, rcs_has_gravity_authority, well_samples, ArrivalTargetQuery,
        FlightWellQuery,
    },
    capability::{
        ship_capabilities, ship_has_attitude_authority, LiveFlightComputers, ShipCapabilityQuery,
    },
    Autopilot, FlightSettings, NeutralizedBrakeType, RcsBudget, RcsIntent,
};
use crate::prelude::*;

/// Hand a newly neutralized non-player ship root to its RCS brake. A thaw
/// inserts the frozen brake state before the marker, and
/// `try_insert_if_new` keeps it.
pub(super) fn on_neutralized_engage_brake(
    add: On<Add, NeutralizedMarker>,
    mut commands: Commands,
    q_root: Query<
        (),
        (
            With<SpaceshipRootMarker>,
            Without<PlayerSpaceshipMarker>,
            Without<StationMarker>,
        ),
    >,
) {
    if !q_root.contains(add.entity) {
        return;
    }
    commands
        .entity(add.entity)
        .try_insert_if_new((NeutralizedBrakeType::Braking, RcsIntent::default()))
        .try_remove::<Autopilot>();
}

/// Write each braking hull's RCS command toward world rest, or end its brake
/// with the first reason it cannot go on. Runs before `rcs_burn_system`,
/// which pays the command from the hull's own [`RcsBudget`].
#[expect(
    clippy::type_complexity,
    reason = "one query over the braking root's state, motion, magazine and dock"
)]
pub(super) fn neutralized_brake_system(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    gravity_settings: Res<GravitySettings>,
    mut q_ship: Query<(
        Entity,
        Option<&Name>,
        &mut NeutralizedBrakeType,
        &mut RcsIntent,
        &RcsBudget,
        (&LinearVelocity, &Position, &Rotation),
        Option<&ComputedCenterOfMass>,
        Has<DockedShip>,
    )>,
    q_computer: LiveFlightComputers,
    q_capabilities: ShipCapabilityQuery,
    q_wells: FlightWellQuery,
    q_target: ArrivalTargetQuery,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let wells = well_samples(&q_wells, &q_target);
    for (ship, name, mut brake, mut intent, budget, (velocity, position, rotation), com, docked) in
        &mut q_ship
    {
        // A terminal brake commands nothing, including the tick after the
        // final residual push.
        if *brake != NeutralizedBrakeType::Braking {
            if intent.0 != Vec3::ZERO {
                intent.0 = Vec3::ZERO;
            }
            continue;
        }
        let full_step = settings.rcs_accel * dt;
        let speed = velocity.length();
        let com_world = position.0 + rotation.mul_vec3(com.map_or(Vec3::ZERO, |com| com.0));
        let end = if docked {
            Some(NeutralizedBrakeType::Docked)
        } else if !ship_capabilities(ship, &q_capabilities).rcs_enabled {
            Some(NeutralizedBrakeType::RcsDisabled)
        } else if !ship_has_attitude_authority(ship, &q_computer) {
            Some(NeutralizedBrakeType::NoFlightComputer)
        } else if !rcs_has_gravity_authority(
            local_gravity_accel(&wells, com_world, &gravity_settings),
            settings.rcs_accel,
        ) {
            Some(NeutralizedBrakeType::Gravity)
        } else if budget.is_empty() {
            Some(NeutralizedBrakeType::RcsEmpty)
        } else {
            None
        };
        if let Some(end) = end {
            intent.0 = Vec3::ZERO;
            *brake = end;
            log_brake_end(ship, name, end, speed, budget);
            continue;
        }
        // The whole velocity as this tick's delta-v, in the hull's frame:
        // `rcs_push` clamps it to one full-deflection step, so the last tick
        // delivers only the residual and the brake never overshoots rest.
        intent.0 = rotation
            .inverse()
            .mul_vec3(-velocity.0 / full_step)
            .clamp_length_max(1.0);
        // This tick's push nulls what is left if the magazine covers it. A
        // pull the RCS out-pushes adds less than one step per tick, so a
        // braking hull always gets here; one the magazine cannot cover
        // empties it and ends as `RcsEmpty` next tick.
        if speed <= full_step && speed <= budget.remaining() {
            *brake = NeutralizedBrakeType::Rested;
            log_brake_end(ship, name, NeutralizedBrakeType::Rested, speed, budget);
        }
    }
}

/// Say once, at the edge, why a hull's RCS brake ended.
fn log_brake_end(
    ship: Entity,
    name: Option<&Name>,
    end: NeutralizedBrakeType,
    speed: f32,
    budget: &RcsBudget,
) {
    let label = name.map_or_else(|| format!("{ship:?}"), |name| name.to_string());
    info!(
        "neutralized_brake_system: '{label}' ends its RCS brake: {end:?} at {speed:.3} u/s, \
         {:.2} u/s of RCS left",
        budget.remaining()
    );
}
