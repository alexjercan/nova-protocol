//! How an engaged AI ship moves: the standoff envelope and jink directions,
//! the absolute rotation command fed to the flight controller, and the
//! alignment-gated thrust that flies them.
//!
//! Engine units, as everywhere under `ai/`. The AUTHORED fields the envelope
//! is bounded by - a turret's `muzzle_speed`, the player's speed cap - are
//! quoted in the meters a creator reads in the content file.

use avian3d::prelude::*;
use bevy::prelude::*;
use nova_gameplay::prelude::*;

#[cfg(test)]
use super::acquisition::{update_ai_target, update_point_defense_target};
#[cfg(test)]
use super::behavior::update_behavior_state;
use super::threat::AI_EVADE_THRUST_ALIGNMENT;
use crate::prelude::*;

// AI "brain" tuning constants. The AI flies a standoff envelope around its
// target: approach when far, orbit at the preferred range, extend when too
// close, and brake when it overshoots.
/// Target speed per unit of RANGE ERROR (distance outside the standoff
/// band), so the ship slows as it nears the band instead of the target.
const AI_CHASE_SPEED_GAIN: f32 = 0.2;
/// Orbit speed floor: inside the band the ship keeps circling at least this
/// fast, so it stays a moving target instead of a parked one.
const AI_ORBIT_SPEED: f32 = 8.0;
const AI_MAX_CHASE_SPEED: f32 = 20.0;
/// Preferred engagement range (u): where a fight SETTLES, so this - not the
/// fire gate - is the distance a player sees combat happen at. 100 u = 1.0 km.
///
/// Bounded from above by the fire gate: `AI_STANDOFF_RANGE +
/// AI_STANDOFF_BAND` (125 u) must sit inside the WEAKEST shipped gun's
/// `muzzle_speed * projectile_lifetime * AI_FIRE_RANGE_FACTOR` - 180 u for
/// every shipped PDC, which authors 1 000 m/s over 2.0 s - or a ship orbits
/// outside its own reach and never fires - silently. Moves with every lifetime
/// change: see AI_FIRE_RANGE_FACTOR in `guns.rs` for the whole chain.
const AI_STANDOFF_RANGE: f32 = 100.0;
/// Half-width (u) of the band around the preferred range where the orbit
/// term dominates the radial term. Kept at ~a quarter of the standoff: the
/// RATIO is the fight's shape (a band as wide as the standoff is a charge,
/// not an orbit), and the sum is what the fire gate has to cover.
const AI_STANDOFF_BAND: f32 = 25.0;
/// The far edge (u) of the orbit band a fight settles into, and therefore the
/// distance EVERY gun an AI ship carries must be able to reach. Authoring a
/// turret whose `muzzle_speed * projectile_lifetime * AI_FIRE_RANGE_FACTOR`
/// falls short of this gives a ship that flies its fight correctly and never
/// pulls the trigger, with nothing logged. Exported so the content audit can
/// grade authored prototypes against it.
pub const AI_STANDOFF_OUTER_EDGE: f32 = AI_STANDOFF_RANGE + AI_STANDOFF_BAND;
/// The ship brakes once its speed exceeds the target chase speed by this margin.
const AI_BRAKE_SPEED_MARGIN: f32 = 1.0;
/// Only thrust when the ship's forward vector aligns with the desired direction at least
/// this much (dot product, 1.0 == perfectly aligned).
const AI_THRUST_ALIGNMENT: f32 = 0.95;

