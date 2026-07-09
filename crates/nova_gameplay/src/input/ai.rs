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
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    q_spaceship: Query<
        (Entity, &Transform, &LinearVelocity),
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

    for (entity, transform, velocity) in &q_spaceship {
        let to_player = player_anchor - transform.translation;
        let desired_direction = ai_desired_direction(to_player, **velocity);

        let forward = transform.forward().into();
        let target_rotation = Quat::from_rotation_arc(forward, desired_direction);

        for (mut controller, _) in q_controller
            .iter_mut()
            .filter(|(_, ChildOf(parent))| *parent == entity)
        {
            **controller = target_rotation;
        }
    }
}

fn on_thruster_input(
    mut q_thruster: Query<
        (&mut ThrusterSectionInput, &GlobalTransform, &ChildOf),
        With<ThrusterSectionMarker>,
    >,
    q_spaceship: Query<
        (Entity, &Transform, &LinearVelocity),
        (With<SpaceshipRootMarker>, With<AISpaceshipMarker>),
    >,
    player: Single<
        (&Transform, Option<&ComputedCenterOfMass>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let (player_transform, player_com) = player.into_inner();
    let player_anchor = live_structure_anchor(player_transform, player_com);

    for (entity, transform, velocity) in &q_spaceship {
        let to_player = player_anchor - transform.translation;
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
