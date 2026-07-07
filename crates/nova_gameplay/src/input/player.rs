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
fn update_controller_target_rotation_torque(
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
    spaceship: Single<Entity, (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>)>,
) {
    let point_rotation = point_rotation.into_inner();
    let spaceship = spaceship.into_inner();

    for (mut controller, _) in q_controller
        .iter_mut()
        .filter(|(_, ChildOf(c_parent))| *c_parent == spaceship)
    {
        **controller = **point_rotation;
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

// TODO(20260525-133018): Implement a more sophisticated target selection mechanism.
// Maybe we can project the 3D objects onto the 2D screen and select the closest one to the
// center of the screen.
// TODO(20260525-133022): Add a HUD for the torpedo target selection.

fn update_spaceship_target_input(
    query: SpatialQuery,
    point_rotation: Single<
        &PointRotationOutput,
        (
            With<SpaceshipCameraInputMarker>,
            With<SpaceshipCameraTurretInputMarker>,
        ),
    >,
    // Exclude from the aim cast only torpedoes that have not committed their
    // launch-time targeting yet: a fresh torpedo spawns right on the aim ray, and
    // if the cast could hit it, it could be assigned as its own target. Once
    // committed (`TorpedoTargetChosen`) a torpedo can never receive a target
    // again, so it becomes a normal lockable body - e.g. you can lock and shoot
    // down your own dumb-fired torpedo.
    q_torpedo: Query<
        (Entity, &TorpedoProjectileOwner, &Children),
        (With<TorpedoProjectileMarker>, Without<TorpedoTargetChosen>),
    >,
    spaceship: Single<
        (&Transform, &Children),
        (With<SpaceshipRootMarker>, With<PlayerSpaceshipMarker>),
    >,
    q_hits: Query<&ColliderOf>,
    mut res_target: ResMut<SpaceshipPlayerTorpedoTargetEntity>,
) {
    let point_rotation = point_rotation.into_inner();
    let (transform, children) = spaceship.into_inner();

    let shape = Collider::sphere(1.0);
    let origin = transform.translation;
    let shape_rotation = Quat::IDENTITY;
    let forward = Dir3::new_unchecked((**point_rotation * Vec3::NEG_Z).normalize());
    let mut children = children.iter().collect::<Vec<Entity>>();
    q_torpedo.iter().for_each(|(_, _, torpedo_children)| {
        for child in torpedo_children.iter() {
            children.push(child);
        }
    });
    let config = ShapeCastConfig::default();
    let filter = SpatialQueryFilter::from_excluded_entities(children);

    let Some(ray_hit_data) =
        query.cast_shape(&shape, origin, shape_rotation, forward, &config, &filter)
    else {
        **res_target = None;
        return;
    };
    let target_entity = ray_hit_data.entity;
    let Ok(collider_of) = q_hits.get(target_entity) else {
        **res_target = None;
        return;
    };
    let target_entity = collider_of.body;

    **res_target = Some(target_entity);
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
        (Entity, &TorpedoProjectileOwner),
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
    mut q_input: Query<&mut ThrusterSectionInput, With<ThrusterInputMarker>>,
) {
    let entity = fire.event().context;
    trace!("on_thruster_input: entity {:?}", entity);

    let Ok(mut input) = q_input.get_mut(entity) else {
        error!(
            "on_thruster_input: entity {:?} not found in q_input",
            entity
        );
        return;
    };

    **input = 1.0;
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
mod tests {
    use super::*;

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
            .spawn((TorpedoProjectileMarker, TorpedoProjectileOwner(ship)))
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
            .spawn((TorpedoProjectileMarker, TorpedoProjectileOwner(ship)))
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
            .spawn((TorpedoProjectileMarker, TorpedoProjectileOwner(ship)))
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
                TorpedoProjectileOwner(ship),
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
