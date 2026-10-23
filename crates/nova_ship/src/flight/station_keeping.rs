//! Station keeping: a living station rides back onto its [`StationPost`] on
//! its cold-gas RCS after a ram or a kick moves it off.
//!
//! The law is the ORBIT verb's hold loop with a station's numbers: the
//! desired velocity on the post (circular speed on the ring, or rest at a
//! fixed point) plus a bounded return toward the post, flown as a
//! proportional `RcsIntent` that `rcs_burn_system` spends from the station's
//! own magazine and spreads over every hull docked at it. Torque-free at
//! each root's centre of mass, so the Controller section's attitude hold is
//! untouched and the dock joints see no load.
//!
//! Intentional motion wins: a scenario order on the station
//! ([`ShipOrderHelmAuthority`]), a dead station a docked player tows
//! ([`DerelictShipMarker`], or a hub that does not drive its group) and the
//! orbit itself (the post IS the ring) write no command. The keeper is the
//! one writer of a station's `RcsIntent`, so it writes zero where it does
//! not own the motion and a stale command never outlives its authority.
//!
//! Engine units throughout, as every flight system.

use avian3d::prelude::*;
use bevy::{ecs::entity::EntityHashSet, prelude::*};
use nova_gameplay::prelude::*;

use super::guidance::{arrival_speed_limit, orbit_ring_offset, orbit_ring_radial};
use crate::prelude::*;

/// The body-frame RCS command that moves a station at `position` with
/// `velocity` and attitude `rotation` back onto `post`: the velocity error
/// toward the post's desired velocity over
/// [`FlightSettings::orbit_hold_enter`], each axis clamped to unit. Zero
/// inside both deadbands - within [`FlightSettings::station_post_deadband`]
/// of the post and under [`FlightSettings::station_hold_deadband`] of
/// velocity error - so a settled station spends nothing and its magazine
/// refills, and zero for an `Orbit` post whose well is not matched (`mu`
/// is `None`): nothing to ride, nothing to fight.
///
/// The return toward the post is the arrival curve from the post deadband's
/// edge at [`FlightSettings::station_rcs_accel`], capped at
/// [`FlightSettings::station_return_speed`]; inside the deadband it is zero
/// and only the velocity error is trimmed. Pure for unit testing.
pub(super) fn station_keeping_command(
    post: &StationPost,
    mu: Option<f32>,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    settings: &FlightSettings,
) -> Vec3 {
    let (desired, offset) = match *post {
        StationPost::Orbit {
            centre,
            radius,
            normal,
        } => {
            let Some(mu) = mu else {
                return Vec3::ZERO;
            };
            let plan = OrbitPlan { radius, normal };
            let r_vec = position - centre;
            // The circular speed at the station's CURRENT in-plane radius,
            // not the ring's: an RCS far weaker than the pull can only
            // circularize where it is and let the return walk the radius
            // in. Holding the ring's speed off the ring is positive
            // feedback - too fast outside raises the orbit further - and
            // the proof's well runs away on it; this target is neutral.
            let in_plane = (r_vec - normal * r_vec.dot(normal)).length();
            let speed = circular_orbit_speed(mu, if in_plane > 0.0 { in_plane } else { radius });
            let tangential = normal.cross(orbit_ring_radial(r_vec, &plan)) * speed;
            let to_ring = orbit_ring_offset(r_vec, &plan);
            (
                tangential + return_speed(to_ring, settings),
                to_ring.length(),
            )
        }
        StationPost::Fixed { position: post } => {
            let to_post = post - position;
            (return_speed(to_post, settings), to_post.length())
        }
    };
    let error = desired - velocity;
    if offset < settings.station_post_deadband && error.length() < settings.station_hold_deadband {
        return Vec3::ZERO;
    }
    (rotation.inverse() * error / settings.orbit_hold_enter)
        .clamp(Vec3::splat(-1.0), Vec3::splat(1.0))
}

