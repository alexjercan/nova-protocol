use avian3d::prelude::*;
use bevy::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{AISpaceshipMarker, SpaceshipAIInputPlugin};
}

pub struct SpaceshipAIInputPlugin;

impl Plugin for SpaceshipAIInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipAIInputPlugin: build");

        app.add_systems(
            Update,
            (
                update_controller_target_rotation_torque,
                on_thruster_input,
                update_turret_target_input,
                on_projectile_input,
            )
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// Marker component to identify the ai's spaceship.
///
/// This should be added to the root entity of the ai's spaceship.
/// Carries [`Allegiance::Enemy`] by requirement, so every AI-marked root
/// participates in the relation model without extra spawn wiring.
#[derive(Component, Debug, Clone, Reflect)]
#[require(SpaceshipRootMarker, Allegiance = Allegiance::Enemy)]
pub struct AISpaceshipMarker;

// AI "brain" tuning constants. The AI chases the player at a speed that scales with
// distance (so it slows as it closes in) and brakes when it overshoots.
/// Target chase speed per unit of distance to the player.
const AI_CHASE_SPEED_GAIN: f32 = 0.2;
/// Lower/upper clamp on the distance-scaled chase speed.
const AI_MIN_CHASE_SPEED: f32 = 2.0;
const AI_MAX_CHASE_SPEED: f32 = 20.0;
/// The ship brakes once its speed exceeds the target chase speed by this margin.
const AI_BRAKE_SPEED_MARGIN: f32 = 1.0;
/// Only thrust when the ship's forward vector aligns with the desired direction at least
/// this much (dot product, 1.0 == perfectly aligned).
const AI_THRUST_ALIGNMENT: f32 = 0.95;
/// Only fire when the muzzle aligns with the player at least this much.
const AI_FIRE_ALIGNMENT: f32 = 0.95;

/// The direction an AI ship should face: toward the player while it is slower than its
/// distance-scaled target speed, or opposite its velocity when overshooting (braking).
/// Falls back to facing the player if the computed direction degenerates to zero.
fn ai_desired_direction(to_player: Vec3, velocity: Vec3) -> Vec3 {
    let target_speed =
        (to_player.length() * AI_CHASE_SPEED_GAIN).clamp(AI_MIN_CHASE_SPEED, AI_MAX_CHASE_SPEED);
    let too_fast = velocity.length() > target_speed + AI_BRAKE_SPEED_MARGIN;

    let desired = if too_fast {
        // Brake: point opposite the current velocity.
        -velocity.normalize_or_zero()
    } else {
        // Chase: point toward the player.
        to_player.normalize()
    };

    if desired.length_squared() == 0.0 {
        to_player.normalize_or_zero()
    } else {
        desired
    }
}

fn update_controller_target_rotation_torque(
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
            &ComputedAngularInertia,
            Option<&ComputedCenterOfMass>,
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Single<
        (&Transform, Option<&ComputedCenterOfMass>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    // Chase the player's live structure, not the root origin: the origin is
    // the build spot of the first sections and floats in empty space once
    // they are destroyed (task 20260709-150711).
    let (player_transform, player_com) = player.into_inner();
    let player_anchor = live_structure_anchor(player_transform, player_com);

    for (entity, transform, velocity, inertia, com) in &q_spaceship {
        // Both ends of the chase vector track live structure: the AI's own
        // root origin goes as stale as the player's once sections die.
        let own_anchor = live_structure_anchor(transform, com);
        let to_player = player_anchor - own_anchor;
        let desired_direction = ai_desired_direction(to_player, **velocity);

        // Slew the command at the hull's torque-budget turn rate instead of
        // rewriting it every frame: a distant setpoint drives the PD into
        // torque saturation where its damping is swamped and the hull
        // limit-cycles - the regime the player path was fixed for in the
        // flight-feel retune (20260709-095043). Same derivation as the
        // player path and the autopilot (flight::ship_turn_rate). With no
        // live computer the command FREEZES, matching the player path:
        // nothing consumes it, and slewing a dead helm would drift it so a
        // later re-activation snaps the hull.
        let Some(turn_rate) = crate::flight::ship_turn_rate(
            q_computer
                .iter()
                .filter(|(_, &ChildOf(parent))| parent == entity)
                .map(|(pd, _)| pd.max_torque),
            inertia,
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

fn on_thruster_input(
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
        ),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Single<
        (&Transform, Option<&ComputedCenterOfMass>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let (player_transform, player_com) = player.into_inner();
    let player_anchor = live_structure_anchor(player_transform, player_com);

    for (entity, transform, velocity, com) in &q_spaceship {
        // Same live-structure vector as the rotation system, so the thrust
        // gate and the rotation command agree on where "toward the player" is.
        let to_player = player_anchor - live_structure_anchor(transform, com);
        let desired_direction = ai_desired_direction(to_player, **velocity);

        // Thrust only when the ship is pointing roughly toward the desired direction.
        let forward = transform.forward();
        let alignment = forward.dot(desired_direction);
        let thrust_level = if alignment > AI_THRUST_ALIGNMENT {
            1.0
        } else {
            0.0
        };

        for (mut thruster_input, _, _) in q_thruster
            .iter_mut()
            .filter(|(_, _, ChildOf(parent))| *parent == entity)
        {
            **thruster_input = thrust_level;
        }
    }
}

fn update_turret_target_input(
    mut q_turret: Query<(&mut TurretSectionTargetInput, &ChildOf), With<TurretSectionMarker>>,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<AISpaceshipMarker>)>,
    player: Single<
        (&Transform, Option<&ComputedCenterOfMass>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    // Aim at the live structure: fire converging on the root origin lands in
    // empty space once the player's front sections die (task 20260709-150711).
    let (transform, com) = player.into_inner();
    let player_anchor = live_structure_anchor(transform, com);

    for entity in &q_spaceship {
        for (mut turret_input, _) in q_turret
            .iter_mut()
            .filter(|(_, ChildOf(c_parent))| *c_parent == entity)
        {
            **turret_input = Some(player_anchor);
        }
    }
}

fn on_projectile_input(
    mut q_turret: Query<
        (
            &TurretSectionMuzzleEntity,
            &mut TurretSectionInput,
            &ChildOf,
        ),
        With<TurretSectionMarker>,
    >,
    q_muzzle: Query<&GlobalTransform, With<TurretSectionBarrelMuzzleMarker>>,
    q_spaceship: Query<Entity, (With<SpaceshipRootMarker>, With<AISpaceshipMarker>)>,
    player: Single<
        (&Transform, Option<&ComputedCenterOfMass>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let (player_transform, player_com) = player.into_inner();
    let player_anchor = live_structure_anchor(player_transform, player_com);

    for entity in &q_spaceship {
        for (muzzle, mut input, _) in q_turret
            .iter_mut()
            .filter(|(_, _, ChildOf(c_parent))| *c_parent == entity)
        {
            let Ok(muzzle_transform) = q_muzzle.get(**muzzle) else {
                error!(
                    "on_projectile_input: muzzle entity {:?} not found in q_muzzle",
                    **muzzle
                );
                continue;
            };

            let direction_to_player = (player_anchor - muzzle_transform.translation()).normalize();
            let forward = muzzle_transform.forward();

            let alignment = forward.dot(direction_to_player);
            **input = alignment > AI_FIRE_ALIGNMENT;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn ai_turrets_target_the_live_structure_anchor() {
        // AI fire must converge on the player's surviving structure, not the
        // root origin build-spot (task 20260709-150711).
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            ComputedCenterOfMass(Vec3::new(0.0, 0.0, 3.0)),
        ));
        let ai_ship = world.spawn(AISpaceshipMarker).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(10.0, 0.0, 3.0)),
            "AI turret input = the player's live-structure anchor"
        );
    }

    #[test]
    fn ai_turrets_fall_back_to_the_origin_without_a_com() {
        let mut world = World::new();
        world.spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
        ));
        let ai_ship = world.spawn(AISpaceshipMarker).id();
        let turret = world
            .spawn((
                TurretSectionMarker,
                TurretSectionTargetInput(None),
                ChildOf(ai_ship),
            ))
            .id();

        world.run_system_once(update_turret_target_input).unwrap();

        assert_eq!(
            **world
                .entity(turret)
                .get::<TurretSectionTargetInput>()
                .unwrap(),
            Some(Vec3::new(1.0, 2.0, 3.0))
        );
    }
}

#[cfg(test)]
mod rotation_tests {
    // Command-level harness with manual time, mirroring the player path's
    // command_lag_tests: the AI rotation command must be an ABSOLUTE world
    // rotation slewed at the hull's derived turn rate (task 20260709-155921).
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
        app.add_systems(Update, update_controller_target_rotation_torque);

        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
        ));
        // The stock ship's numbers: inertia ~2.3, computer torque 10.
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
                    max_torque: 10.0,
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
        let expected = crate::flight::hull_turn_rate(
            10.0,
            2.3,
            &app.world().resource::<FlightSettings>().clone(),
        ) / 60.0;
        // One frame advances exactly one slew step of the DERIVED rate -
        // this pins hull_turn_rate's wiring, not just "some" slew.
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one torque-budget slew step \
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
    // A real avian world with the real PD, mirroring flight.rs's
    // physics-level harness: AI rotation command -> PD torque -> hull
    // swings. Covers the task's acceptance: the AI swings to the target
    // attitude and settles without limit-cycling (task 20260709-155921).
    use super::*;
    use crate::{
        integrity::test_support::{settle, unfinished_integrity_physics_app},
        sections::controller_section::{
            sync_controller_section_forces, update_controller_section_rotation_input,
        },
    };

    #[test]
    fn the_ai_swings_onto_the_player_and_settles() {
        let mut app = unfinished_integrity_physics_app();
        app.init_resource::<FlightSettings>();
        app.add_plugins(PDControllerPlugin);
        app.configure_sets(
            FixedUpdate,
            (
                super::super::SpaceshipInputSystems,
                PDControllerSystems::Sync,
                SpaceshipSectionSystems,
            )
                .chain(),
        );
        app.add_systems(
            FixedUpdate,
            (
                update_controller_target_rotation_torque,
                update_controller_section_rotation_input,
            )
                .chain()
                .in_set(super::super::SpaceshipInputSystems),
        );
        app.add_systems(
            FixedUpdate,
            sync_controller_section_forces.in_set(SpaceshipSectionSystems),
        );
        app.finish();

        // Player abeam at +X: a 90-degree swing from the AI's initial -Z.
        app.world_mut().spawn((
            SpaceshipRootMarker,
            PlayerSpaceshipMarker,
            Transform::from_translation(Vec3::new(200.0, 0.0, 0.0)),
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
                max_torque: 10.0,
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
        // The aim axes are quiet; what residual spin remains is pure ROLL
        // about the nose, which the bcs PD cannot damp (open bug
        // 20260709-125640, amplitude ~0.23 rad/s in this rig). Bound it so a
        // regression in THIS path still trips, and tighten toward ~0 when
        // the bcs fix lands.
        assert!(
            max_spin < 0.5,
            "residual spin must stay within the known roll-damping bound \
             (20260709-125640), got {max_spin} rad/s"
        );
    }
}
