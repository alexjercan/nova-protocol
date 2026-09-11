//! How an engaged AI ship moves: the standoff envelope and jink legs, as a
//! velocity handed to the flight computer. The computer flies it - the AI
//! writes no controller or thruster input of its own.
//!
//! Engine units, as everywhere under `ai/`. The AUTHORED fields the envelope
//! is bounded by - a turret's `muzzle_speed`, the player's speed cap - are
//! quoted in the meters a creator reads in the content file.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::acquisition::update_ai_target;
#[cfg(test)]
use super::behavior::update_behavior_state;
#[cfg(test)]
use crate::input::targeting::update_sensor_contacts;
use crate::{
    flight::{arrival_speed_limit, flip_lead},
    prelude::*,
};

// AI "brain" tuning constants. The AI flies a standoff envelope around its
// target: approach when far, orbit at the preferred range, extend when too
// close, and brake when it overshoots.
/// How much of a hull's live authority the ORBIT term may spend.
///
/// Circling is not free: holding a radius costs `v^2 / r` of continuous
/// lateral acceleration and holding the guns on the target costs `v / r` of
/// continuous turn. Both are paid out of the same authority the ship needs
/// for everything else it does in a fight - closing, extending, jinking,
/// correcting - so the circle is allowed half of each and no more. A ship
/// that spent all of it would be committed to its own orbit.
const AI_ORBIT_AUTHORITY_RESERVE: f32 = 0.25;
/// Preferred engagement clearance (u): the space a fight SETTLES with between
/// the two hulls' FACES, so this - not the fire gate - is the distance a
/// player sees combat happen at. 100 u = 1.0 km.
///
/// A FACE distance, and the preferred centre distance adds both ships' live
/// [`HullRadius`] to it. Anchor to anchor the same number meant something
/// different in every fight: two skiffs settled 904 m apart face to face and
/// the warship and the carrier 703 m, on one constant, and a pair of hulls
/// with 50 u arms would have parked inside each other.
///
/// Overridable per ship with [`AIStandoffClearance`]. Zero is meaningful and
/// authorable: it asks for contact.
const AI_STANDOFF_CLEARANCE: f32 = 100.0;
/// Half-width (u) of the band around the preferred clearance where the orbit
/// term dominates the radial term. Kept at ~a quarter of the clearance: the
/// RATIO is the fight's shape (a band as wide as the clearance is a charge,
/// not an orbit), and the sum is what the fire gate has to cover.
const AI_STANDOFF_BAND: f32 = 25.0;
/// The far edge (u) of the orbit band a fight settles into, as a FACE
/// distance, and therefore the distance EVERY gun an AI ship carries must be
/// able to reach BEFORE the two hulls are counted. Authoring a turret whose
/// `muzzle_speed * projectile_lifetime * AI_FIRE_RANGE_FACTOR` falls short of
/// this gives a ship that flies its fight correctly and never pulls the
/// trigger, with nothing logged. Exported so the content audit can grade
/// authored prototypes against it.
///
/// Necessary, not sufficient: the round crosses the centre distance, which
/// adds both live arms on top. The shipped fleet's largest pair is the
/// warship against the carrier at 118 m and 194 m, so 1 250 m of band becomes
/// 1 562 m of travel against a 1 800 m gate - `system_ai_combat` measures it.
/// A mod whose hulls are arms of several hundred metres has to author reach
/// for them.
pub const AI_STANDOFF_OUTER_EDGE: f32 = AI_STANDOFF_CLEARANCE + AI_STANDOFF_BAND;

/// Per-ship override of [`AI_STANDOFF_CLEARANCE`]: the clearance this ship
/// wants between its own face and its target's while it fights.
///
/// A FACE distance, like the default it replaces: the preferred centre
/// distance is this plus both hulls' live [`HullRadius`], so the same authored
/// number reads the same way whatever the two ships are. Author it short on a
/// knife-fighter and zero on a boarder or a rammer - zero asks for contact,
/// which is why this is not guarded against zero the way `engage_range` is.
///
/// Authored in meters via `AIControllerConfig::standoff_clearance`; this
/// component holds the world units the envelope compares against.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct AIStandoffClearance(pub f32);
/// The speed (u/s) an evading ship flies its jink legs at.
///
/// Still a constant, and the last one in this module: a jink leg is a
/// DISPLACEMENT - it has to carry the hull out from under the guns inside one
/// leg - so its speed follows from the clearance the leg has to make and the
/// authority the hull has to make it with. Deriving that is the evade item's
/// own work; until then this holds the figure the chase cap used to set.
pub(super) const AI_EVADE_SPEED: f32 = 20.0;

/// The velocity an engaged AI ship wants to be flying: the standoff envelope
/// around its target, whose preferred CENTRE distance `standoff` is. Outside
/// the band it closes; inside the band it circles (tangential to the line of
/// sight, stable handedness); too close it extends away - pure pursuit is
/// what parked the old AI at zero range in a turret duel, or rammed.
///
/// `standoff` is a centre distance because that is what `to_target` measures:
/// the caller sums both hulls' live arms and the authored face clearance, so
/// this function never sees a hull.
///
/// TWO terms, summed, each bounded by what the hull can actually do and each
/// weighted by how far outside the band the ship is:
///
/// - The RADIAL term is the player's own arrival rule ([`ai_radial_speed`]):
///   a ship closes as fast as it can still stop on the ring, and no faster,
///   so a hull with its drives shot off creeps in rather than promising a
///   closing speed it cannot shed. It owns the whole velocity from the band's
///   outer edge outward.
/// - The TANGENTIAL term is [`ai_orbit_speed`]: what the hull can hold the
///   RING at while keeping its guns on the target. It fades in across the
///   band and owns the whole velocity on the ring itself.
///
/// Neither is a constant any more. A picket and a barge used to circle at the
/// same 80 m/s and close at the same 200, which is why a carrier spent a
/// whole fight alternating thrust and brake against a speed it was never
/// going to hold.
///
/// RELATIVE to the target: the caller adds the target's own motion back on
/// before handing the velocity to the computer. A ring around a ship running
/// at 100 m/s is held by matching that 100 m/s and circling on top of it.
///
/// A VELOCITY rather than a heading, because the flight computer flies it.
/// The old heading had to carry a brake regime of its own - point opposite
/// the velocity once the speed budget was overshot - because a heading cannot
/// say "slower". A velocity can: a ship going too fast has an error pointing
/// backwards, and braking is the same act as accelerating.
///
/// Falls back to the line of sight if the envelope direction degenerates.
/// Pure for unit testing.
fn ai_desired_velocity(
    to_target: Vec3,
    standoff: f32,
    authority: FlightAuthority,
    settings: &FlightSettings,
) -> Vec3 {
    let distance = to_target.length();
    if distance <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let los = to_target / distance;

    // Positive = too far (approach), negative = too close (extend).
    let range_error = distance - standoff;
    // Orbit tangent with a stable handedness; the X fallback covers a
    // dead-polar line of sight. Global handedness (every ship circles the
    // same way) is fine for one archetype - see task Notes.
    let tangent = los
        .cross(Vec3::Y)
        .try_normalize()
        .unwrap_or_else(|| los.cross(Vec3::X).normalize());

    // Radial weight ramps with how far outside the band the ship is; inside
    // the band the orbit term takes over. The band is the HANDOVER width,
    // and it is the only thing it is: both speeds are read off the hull.
    let radial_weight = (range_error.abs() / AI_STANDOFF_BAND).clamp(0.0, 1.0);
    let radial = los
        * range_error.signum()
        * ai_radial_speed(range_error, authority, settings)
        * radial_weight;
    radial + tangent * ai_orbit_speed(standoff, authority) * (1.0 - radial_weight)
}

