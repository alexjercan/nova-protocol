//! Content-authored section bindings: each section's `input_mapping` becomes a
//! rig whose observers hold and release the section trigger.
//!
//! The binding a section CARRIES is a Nova [`InputSource`], the same vocabulary
//! the registry, the settings readout and the save file speak. A section binds
//! a modifier-free button and nothing else, so `source_bindings` builds the
//! whole rig side of it and upstream's `Binding` never appears here.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use nova_gameplay::prelude::*;
use nova_input::prelude::{source_bindings, InputSource};

use super::hints::HOLD_FIRE_DURING_RADAR;
use crate::prelude::*;

/// The player input bindings that fire a thruster section, snapshotted from its
/// content `input_mapping` onto the section entity. One section may bind several
/// sources. Must not reuse a [`flight_rig_reserved_sources`] source.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipThrusterInputBinding(pub Vec<InputSource>);

/// A live player-ship section changed its complete input binding list.
#[derive(Message, Clone, Debug)]
pub struct SectionInputBindingChanged {
    /// Player spaceship root that owns the section.
    pub spaceship: Entity,
    /// Live section entity.
    pub section: Entity,
    /// Stable authored section id.
    pub section_id: String,
    /// Complete replacement binding list.
    pub bindings: Vec<InputSource>,
}

#[derive(Component, Debug, Clone)]
pub(super) struct ThrusterInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct ThrusterInput;

pub(super) fn on_thruster_input_binding(
    add: On<Insert, SpaceshipThrusterInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipThrusterInputBinding>,
    q_actions: Query<&Actions<ThrusterInputMarker>>,
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

    if let Ok(actions) = q_actions.get(entity) {
        for action in actions {
            commands.entity(action).despawn();
        }
    }
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
                source_bindings(binding.0.iter().copied()),
            )]
        ),
    ));
}