/// The direction an AI ship should face: the standoff envelope around its
/// target. Far outside the band it approaches; inside the band it orbits
/// (tangential to the line of sight, stable handedness); too close it
/// extends away - pure pursuit is what parked the old AI at zero range in
/// a turret duel, or rammed. Overshooting its speed budget it brakes
/// (opposite its velocity), as before. Falls back to facing the target if
/// the computed direction degenerates to zero. Pure for unit testing.
fn ai_desired_direction(to_target: Vec3, velocity: Vec3) -> Vec3 {
    let distance = to_target.length();
    if distance <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let los = to_target / distance;

    // Positive = too far (approach), negative = too close (extend).
    let range_error = distance - AI_STANDOFF_RANGE;
    // Orbit tangent with a stable handedness; the X fallback covers a
    // dead-polar line of sight. Global handedness (every ship circles the
    // same way) is fine for one archetype - see task Notes.
    let tangent = los
        .cross(Vec3::Y)
        .try_normalize()
        .unwrap_or_else(|| los.cross(Vec3::X).normalize());
    // Radial weight ramps with how far outside the band the ship is; inside
    // the band the orbit term dominates.
    let radial_weight = (range_error.abs() / AI_STANDOFF_BAND).clamp(0.0, 1.0);
    let radial = los * range_error.signum();
    let desired = radial * radial_weight + tangent * (1.0 - radial_weight);

    // Speed budget scales with the range error, never below orbit speed;
    // overshooting it brakes, exactly as the old chase regime did.
    let target_speed =
        (range_error.abs() * AI_CHASE_SPEED_GAIN).clamp(AI_ORBIT_SPEED, AI_MAX_CHASE_SPEED);
    let too_fast = velocity.length() > target_speed + AI_BRAKE_SPEED_MARGIN;

    let desired = if too_fast {
        // Brake: point opposite the current velocity.
        -velocity.normalize_or_zero()
    } else {
        desired.normalize_or_zero()
    };

    if desired.length_squared() == 0.0 {
        to_target.normalize_or_zero()
    } else {
        desired
    }
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

pub(super) fn update_controller_target_rotation_torque(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
            &AIEvade,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (entity, transform, velocity, com, state, target, evade) in &q_spaceship {
        // A non-engaging state (Idle/Patrol) holds its helm: the command
        // freezes exactly like a dead helm, so re-engaging resumes from
        // where the hull actually points. No target freezes it the same way.
        if !state.engages() {
            continue;
        }
        // Chase the target's live structure, not its root origin: the origin
        // is the build spot of the first sections and floats in empty space
        // once they are destroyed.
        let Some(target_anchor) = ai_target_anchor(**target, &q_target) else {
            continue;
        };
        // Both ends of the chase vector track live structure: the AI's own
        // root origin goes as stale as the target's once sections die.
        let own_anchor = live_structure_anchor(transform, com);
        let to_target = target_anchor - own_anchor;
        // Evade swaps the standoff envelope for the jink weave; the guns
        // stay on target regardless (turret aim is hull-independent).
        let desired_direction = if *state == AIBehaviorState::Evade {
            ai_evade_direction(to_target, evade.leg)
        } else {
            ai_desired_direction(to_target, **velocity)
        };

        // Slew the command at the computer's acceleration-derived turn rate
        // instead of rewriting it every frame: a distant setpoint drives the
        // PD into saturation where its damping is swamped and the hull limit-
        // cycles. Same derivation as the player path and the autopilot
        // (flight::ship_turn_rate). With no live computer the command
        // FREEZES, matching the player path: nothing consumes it, and slewing
        // a dead helm would drift it so a later re-activation snaps the hull.
        let Some(turn_rate) = crate::flight::ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent))| parent == entity)
                .map(|(pd, _)| pd.max_angular_acceleration),
            &settings,
        ) else {
            continue;
        };
        let max_step = turn_rate * time.delta_secs();

        for (mut controller, _) in q_controller
            .iter_mut()
            .filter(|(_, ChildOf(parent))| *parent == entity)
        {
            // The input is an ABSOLUTE world rotation - every other writer
            // treats it that way; the old code wrote a delta arc (the bug
            // this task fixes). The goal carries the command's own forward
            // onto the desired direction, and the command evolves from ITS
            // OWN previous state, never from the hull: a command rebuilt
            // from the hull each tick inherits the hull's roll, the PD then
            // sees zero roll error, and roll picked up during a swing spins
            // the ship forever (see the autopilot's rotation step).
            let command = **controller;
            let command_forward = command * Vec3::NEG_Z;
            let goal = Quat::from_rotation_arc(command_forward, desired_direction) * command;
            **controller = crate::flight::slew_rotation(command, goal, max_step);
        }
    }
}