/// The radial speed (u/s) a hull may carry toward - or away from - the ring
/// it circles, `range_error` out from it.
///
/// The shared arrival rule ([`arrival_speed_limit`]) and the shared flip
/// budget ([`flip_lead`]), asked with this hull's live brake authority, turn
/// rate and tracking lag.
///
/// The brake angle budgeted is a HALF TURN, the worst one: the decision is
/// made before the computer has chosen a cluster to brake with, so a hull
/// with retros turns less than that and a main-drive-only hull turns exactly
/// that. The lead is what the approach is actually bounded by - at a quarter
/// of it the test rig closed from 900 u, flipped two seconds late and dived
/// to 40 m of a target it meant to circle at a kilometre. A hull that cannot
/// turn at all gets a lead long enough to make the whole approach a crawl,
/// which is the honest answer rather than a special case. Pure for unit
/// testing.
fn ai_radial_speed(range_error: f32, authority: FlightAuthority, settings: &FlightSettings) -> f32 {
    arrival_speed_limit(
        range_error.abs(),
        authority.linear_acceleration,
        settings.decel_margin,
        flip_lead(
            core::f32::consts::PI,
            authority.turn_rate.max(1e-3),
            authority.tracking_lag,
            settings.align_cos,
            settings.arrival_spool_pad,
        ),
        0.0,
    )
}

/// The speed (u/s) a hull may circle a target at, on a ring `radius` out.
///
/// The lower of the two limits a circle is actually bounded by, each spending
/// [`AI_ORBIT_AUTHORITY_RESERVE`] of its authority:
///
/// - CENTRIPETAL, `sqrt(a * r)`: the lateral acceleration that holds the
///   radius. It is what stops a heavy hull from being asked for a circle its
///   drives have to fight all the way round.
/// - ATTITUDE, `w * r`: the line of sight sweeps at `v / r`, and a ship whose
///   guns are on the target is turning at exactly that rate all fight. Past
///   it the nose falls behind the target it is circling, which is the 38
///   degrees of lag `system_ai_combat` measured on the warship.
///
/// Zero at zero radius, which is the honest answer: nothing circles a point
/// it is standing on. Pure for unit testing.
fn ai_orbit_speed(radius: f32, authority: FlightAuthority) -> f32 {
    let radius = radius.max(0.0) * AI_ORBIT_AUTHORITY_RESERVE;
    let centripetal = (authority.linear_acceleration.max(0.0) * radius).sqrt();
    let attitude = authority.turn_rate.max(0.0) * radius;
    centripetal.min(attitude)
}

/// The centre distance a fight between these two hulls should settle at: the
/// authored face clearance with both live structural arms put back on.
///
/// Both arms, not one: a standoff is the space a player sees BETWEEN two
/// ships, and each hull spends its own arm out of the centre distance first.
/// A hull with no published arm - a target that is not a ship, a root avian
/// has not weighed yet - contributes nothing and the fight degrades to the
/// old anchor-to-anchor reading rather than to a panic.
fn ai_standoff_centre_distance(mover_arm: f32, target_arm: f32, clearance: f32) -> f32 {
    mover_arm + target_arm + clearance
}

/// The direction an evading ship flies on jink pattern leg `leg`: a box
/// weave off the pursuit vector. Each leg is mostly lateral (the four
/// tangent quadrants around the line of sight in turn) with a small
/// alternating along-LOS bias, so consecutive legs swing the heading hard
/// off the pursuit vector AND vary the closure rate - the "timed jink"
/// the task asks for. Deterministic by design: unit-testable, and one
/// archetype does not need unpredictability yet (playtest knob). Falls
/// back to zero on a degenerate line of sight. Pure for unit testing.
pub(super) fn ai_evade_direction(to_target: Vec3, leg: u32) -> Vec3 {
    let Some(los) = to_target.try_normalize() else {
        return Vec3::ZERO;
    };
    // The same stable tangent basis as the standoff orbit, with the X
    // fallback covering a dead-polar line of sight.
    let tangent = los
        .cross(Vec3::Y)
        .try_normalize()
        .unwrap_or_else(|| los.cross(Vec3::X).normalize());
    // Perpendicular to both, unit length (los and tangent are orthonormal).
    let bitangent = los.cross(tangent);
    let lateral = match leg % 4 {
        0 => tangent,
        1 => bitangent,
        2 => -tangent,
        _ => -bitangent,
    };
    let along = if leg.is_multiple_of(2) { 0.25 } else { -0.25 };
    (lateral + los * along).normalize()
}

/// The live-structure anchor of a target entity, or `None` without one (or
/// when it despawned this frame). The shared aim/chase point of every AI
/// behavior system, for both the primary and the point-defense target.
pub(super) fn ai_target_anchor(
    target: Option<Entity>,
    q_target: &Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) -> Option<Vec3> {
    let (transform, com) = q_target.get(target?).ok()?;
    Some(live_structure_anchor(transform, com))
}

