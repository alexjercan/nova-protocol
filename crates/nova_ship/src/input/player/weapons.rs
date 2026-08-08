//! Content-authored weapon bindings: each section's `input_mapping` becomes a
//! rig whose observers hold and release the section trigger.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;

use super::hints::HOLD_FIRE_DURING_RADAR;
use crate::prelude::*;

/// The player input bindings that fire a thruster section, snapshotted from its
/// content `input_mapping` onto the section entity. One section may bind several
/// [`Binding`]s. Must not reuse a [`flight_rig_reserved_sources`] source.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipThrusterInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
pub(super) struct ThrusterInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct ThrusterInput;

pub(super) fn on_thruster_input_binding(
    add: On<Add, SpaceshipThrusterInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipThrusterInputBinding>,
) {
    let entity = add.entity;
    trace!("on_thruster_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        error!(
            "on_thruster_input_binding: entity {:?} not found in q_binding",
            entity
        );
        return;
    };

    commands.entity(entity).insert((
        ThrusterInputMarker,
        actions!(
            ThrusterInputMarker[(
                Name::new("Input: Thruster"),
                Action::<ThrusterInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                Bindings::spawn(binding.0.clone()),
            )]
        ),
    ));
}

pub(super) fn on_thruster_input(
    fire: On<Start<ThrusterInput>>,
    mut commands: Commands,
    mut q_input: Query<(&mut ThrusterSectionInput, Option<&ChildOf>), With<ThrusterInputMarker>>,
    pause: Res<State<nova_gameplay::PauseStates>>,
) {
    // NOTE: observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }

    let entity = fire.event().context;
    trace!("on_thruster_input: entity {:?}", entity);

    let Ok((mut input, child_of)) = q_input.get_mut(entity) else {
        error!(
            "on_thruster_input: entity {:?} not found in q_input",
            entity
        );
        return;
    };

    **input = 1.0;
    // Grabbing a bound throttle is a flight input: it takes the ship back
    // from an engaged autopilot (removing an absent component is a no-op).
    if let Some(&ChildOf(ship)) = child_of {
        commands.entity(ship).remove::<Autopilot>();
    }
}

pub(super) fn on_thruster_input_completed(
    fire: On<Complete<ThrusterInput>>,
    mut q_input: Query<&mut ThrusterSectionInput, With<ThrusterInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_thruster_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = 0.0;
}

/// The player input bindings that fire a turret section, snapshotted from its
/// content `input_mapping`. Same rules as [`SpaceshipThrusterInputBinding`].
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipTurretInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
pub(super) struct TurretInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct TurretInput;

pub(super) fn on_turret_input_binding(
    add: On<Add, SpaceshipTurretInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipTurretInputBinding>,
) {
    let entity = add.entity;
    trace!("on_turret_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };

    commands.entity(entity).insert((
        TurretInputMarker,
        actions!(
            TurretInputMarker[(
                Name::new("Input: Turret"),
                Action::<TurretInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                Bindings::spawn(binding.0.clone()),
            )]
        ),
    ));
}

pub(super) fn on_turret_input(
    fire: On<Start<TurretInput>>,
    mut q_input: Query<&mut TurretSectionInput, With<TurretInputMarker>>,
    q_player_safety: Query<
        (&WeaponsHot, Option<&RadarState>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<nova_gameplay::PauseStates>>,
) {
    // NOTE: observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }

    // The weapons safety denies the PRESS on a managed cold ship (the live
    // section-side gate is the enforcement; this is the immediate feedback
    // path - the input bool never even latches). HOLD_FIRE_DURING_RADAR:
    // optional playtest flag from the adversarial round (sweeping with the
    // trigger down rakes bystanders); off by default.
    let cold = q_player_safety
        .iter()
        .next()
        .is_some_and(|(hot, radar)| !hot.0 || (HOLD_FIRE_DURING_RADAR && radar.is_some()));
    if cold {
        return;
    }

    let entity = fire.event().context;
    trace!("on_turret_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

pub(super) fn on_turret_input_completed(
    fire: On<Complete<TurretInput>>,
    mut q_input: Query<&mut TurretSectionInput, With<TurretInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_turret_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = false;
}

/// The player input bindings that fire a torpedo section, snapshotted from its
/// content `input_mapping`. Same rules as [`SpaceshipThrusterInputBinding`].
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipTorpedoInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
pub(super) struct TorpedoInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct TorpedoInput;

pub(super) fn on_torpedo_input_binding(
    add: On<Add, SpaceshipTorpedoInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipTorpedoInputBinding>,
) {
    let entity = add.entity;
    trace!("on_torpedo_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };

    commands.entity(entity).insert((
        TorpedoInputMarker,
        actions!(
            TorpedoInputMarker[(
                Name::new("Input: Torpedo"),
                Action::<TorpedoInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                Bindings::spawn(binding.0.clone()),
            )]
        ),
    ));
}

pub(super) fn on_torpedo_input(
    fire: On<Start<TorpedoInput>>,
    mut q_input: Query<&mut TorpedoSectionInput, With<TorpedoInputMarker>>,
    q_player_safety: Query<
        (&WeaponsHot, Option<&RadarState>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<nova_gameplay::PauseStates>>,
) {
    // NOTE: observers bypass system-set gating; freeze intent changes while the
    // pause overlay is up. Releases stay ungated so held keys clear cleanly
    // during a pause.
    if pause.get().is_frozen() {
        return;
    }

    // The weapons safety denies the PRESS on a managed cold ship (the live
    // section-side gate is the enforcement; this is the immediate feedback
    // path - the input bool never even latches). HOLD_FIRE_DURING_RADAR:
    // optional playtest flag from the adversarial round (sweeping with the
    // trigger down rakes bystanders); off by default.
    let cold = q_player_safety
        .iter()
        .next()
        .is_some_and(|(hot, radar)| !hot.0 || (HOLD_FIRE_DURING_RADAR && radar.is_some()));
    if cold {
        return;
    }

    let entity = fire.event().context;
    trace!("on_torpedo_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

pub(super) fn on_torpedo_input_completed(
    fire: On<Complete<TorpedoInput>>,
    mut q_input: Query<&mut TorpedoSectionInput, With<TorpedoInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_torpedo_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = false;
}
