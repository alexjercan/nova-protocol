//! Scripted torpedo launches: an authored order on a bay that fires one
//! torpedo at a chosen target with no controller involved. The scenario layer
//! inserts a [`ScriptedTorpedoOrder`]; the trigger system holds the bay's
//! [`TorpedoSectionInput`] until the bay actually launches (cooldown, ammo and
//! the inactive-section gate all still apply in the spawn system), and the
//! commit system then locks the fresh projectile to the ordered target -
//! the same launch-time commit the AI and player sides perform, which is
//! also what makes the ordnance visible to hostile point defense.
//!
//! Meant for controller-less ships (dumb emplacements fired by scenario
//! timers). On an AI-controlled ship the AI's own trigger system rewrites the
//! bay input every frame and wins.

use bevy::prelude::*;
use nova_gameplay::prelude::*;

use super::{TorpedoSectionInput, TorpedoSectionPartOf, TorpedoTargetChosen, TorpedoTargetEntity};

/// A pending scripted launch on a torpedo bay section: fire when ready, then
/// commit the projectile to `target`. One-shot - the commit removes it.
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct ScriptedTorpedoOrder {
    /// The ship the launched torpedo homes on.
    pub target: Entity,
}

/// Hold the trigger of every bay carrying an order. The bay's own gates
/// (fire cooldown, ammo, inactive section) decide when the launch actually
/// happens; the order just keeps the trigger down until it does.
pub(super) fn hold_scripted_torpedo_trigger(
    mut q_bay: Query<&mut TorpedoSectionInput, With<ScriptedTorpedoOrder>>,
) {
    for mut input in &mut q_bay {
        // Change-detection hygiene, like the AI trigger side.
        if !**input {
            **input = true;
        }
    }
}

/// Commit each fresh torpedo whose sourcing bay holds an order: target lock
/// plus the one-time [`TorpedoTargetChosen`] decision, then release the
/// trigger and consume the order. A target that died between order and
/// launch commits as a dumb-fire shot - the same rule as the AI commit.
pub(super) fn commit_scripted_torpedo(
    mut commands: Commands,
    q_torpedo: Query<
        (Entity, &TorpedoSectionPartOf),
        (
            With<TorpedoProjectileMarker>,
            Without<TorpedoTargetEntity>,
            Without<TorpedoTargetChosen>,
        ),
    >,
    mut q_bay: Query<(&ScriptedTorpedoOrder, &mut TorpedoSectionInput)>,
    q_ship_root: Query<(), With<SpaceshipRootMarker>>,
) {
    for (torpedo, part_of) in &q_torpedo {
        let Ok((order, mut input)) = q_bay.get_mut(**part_of) else {
            continue;
        };
        let target = Some(order.target).filter(|&target| q_ship_root.contains(target));

        debug!(
            "commit_scripted_torpedo: committing torpedo {:?} to target {:?}",
            torpedo, target
        );

        let mut torpedo_commands = commands.entity(torpedo);
        torpedo_commands.insert(TorpedoTargetChosen);
        if let Some(target) = target {
            torpedo_commands.insert(TorpedoTargetEntity(target));
        }
        **input = false;
        commands.entity(**part_of).remove::<ScriptedTorpedoOrder>();
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::*;
    use bevy::time::TimeUpdateStrategy;

    use super::{
        super::{
            bay::{shoot_spawn_projectile, update_spawner_fire_state},
            TorpedoSectionConfig, TorpedoSectionConfigHelper, TorpedoSectionSpawnerEntity,
            TorpedoSectionSpawnerFireState, TorpedoSectionSpawnerMarker,
        },
        *,
    };
    use crate::Cooldown;

    /// The full scripted loop on a manual clock: hold -> bay spawn -> commit.
    fn scripted_app(dt: f32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(dt),
        ));
        app.add_systems(
            Update,
            (
                hold_scripted_torpedo_trigger,
                update_spawner_fire_state,
                shoot_spawn_projectile,
                commit_scripted_torpedo,
            )
                .chain(),
        );
        app
    }

    /// A controller-less ship with one idle bay (trigger down), plus a bare
    /// target ship root.
    fn spawn_battery(app: &mut App) -> (Entity, Entity, Entity) {
        let config = TorpedoSectionConfig::default();
        let interval = 1.0 / config.fire_rate;

        let world = app.world_mut();
        let target = world.spawn(SpaceshipRootMarker).id();
        let ship = world
            .spawn((
                SpaceshipRootMarker,
                Position(Vec3::ZERO),
                Rotation::default(),
                LinearVelocity(Vec3::ZERO),
                AngularVelocity(Vec3::ZERO),
                ComputedCenterOfMass(Vec3::ZERO),
            ))
            .id();
        let section = world
            .spawn((
                TorpedoSectionMarker,
                TorpedoSectionConfigHelper(config),
                TorpedoSectionInput(false),
                Transform::default(),
                ChildOf(ship),
            ))
            .id();
        let spawner = world
            .spawn((
                TorpedoSectionSpawnerMarker,
                TorpedoSectionPartOf(section),
                TorpedoSectionSpawnerFireState(Cooldown::new(interval)),
                Transform::default(),
                ChildOf(section),
            ))
            .id();
        world
            .entity_mut(section)
            .insert(TorpedoSectionSpawnerEntity(spawner));
        (section, target, ship)
    }

    fn torpedoes(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<TorpedoProjectileMarker>>()
            .iter(app.world())
            .collect()
    }

    /// One order = one launch, committed to the ordered target, with the
    /// trigger released and the order consumed - a battery with no controller
    /// fires exactly when scripted and its ordnance is a real committed
    /// torpedo (which is what point defense acquisition requires).
    #[test]
    fn an_order_launches_once_and_commits_to_the_target() {
        let mut app = scripted_app(0.5);
        let (section, target, _ship) = spawn_battery(&mut app);
        app.world_mut()
            .entity_mut(section)
            .insert(ScriptedTorpedoOrder { target });

        for _ in 0..8 {
            app.update();
        }

        let launched = torpedoes(&mut app);
        assert_eq!(launched.len(), 1, "one order, one torpedo - no relatch");
        let torpedo = launched[0];
        assert!(
            app.world().get::<TorpedoTargetChosen>(torpedo).is_some(),
            "the scripted commit made the launch-time decision"
        );
        assert_eq!(
            app.world()
                .get::<TorpedoTargetEntity>(torpedo)
                .map(|entity| **entity),
            Some(target),
            "the torpedo homes on the ordered target"
        );
        assert!(
            app.world().get::<ScriptedTorpedoOrder>(section).is_none(),
            "the order is consumed"
        );
        assert!(
            !**app.world().get::<TorpedoSectionInput>(section).unwrap(),
            "the trigger is released after the launch"
        );
    }

    /// A target that died between order and launch still launches but commits
    /// dumb-fire (chosen, no entity lock) - the AI commit's exact rule.
    #[test]
    fn a_dead_target_commits_as_dumb_fire() {
        let mut app = scripted_app(0.5);
        let (section, target, _ship) = spawn_battery(&mut app);
        app.world_mut().entity_mut(target).despawn();
        app.world_mut()
            .entity_mut(section)
            .insert(ScriptedTorpedoOrder { target });

        for _ in 0..4 {
            app.update();
        }

        let launched = torpedoes(&mut app);
        assert_eq!(launched.len(), 1);
        assert!(app
            .world()
            .get::<TorpedoTargetChosen>(launched[0])
            .is_some());
        assert!(
            app.world()
                .get::<TorpedoTargetEntity>(launched[0])
                .is_none(),
            "no dangling lock on a dead target"
        );
    }
}