/// Fly the combat states through the flight computer: the standoff envelope
/// (or the jink leg) becomes a held velocity, and the nose is asked to stay on
/// the target while the computer holds it.
///
/// The AI writes no controller or thruster input here. It used to write both:
/// one absolute rotation command, and one throttle scalar broadcast to EVERY
/// live thruster - which lit a carrier's retros and laterals together with its
/// mains, torqued the hull off its own command, and gated the whole burn on a
/// single alignment cone. The autopilot already clusters engines, allocates a
/// torque-nulling throttle across the cluster and spools it, so combat asks it
/// for a velocity the way every other maneuver does.
pub(super) fn update_combat_flight(
    mut commands: Commands,
    // The shared stopping rule's own margin: the AI plans its approach with
    // the same discipline the player's arrival legs are flown with.
    settings: Res<FlightSettings>,
    mut q_spaceship: Query<
        (
            Entity,
            &Transform,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
            &AIEvade,
            Option<&AIStandoffClearance>,
            // What this hull can still do, published a schedule earlier. A
            // hull that has not been weighed yet reads as having nothing, and
            // holds station for the tick it takes to be measured.
            Option<&FlightAuthority>,
            Option<&mut Autopilot>,
        ),
        // Silent while a scenario order holds the helm: the order's own drive
        // is the single writer then (see `ShipOrderHelmAuthority`), and the
        // order's takeover is what clears a combat maneuver left behind.
        (
            With<SpaceshipRootMarker>,
            With<AISpaceshipMarker>,
            Without<ShipOrderHelmAuthority>,
        ),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
    // The target's own motion, so the whole envelope is flown in its frame.
    q_target_velocity: Query<&LinearVelocity>,
    // Both ends of the standoff, read off whichever hull is at each end.
    q_arm: Query<&HullRadius>,
    // What the computer needs to fly anything: an engine to burn and a live
    // controller to steer with. Checked HERE rather than left to the
    // autopilot's own disengage, because this driver would re-engage the very
    // next frame - a hulk with its thrusters shot off churns the maneuver
    // sixty times a second and logs a disengage for each one.
    q_engine: Query<&ChildOf, (With<ThrusterSectionMarker>, Without<SectionInactiveMarker>)>,
    q_computer: Query<
        &ChildOf,
        (
            With<ControllerSectionMarker>,
            With<PDController>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    for (ship, transform, com, state, target, evade, clearance, authority, autopilot) in
        &mut q_spaceship
    {
        let held = autopilot.as_deref().is_some_and(|autopilot| {
            matches!(autopilot.action, AutopilotAction::MatchVelocity { .. })
        });
        // A non-engaging state, or no target left to chase, hands the helm
        // back EXPLICITLY: a held velocity never completes itself, and the
        // passive pilot only engages its own maneuver on a free helm.
        let engagement = state
            .engages()
            .then_some(**target)
            .flatten()
            .filter(|_| {
                q_engine.iter().any(|&ChildOf(parent)| parent == ship)
                    && q_computer.iter().any(|&ChildOf(parent)| parent == ship)
            })
            .and_then(|enemy| Some((enemy, ai_target_anchor(Some(enemy), &q_target)?)));
        let Some((enemy, target_anchor)) = engagement else {
            if held {
                commands.entity(ship).remove::<Autopilot>();
            }
            continue;
        };
        // Both ends of the chase vector track live structure: a root origin is
        // the build spot of the first sections and floats in empty space once
        // they are destroyed.
        let to_target = target_anchor - live_structure_anchor(transform, com);
        // A target with no rigid body of its own is a fixed installation.
        let target_velocity = q_target_velocity
            .get(enemy)
            .map_or(Vec3::ZERO, |velocity| **velocity);
        // Evade swaps the standoff envelope for the jink weave. The nose is
        // asked for the target either way, so the guns keep bearing through
        // the weave instead of following the hull off its leg.
        let velocity = if *state == AIBehaviorState::Evade {
            ai_evade_direction(to_target, evade.leg) * AI_EVADE_SPEED
        } else {
            let arm = |hull: Entity| q_arm.get(hull).map_or(0.0, |radius| **radius);
            ai_desired_velocity(
                to_target,
                ai_standoff_centre_distance(
                    arm(ship),
                    arm(enemy),
                    clearance.map_or(AI_STANDOFF_CLEARANCE, |clearance| clearance.0.max(0.0)),
                ),
                authority.copied().unwrap_or_default(),
                &settings,
            )
        };
        // Everything above is a velocity RELATIVE to the target, which is
        // the only frame a fight means anything in: a ring held around a ship
        // running at 100 m/s is held by matching that 100 m/s and circling on
        // top of it. Absolute, the same numbers would leave the AI trailing a
        // moving target by its whole speed.
        let action = AutopilotAction::MatchVelocity {
            velocity: target_velocity + velocity,
            facing: Dir3::new(to_target).ok(),
        };
        match autopilot {
            // Already holding a velocity: retarget it in place. Engaging
            // afresh every tick would reset the phase the computer reports
            // and the instruments read.
            Some(mut autopilot) if held => autopilot.action = action,
            // Anything else - nothing engaged, or a passive leg still flying
            // out - loses the helm. Combat does not wait for a waypoint.
            _ => {
                commands.entity(ship).insert(Autopilot::engage(action));
            }
        }
    }
}

#[cfg(test)]
mod combat_flight_tests {
    // What the combat driver ASKS FOR. How the ask is flown - the cluster
    // choice, the torque-nulled throttle, the facing hold - is the
    // autopilot's own, and is covered in `flight::tests::match_velocity`.
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// The authority the rigs below publish: enough of both to be handed a
    /// maneuver, and round numbers so a case can name what it expects.
    const FLYABLE: FlightAuthority = FlightAuthority {
        linear_acceleration: 20.0,
        turn_rate: 0.5,
        tracking_lag: 0.5,
    };

    /// An engine, a flight computer and the authority pass's reading of them,
    /// so the hull can actually be handed a maneuver.
    fn make_flyable(world: &mut World, ship: Entity) {
        world.entity_mut(ship).insert(FLYABLE);
        world.spawn((
            ChildOf(ship),
            ThrusterSectionMarker,
            ThrusterSectionInput(0.0),
        ));
        world.spawn((
            ChildOf(ship),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 10.0,
                sustained_angular_speed: f32::INFINITY,
            },
        ));
    }

    /// An AI ship at the origin with a hostile dead ahead at -Z, `range`
    /// world units out, already in the given behavior state.
    fn combat_world(state: AIBehaviorState, range: f32) -> (World, Entity, Entity) {
        let mut world = crate::input::ai::ai_test_world();
        let target = world
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(Vec3::new(0.0, 0.0, -range)),
            ))
            .id();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                AISpaceshipMarker,
                RigidBody::Dynamic,
                state,
                AITarget(Some(target)),
                AIEvade::default(),
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
            ))
            .id();
        make_flyable(&mut world, ship);
        (world, ship, target)
    }

    fn engaged(world: &World, ship: Entity) -> Option<AutopilotAction> {
        world.entity(ship).get::<Autopilot>().map(|ap| ap.action)
    }

    #[test]
    fn an_engaging_ship_asks_the_computer_to_hold_the_envelope_velocity() {
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, 1000.0);
        world.run_system_once(update_combat_flight).unwrap();

        let Some(AutopilotAction::MatchVelocity { velocity, .. }) = engaged(&world, ship) else {
            panic!("engaging must hand the computer a velocity to hold");
        };
        assert_eq!(
            velocity,
            ai_desired_velocity(
                Vec3::new(0.0, 0.0, -1000.0),
                AI_STANDOFF_CLEARANCE,
                FLYABLE,
                &FlightSettings::default(),
            ),
            "the held velocity is the envelope's"
        );
    }

    #[test]
    fn the_envelope_makes_room_for_both_hulls() {
        // The item: a fight settles with the authored clearance between the
        // two SKINS, so the centre distance the envelope aims for grows with
        // the structure standing in the way. Parked both ships exactly on
        // the bare clearance, which is inside the envelope for a pair of
        // points and outside it for a pair of hulls - so the same geometry
        // reverses the radial term.
        let (mut world, ship, target) =
            combat_world(AIBehaviorState::Engage, AI_STANDOFF_CLEARANCE);
        world.entity_mut(ship).insert(HullRadius(19.42));
        world.entity_mut(target).insert(HullRadius(19.42));
        world.run_system_once(update_combat_flight).unwrap();

        let Some(AutopilotAction::MatchVelocity { velocity, .. }) = engaged(&world, ship) else {
            panic!("engaging must hand the computer a velocity to hold");
        };
        assert!(
            velocity.normalize().dot(Vec3::NEG_Z) < -0.9,
            "two carrier-sized hulls on the bare clearance are INSIDE the envelope and must \
             extend away from each other, got {velocity:?}"
        );
    }

    #[test]
    fn the_whole_envelope_is_flown_in_the_targets_frame() {
        // A fight is a geometry between two ships, so every speed in it is
        // relative. Parked on the standoff with the target running: the ship
        // has to carry the target's whole velocity plus the circle, or it
        // holds a ring the target has already left.
        let (mut world, ship, target) =
            combat_world(AIBehaviorState::Engage, AI_STANDOFF_CLEARANCE);
        let running = Vec3::new(7.0, -3.0, 11.0);
        world.entity_mut(target).insert(LinearVelocity(running));
        world.run_system_once(update_combat_flight).unwrap();

        let Some(AutopilotAction::MatchVelocity { velocity, .. }) = engaged(&world, ship) else {
            panic!("engaging must hand the computer a velocity to hold");
        };
        let envelope = ai_desired_velocity(
            Vec3::new(0.0, 0.0, -AI_STANDOFF_CLEARANCE),
            AI_STANDOFF_CLEARANCE,
            FLYABLE,
            &FlightSettings::default(),
        );
        assert!(
            (velocity - (running + envelope)).length() < 1e-3,
            "the held velocity is the target's plus the envelope's, got {velocity:?}"
        );
    }

    #[test]
    fn an_authored_clearance_moves_where_the_fight_settles() {
        // The override, and the case the guard shape is for: zero clearance
        // asks for contact, so a pair parked a kilometre apart is far
        // outside the envelope and closes.
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, AI_STANDOFF_CLEARANCE);
        world.entity_mut(ship).insert(AIStandoffClearance(0.0));
        world.run_system_once(update_combat_flight).unwrap();

        let Some(AutopilotAction::MatchVelocity { velocity, .. }) = engaged(&world, ship) else {
            panic!("engaging must hand the computer a velocity to hold");
        };
        assert!(
            velocity.normalize().dot(Vec3::NEG_Z) > 0.9,
            "a zero clearance closes to contact, got {velocity:?}"
        );
    }

    #[test]
    fn the_nose_is_asked_to_stay_on_the_target() {
        // Dead ON the standoff, where the envelope velocity is tangential:
        // the ship flies sideways and still wants its guns on the target,
        // which is the whole reason the facing is separate.
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, AI_STANDOFF_CLEARANCE);
        world.run_system_once(update_combat_flight).unwrap();

        let Some(AutopilotAction::MatchVelocity { velocity, facing }) = engaged(&world, ship)
        else {
            panic!("engaging must hand the computer a velocity to hold");
        };
        let facing = facing.expect("a live target is a direction to face");
        assert!(
            Vec3::from(facing).dot(Vec3::NEG_Z) > 0.999,
            "the nose is asked for the target, got {facing:?}"
        );
        assert!(
            velocity.normalize().dot(Vec3::NEG_Z).abs() < 0.05,
            "and the ship flies across it, not at it, got {velocity:?}"
        );
    }

    #[test]
    fn an_evading_ship_holds_the_jink_leg_not_the_pursuit_vector() {
        let (mut world, ship, _) = combat_world(AIBehaviorState::Evade, 1000.0);
        world.run_system_once(update_combat_flight).unwrap();

        let Some(AutopilotAction::MatchVelocity { velocity, facing }) = engaged(&world, ship)
        else {
            panic!("evading must hand the computer a velocity to hold");
        };
        assert_eq!(
            velocity,
            ai_evade_direction(Vec3::new(0.0, 0.0, -1000.0), 0) * AI_EVADE_SPEED,
            "the held velocity is the jink leg's, at the evade budget"
        );
        assert!(
            Vec3::from(facing.expect("a live target is a direction to face")).dot(Vec3::NEG_Z)
                > 0.999,
            "the guns stay on the target through the weave"
        );
    }

    #[test]
    fn a_hull_the_computer_cannot_fly_is_never_handed_a_maneuver() {
        // A hulk with its engines shot off, or with its flight computer dead,
        // has nothing to fly the velocity with. The autopilot would refuse it
        // and disengage - and this driver would engage it again the next
        // frame, for ever.
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, 1000.0);
        let engines: Vec<Entity> = world
            .query_filtered::<Entity, With<ThrusterSectionMarker>>()
            .iter(&world)
            .collect();
        for engine in engines {
            world.entity_mut(engine).despawn();
        }
        world.run_system_once(update_combat_flight).unwrap();
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "no engine, no maneuver"
        );

        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, 1000.0);
        let computers: Vec<Entity> = world
            .query_filtered::<Entity, With<ControllerSectionMarker>>()
            .iter(&world)
            .collect();
        for computer in computers {
            world.entity_mut(computer).insert(SectionInactiveMarker);
        }
        world.run_system_once(update_combat_flight).unwrap();
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "no live flight computer, no maneuver"
        );
    }

    #[test]
    fn breaking_off_hands_the_helm_back() {
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, 1000.0);
        world.run_system_once(update_combat_flight).unwrap();
        assert!(engaged(&world, ship).is_some(), "engaged first");

        *world.entity_mut(ship).get_mut::<AIBehaviorState>().unwrap() = AIBehaviorState::Patrol;
        world.run_system_once(update_combat_flight).unwrap();
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "a held velocity never completes itself, so leaving combat must release it"
        );
    }

    #[test]
    fn losing_the_target_hands_the_helm_back() {
        let (mut world, ship, target) = combat_world(AIBehaviorState::Engage, 1000.0);
        world.run_system_once(update_combat_flight).unwrap();
        assert!(engaged(&world, ship).is_some(), "engaged first");

        world.entity_mut(target).despawn();
        world.run_system_once(update_combat_flight).unwrap();
        assert!(
            world.entity(ship).get::<Autopilot>().is_none(),
            "nothing left to chase releases the helm"
        );
    }

    #[test]
    fn combat_takes_the_helm_from_a_patrol_leg() {
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, 1000.0);
        world
            .entity_mut(ship)
            .insert(Autopilot::engage(AutopilotAction::GotoPos {
                position: Vec3::new(500.0, 0.0, 0.0),
            }));
        world.run_system_once(update_combat_flight).unwrap();

        assert!(
            matches!(
                engaged(&world, ship),
                Some(AutopilotAction::MatchVelocity { .. })
            ),
            "a fight does not wait for a waypoint"
        );
    }

    #[test]
    fn a_retargeted_hold_keeps_the_phase_the_computer_reported() {
        let (mut world, ship, _) = combat_world(AIBehaviorState::Engage, 1000.0);
        world.run_system_once(update_combat_flight).unwrap();
        world.entity_mut(ship).get_mut::<Autopilot>().unwrap().phase = AutopilotPhase::Burn;

        // The target moves, so the envelope velocity changes: the action must
        // be rewritten in place rather than re-engaged, or every tick would
        // reset the phase to Align.
        world
            .entity_mut(ship)
            .get_mut::<Transform>()
            .unwrap()
            .translation = Vec3::new(0.0, 0.0, 100.0);
        world.run_system_once(update_combat_flight).unwrap();

        assert_eq!(
            world.entity(ship).get::<Autopilot>().map(|ap| ap.phase),
            Some(AutopilotPhase::Burn),
            "retargeting a held velocity is not a new maneuver"
        );
    }
}

