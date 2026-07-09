use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::prelude::*;

pub mod prelude {
    pub use super::{
        PlayerSpaceshipMarker, SpaceshipPlayerInputPlugin, SpaceshipPlayerTorpedoTargetEntity,
        SpaceshipThrusterInputBinding, SpaceshipTorpedoInputBinding, SpaceshipTurretInputBinding,
    };
}

// TODO(20260706-162913): NEED TO REFACTOR THIS, right now we just scuff it out to make it work
#[derive(Resource, Debug, Clone, Deref, DerefMut, Default)]
pub struct SpaceshipPlayerTorpedoTargetEntity(pub Option<Entity>);

pub struct SpaceshipPlayerInputPlugin;

impl Plugin for SpaceshipPlayerInputPlugin {
    fn build(&self, app: &mut App) {
        debug!("SpaceshipPlayerInputPlugin: build");

        app.insert_resource(SpaceshipPlayerTorpedoTargetEntity::default());

        app.add_input_context::<FlightInputMarker>();
        app.add_observer(on_player_added_spawn_flight_input);
        app.add_observer(on_player_removed_despawn_flight_input);
        app.add_observer(on_flight_burn_input);
        app.add_observer(on_flight_burn_input_completed);
        app.add_observer(on_autopilot_stop_input);
        app.add_observer(on_autopilot_goto_input);
        app.add_observer(on_autopilot_off_input);

        app.add_input_context::<ThrusterInputMarker>();
        app.add_observer(on_thruster_input_binding);
        app.add_observer(on_thruster_input);
        app.add_observer(on_thruster_input_completed);

        app.add_input_context::<TurretInputMarker>();
        app.add_observer(on_turret_input_binding);
        app.add_observer(on_turret_input);
        app.add_observer(on_turret_input_completed);

        app.add_input_context::<TorpedoInputMarker>();
        app.add_observer(on_torpedo_input_binding);
        app.add_observer(on_torpedo_input);
        app.add_observer(on_torpedo_input_completed);

        app.add_systems(
            Update,
            (
                update_controller_target_rotation_torque,
                update_turret_target_input,
                (update_spaceship_target_input, update_torpedo_target_input).chain(),
            )
                .in_set(super::SpaceshipInputSystems),
        );
    }
}

/// Marker component to identify the player's spaceship.
///
/// This should be added to the root entity of the player's spaceship.
#[derive(Component, Debug, Clone, Reflect)]
#[require(SpaceshipRootMarker)]
pub struct PlayerSpaceshipMarker;

/// System that takes the point rotation output from the chase camera and applies it to the
/// controller of the player's spaceship.
///
/// Gated on `Without<Autopilot>`: while a maneuver is engaged the autopilot
/// owns the rotation command, and the mouse - which keeps driving the camera
/// rig - becomes camera-only free-look for free.
fn update_controller_target_rotation_torque(
    time: Res<Time>,
    settings: Res<FlightSettings>,
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraNormalInputMarker>,
        ),
    >,
    mut q_controller: Query<
        (&mut ControllerSectionRotationInput, &ChildOf),
        With<ControllerSectionMarker>,
    >,
    spaceship: Single<
        (Entity, &ComputedAngularInertia),
        (
            With<SpaceshipRootMarker>,
            With<PlayerSpaceshipMarker>,
            Without<Autopilot>,
        ),
    >,
    q_computer: Query<
        (&PDController, &ChildOf),
        (
            With<ControllerSectionMarker>,
            Without<SectionInactiveMarker>,
        ),
    >,
) {
    let point_rotation = point_rotation.into_inner();
    let (spaceship, inertia) = spaceship.into_inner();
    // Slew the command toward the camera instead of jumping: a mouse 180 fed
    // to the PD in one step drives it into torque saturation where its
    // damping is swamped and the hull limit-cycles (the high-speed flip
    // wobble). The camera stays instant; the hull's commanded target ramps
    // at the hull's own torque-budget turn rate - the same one the autopilot
    // plans with - so a heavy build swings slower than a stripped one. (PD
    // outputs stack additively across computers; max is a conservative
    // simplification, matching the autopilot.) With no live computer the
    // command FREEZES: nothing consumes it, and slewing a dead helm would
    // drift it so a later re-activation snaps the hull.
    let Some(computer_torque) = q_computer
        .iter()
        .filter(|(_, &ChildOf(parent))| parent == spaceship)
        .map(|(pd, _)| pd.max_torque)
        .reduce(f32::max)
    else {
        return;
    };
    let (principal, _) = inertia.principal_angular_inertia_with_local_frame();
    let turn_rate =
        crate::flight::hull_turn_rate(computer_torque, principal.max_element(), &settings);
    let max_step = turn_rate * time.delta_secs();

    for (mut controller, _) in q_controller
        .iter_mut()
        .filter(|(_, ChildOf(c_parent))| *c_parent == spaceship)
    {
        **controller = crate::flight::slew_rotation(**controller, **point_rotation, max_step);
    }
}

