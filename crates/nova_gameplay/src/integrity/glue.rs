//! Section-specific "glue" between the generic integrity core (in `bevy_common_systems`) and
//! the spaceship sections. These systems know about `SectionMarker` and the ship hierarchy;
//! the integrity core itself only deals with generic nodes ([`ConnectedTo`]) and roots
//! ([`IntegrityRoot`]). Keeping them here stops the core from depending on sections.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use crate::prelude::{SectionInactiveMarker, SectionMarker, SpaceshipRootMarker};

pub(super) struct IntegrityGluePlugin;

impl Plugin for IntegrityGluePlugin {
    fn build(&self, app: &mut App) {
        debug!("IntegrityGluePlugin: build");

        app.add_observer(on_section_disable);
        app.add_observer(build_integrity_relations);
        app.add_systems(Update, aggregate_ship_health.in_set(IntegritySystems));
    }
}

/// A disabled section that is not (yet) a leaf is visually/functionally deactivated but kept
/// in place. A disabled leaf is instead destroyed (see `handle_destroy` in the core).
fn on_section_disable(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_section: Query<
        Entity,
        (
            With<SectionMarker>,
            With<IntegrityDisabledMarker>,
            Without<IntegrityLeafMarker>,
        ),
    >,
) {
    let entity = add.entity;
    if !q_section.contains(entity) {
        return;
    }

    trace!(
        "on_section_disable: entity {:?} integrity disabled, disabling section",
        entity
    );

    commands.entity(entity).insert(SectionInactiveMarker);
}

/// Build (or rebuild) the integrity relations for a body whenever one of its colliders is
/// physics-linked (avian adds `ColliderOf`).
///
/// Keyed on `ColliderOf` rather than `SectionMarker`: avian adds `ColliderOf` *after* the
/// section entities are spawned, so by the time this fires every section of the body already
/// exists and can be seen. Rebuilding on each collider is idempotent - the last call, once
/// every collider is linked, yields the complete set of neighbor lists.
///
/// - Ship: connect sections that are one unit apart (adjacent in the section grid); each
///   section gets a [`ConnectedTo`] neighbor list.
/// - Lone body (e.g. an asteroid): the single collider node gets an empty [`ConnectedTo`], so
///   it is a leaf and is destroyed as soon as it is disabled.
///
/// The body itself is marked [`IntegrityRoot`] so aggregate health and whole-body
/// destruction can find it.
fn build_integrity_relations(
    add: On<Add, ColliderOf>,
    mut commands: Commands,
    q_collider: Query<&ChildOf, With<ColliderOf>>,
    q_sections: Query<(Entity, &Transform, &ChildOf), With<SectionMarker>>,
) {
    let entity = add.entity;
    trace!("build_integrity_relations: entity {:?}", entity);

    let Ok(ChildOf(rigidbody)) = q_collider.get(entity) else {
        return;
    };
    let rigidbody = *rigidbody;

    // All sections belonging to this rigidbody, with their local (root-relative) positions.
    let sections: Vec<(Entity, Vec3)> = q_sections
        .iter()
        .filter(|(_, _, ChildOf(parent))| *parent == rigidbody)
        .map(|(section, transform, _)| (section, transform.translation))
        .collect();

    if sections.is_empty() {
        // Non-section body (e.g. an asteroid): a single collider node with no neighbors.
        commands.entity(entity).insert(ConnectedTo(Vec::new()));
    } else {
        for (section, position) in &sections {
            let neighbors: Vec<Entity> = sections
                .iter()
                .filter(|(other, other_position)| {
                    *other != *section && (position.distance(*other_position) - 1.0).abs() < 0.1
                })
                .map(|(other, _)| *other)
                .collect();
            commands.entity(*section).insert(ConnectedTo(neighbors));
        }
    }

    commands.entity(rigidbody).insert(IntegrityRoot);
}