#[cfg(test)]
mod physics_tests {
    // The whole diegetic loop on a real avian world, with the REAL flight
    // computer: acquisition -> behavior -> held velocity -> autopilot ->
    // PD torque -> hull swing -> spooled burn -> impulse. These cover the
    // acceptance the direct-actuation path used to carry - the hull holds
    // its aim without limit-cycling, and the fight settles into the standoff
    // band instead of closing to zero.
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::{
        flight::NovaFlightSystems,
        sections::{
            controller_section::ControllerSectionPlugin, thruster_section::thruster_impulse_system,
        },
    };

    /// The production plugins the combat driver hands its velocity to, plus
    /// the AI chain that writes it. The AI chain runs on the fixed clock here
    /// (it is on the render clock in the game) so one `update` is one whole
    /// decide-and-fly pass.
    fn combat_physics_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        // The sensor pass reads the shipped lock settings.
        app.init_resource::<TargetingSettings>();
        app.add_plugins((
            PDControllerPlugin,
            ControllerSectionPlugin { render: false },
            crate::flight::NovaFlightPlugin,
        ));
        app.configure_sets(
            FixedUpdate,
            crate::input::SpaceshipInputSystems.before(NovaFlightSystems),
        );
        app.add_systems(
            FixedUpdate,
            (
                crate::sections::signature::publish_ship_signatures,
                update_sensor_contacts,
                update_ai_target,
                update_behavior_state,
                update_combat_flight,
            )
                .chain()
                .in_set(crate::input::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            thruster_impulse_system.in_set(SpaceshipSectionSystems),
        );
        app.finish();
        app
    }