/// System that takes the point rotation output from the chase camera and applies it to the
/// turret target input of the player's spaceship.
fn update_turret_target_input(
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    mut q_turret: Query<(&mut TurretSectionTargetInput, &ChildOf), With<TurretSectionMarker>>,
    spaceship: Single<
        (&Transform, Entity),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
) {
    let point_rotation = point_rotation.into_inner();
    let (transform, spaceship) = spaceship.into_inner();

    for (mut turret, _) in q_turret
        .iter_mut()
        .filter(|(_, ChildOf(t_parent))| *t_parent == spaceship)
    {
        let forward = **point_rotation * Vec3::NEG_Z;
        let position = transform.translation;
        let distance = 100.0;

        **turret = Some(position + forward * distance);
    }
}

/// Maximum distance at which the aim-assist will lock a target. Bodies further
/// than this from the ship are ignored, so distant clutter never steals the lock.
const TARGETING_MAX_RANGE: f32 = 2000.0;

/// Half-angle (degrees) of the lock-on cone around the aim direction. Any lockable
/// body whose bearing from the ship falls within this angle of where the player is
/// aiming is eligible, and the one closest to the aim ray wins. This is the whole
/// point of the aim-assist: a wide cone means the player only has to point roughly
/// at a target instead of landing a pixel-perfect ray on it. Pan the view and the
/// lock snaps to whichever eligible body is now nearest the center, so cycling
/// between targets is just "look at the next one".
const TARGETING_CONE_HALF_ANGLE_DEG: f32 = 18.0;

/// Choose the best lock-on target from `candidates` (each an entity and its world
/// position): the one whose bearing from `origin` is closest to the `aim`
/// direction, as long as it is within `max_range` and inside the cone (bearing
/// dot aim `>= min_cos`, i.e. `min_cos = cos(half_angle)`). Returns `None` when
/// nothing qualifies - e.g. the player is looking at empty space - which drops the
/// lock and hides the reticle.
///
/// Pure and camera/physics-free so the selection rule can be unit-tested directly.
fn pick_target(
    origin: Vec3,
    aim: Vec3,
    max_range: f32,
    min_cos: f32,
    candidates: impl Iterator<Item = (Entity, Vec3)>,
) -> Option<Entity> {
    candidates
        .filter_map(|(entity, position)| {
            let to_target = position - origin;
            let distance = to_target.length();
            if distance > max_range || distance < f32::EPSILON {
                return None;
            }
            let cos_angle = to_target.normalize().dot(aim);
            (cos_angle >= min_cos).then_some((entity, cos_angle))
        })
        // Largest cosine == smallest angle from the aim ray == closest to center.
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
}

