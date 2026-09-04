//! Scenario-owned suspension of the human helm and its held intent.

use bevy::prelude::*;
use nova_gameplay::{
    prelude::*,
    transform::prelude::{PointRotationInput, PointRotationOutput},
};

use crate::prelude::*;

/// Whether a cinematic or another explicit scenario beat owns player control.
///
/// This blocks only human gameplay input. Simulation, autopilot, AI, scripted
/// orders, timers, and weapons already in flight continue normally.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Reflect)]
pub struct PlayerControlSuspended(pub bool);

impl PlayerControlSuspended {
    /// Whether human gameplay input is currently blocked.
    pub fn is_suspended(&self) -> bool {
        self.0
    }
}

/// Whether player gameplay input must be ignored.
///
/// The optional resource keeps focused input test rigs valid when they do not
/// install the full player-input plugin.
pub(crate) fn player_control_is_suspended(control: Option<Res<PlayerControlSuspended>>) -> bool {
    control.is_some_and(|control| control.is_suspended())
}

/// Suspend player gameplay input and clear every held ship-control intent.
///
/// Idempotent so repeated authored suspension cannot accumulate ownership or
/// require matching resume counts.
pub fn suspend_player_control(world: &mut World) {
    world.insert_resource(PlayerControlSuspended(true));
    clear_player_control_intent(world);
}

/// Restore player gameplay input. Idempotent for reliable teardown and repeat
/// action handling.
pub fn resume_player_control(world: &mut World) {
    world.insert_resource(PlayerControlSuspended(false));
}

fn clear_player_control_intent(world: &mut World) {
    let players: Vec<(Entity, Quat)> = world
        .query_filtered::<(Entity, &Transform), With<PlayerSpaceshipMarker>>()
        .iter(world)
        .map(|(entity, transform)| (entity, transform.rotation))
        .collect();

    for &(player, rotation) in &players {
        if let Some(mut intent) = world.get_mut::<FlightIntent>(player) {
            intent.burn = 0.0;
        }
        if let Some(mut intent) = world.get_mut::<RcsIntent>(player) {
            intent.0 = Vec3::ZERO;
        }
        if let Some(mut raised) = world.get_mut::<WeaponsRaised>(player) {
            raised.0 = false;
        }
        world
            .entity_mut(player)
            .remove::<RcsActive>()
            .remove::<RadarState>();

        let controllers: Vec<Entity> = world
            .query::<(Entity, &ChildOf, &ControllerSectionRotationInput)>()
            .iter(world)
            .filter_map(|(entity, &ChildOf(parent), _)| (parent == player).then_some(entity))
            .collect();
        for controller in controllers {
            **world
                .get_mut::<ControllerSectionRotationInput>(controller)
                .expect("the collected controller still exists") = rotation;
        }

        clear_section_inputs(world, player);
    }

    if let Some(mut mode) = world.get_resource_mut::<SpaceshipCameraControlMode>() {
        *mode = SpaceshipCameraControlMode::Normal;
    }
    for mut input in world.query::<&mut PointRotationInput>().iter_mut(world) {
        **input = Vec2::ZERO;
    }
    let player_rotation = players
        .first()
        .map_or(Quat::IDENTITY, |(_, rotation)| *rotation);
    for mut output in world.query::<&mut PointRotationOutput>().iter_mut(world) {
        **output = player_rotation;
    }
}

fn clear_section_inputs(world: &mut World, player: Entity) {
    for mut input in world
        .query::<(&ChildOf, &mut ThrusterSectionInput)>()
        .iter_mut(world)
        .filter_map(|(&ChildOf(parent), input)| (parent == player).then_some(input))
    {
        **input = 0.0;
    }
    for mut input in world
        .query::<(&ChildOf, &mut TurretSectionInput)>()
        .iter_mut(world)
        .filter_map(|(&ChildOf(parent), input)| (parent == player).then_some(input))
    {
        **input = false;
    }
    for mut input in world
        .query::<(&ChildOf, &mut TorpedoSectionInput)>()
        .iter_mut(world)
        .filter_map(|(&ChildOf(parent), input)| (parent == player).then_some(input))
    {
        **input = false;
    }
    for mut input in world
        .query::<(&ChildOf, &mut RailgunSectionInput)>()
        .iter_mut(world)
        .filter_map(|(&ChildOf(parent), input)| (parent == player).then_some(input))
    {
        **input = false;
    }
}

#[cfg(test)]
mod tests {
    use nova_gameplay::transform::prelude::{PointRotationInput, PointRotationOutput};

    use super::*;

    #[test]
    fn suspension_clears_every_held_player_intent_and_repeat_actions_are_safe() {
        let mut world = World::new();
        world.insert_resource(SpaceshipCameraControlMode::Turret);
        let rotation = Quat::from_rotation_y(0.7);
        let player = world
            .spawn((
                PlayerSpaceshipMarker,
                Transform::from_rotation(rotation),
                FlightIntent { burn: 1.0 },
                RcsIntent(Vec3::ONE),
                RcsActive,
                RadarState::default(),
                WeaponsRaised(true),
            ))
            .id();
        let controller = world
            .spawn((
                ChildOf(player),
                ControllerSectionRotationInput(Quat::IDENTITY),
            ))
            .id();
        let thruster = world
            .spawn((ChildOf(player), ThrusterSectionInput(1.0)))
            .id();
        let turret = world
            .spawn((ChildOf(player), TurretSectionInput(true)))
            .id();
        let torpedo = world
            .spawn((ChildOf(player), TorpedoSectionInput(true)))
            .id();
        let railgun = world
            .spawn((ChildOf(player), RailgunSectionInput(true)))
            .id();
        let look = world
            .spawn((
                PointRotationInput(Vec2::ONE),
                PointRotationOutput(Quat::from_rotation_x(0.4)),
            ))
            .id();

        suspend_player_control(&mut world);
        suspend_player_control(&mut world);

        assert!(world.resource::<PlayerControlSuspended>().is_suspended());
        assert_eq!(world.get::<FlightIntent>(player).unwrap().burn, 0.0);
        assert_eq!(world.get::<RcsIntent>(player).unwrap().0, Vec3::ZERO);
        assert!(world.get::<RcsActive>(player).is_none());
        assert!(world.get::<RadarState>(player).is_none());
        assert_eq!(
            world.get::<WeaponsRaised>(player),
            Some(&WeaponsRaised(false))
        );
        assert_eq!(
            **world
                .get::<ControllerSectionRotationInput>(controller)
                .unwrap(),
            rotation
        );
        assert_eq!(**world.get::<ThrusterSectionInput>(thruster).unwrap(), 0.0);
        assert!(!**world.get::<TurretSectionInput>(turret).unwrap());
        assert!(!**world.get::<TorpedoSectionInput>(torpedo).unwrap());
        assert!(!**world.get::<RailgunSectionInput>(railgun).unwrap());
        assert_eq!(**world.get::<PointRotationInput>(look).unwrap(), Vec2::ZERO);
        assert_eq!(**world.get::<PointRotationOutput>(look).unwrap(), rotation);
        assert_eq!(
            *world.resource::<SpaceshipCameraControlMode>(),
            SpaceshipCameraControlMode::Normal
        );

        resume_player_control(&mut world);
        resume_player_control(&mut world);
        assert!(!world.resource::<PlayerControlSuspended>().is_suspended());
    }
}