    /// A hostile hull for the AI to fight, `at`, with its nose pointed up
    /// the world Y axis.
    ///
    /// Looking AWAY is the point. A hostile holding its nose on the AI
    /// inside `AI_THREAT_AIM_RANGE` breaks Engage into an evade cycle, and
    /// an orbiting ship crosses a fixed nose once a lap - so a rig that
    /// measures the engage envelope has to be a rig the AI is not being
    /// aimed at. The evade cycle has its own tests in `threat.rs`.
    fn spawn_unthreatening_player(app: &mut App, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                RigidBody::Dynamic,
                Transform::from_translation(at)
                    .with_rotation(Quat::from_rotation_x(core::f32::consts::FRAC_PI_2)),
            ))
            .id()
    }

    /// An AI hull with a hull block, one main drive aft and a live flight
    /// computer of the given attitude authority.
    fn spawn_ai_ship(app: &mut App, max_angular_acceleration: f32) -> Entity {
        let ship = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default(), AISpaceshipMarker))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::from_xyz(0.0, 0.0, -1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("thruster"),
            ThrusterSectionMarker,
            ThrusterSectionMagnitude(1.0),
            ThrusterSectionInput(0.0),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));
        ship
    }

    #[test]
    fn the_ai_swings_onto_the_player_and_settles() {
        let mut app = combat_physics_app();

        // Player abeam at +X, far outside the standoff band (approach
        // regime): a 90-degree swing from the AI's initial -Z onto +X. The
        // 5u structural arm returns 56u, which gates at 1665u, so the picket
        // can see it out there.
        let player = spawn_unthreatening_player(&mut app, Vec3::new(1000.0, 0.0, 0.0));
        app.world_mut().entity_mut(player).insert(HullRadius(5.0));
        // High authority keeps the swing short; the chase is what this rig
        // measures, not the slew.
        let ship = spawn_ai_ship(&mut app, 10.0);

        settle(&mut app);
        // 60 simulated seconds: the swing, the run-in, the flip and the
        // capture. The run-in is no longer a 20 u/s crawl - this rig's drive
        // stops it from 155 u/s inside the 900 u it has - so the nose spends
        // part of the leg retrograde, on the brake, by design.
        for _ in 0..3600 {
            app.update();
        }

        // No limit cycle on the aim: the nose must be ON the player and STAY
        // there for a further simulated second. The hull is circling now, so
        // the bearing it holds is a MOVING one and the nose trails it by the
        // loop's tracking lag - this rig settles at 198 m/s on a 1,090 m ring,
        // a line of sight turning at 0.18 rad/s, which its 0.5 s lag puts 5.2
        // degrees behind. What must not appear is a hunt on top of that.
        let mut min_aim = f32::INFINITY;
        let mut max_roll = 0.0f32;
        for _ in 0..60 {
            app.update();
            let forward: Vec3 = app.world().get::<Transform>(ship).unwrap().forward().into();
            let to_player = (Vec3::new(1000.0, 0.0, 0.0)
                - app.world().get::<Transform>(ship).unwrap().translation)
                .normalize();
            min_aim = min_aim.min(forward.dot(to_player));
            let spin = **app.world().get::<AngularVelocity>(ship).unwrap();
            max_roll = max_roll.max(spin.dot(forward).abs());
        }
        assert!(
            min_aim > 0.99,
            "the hull must hold its nose on the player (within ~8 degrees) \
             for a full second, worst aim cos {min_aim}"
        );
        // The turn that tracks the target is about an axis ACROSS the nose;
        // about the nose itself the hull must stay quiet. Since the bcs
        // inertia-frame fix the roll in this rig measures ~5e-6 rad/s, so the
        // bound leaves ~4 orders of margin for solver noise while still
        // tripping on any real roll-damping regression (the pre-fix amplitude
        // was ~0.23 rad/s).
        assert!(
            max_roll < 0.05,
            "residual roll must stay damped (20260709-125640), \
             got {max_roll} rad/s"
        );
    }

    #[test]
    fn the_burn_lights_one_cluster_not_every_engine() {
        // The defect this migration fixes, on a hull that can show it: mains
        // aft, a retro forward and a lateral pair, all symmetric about the
        // centre of mass so nothing needs recruiting for counter-torque. The
        // old combat writer put ONE scalar on every one of them, so a ship
        // closing on its target lit its retro and both laterals with its
        // mains - thrust it paid for and then cancelled, plus whatever torque
        // the asymmetry left.
        let mut app = combat_physics_app();
        spawn_unthreatening_player(&mut app, Vec3::new(0.0, 0.0, -600.0));
        let ship = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default(), AISpaceshipMarker))
            .id();
        app.world_mut().spawn((
            ChildOf(ship),
            Name::new("hull"),
            Transform::default(),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 10.0,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
        ));
        // A section's local rotation is the way its engine points; -Z is a
        // thruster's own thrust axis.
        let engine = |app: &mut App, name: &'static str, at: Vec3, facing: Quat| {
            app.world_mut()
                .spawn((
                    ChildOf(ship),
                    Name::new(name),
                    ThrusterSectionMarker,
                    ThrusterSectionMagnitude(1.0),
                    ThrusterSectionInput(0.0),
                    Transform::from_translation(at).with_rotation(facing),
                    Collider::cuboid(0.5, 0.5, 0.5),
                    ColliderDensity(1.0),
                ))
                .id()
        };
        let main = engine(&mut app, "main", Vec3::new(0.0, 0.0, 1.0), Quat::IDENTITY);
        let retro = engine(
            &mut app,
            "retro",
            Vec3::new(0.0, 0.0, -1.0),
            Quat::from_rotation_y(core::f32::consts::PI),
        );
        let port = engine(
            &mut app,
            "port",
            Vec3::new(-1.0, 0.0, 0.0),
            Quat::from_rotation_y(-core::f32::consts::FRAC_PI_2),
        );
        let starboard = engine(
            &mut app,
            "starboard",
            Vec3::new(1.0, 0.0, 0.0),
            Quat::from_rotation_y(core::f32::consts::FRAC_PI_2),
        );

        settle(&mut app);
        let input =
            |app: &App, engine: Entity| **app.world().get::<ThrusterSectionInput>(engine).unwrap();
        let mut hottest_main = 0.0f32;
        let mut worst_cancelled = 0.0f32;
        // 45 simulated seconds: the run-in, the flip and the settle into the
        // band, so every regime gets its chance to light something it should
        // not. WHICH engine burns is the planner's to choose - the retro is
        // the right answer on the brake leg, a lateral is the right answer on
        // a trim, and the allocator may recruit a third for counter-torque -
        // so the claim is about OPPOSED engines: whatever the plan, the ship
        // must not be paying for thrust it cancels. The broadcast writer ran
        // every opposed pair at 1.0 against 1.0; the worst this measures is
        // 9.5 percent, on the brake leg.
        for _ in 0..2700 {
            app.update();
            hottest_main = hottest_main.max(input(&app, main));
            for (one, other) in [(main, retro), (port, starboard)] {
                let (one, other) = (input(&app, one), input(&app, other));
                let (quiet, loud) = (one.min(other), one.max(other));
                if loud > 0.05 {
                    worst_cancelled = worst_cancelled.max(quiet / loud);
                }
            }
        }

        assert!(
            hottest_main > 0.5,
            "the run-in must actually burn its mains, hottest {hottest_main}"
        );
        assert!(
            worst_cancelled < 0.15,
            "an opposed pair burned together at {:.0} percent of each other; \
             that is thrust the ship paid for and cancelled",
            worst_cancelled * 100.0
        );
    }

    #[test]
    fn the_ship_settles_into_the_standoff_band() {
        let mut app = combat_physics_app();

        // The target dead ahead (-Z), outside the band.
        let player_position = Vec3::new(0.0, 0.0, -600.0);
        spawn_unthreatening_player(&mut app, player_position);
        // Deliberately weak attitude authority: the envelope has to be flown
        // by a hull that turns slowly, which is where the old broadcast
        // throttle alternated thrust and brake.
        let ship = spawn_ai_ship(&mut app, 0.5);

        settle(&mut app);
        // Fly for 45 simulated seconds: the approach (~500 u, and this hull
        // turns so slowly that the flip budget holds it to a fraction of what
        // its drive could carry), plus braking and orbit capture.
        let mut min_distance = f32::INFINITY;
        for _ in 0..2700 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            min_distance = min_distance.min(position.distance(player_position));
        }

        // The last simulated second must stay inside a generous band around
        // the standoff - the old pure pursuit closes to ~zero. Neither body
        // in this rig publishes a `HullRadius`, so the centre distance the
        // envelope settles at is the bare clearance.
        let mut worst_error = 0.0f32;
        for _ in 0..60 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            let error = (position.distance(player_position) - AI_STANDOFF_CLEARANCE).abs();
            worst_error = worst_error.max(error);
        }
        assert!(
            worst_error < AI_STANDOFF_BAND * 2.0,
            "the ship must hold the standoff band (worst error {worst_error} u)"
        );
        // Relative to the envelope, not a literal: the standoff is retuned
        // whenever turret reach is, and a hardcoded floor silently becomes
        // either unfalsifiable or impossible.
        let floor = AI_STANDOFF_CLEARANCE - AI_STANDOFF_BAND * 2.0;
        assert!(
            min_distance > floor,
            "the ship must never dive far inside the envelope \
             (closest approach {min_distance} u, floor {floor} u)"
        );
    }
}