/// Update the player's torpedo lock from where the crosshair is aimed, using
/// angular aim-assist rather than a single ray: enumerate the physical bodies in
/// front of the ship and lock the one nearest the aim direction (see
/// [`pick_target`]).
fn update_spaceship_target_input(
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    // Turret bullets are excluded outright: they are dynamic bodies that stream
    // straight down the aim ray, so without this the lock would constantly snap
    // onto the player's own gunfire instead of the enemy behind it.
    q_candidates: Query<
        (
            Entity,
            &GlobalTransform,
            &RigidBody,
            Option<&TorpedoProjectileMarker>,
            Option<&TorpedoTargetChosen>,
        ),
        Without<TurretBulletProjectileMarker>,
    >,
    spaceship: Single<
        (&GlobalTransform, Entity),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    mut res_target: ResMut<SpaceshipPlayerTorpedoTargetEntity>,
) {
    let point_rotation = point_rotation.into_inner();
    let (ship_transform, ship_entity) = spaceship.into_inner();

    let origin = ship_transform.translation();
    let aim = (**point_rotation * Vec3::NEG_Z).normalize();
    let min_cos = TARGETING_CONE_HALF_ANGLE_DEG.to_radians().cos();

    let candidates = q_candidates.iter().filter_map(
        |(entity, transform, rigid_body, is_torpedo, torpedo_committed)| {
            // Only physical, movable bodies are lockable. This skips static sensor
            // volumes such as scenario trigger areas (`RigidBody::Static`), which
            // are invisible and must never be locked.
            if !matches!(rigid_body, RigidBody::Dynamic) {
                return None;
            }
            // Never lock the player's own ship.
            if entity == ship_entity {
                return None;
            }
            // Skip a freshly launched torpedo that has not committed its
            // launch-time target yet: it spawns right on the aim ray and would
            // otherwise be picked as its own target. Once committed
            // (`TorpedoTargetChosen`) a torpedo is a normal lockable body - e.g.
            // you can lock and shoot down your own dumb-fired torpedo.
            if is_torpedo.is_some() && torpedo_committed.is_none() {
                return None;
            }
            Some((entity, transform.translation()))
        },
    );

    **res_target = pick_target(origin, aim, TARGETING_MAX_RANGE, min_cos, candidates);
}

/// Commit each freshly launched torpedo to its launch-time target.
///
/// A torpedo's targeting decision is made exactly once, right after launch:
/// whatever the crosshair has locked at that moment becomes the torpedo's target
/// for life (`TorpedoTargetChosen` marks the decision as made). No lock means a
/// dumb-fire shot that never acquires anything mid-flight - so, e.g., bullets
/// fired past a loitering torpedo are not picked up as targets, and a torpedo
/// whose target died (link dropped by `update_target_position`, position frozen)
/// is not re-assigned to whatever the player locks next.
fn update_torpedo_target_input(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &ProjectileOwner),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    spaceship: Single<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
    res_target: Res<SpaceshipPlayerTorpedoTargetEntity>,
) {
    let spaceship = spaceship.into_inner();

    for (torpedo, owner) in &q_torpedo {
        if **owner != spaceship {
            continue;
        }

        debug!(
            "update_torpedo_target_input: committing torpedo {:?} to target {:?}",
            torpedo, **res_target
        );

        let mut torpedo_commands = commands.entity(torpedo);
        torpedo_commands.insert(TorpedoTargetChosen);
        if let Some(target_entity) = **res_target {
            torpedo_commands.insert(TorpedoTargetEntity(target_entity));
        }
    }
}

/// Input context for the player's flight controls: analog main-drive burn
/// plus the autopilot engagements. One rig exists while a player ship does;
/// the observers below write the ship's [`FlightIntent`] and insert/remove
/// its [`Autopilot`] (`crate::flight`). Any flight input while an autopilot
/// is engaged disengages it - mouse-look does not, so watching a maneuver
/// never cancels it.
#[derive(Component, Debug, Clone)]
struct FlightInputMarker;

/// Analog main-drive burn (`0..1`).
#[derive(InputAction)]
#[action_output(f32)]
struct FlightBurnInput;

/// Engage the STOP maneuver (kill all velocity); pressing it again while
/// stopping disengages.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotStopInput;

/// Engage the GOTO maneuver on the current aim-assist lock; pressing it again
/// while flying there disengages.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotGotoInput;

/// Plain autopilot off.
#[derive(InputAction)]
#[action_output(bool)]
struct AutopilotOffInput;

fn on_player_added_spawn_flight_input(
    add: On<Add, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_existing: Query<(), With<FlightInputMarker>>,
) {
    trace!(
        "on_player_added_spawn_flight_input: entity {:?}",
        add.entity
    );
    // One player, one flight rig; a respawn reuses the existing one.
    if !q_existing.is_empty() {
        return;
    }

    commands.spawn((
        Name::new("Input: Flight"),
        FlightInputMarker,
        actions!(
            FlightInputMarker[
                (
                    Name::new("Input: Flight Burn"),
                    Action::<FlightBurnInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![
                        KeyCode::KeyW,
                        KeyCode::Space,
                        GamepadButton::RightTrigger
                    ],
                ),
                (
                    Name::new("Input: Autopilot Stop"),
                    Action::<AutopilotStopInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![KeyCode::KeyX, GamepadButton::East],
                ),
                (
                    Name::new("Input: Autopilot Goto"),
                    Action::<AutopilotGotoInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![KeyCode::KeyG, GamepadButton::North],
                ),
                (
                    Name::new("Input: Autopilot Off"),
                    Action::<AutopilotOffInput>::new(),
                    ActionSettings {
                        consume_input: false,
                        ..default()
                    },
                    bindings![KeyCode::KeyZ, GamepadButton::West],
                ),
            ]
        ),
    ));
}