/// The velocity a station `to_post` away from its post rides back with:
/// the arrival curve over the distance outside the post deadband, capped at
/// `station_return_speed`, zero inside the deadband.
fn return_speed(to_post: Vec3, settings: &FlightSettings) -> Vec3 {
    let Some(direction) = to_post.try_normalize() else {
        return Vec3::ZERO;
    };
    let outside = to_post.length() - settings.station_post_deadband;
    if outside <= 0.0 {
        return Vec3::ZERO;
    }
    direction
        * arrival_speed_limit(
            outside,
            settings.station_rcs_accel,
            settings.decel_margin,
            0.0,
            0.0,
        )
        .min(settings.station_return_speed)
}

/// The strength of the well an `Orbit` post rides: the nearest well whose
/// reach covers `centre`. A planet is on rails, so the match is exact in
/// practice; the reach only spares a tolerance constant.
fn post_well_mu(
    centre: Vec3,
    q_wells: &Query<(&Position, &GravityWell), Without<SpaceshipRootMarker>>,
) -> Option<f32> {
    q_wells
        .iter()
        .filter(|(position, well)| position.0.distance(centre) <= well.soi_radius)
        .min_by(|(a, _), (b, _)| {
            a.0.distance_squared(centre)
                .total_cmp(&b.0.distance_squared(centre))
        })
        .map(|(_, well)| well.mu)
}