#[cfg(test)]
mod jink_tests {
    use super::*;

    const LOS_TARGET: Vec3 = Vec3::new(0.0, 0.0, -400.0);

    #[test]
    fn every_leg_stays_off_the_pursuit_vector() {
        let los = LOS_TARGET.normalize();
        for leg in 0..8 {
            let direction = ai_evade_direction(LOS_TARGET, leg);
            assert!(
                direction.dot(los).abs() < 0.5,
                "leg {leg} hugs the pursuit vector: {direction:?}"
            );
            assert!(
                (direction.length() - 1.0).abs() < 1e-3,
                "leg {leg} is not a unit direction"
            );
        }
    }

    #[test]
    fn consecutive_legs_swing_the_heading_hard() {
        for leg in 0..8 {
            let a = ai_evade_direction(LOS_TARGET, leg);
            let b = ai_evade_direction(LOS_TARGET, leg + 1);
            assert!(
                a.dot(b) < 0.5,
                "legs {leg} and {} barely differ: {a:?} vs {b:?}",
                leg + 1
            );
        }
    }

    #[test]
    fn the_pattern_wraps_and_survives_degenerate_geometry() {
        assert_eq!(
            ai_evade_direction(LOS_TARGET, 0),
            ai_evade_direction(LOS_TARGET, 4),
            "the box weave is a 4-leg loop"
        );
        // A polar line of sight uses the X-fallback tangent basis.
        let polar = ai_evade_direction(Vec3::new(0.0, 300.0, 0.0), 1);
        assert!(polar.is_finite() && polar.length() > 0.9);
        // A degenerate (zero) line of sight yields no direction at all.
        assert_eq!(ai_evade_direction(Vec3::ZERO, 0), Vec3::ZERO);
    }
}