pub(super) fn on_thruster_input(
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &GlobalTransform, &ChildOf),
        With<ThrusterSectionMarker>,
    >,
    q_spaceship: Query<
        (
            Entity,
            &Transform,
            &LinearVelocity,
            Option<&ComputedCenterOfMass>,
            &AIBehaviorState,
            &AITarget,
            &AIEvade,
            Has<Autopilot>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    q_target: Query<(&Transform, Option<&ComputedCenterOfMass>)>,
) {
    for (entity, transform, velocity, com, state, target, evade, has_autopilot) in &q_spaceship {
        // While a passive-state maneuver is engaged the flight computer
        // owns the engines: writing here - even an explicit 0.0 - would
        // fight the autopilot's spooled inputs every frame.
        if has_autopilot {
            continue;
        }
        // A non-engaging state (or no target left to chase) cuts the burn -
        // written as an explicit 0.0, not a skip, so a ship that was
        // thrusting when the state flipped actually stops.
        let thrust_level = match ai_target_anchor(**target, &q_target) {
            Some(target_anchor) if state.engages() => {
                // Same live-structure vector as the rotation system, so the
                // thrust gate and the rotation command agree on where
                // "toward the target" is - including the jink swap, and with
                // a looser gate while evading so the lateral burst fires
                // mid-swing instead of waiting out the slew.
                let to_target = target_anchor - live_structure_anchor(transform, com);
                let (desired_direction, gate) = if *state == AIBehaviorState::Evade {
                    (
                        ai_evade_direction(to_target, evade.leg),
                        AI_EVADE_THRUST_ALIGNMENT,
                    )
                } else {
                    (
                        ai_desired_direction(to_target, **velocity),
                        AI_THRUST_ALIGNMENT,
                    )
                };

                // Thrust only when the ship is pointing roughly toward the
                // desired direction.
                let forward = transform.forward();
                let alignment = forward.dot(desired_direction);
                if alignment > gate {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };

        for (mut thruster_input, _, _) in q_thruster
            .iter_mut()
            .filter(|(_, _, ChildOf(parent))| *parent == entity)
        {
            **thruster_input = thrust_level;
        }
    }
}

#[cfg(test)]
mod rotation_tests {
    // Command-level harness with manual time, mirroring the player path's
    // command_lag_tests: the AI rotation command must be an ABSOLUTE world
    // rotation slewed at the hull's derived turn rate.
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// An AI ship + controller facing -Z with the player dead astern (+Z),
    /// so the desired direction is a 180 flip from the initial command.
    fn flip_world() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.init_resource::<FlightSettings>();
        // The real acquisition system feeds the rotation system, so the
        // harness drives the same pipeline the plugin chains.
        app.add_systems(
            Update,
            (update_ai_target, update_controller_target_rotation_torque).chain(),
        );

        // Dead astern, well OUTSIDE the standoff band so the approach
        // regime points straight at the player and the flip semantics hold.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(0.0, 0.0, 800.0)),
        ));
        // High authority keeps this command-slew test short.
        let ship = app
            .world_mut()
            .spawn((
                AISpaceshipMarker,
                Transform::default(),
                LinearVelocity(Vec3::ZERO),
                ComputedAngularInertia::new(Vec3::splat(2.3)),
            ))
            .id();
        let controller = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ControllerSectionMarker,
                PDController {
                    frequency: 4.0,
                    damping_ratio: 4.0,
                    max_angular_acceleration: 10.0,
                    sustained_angular_speed: f32::INFINITY,
                },
                ControllerSectionRotationInput::default(),
            ))
            .id();
        (app, controller)
    }

    #[test]
    fn an_ai_flip_reaches_the_command_over_many_frames() {
        // The old code rewrote the command every frame with no slew - the
        // exact PD-saturation regime the player path was fixed for.
        let (mut app, controller) = flip_world();

        // First update has dt = 0; the second advances one real frame.
        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let moved = command.angle_between(Quat::IDENTITY);
        let expected =
            crate::flight::hull_turn_rate(10.0, &app.world().resource::<FlightSettings>().clone())
                / 60.0;
        // One frame advances exactly one slew step of the DERIVED rate -
        // this pins hull_turn_rate's wiring, not just "some" slew.
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one acceleration-authority slew step \
             (moved {moved}, expected {expected})"
        );
        let flip = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::Z);
        assert!(
            command.angle_between(flip) > 2.0,
            "a 180 flip must not reach the command in one frame"
        );
    }

    #[test]
    fn the_command_converges_to_the_absolute_look_at_rotation() {
        // The input is an absolute world rotation; the old code wrote a
        // DELTA (`from_rotation_arc(forward, desired)`), which for a
        // constant bearing never points the commanded forward at the
        // player. Slewed long enough, the command's forward must land on
        // the player bearing exactly.
        let (mut app, controller) = flip_world();

        for _ in 0..600 {
            app.update();
        }

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let commanded_forward = command * Vec3::NEG_Z;
        let to_player = Vec3::Z; // player at +Z, ship at the origin
        assert!(
            commanded_forward.dot(to_player) > 0.999,
            "the commanded forward must converge on the player bearing, \
             got {commanded_forward:?}"
        );
    }

    #[test]
    fn a_dead_helm_freezes_the_command() {
        // With no live computer the command must not drift (matches the
        // player path): slewing a dead helm would snap the hull on a later
        // re-activation.
        let (mut app, controller) = flip_world();
        app.world_mut()
            .entity_mut(controller)
            .insert(SectionInactiveMarker);

        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        assert_eq!(command, Quat::IDENTITY, "dead helm: the command freezes");
    }
}