pub(super) fn on_thruster_input(
    fire: On<Start<ThrusterInput>>,
    mut commands: Commands,
    mut q_input: Query<(&mut ThrusterSectionInput, Option<&ChildOf>), With<ThrusterInputMarker>>,
    q_docked: Query<&DockedShip>,
    pause: Res<State<nova_gameplay::PauseStates>>,
    control: Option<Res<PlayerControlSuspended>>,
) {
    // Observers bypass system-set gating; block presses while an overlay or
    // cinematic owns input. Releases stay live so held intent still clears.
    if pause.get().is_frozen() || super::control::player_control_is_suspended(control) {
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

    // A docked hull that does not drive its pair makes no thrust, so a bound
    // throttle must not light a plume for a burn that cannot move it. HELM is
    // the way in, and it is a verb, not a throttle.
    if child_of.is_some_and(|&ChildOf(ship)| q_docked.get(ship).is_ok_and(|docked| !docked.drives))
    {
        debug!("on_thruster_input: a docked hull's throttle is inert without the helm");
        return;
    }

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
pub struct SpaceshipTurretInputBinding(pub Vec<InputSource>);

#[derive(Component, Debug, Clone)]
pub(super) struct TurretInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct TurretInput;

pub(super) fn on_turret_input_binding(
    add: On<Insert, SpaceshipTurretInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipTurretInputBinding>,
    q_actions: Query<&Actions<TurretInputMarker>>,
) {
    let entity = add.entity;
    trace!("on_turret_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };
    if let Ok(actions) = q_actions.get(entity) {
        for action in actions {
            commands.entity(action).despawn();
        }
    }

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
                source_bindings(binding.0.iter().copied()),
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
    control: Option<Res<PlayerControlSuspended>>,
) {
    if pause.get().is_frozen() || super::control::player_control_is_suspended(control) {
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
pub struct SpaceshipTorpedoInputBinding(pub Vec<InputSource>);

#[derive(Component, Debug, Clone)]
pub(super) struct TorpedoInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct TorpedoInput;

pub(super) fn on_torpedo_input_binding(
    add: On<Insert, SpaceshipTorpedoInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipTorpedoInputBinding>,
    q_actions: Query<&Actions<TorpedoInputMarker>>,
) {
    let entity = add.entity;
    trace!("on_torpedo_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };
    if let Ok(actions) = q_actions.get(entity) {
        for action in actions {
            commands.entity(action).despawn();
        }
    }

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
                source_bindings(binding.0.iter().copied()),
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
    control: Option<Res<PlayerControlSuspended>>,
) {
    if pause.get().is_frozen() || super::control::player_control_is_suspended(control) {
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

/// The player input bindings that commit a railgun shot, snapshotted from its
/// content `input_mapping`. Same rules as [`SpaceshipThrusterInputBinding`].
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipRailgunInputBinding(pub Vec<InputSource>);

#[derive(Component, Debug, Clone)]
pub(super) struct RailgunInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct RailgunInput;

pub(super) fn on_railgun_input_binding(
    add: On<Insert, SpaceshipRailgunInputBinding>,
    mut commands: Commands,
    q_binding: Query<&SpaceshipRailgunInputBinding>,
    q_actions: Query<&Actions<RailgunInputMarker>>,
) {
    let entity = add.entity;
    trace!("on_railgun_input_binding: entity {:?}", entity);

    let Ok(binding) = q_binding.get(entity) else {
        return;
    };
    if let Ok(actions) = q_actions.get(entity) {
        for action in actions {
            commands.entity(action).despawn();
        }
    }

    commands.entity(entity).insert((
        RailgunInputMarker,
        actions!(
            RailgunInputMarker[(
                Name::new("Input: Railgun"),
                Action::<RailgunInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                source_bindings(binding.0.iter().copied()),
            )]
        ),
    ));
}

/// The COMMIT. A press starts the charge and the charge runs to completion
/// whatever happens to the button, so unlike the turret and the bay there is
/// nothing here that a release has to undo - see `on_railgun_input_completed`.
pub(super) fn on_railgun_input(
    fire: On<Start<RailgunInput>>,
    mut q_input: Query<&mut RailgunSectionInput, With<RailgunInputMarker>>,
    q_player_safety: Query<
        (&WeaponsHot, Option<&RadarState>),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    pause: Res<State<nova_gameplay::PauseStates>>,
    control: Option<Res<PlayerControlSuspended>>,
) {
    if pause.get().is_frozen() || super::control::player_control_is_suspended(control) {
        return;
    }

    // The weapons safety denies the PRESS on a managed cold ship, exactly as
    // it does for the other two weapons. The live section-side gate is still
    // the enforcement, and it is the half that matters here: safing a ship
    // mid-charge dumps the charge rather than firing it.
    let cold = q_player_safety
        .iter()
        .next()
        .is_some_and(|(hot, radar)| !hot.0 || (HOLD_FIRE_DURING_RADAR && radar.is_some()));
    if cold {
        return;
    }

    let entity = fire.event().context;
    trace!("on_railgun_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

/// Drop the trigger. It cannot abort a charge already running - the section
/// reads this only to decide whether to COMMIT - so a release means "do not
/// start another shot", which is what makes a tap one shot and a hold the gun
/// cycling at its own cadence.
pub(super) fn on_railgun_input_completed(
    fire: On<Complete<RailgunInput>>,
    mut q_input: Query<&mut RailgunSectionInput, With<RailgunInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_railgun_input_completed: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = false;
}

/// The player input bindings that hold one mining emitter out, snapshotted
/// from its content `input_mapping`. Same rules as
/// [`SpaceshipThrusterInputBinding`]. A player ship's mining section must
/// carry one; the lint and the spawn refuse a section without it.
#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipMiningInputBinding(pub Vec<InputSource>);

#[derive(Component, Debug, Clone)]
pub(super) struct MiningInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
pub(super) struct MiningSectionInput;

pub(super) fn on_mining_input_binding(
    add: On<Insert, SpaceshipMiningInputBinding>,
    mut commands: Commands,
    mut q_binding: Query<(&SpaceshipMiningInputBinding, Option<&mut MiningSectionHeld>)>,
    q_actions: Query<&Actions<MiningInputMarker>>,
) {
    let entity = add.entity;
    trace!("on_mining_input_binding: entity {:?}", entity);

    let Ok((binding, held)) = q_binding.get_mut(entity) else {
        return;
    };
    // The old action is despawned below without a Complete, so a key held
    // through the rebind would leave the emitter out with nothing to release.
    if let Some(mut held) = held {
        **held = false;
    }
    if let Ok(actions) = q_actions.get(entity) {
        for action in actions {
            commands.entity(action).despawn();
        }
    }

    commands.entity(entity).insert((
        MiningInputMarker,
        actions!(
            MiningInputMarker[(
                Name::new("Input: Mining"),
                Action::<MiningSectionInput>::new(),
                ActionSettings {
                    consume_input: false,
                    ..default()
                },
                source_bindings(binding.0.iter().copied()),
            )]
        ),
    ));
}

/// Hold this emitter out. Not gated by the weapons safety: a beam cuts the
/// travel-locked rock only, so a cold ship still mines.
pub(super) fn on_mining_input(
    fire: On<Start<MiningSectionInput>>,
    mut q_input: Query<&mut MiningSectionHeld, With<MiningInputMarker>>,
    pause: Res<State<nova_gameplay::PauseStates>>,
    control: Option<Res<PlayerControlSuspended>>,
) {
    if pause.get().is_frozen() || super::control::player_control_is_suspended(control) {
        return;
    }

    let entity = fire.event().context;
    trace!("on_mining_input: entity {:?}", entity);

    let Ok(mut held) = q_input.get_mut(entity) else {
        return;
    };

    **held = true;
}

pub(super) fn on_mining_input_completed(
    fire: On<Complete<MiningSectionInput>>,
    mut q_input: Query<&mut MiningSectionHeld, With<MiningInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_mining_input_completed: entity {:?}", entity);

    let Ok(mut held) = q_input.get_mut(entity) else {
        return;
    };

    **held = false;
}

#[cfg(test)]
mod tests {
    use nova_input::prelude::{binding_source, InputSource};

    use super::*;

    #[test]
    fn replacing_a_section_binding_rebuilds_one_live_action_tree() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, EnhancedInputPlugin));
        app.add_input_context::<TurretInputMarker>();
        app.add_observer(on_turret_input_binding);
        app.finish();
        app.cleanup();
        app.update();
        let section = app
            .world_mut()
            .spawn(SpaceshipTurretInputBinding(vec![KeyCode::KeyF.into()]))
            .id();
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Binding>()
                .iter(app.world())
                .count(),
            1
        );

        app.world_mut()
            .entity_mut(section)
            .insert(SpaceshipTurretInputBinding(vec![KeyCode::KeyB.into()]));
        app.update();

        let bindings: Vec<Binding> = app
            .world_mut()
            .query::<&Binding>()
            .iter(app.world())
            .cloned()
            .collect();
        assert_eq!(bindings.len(), 1, "the replaced action tree was reaped");
        assert_eq!(
            binding_source(&bindings[0]),
            Some(InputSource::Keyboard(KeyCode::KeyB))
        );
    }

    /// Each mining section answers its own key alone, and a rebind moves only
    /// that section: the old key stops holding it and the other emitter's key
    /// is untouched.
    #[test]
    fn two_mining_sections_hold_on_their_own_keys_only() {
        use bevy::input::InputPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, EnhancedInputPlugin));
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<nova_gameplay::PauseStates>();
        app.add_input_context::<MiningInputMarker>();
        app.add_observer(on_mining_input_binding);
        app.add_observer(on_mining_input);
        app.add_observer(on_mining_input_completed);
        app.finish();
        app.cleanup();
        app.update();
        let bow = app
            .world_mut()
            .spawn((
                MiningSectionHeld(false),
                SpaceshipMiningInputBinding(vec![KeyCode::KeyV.into()]),
            ))
            .id();
        let keel = app
            .world_mut()
            .spawn((
                MiningSectionHeld(false),
                SpaceshipMiningInputBinding(vec![KeyCode::KeyB.into()]),
            ))
            .id();
        app.update();

        let held = |app: &App| {
            [bow, keel].map(|section| app.world().get::<MiningSectionHeld>(section).unwrap().0)
        };
        let tap = |app: &mut App, key: KeyCode| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            app.update();
            let during = held(app);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
            app.update();
            app.update();
            assert_eq!(held(app), [false, false], "{key:?} released both");
            during
        };

        assert_eq!(tap(&mut app, KeyCode::KeyV), [true, false]);
        assert_eq!(tap(&mut app, KeyCode::KeyB), [false, true]);

        app.world_mut()
            .entity_mut(bow)
            .insert(SpaceshipMiningInputBinding(vec![KeyCode::KeyJ.into()]));
        app.update();
        assert_eq!(tap(&mut app, KeyCode::KeyV), [false, false]);
        assert_eq!(tap(&mut app, KeyCode::KeyJ), [true, false]);
        assert_eq!(tap(&mut app, KeyCode::KeyB), [false, true]);
    }
}