/// Write every station's `RcsIntent` from its post. Runs in
/// `NovaFlightSystems` after `autopilot_system` and before
/// `rcs_burn_system`, so the command is spent this tick.
///
/// A living station holds its post. A dead station, a station under a
/// scenario order and a hub that does not drive its group get a zero
/// command: whoever moves them is not fought. An `Orbit` post whose well is
/// not up gets zero too, logged once per loss: a cell still materializing
/// is a valid wait, and the ECS proof asserts it never fires in a rig.
#[expect(
    clippy::type_complexity,
    reason = "one query over the station root's post, pose, motion and helm state"
)]
pub(super) fn station_keeping_system(
    settings: Res<FlightSettings>,
    mut commands: Commands,
    mut q_station: Query<
        (
            Entity,
            &StationPost,
            &Position,
            &Rotation,
            &LinearVelocity,
            Option<&DockedStation>,
            Has<DerelictShipMarker>,
            Has<ShipOrderHelmAuthority>,
            Option<&mut RcsIntent>,
        ),
        With<StationMarker>,
    >,
    q_wells: Query<(&Position, &GravityWell), Without<SpaceshipRootMarker>>,
    // Orbit posts whose well is missing and already logged.
    mut unmatched: Local<EntityHashSet>,
) {
    for (station, post, position, rotation, velocity, hub, derelict, ordered, intent) in
        &mut q_station
    {
        let mu = match post {
            StationPost::Orbit { centre, .. } => {
                let mu = post_well_mu(*centre, &q_wells);
                if mu.is_none() {
                    if unmatched.insert(station) {
                        warn!(
                            "station_keeping_system: station {station:?} orbits a well at \
                             {centre:?} that is not up; holding nothing until it is"
                        );
                    }
                } else {
                    unmatched.remove(&station);
                }
                mu
            }
            StationPost::Fixed { .. } => None,
        };
        let held = derelict || ordered || hub.is_some_and(|hub| !hub.drives);
        let command = if held {
            Vec3::ZERO
        } else {
            station_keeping_command(post, mu, position.0, velocity.0, rotation.0, &settings)
        };
        // A station without the component only needs one once it pushes, as
        // the autopilot's step does.
        match intent {
            Some(mut intent) => {
                if intent.0 != command {
                    intent.0 = command;
                }
            }
            None if command != Vec3::ZERO => {
                commands.entity(station).insert(RcsIntent(command));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MU: f32 = 43_350.0;
    const RADIUS: f32 = 140.0;

    fn orbit_post() -> StationPost {
        StationPost::Orbit {
            centre: Vec3::ZERO,
            radius: RADIUS,
            normal: Vec3::Y,
        }
    }

    /// The circular velocity on the +X radial of [`orbit_post`]: travel is
    /// `normal x radial = Y x X = -Z`.
    fn v_circ() -> Vec3 {
        Vec3::NEG_Z * circular_orbit_speed(MU, RADIUS)
    }

    #[test]
    fn a_station_on_its_ring_at_circular_speed_is_commanded_nothing() {
        let settings = FlightSettings::default();
        let command = station_keeping_command(
            &orbit_post(),
            Some(MU),
            Vec3::X * RADIUS,
            v_circ(),
            Quat::IDENTITY,
            &settings,
        );
        assert_eq!(command, Vec3::ZERO, "on the ring the keeper is quiet");
    }

    /// Off the ring the keeper circularizes WHERE THE STATION IS and walks
    /// the radius in: the tangential target is the circular speed at the
    /// current radius (the ring's speed there is excess speed and raises the
    /// orbit further), and the radial return never exceeds the return speed.
    #[test]
    fn a_station_off_its_ring_is_commanded_inward_no_faster_than_the_return_speed() {
        let settings = FlightSettings::default();
        let here = RADIUS + 2.0;
        let command = station_keeping_command(
            &orbit_post(),
            Some(MU),
            Vec3::X * here,
            v_circ(),
            Quat::IDENTITY,
            &settings,
        );
        let error = command * settings.orbit_hold_enter;
        assert!(
            error.x < 0.0,
            "the command points back at the ring: {command:?}"
        );
        assert!(
            error.x.abs() <= settings.station_return_speed + 1e-4,
            "the return never exceeds station_return_speed: {command:?}"
        );
        assert!(
            error.x.abs() > settings.station_hold_deadband,
            "two units off the ring is a real correction: {command:?}"
        );
        let expected_tangential = circular_orbit_speed(MU, here) - circular_orbit_speed(MU, RADIUS);
        assert!(
            (error.z + expected_tangential).abs() < 1e-4,
            "the tangential target is the circular speed at {here} u, not the ring's: {error:?}"
        );
    }

    #[test]
    fn a_fixed_post_far_off_is_commanded_at_the_return_speed_in_the_body_frame() {
        let settings = FlightSettings::default();
        let post = StationPost::Fixed {
            position: Vec3::ZERO,
        };
        // The hull faces +X with its nose (-Z): the post is behind it in the
        // world, so the body-frame command points down its own -X.
        let rotation = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        let command = station_keeping_command(
            &post,
            None,
            Vec3::X * 100.0,
            Vec3::ZERO,
            rotation,
            &settings,
        );
        let world = rotation * command * settings.orbit_hold_enter;
        assert!(
            (world.length() - settings.station_return_speed).abs() < 1e-4,
            "far off the post the return is capped at station_return_speed: {world:?}"
        );
        assert!(
            world.normalize().dot(Vec3::NEG_X) > 0.999,
            "the return points at the post in the world frame: {world:?}"
        );
    }

    #[test]
    fn a_station_inside_both_deadbands_or_with_no_well_is_commanded_nothing() {
        let settings = FlightSettings::default();
        let post = StationPost::Fixed {
            position: Vec3::ZERO,
        };
        let inside = Vec3::X * (settings.station_post_deadband * 0.5);
        let creeping = Vec3::Z * (settings.station_hold_deadband * 0.5);
        assert_eq!(
            station_keeping_command(&post, None, inside, creeping, Quat::IDENTITY, &settings),
            Vec3::ZERO,
            "inside both deadbands the keeper is quiet"
        );
        let drifting = Vec3::Z * (settings.station_hold_deadband * 2.0);
        assert_ne!(
            station_keeping_command(&post, None, inside, drifting, Quat::IDENTITY, &settings),
            Vec3::ZERO,
            "a velocity error over the hold deadband is trimmed even inside the post deadband"
        );
        assert_eq!(
            station_keeping_command(
                &orbit_post(),
                None,
                Vec3::X * (RADIUS + 5.0),
                Vec3::ZERO,
                Quat::IDENTITY,
                &settings
            ),
            Vec3::ZERO,
            "an orbit post with no well up holds nothing"
        );
    }
}