#[cfg(test)]
mod physics_tests {
    // A real avian world with the real PD, mirroring the flight module's
    // level harness: AI rotation command -> PD torque -> hull swings. Covers
    // the task's acceptance: the AI swings to the target attitude and settles
    // without limit-cycling.
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::sections::controller_section::{
        sync_controller_section_forces, update_controller_section_rotation_input,
    };
    #[test]
    fn the_ai_swings_onto_the_player_and_settles() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                crate::input::SpaceshipInputSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_ai_target,
                update_point_defense_target,
                update_behavior_state,
                update_controller_target_rotation_torque,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(crate::input::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(SpaceshipSectionSystems),
        );
        app.finish();

        // Player abeam at +X, far outside the standoff band (approach
        // regime): a 90-degree swing from the AI's initial -Z onto +X.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1000.0, 0.0, 0.0)),
        ));
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
            Name::new("controller"),
            ControllerSectionMarker,
            ControllerSectionRotationInput::default(),
            PDController {
                frequency: 4.0,
                damping_ratio: 4.0,
                max_angular_acceleration: 10.0,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

        settle(&mut app);
        // 10 simulated seconds: ample for the swing plus settling.
        for _ in 0..600 {
            app.update();
        }

        // No limit cycle on the aim: the nose must be ON the player and STAY
        // there for a further simulated second. The old delta-command code
        // fails this two ways: the delta setpoint never points the hull at
        // the player at all, and the unslewed rewrite saturates the PD into
        // an attitude limit cycle.
        let mut min_aim = f32::INFINITY;
        let mut max_spin = 0.0f32;
        for _ in 0..60 {
            app.update();
            let forward: Vec3 = app.world().get::<Transform>(ship).unwrap().forward().into();
            min_aim = min_aim.min(forward.dot(Vec3::X));
            let spin = app.world().get::<AngularVelocity>(ship).unwrap().length();
            max_spin = max_spin.max(spin);
        }
        assert!(
            min_aim > 0.996,
            "the hull must hold its nose on the player (within ~5 degrees) \
             for a full second, worst aim cos {min_aim}"
        );
        // The aim axes are quiet and, since the bcs inertia-frame fix, so is
        // the roll: the residual spin in this rig measures ~5e-6 rad/s. The
        // bound leaves ~4 orders of margin for solver noise while still
        // tripping on any real roll-damping regression (the pre-fix amplitude
        // was ~0.23 rad/s).
        assert!(
            max_spin < 0.05,
            "residual spin must stay damped (20260709-125640), \
             got {max_spin} rad/s"
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

    #[test]
    fn far_outside_the_band_the_ship_approaches() {
        let to_target = Vec3::new(0.0, 0.0, -1000.0);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.dot(to_target.normalize()) > 0.999,
            "far away: point straight at the target, got {desired:?}"
        );
    }

    #[test]
    fn inside_the_band_the_ship_orbits() {
        // Dead on the preferred range: the radial term vanishes and the
        // desired direction is tangential to the line of sight.
        let to_target = Vec3::new(0.0, 0.0, -AI_STANDOFF_RANGE);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.dot(to_target.normalize()).abs() < 0.05,
            "in band: orbit, not chase (los dot {})",
            desired.dot(to_target.normalize())
        );
        assert!(
            (desired.length() - 1.0).abs() < 1e-3,
            "the desired direction stays a unit vector"
        );
    }

    #[test]
    fn too_close_the_ship_extends_away() {
        let to_target = Vec3::new(0.0, 0.0, -50.0);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.dot(to_target.normalize()) < -0.9,
            "well inside the envelope: extend AWAY from the target, got {desired:?}"
        );
    }

    #[test]
    fn the_overshoot_brake_regime_survives_the_envelope() {
        // Screaming toward the target far faster than the speed budget:
        // the ship points opposite its velocity, exactly as pre-envelope.
        let to_target = Vec3::new(0.0, 0.0, -1000.0);
        let velocity = Vec3::new(0.0, 0.0, -100.0);
        let desired = ai_desired_direction(to_target, velocity);
        assert!(
            desired.dot(velocity.normalize()) < -0.999,
            "overshooting: brake against the velocity, got {desired:?}"
        );
    }

    #[test]
    fn a_polar_line_of_sight_still_orbits() {
        // Line of sight straight up Y: the Y-cross tangent degenerates and
        // the X fallback must keep the orbit term finite.
        let to_target = Vec3::new(0.0, AI_STANDOFF_RANGE, 0.0);
        let desired = ai_desired_direction(to_target, Vec3::ZERO);
        assert!(
            desired.is_finite() && desired.length() > 0.9,
            "polar approach must not degenerate, got {desired:?}"
        );
    }
}