#[cfg(test)]
mod standoff_tests {
    use super::*;

    /// The centre distance a fight between two armless test points settles
    /// at: the bare clearance, which is what every case below that is not
    /// about hulls wants to read.
    const BARE: f32 = AI_STANDOFF_CLEARANCE;

    /// A hull that can fly: roughly a shipped escort's acceleration and a
    /// turn rate that carries its nose around a kilometre-wide circle.
    const ESCORT: FlightAuthority = FlightAuthority {
        linear_acceleration: 20.0,
        turn_rate: 0.5,
        tracking_lag: 0.5,
    };

    /// A hull that can barely fly: a tenth of the escort's drive and a fifth
    /// of its turn rate, which is the shape of a capital against a picket.
    const BARGE: FlightAuthority = FlightAuthority {
        linear_acceleration: 2.0,
        turn_rate: 0.1,
        tracking_lag: 1.0,
    };

    fn envelope(to_target: Vec3, authority: FlightAuthority) -> Vec3 {
        ai_desired_velocity(to_target, BARE, authority, &FlightSettings::default())
    }

    fn radial(range_error: f32, authority: FlightAuthority) -> f32 {
        ai_radial_speed(range_error, authority, &FlightSettings::default())
    }

    #[test]
    fn far_outside_the_band_the_ship_approaches() {
        let to_target = Vec3::new(0.0, 0.0, -1000.0);
        let desired = envelope(to_target, ESCORT);
        assert!(
            desired.normalize().dot(to_target.normalize()) > 0.999,
            "far away: fly straight at the target, got {desired:?}"
        );
        assert_eq!(
            desired.length(),
            radial(900.0, ESCORT),
            "and at the speed it could still stop from"
        );
    }