fn on_player_removed_despawn_flight_input(
    remove: On<Remove, PlayerSpaceshipMarker>,
    mut commands: Commands,
    q_rig: Query<Entity, With<FlightInputMarker>>,
) {
    trace!(
        "on_player_removed_despawn_flight_input: entity {:?}",
        remove.entity
    );
    for rig in &q_rig {
        commands.entity(rig).try_despawn();
    }
}

fn on_flight_burn_input(
    fire: On<Fire<FlightBurnInput>>,
    mut commands: Commands,
    ship: Single<(Entity, &mut FlightIntent, Has<Autopilot>), With<PlayerSpaceshipMarker>>,
) {
    let (entity, mut intent, engaged) = ship.into_inner();
    intent.burn = fire.value;
    // Grabbing the throttle is a flight input: it takes the ship back.
    if engaged {
        debug!("on_flight_burn_input: manual burn disengages the autopilot");
        commands.entity(entity).remove::<Autopilot>();
    }
}

fn on_flight_burn_input_completed(
    _: On<Complete<FlightBurnInput>>,
    ship: Single<&mut FlightIntent, With<PlayerSpaceshipMarker>>,
) {
    let mut intent = ship.into_inner();
    intent.burn = 0.0;
}

fn on_autopilot_stop_input(
    _: On<Start<AutopilotStopInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Option<&Autopilot>), With<PlayerSpaceshipMarker>>,
) {
    let (entity, autopilot) = ship.into_inner();
    match autopilot.map(|ap| ap.action) {
        // Toggle off an active STOP...
        Some(AutopilotAction::Stop) => {
            debug!("on_autopilot_stop_input: disengaging STOP");
            commands.entity(entity).remove::<Autopilot>();
        }
        // ...but braking overrides any other maneuver (or engages fresh).
        _ => {
            debug!("on_autopilot_stop_input: engaging STOP");
            commands
                .entity(entity)
                .insert(Autopilot::engage(AutopilotAction::Stop));
        }
    }
}

fn on_autopilot_goto_input(
    _: On<Start<AutopilotGotoInput>>,
    mut commands: Commands,
    res_target: Res<SpaceshipPlayerTorpedoTargetEntity>,
    ship: Single<(Entity, Option<&Autopilot>), With<PlayerSpaceshipMarker>>,
) {
    let (entity, autopilot) = ship.into_inner();

    // Already flying somewhere? G toggles the trip off.
    if let Some(Autopilot {
        action: AutopilotAction::Goto { .. },
        ..
    }) = autopilot
    {
        debug!("on_autopilot_goto_input: disengaging GOTO");
        commands.entity(entity).remove::<Autopilot>();
        return;
    }

    // A destination needs a lock; without one this is a no-op (the status
    // line keeps reading MAN, which is the v1 hint).
    let Some(target) = **res_target else {
        debug!("on_autopilot_goto_input: no lock, nothing to fly to");
        return;
    };

    debug!("on_autopilot_goto_input: engaging GOTO {target:?}");
    commands
        .entity(entity)
        .insert(Autopilot::engage(AutopilotAction::Goto { target }));
}

fn on_autopilot_off_input(
    _: On<Start<AutopilotOffInput>>,
    mut commands: Commands,
    ship: Single<(Entity, Has<Autopilot>), With<PlayerSpaceshipMarker>>,
) {
    let (entity, engaged) = ship.into_inner();
    if engaged {
        debug!("on_autopilot_off_input: disengaging");
        commands.entity(entity).remove::<Autopilot>();
    }
}

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipThrusterInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
struct ThrusterInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct ThrusterInput;