#[cfg(test)]
mod standoff_physics_tests {
    // The full diegetic loop on the physics harness: acquisition ->
    // behavior -> rotation command -> PD torque -> hull swing -> aligned
    // thrust -> impulses. Pins the task's acceptance: the ship settles
    // into the standoff band instead of closing to zero (ramming/parking).
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::sections::{
        controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
        },
        thruster_section::thruster_impulse_system,
    };
    #[test]
    fn the_ship_settles_into_the_standoff_band() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                crate::input::SpaceshipInputSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_ai_target,
                update_behavior_state,
                update_controller_target_rotation_torque,
                on_thruster_input,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(crate::input::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            (sync_controller_section_forces, thruster_impulse_system)
                .in_set(SpaceshipSectionSystems),
        );
        app.finish();

        // The target dead ahead (-Z), outside the band.
        let player_position = Vec3::new(0.0, 0.0, -600.0);
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(player_position),
        ));
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
                max_angular_acceleration: 0.5,
                sustained_angular_speed: f32::INFINITY,
            },
            PDControllerTarget(ship),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Collider::cuboid(1.0, 1.0, 1.0),
            ColliderDensity(1.0),
        ));

        settle(&mut app);
        // Fly for 45 simulated seconds: approach (~500 u at up to ~20 u/s)
        // plus braking and orbit capture.
        let mut min_distance = f32::INFINITY;
        for _ in 0..2700 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            min_distance = min_distance.min(position.distance(player_position));
        }

        // The last simulated second must stay inside a generous band around
        // the standoff range - the old pure pursuit closes to ~zero.
        let mut worst_error = 0.0f32;
        for _ in 0..60 {
            app.update();
            let position = app.world().get::<Transform>(ship).unwrap().translation;
            let error = (position.distance(player_position) - AI_STANDOFF_RANGE).abs();
            worst_error = worst_error.max(error);
        }
        assert!(
            worst_error < AI_STANDOFF_BAND * 2.0,
            "the ship must hold the standoff band (worst error {worst_error} u)"
        );
        // Relative to the envelope, not a literal: the standoff is retuned
        // whenever turret reach is, and a hardcoded floor silently becomes
        // either unfalsifiable or impossible.
        let floor = AI_STANDOFF_RANGE - AI_STANDOFF_BAND * 2.0;
        assert!(
            min_distance > floor,
            "the ship must never dive far inside the envelope \
             (closest approach {min_distance} u, floor {floor} u)"
        );
    }
}