    #[test]
    fn inside_the_band_the_ship_orbits() {
        // Dead on the preferred range: the radial term vanishes and the
        // desired velocity is tangential to the line of sight.
        let to_target = Vec3::new(0.0, 0.0, -BARE);
        let desired = envelope(to_target, ESCORT);
        assert!(
            desired.normalize().dot(to_target.normalize()).abs() < 0.05,
            "in band: orbit, not chase (los dot {})",
            desired.normalize().dot(to_target.normalize())
        );
        assert_eq!(
            desired.length(),
            ai_orbit_speed(BARE, ESCORT),
            "at the circle the hull can hold: a parked ship is a free shot"
        );
    }

    #[test]
    fn too_close_the_ship_extends_away() {
        let to_target = Vec3::new(0.0, 0.0, -50.0);
        let desired = envelope(to_target, ESCORT);
        assert!(
            desired.normalize().dot(to_target.normalize()) < -0.9,
            "well inside the envelope: extend AWAY from the target, got {desired:?}"
        );
    }

    #[test]
    fn the_speed_budget_falls_as_the_ship_nears_the_band() {
        // The budget is earned by RANGE ERROR, not by distance: a ship half
        // the way in is commanded slower, and the envelope hands the computer
        // the brake rather than carrying a brake regime of its own.
        let far = envelope(Vec3::new(0.0, 0.0, -1000.0), ESCORT).length();
        let near = envelope(Vec3::new(0.0, 0.0, -BARE - 50.0), ESCORT).length();
        assert!(
            near < far,
            "closing on the band must lower the budget, {near} u/s against {far} u/s"
        );
        assert_eq!(near, radial(50.0, ESCORT));
    }

    #[test]
    fn a_weaker_hull_is_asked_for_less_of_everything() {
        // The item: the speeds are readings of the hull, not constants. The
        // same geometry asks a barge to close slower than an escort and to
        // circle slower than it too.
        let run_in = Vec3::new(0.0, 0.0, -1000.0);
        let on_the_ring = Vec3::new(0.0, 0.0, -BARE);
        assert!(
            envelope(run_in, BARGE).length() < 0.5 * envelope(run_in, ESCORT).length(),
            "a barge must not be asked for the escort's closing speed"
        );
        assert!(
            envelope(on_the_ring, BARGE).length() < 0.5 * envelope(on_the_ring, ESCORT).length(),
            "nor for its circle"
        );
    }

    #[test]
    fn a_hull_with_nothing_left_asks_for_nothing() {
        // The tick before a root is weighed, and every tick after its drives
        // are gone: no authority is no plan, not a plan the hull cannot fly.
        assert_eq!(
            envelope(Vec3::new(0.0, 0.0, -1000.0), FlightAuthority::default()),
            Vec3::ZERO
        );
    }

    #[test]
    fn the_circle_is_bounded_by_the_nose_not_only_by_the_drives() {
        // A hull with drive to spare and a slow helm is held to the rate its
        // guns can track at - the limit that separates a capital's circle
        // from a picket's.
        let strong_and_slow = FlightAuthority {
            linear_acceleration: 200.0,
            turn_rate: 0.05,
            tracking_lag: 0.5,
        };
        let held = ai_orbit_speed(BARE, strong_and_slow);
        assert_eq!(
            held,
            strong_and_slow.turn_rate * AI_ORBIT_AUTHORITY_RESERVE * BARE,
            "the attitude limit binds, got {held} u/s"
        );
        assert!(
            held < (strong_and_slow.linear_acceleration * AI_ORBIT_AUTHORITY_RESERVE * BARE).sqrt(),
            "and it is the lower of the two"
        );
    }

    #[test]
    fn a_polar_line_of_sight_still_orbits() {
        // Line of sight straight up Y: the Y-cross tangent degenerates and
        // the X fallback must keep the orbit term finite.
        let to_target = Vec3::new(0.0, BARE, 0.0);
        let desired = envelope(to_target, ESCORT);
        assert!(
            desired.is_finite() && desired.length() > 0.9,
            "polar approach must not degenerate, got {desired:?}"
        );
    }

    #[test]
    fn a_bigger_pair_of_hulls_settles_further_apart() {
        // The clearance is what a player sees between two skins, so the
        // centre distance has to grow by exactly the structure that stands
        // in the way. Both hulls count: the space is between them.
        let skiff = 4.78;
        let carrier = 19.42;
        assert_eq!(
            ai_standoff_centre_distance(skiff, skiff, AI_STANDOFF_CLEARANCE),
            AI_STANDOFF_CLEARANCE + 2.0 * skiff
        );
        assert_eq!(
            ai_standoff_centre_distance(carrier, carrier, AI_STANDOFF_CLEARANCE)
                - ai_standoff_centre_distance(skiff, skiff, AI_STANDOFF_CLEARANCE),
            2.0 * (carrier - skiff),
            "two carriers stand off further than two skiffs by exactly the hull between them"
        );
    }

    #[test]
    fn the_face_clearance_is_what_two_hulls_actually_leave_between_them() {
        // The point of the item, stated as the reading a player takes: pick
        // any two hulls, fly the envelope, and the gap between the skins is
        // the authored number.
        for (mover, target) in [(4.78, 4.78), (4.78, 19.42), (11.8, 19.42), (50.0, 50.0)] {
            let centre = ai_standoff_centre_distance(mover, target, AI_STANDOFF_CLEARANCE);
            assert_eq!(
                centre - mover - target,
                AI_STANDOFF_CLEARANCE,
                "arms {mover} u and {target} u settled with a face gap that is not the clearance"
            );
        }
    }

    #[test]
    fn a_target_with_no_published_arm_costs_the_fight_nothing() {
        // A root avian has not weighed yet, or a target that is not a ship,
        // publishes no arm. That reads as a point, which is the old
        // anchor-to-anchor behavior, not as a panic or a zero standoff.
        assert_eq!(
            ai_standoff_centre_distance(0.0, 0.0, AI_STANDOFF_CLEARANCE),
            AI_STANDOFF_CLEARANCE
        );
    }

    #[test]
    fn an_authored_zero_clearance_asks_for_contact() {
        // Zero is a real answer here - a boarder, a rammer - so the envelope
        // must settle where the two faces meet rather than refuse it.
        let mover = 4.78;
        let target = 19.42;
        assert_eq!(
            ai_standoff_centre_distance(mover, target, 0.0),
            mover + target,
            "a zero clearance is face on face, not centre on centre"
        );
    }
}