fn on_thruster_input_binding(
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

fn on_thruster_input(
    fire: On<Start<ThrusterInput>>,
    mut commands: Commands,
    mut q_input: Query<(&mut ThrusterSectionInput, Option<&ChildOf>), With<ThrusterInputMarker>>,
) {
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

fn on_thruster_input_completed(
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

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipTurretInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
struct TurretInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct TurretInput;

fn on_turret_input_binding(
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

fn on_turret_input(
    fire: On<Start<TurretInput>>,
    mut q_input: Query<&mut TurretSectionInput, With<TurretInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_turret_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

fn on_turret_input_completed(
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

#[derive(Component, Debug, Clone, Deref, DerefMut, Reflect)]
pub struct SpaceshipTorpedoInputBinding(pub Vec<Binding>);

#[derive(Component, Debug, Clone)]
struct TorpedoInputMarker;

#[derive(InputAction)]
#[action_output(bool)]
struct TorpedoInput;

fn on_torpedo_input_binding(
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

fn on_torpedo_input(
    fire: On<Start<TorpedoInput>>,
    mut q_input: Query<&mut TorpedoSectionInput, With<TorpedoInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_torpedo_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        return;
    };

    **input = true;
}

fn on_torpedo_input_completed(
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

#[cfg(test)]
mod command_lag_tests {
    // Kept as its own module for its distinct harness (manual time), but
    // named and placed beside `tests` deliberately.
    use core::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// A mouse 180 must NOT reach the rotation command in one frame: the
    /// command slews at the hull's torque-budget turn rate, so the PD tracks
    /// a small error instead of saturating (flip-wobble fix) and a heavy
    /// hull audibly lags the camera (flight-feel retune, 20260709-095043).
    #[test]
    fn a_camera_flip_reaches_the_command_over_many_frames() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
        app.init_resource::<FlightSettings>();
        app.add_systems(Update, update_controller_target_rotation_torque);

        let target = Quat::from_rotation_y(core::f32::consts::PI);
        app.world_mut().spawn((
            SpaceshipCameraInputMarker,
            SpaceshipCameraNormalInputMarker,
            PointRotationOutput(target),
        ));
        // The stock ship's numbers: inertia ~2.3, computer torque 10.
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                PlayerSpaceshipMarker,
                Transform::default(),
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

        // First update has dt = 0; the second advances one real frame.
        app.update();
        app.update();

        let command = **app
            .world()
            .get::<ControllerSectionRotationInput>(controller)
            .unwrap();
        let moved = command.angle_between(Quat::IDENTITY);
        let remaining = command.angle_between(target);
        // One frame advances exactly one slew step of the DERIVED rate - this
        // pins hull_turn_rate's wiring, not just "some" slew.
        let expected = crate::flight::hull_turn_rate(
            10.0,
            2.3,
            &app.world().resource::<FlightSettings>().clone(),
        ) / 60.0;
        assert!(
            (moved - expected).abs() < expected * 0.15,
            "one frame must advance one torque-budget slew step \
             (moved {moved}, expected {expected})"
        );
        assert!(
            remaining > 2.0,
            "a 180 flip must not reach the command in one frame ({remaining} left)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cone_cos(half_angle_deg: f32) -> f32 {
        half_angle_deg.to_radians().cos()
    }

    #[test]
    fn pick_target_locks_the_body_nearest_the_aim_ray() {
        // Two candidates in front: one slightly off-axis, one further off-axis.
        // The nearer-to-center one wins even though it is further away.
        let origin = Vec3::ZERO;
        let aim = Vec3::NEG_Z;
        let near_center = Entity::from_raw_u32(1).unwrap();
        let off_center = Entity::from_raw_u32(2).unwrap();
        let candidates = [
            (near_center, Vec3::new(2.0, 0.0, -100.0)), // ~1.1 deg off axis, far
            (off_center, Vec3::new(3.0, 0.0, -20.0)),   // ~8.5 deg off axis, near
        ];

        let picked = pick_target(
            origin,
            aim,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            candidates.into_iter(),
        );
        assert_eq!(picked, Some(near_center));
    }

    #[test]
    fn pick_target_ignores_bodies_outside_the_cone() {
        // A body 90 deg off the aim direction (straight to the side) is not lockable.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(Entity::from_raw_u32(1).unwrap(), Vec3::new(50.0, 0.0, 0.0))].into_iter(),
        );
        assert_eq!(picked, None, "a body outside the cone must not be locked");
    }

    #[test]
    fn pick_target_ignores_bodies_behind_the_ship() {
        // A body directly behind (dot with aim is negative) is never locked.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            [(Entity::from_raw_u32(1).unwrap(), Vec3::new(0.0, 0.0, 100.0))].into_iter(),
        );
        assert_eq!(picked, None, "a body behind the ship must not be locked");
    }

    #[test]
    fn pick_target_ignores_bodies_beyond_max_range() {
        // Dead ahead but past the range limit: not lockable.
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            100.0,
            cone_cos(18.0),
            [(
                Entity::from_raw_u32(1).unwrap(),
                Vec3::new(0.0, 0.0, -500.0),
            )]
            .into_iter(),
        );
        assert_eq!(picked, None, "a body beyond max range must not be locked");
    }

    #[test]
    fn pick_target_returns_none_with_no_candidates() {
        let picked = pick_target(
            Vec3::ZERO,
            Vec3::NEG_Z,
            TARGETING_MAX_RANGE,
            cone_cos(18.0),
            std::iter::empty(),
        );
        assert_eq!(picked, None);
    }

    #[test]
    fn no_lock_does_not_despawn_untargeted_torpedo() {
        // Regression: with no current lock, an un-targeted torpedo (e.g. one whose
        // target just died and had its link dropped) must keep flying, not vanish.
        let mut app = App::new();
        app.insert_resource(SpaceshipPlayerTorpedoTargetEntity(None));
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        app.update();

        assert!(
            app.world().entities().contains(torpedo),
            "un-targeted torpedo must survive when there is no lock"
        );
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "no target should be assigned when there is no lock"
        );
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the torpedo should be committed to dumb-fire"
        );
    }

    #[test]
    fn lock_assigns_target_to_owned_torpedo() {
        // With a lock, an owned un-targeted torpedo gets the target assigned and
        // is committed to it.
        let mut app = App::new();
        let target = app.world_mut().spawn_empty().id();
        app.insert_resource(SpaceshipPlayerTorpedoTargetEntity(Some(target)));
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        app.update();

        assert_eq!(
            app.world().get::<TorpedoTargetEntity>(torpedo).map(|t| **t),
            Some(target),
            "an owned torpedo should be assigned the locked target"
        );
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the assignment should also commit the torpedo"
        );
    }

    #[test]
    fn dumbfire_torpedo_ignores_later_locks() {
        // THE bullet regression: a torpedo fired with no lock is committed to
        // dumb-fire; a lock appearing later (e.g. the aim cast hitting a bullet
        // fired down the crosshair ray) must not be assigned to it.
        let mut app = App::new();
        app.insert_resource(SpaceshipPlayerTorpedoTargetEntity(None));
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        let torpedo = app
            .world_mut()
            .spawn((TorpedoProjectileMarker, ProjectileOwner(ship)))
            .id();

        // Frame 1: no lock -> committed dumb-fire.
        app.update();
        assert!(app.world().get::<TorpedoTargetChosen>(torpedo).is_some());

        // A "bullet" gets locked by the aim cast afterwards.
        let bullet = app.world_mut().spawn_empty().id();
        app.insert_resource(SpaceshipPlayerTorpedoTargetEntity(Some(bullet)));

        // Frame 2: the committed torpedo must NOT pick it up.
        app.update();
        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "a dumb-fired torpedo must never acquire a target mid-flight"
        );
    }

    #[test]
    fn committed_torpedo_does_not_retarget_after_target_loss() {
        // A torpedo whose target died (link removed by update_target_position,
        // position frozen) keeps its commitment: a fresh lock must not re-target it.
        let mut app = App::new();
        let new_target = app.world_mut().spawn_empty().id();
        app.insert_resource(SpaceshipPlayerTorpedoTargetEntity(Some(new_target)));
        app.add_systems(Update, update_torpedo_target_input);

        let ship = app
            .world_mut()
            .spawn((SpaceshipRootMarker, PlayerSpaceshipMarker))
            .id();
        // Committed, un-targeted: the post-target-death state.
        let torpedo = app
            .world_mut()
            .spawn((
                TorpedoProjectileMarker,
                ProjectileOwner(ship),
                TorpedoTargetChosen,
            ))
            .id();

        app.update();

        assert!(
            app.world().get::<TorpedoTargetEntity>(torpedo).is_none(),
            "a torpedo keeps its first target for life - no re-targeting after loss"
        );
    }
}