/// Keep each ship's aggregate health equal to the sum of its section children, so the health
/// HUD tracks real damage and the ship dies once every section is gone.
///
/// Scoped to spaceship roots ([`SpaceshipRootMarker`]) on purpose: other [`IntegrityRoot`]s,
/// such as a lone asteroid, hold their [`Health`] on the collider body itself and have no
/// [`SectionMarker`] children to sum. Running this on them would just staple a meaningless
/// `Health { current: 0, max: 0 }` onto the root every frame. "Sum a ship's sections" only
/// makes sense for ships, so only ships are matched.
///
/// Sections are direct children of the ship root (which carries [`IntegrityRoot`]). This
/// recomputes the root's health every frame as the sum of its living sections. When the sum
/// hits zero, the fatal damage that removed the last section also bubbles up to the root
/// (`HealthApplyDamage` auto-propagates through `ChildOf`), marking it with `HealthZeroMarker`
/// which flows through disable -> destroy (`handle_parent_destroy`); the meshless root is then
/// despawned and the ship dies (its `PlayerSpaceshipMarker` is removed, reverting the camera
/// and clearing the HUDs).
///
/// The bubbled amount is *clamped to what actually landed on the section*: bcs's `on_damage`
/// propagates `min(amount, section.current)`, not the raw hit. That is why overkill on one
/// section cannot kill the ship (a 1000-damage hit on a 100 hp section costs the root 100, not
/// 1000, task 20260709-144906), while the last-section case still works - there the aggregate
/// equals that lone section, so the clamped amount is exactly enough to zero the root.
fn aggregate_ship_health(
    mut commands: Commands,
    q_root: Query<
        (Entity, Option<&Health>, Option<&Children>),
        (With<IntegrityRoot>, With<SpaceshipRootMarker>),
    >,
    q_section_health: Query<&Health, (With<SectionMarker>, Without<IntegrityRoot>)>,
) {
    for (root, root_health, children) in &q_root {
        let mut current = 0.0;
        let mut max = 0.0;
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(health) = q_section_health.get(child) {
                    current += health.current;
                    max += health.max;
                }
            }
        }

        let changed = match root_health {
            Some(health) => health.current != current || health.max != max,
            None => true,
        };
        if changed {
            // `try_insert`: a root can be despawned the same frame this runs (e.g. a
            // short-lived torpedo warhead, which is itself an IntegrityRoot), and a plain
            // insert on a despawned entity panics at command-apply time.
            commands.entity(root).try_insert(Health { current, max });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_health_is_the_sum_of_its_sections() {
        let mut app = App::new();
        app.add_systems(Update, aggregate_ship_health);

        let s1 = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 50.0,
                    max: 100.0,
                },
            ))
            .id();
        let s2 = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 30.0,
                    max: 100.0,
                },
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((IntegrityRoot, SpaceshipRootMarker))
            .id();
        app.world_mut().entity_mut(root).add_children(&[s1, s2]);

        app.update();

        let health = app.world().get::<Health>(root).unwrap();
        assert_eq!(health.current, 80.0);
        assert_eq!(health.max, 200.0);
    }

    #[test]
    fn ship_health_reaches_zero_when_its_sections_are_gone() {
        let mut app = App::new();
        app.add_systems(Update, aggregate_ship_health);

        let section = app
            .world_mut()
            .spawn((
                SectionMarker,
                Health {
                    current: 40.0,
                    max: 40.0,
                },
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((IntegrityRoot, SpaceshipRootMarker))
            .id();
        app.world_mut().entity_mut(root).add_children(&[section]);

        app.update();
        assert_eq!(app.world().get::<Health>(root).unwrap().current, 40.0);

        // The section is destroyed and despawned; the ship's health drops to zero.
        app.world_mut().entity_mut(section).despawn();
        app.update();
        assert_eq!(app.world().get::<Health>(root).unwrap().current, 0.0);
    }

    #[test]
    fn a_disabled_non_leaf_section_is_deactivated() {
        let mut app = App::new();
        app.add_observer(on_section_disable);

        let section = app.world_mut().spawn(SectionMarker).id();
        app.world_mut()
            .entity_mut(section)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(app.world().get::<SectionInactiveMarker>(section).is_some());
    }

    #[test]
    fn a_disabled_leaf_section_is_not_deactivated() {
        // A disabled leaf section is destroyed by the core, not merely deactivated.
        let mut app = App::new();
        app.add_observer(on_section_disable);

        let section = app
            .world_mut()
            .spawn((SectionMarker, IntegrityLeafMarker))
            .id();
        app.world_mut()
            .entity_mut(section)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(app.world().get::<SectionInactiveMarker>(section).is_none());
    }
}

/// Physics-level tests for `build_integrity_relations`, which derives each node's `ConnectedTo`
/// from the real `ColliderOf` links avian adds once a body's colliders are prepared.
#[cfg(test)]
mod physics_tests {
    use bevy_rand::prelude::*;

    use super::*;
    use crate::integrity::test_support::{
        integrity_physics_app, settle, unfinished_integrity_physics_app,
    };

    /// Spawn a ship section entity (as `base_section` does: `SectionMarker` + cuboid collider
    /// + health/density) at a grid position, parented to `root`.
    fn spawn_section(app: &mut App, root: Entity, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                ChildOf(root),
                SectionMarker,
                Transform::from_translation(at),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id()
    }

    fn neighbors(app: &App, entity: Entity) -> Vec<Entity> {
        app.world().get::<ConnectedTo>(entity).unwrap().0.clone()
    }

    #[test]
    fn a_ship_builds_adjacency_from_section_positions() {
        // Three sections in a line at x = 0, 1, 2. Adjacency is "one grid unit apart", so the
        // middle section neighbors both ends, and each end neighbors only the middle.
        let mut app = integrity_physics_app();
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();
        let left = spawn_section(&mut app, root, Vec3::new(0.0, 0.0, 0.0));
        let mid = spawn_section(&mut app, root, Vec3::new(1.0, 0.0, 0.0));
        let right = spawn_section(&mut app, root, Vec3::new(2.0, 0.0, 0.0));

        settle(&mut app);

        // The body is the integrity root.
        assert!(app.world().get::<IntegrityRoot>(root).is_some());

        // Middle neighbors both ends; ends neighbor only the middle.
        let mid_neighbors = neighbors(&app, mid);
        assert_eq!(mid_neighbors.len(), 2);
        assert!(mid_neighbors.contains(&left) && mid_neighbors.contains(&right));
        assert_eq!(neighbors(&app, left), vec![mid]);
        assert_eq!(neighbors(&app, right), vec![mid]);
    }

    /// The physical half of task 20260709-140620: when a section is gone, the
    /// body's mass, center of mass and angular inertia must follow the
    /// survivors. This is avian ground truth (direct despawn), separating
    /// "avian does not recompute on collider removal" from "our destroy path
    /// never removes the collider".
    #[test]
    fn mass_properties_follow_a_despawned_section() {
        let mut app = integrity_physics_app();
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();
        let _left = spawn_section(&mut app, root, Vec3::ZERO);
        let right = spawn_section(&mut app, root, Vec3::X);
        settle(&mut app);

        let mass_before = app.world().get::<ComputedMass>(root).unwrap().value();
        let com_before = app.world().get::<ComputedCenterOfMass>(root).unwrap().0;
        let (inertia_before, _) = app
            .world()
            .get::<ComputedAngularInertia>(root)
            .unwrap()
            .principal_angular_inertia_with_local_frame();
        assert!(
            (mass_before - 2.0).abs() < 1e-3,
            "two unit-density unit cubes should weigh 2: {mass_before}"
        );
        assert!(
            (com_before.x - 0.5).abs() < 1e-3,
            "COM should start midway between the sections: {com_before:?}"
        );

        app.world_mut().entity_mut(right).despawn();
        settle(&mut app);

        let mass_after = app.world().get::<ComputedMass>(root).unwrap().value();
        let com_after = app.world().get::<ComputedCenterOfMass>(root).unwrap().0;
        let (inertia_after, _) = app
            .world()
            .get::<ComputedAngularInertia>(root)
            .unwrap()
            .principal_angular_inertia_with_local_frame();
        assert!(
            (mass_after - 1.0).abs() < 1e-3,
            "mass must drop with the lost section: {mass_before} -> {mass_after}"
        );
        assert!(
            com_after.x.abs() < 1e-3,
            "COM must shift onto the survivor: {com_before:?} -> {com_after:?}"
        );
        // Analytic solid-cuboid values (sorted principal components; the
        // principal frame may permute axes): two unit cubes side by side are
        // [2*(1/6), 2*(1/6) + 2*(1/4), same] = [1/3, 5/6, 5/6]; the lone
        // survivor is a plain unit cube, 1/6 on every axis.
        let sorted = |v: Vec3| {
            let mut a = v.to_array();
            a.sort_by(f32::total_cmp);
            a
        };
        for (got, expected) in
            sorted(inertia_before)
                .into_iter()
                .zip([1.0 / 3.0, 5.0 / 6.0, 5.0 / 6.0])
        {
            assert!(
                (got - expected).abs() < 0.02,
                "pre-despawn principal inertia off: {inertia_before:?}"
            );
        }
        for got in sorted(inertia_after) {
            assert!(
                (got - 1.0 / 6.0).abs() < 0.02,
                "post-despawn principal inertia off: {inertia_after:?}"
            );
        }
    }

    /// The same claim through the real pipeline: a section driven to zero
    /// health is disabled, destroyed (it is a leaf), despawned - and the mass
    /// properties follow. Exercises health -> integrity -> explode end to end.
    #[test]
    fn mass_properties_follow_a_section_destroyed_by_damage() {
        let mut app = unfinished_integrity_physics_app();
        // The destroy path's debris observers need material assets and the
        // global rng even in a headless run.
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.finish();

        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();
        let _left = spawn_section(&mut app, root, Vec3::ZERO);
        let right = spawn_section(&mut app, root, Vec3::X);
        settle(&mut app);

        let mass_before = app.world().get::<ComputedMass>(root).unwrap().value();
        let com_before = app.world().get::<ComputedCenterOfMass>(root).unwrap().0;

        // Exactly the section's health, torpedo-blast scale. The amount also
        // propagates through ChildOf to the root's aggregate health (200 ->
        // 100 here); exact damage leaves the root alive, while overkill would
        // zero it and kill the whole ship (see task 20260709-144906).
        app.world_mut().trigger(HealthApplyDamage {
            entity: right,
            source: None,
            amount: 100.0,
        });
        for _ in 0..10 {
            app.update();
        }

        assert!(
            !app.world().entities().contains(right),
            "a zero-health leaf section should be destroyed and despawned"
        );
        let mass_after = app.world().get::<ComputedMass>(root).unwrap().value();
        let com_after = app.world().get::<ComputedCenterOfMass>(root).unwrap().0;
        assert!(
            (mass_after - 1.0).abs() < 1e-3,
            "mass must follow the destroyed section: {mass_before} -> {mass_after}"
        );
        assert!(
            com_after.x.abs() < 1e-3,
            "COM must shift onto the survivor: {com_before:?} -> {com_after:?}"
        );
    }

    /// Regression for task 20260709-144906: overkill on ONE section must not
    /// kill the whole ship. A 1000-damage hit on a 100 hp section used to
    /// propagate its full amount to the root aggregate (200 -> -800 -> zeroed),
    /// dragging an otherwise-healthy ship through disable -> destroy. With the
    /// bcs clamp, the root is charged only the section's remaining 100, so the
    /// other section and the ship root survive.
    #[test]
    fn overkill_on_one_section_does_not_kill_the_ship() {
        let mut app = unfinished_integrity_physics_app();
        // The destroy path's debris observers need material assets and the
        // global rng even in a headless run.
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.finish();

        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();
        let survivor = spawn_section(&mut app, root, Vec3::ZERO);
        let hit = spawn_section(&mut app, root, Vec3::X);
        settle(&mut app);

        // Sanity: the aggregate starts at both sections' health.
        assert_eq!(app.world().get::<Health>(root).unwrap().current, 200.0);

        // Ten times the section's health, well past its 100 hp.
        app.world_mut().trigger(HealthApplyDamage {
            entity: hit,
            source: None,
            amount: 1000.0,
        });
        for _ in 0..10 {
            app.update();
        }

        // The hit section is destroyed and gone...
        assert!(
            !app.world().entities().contains(hit),
            "the over-killed section should be destroyed and despawned"
        );

        // ...but the ship survives it: the root still exists, is not marked for
        // death, and its aggregate health is exactly the surviving section's.
        assert!(
            app.world().entities().contains(root),
            "the ship root must not die from overkill on one section"
        );
        assert!(
            app.world().get::<HealthZeroMarker>(root).is_none(),
            "the root must never be marked zero-health while a section lives"
        );
        // The root should have lost only the destroyed section's ~100 hp, not the
        // 1000 overkill (which would zero it). A wide tolerance absorbs the tiny
        // contact damage the two touching unit-cube sections trade in avian - the
        // point is 100, decisively not 0.
        let root_health = app.world().get::<Health>(root).unwrap().current;
        assert!(
            (root_health - 100.0).abs() < 1.0,
            "the ship should have lost only the destroyed section (~100 hp), not \
             the 1000 overkill: root health = {root_health}"
        );

        // The other section survives, carrying essentially all its health (again
        // modulo negligible section-to-section contact damage).
        assert!(
            app.world().entities().contains(survivor),
            "the healthy section must survive its neighbor's destruction"
        );
        let survivor_health = app.world().get::<Health>(survivor).unwrap().current;
        assert!(
            (survivor_health - 100.0).abs() < 1.0,
            "the surviving section should take no damage from the overkill: \
             survivor health = {survivor_health}"
        );
    }

    #[test]
    fn a_lone_body_becomes_an_empty_leaf_root() {
        // An asteroid-shaped body: a single collider node with no sections. It gets an empty
        // neighbor list (so it is a leaf, destroyed as soon as it is disabled) and its body is
        // marked the integrity root.
        let mut app = integrity_physics_app();
        let body = app
            .world_mut()
            .spawn((RigidBody::Dynamic, Transform::default()))
            .id();
        let node = app
            .world_mut()
            .spawn((
                ChildOf(body),
                Collider::sphere(1.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();

        settle(&mut app);

        assert!(app.world().get::<IntegrityRoot>(body).is_some());
        assert_eq!(neighbors(&app, node), Vec::<Entity>::new());
    }
}
