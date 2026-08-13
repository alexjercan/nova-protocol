//! Ship-specific adapter for the generic integrity graph and lifecycle.
//!
//! Change this module when ship structure publication or aggregate health semantics change.

use std::collections::BTreeSet;

use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;

use super::link_points::prelude::*;

/// Ship graph publication, disabled-section behavior, and aggregate health.
pub mod prelude {
    pub use super::ShipIntegrityPlugin;
}

/// Adapts section-based ships to the generic gameplay integrity pipeline.
pub struct ShipIntegrityPlugin;

impl Plugin for ShipIntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("ShipIntegrityPlugin: build");

        app.register_type::<LinkPoint>();
        app.register_type::<SectionLinkPoints>();
        app.add_observer(on_section_disable);
        app.add_systems(Update, build_ship_integrity_graph.before(IntegritySystems));
        app.add_systems(Update, aggregate_ship_health.in_set(IntegritySystems));
    }
}

/// A disabled section that is not yet a leaf is deactivated but kept in place.
/// A disabled leaf is destroyed by the generic integrity core instead.
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

/// Build each ship's authoritative section graph after its section spawn batch is complete.
///
/// An observer on Avian's `Add<ColliderOf>` sees colliders one at a time. A valid ship can be
/// disconnected halfway through that sequence, which briefly publishes an empty graph and
/// emits false errors. `Added<SectionLinkPoints>` is evaluated once per update, after all
/// section commands from the spawn observer have landed, so each affected root is derived once
/// from its complete authored batch.
fn build_ship_integrity_graph(
    mut commands: Commands,
    q_added_sections: Query<&ChildOf, (With<SectionMarker>, Added<SectionLinkPoints>)>,
    q_root: Query<(), With<SpaceshipRootMarker>>,
    q_sections: Query<
        (
            Entity,
            &Transform,
            &SectionLinkPoints,
            &ChildOf,
            Option<&EntityId>,
        ),
        With<SectionMarker>,
    >,
) {
    let roots: BTreeSet<_> = q_added_sections
        .iter()
        .map(|ChildOf(root)| *root)
        .filter(|root| q_root.contains(*root))
        .collect();

    for root in roots {
        let mut sections: Vec<_> = q_sections
            .iter()
            .filter(|(_, _, _, ChildOf(parent), _)| *parent == root)
            .collect();
        sections.sort_by_key(|(entity, ..)| entity.to_bits());
        if sections.is_empty() {
            continue;
        }

        let placed: Vec<_> = sections
            .iter()
            .map(
                |(_, transform, link_points, _, _)| PlacedSectionLinkPoints {
                    position: transform.translation,
                    rotation: transform.rotation,
                    link_points,
                },
            )
            .collect();

        let mut neighbors = vec![BTreeSet::new(); sections.len()];
        match derive_link_point_graph(&placed) {
            Ok(mates) => {
                for mate in mates {
                    let a = mate.a.section_index;
                    let b = mate.b.section_index;
                    neighbors[a].insert(b);
                    neighbors[b].insert(a);
                }
            }
            Err(errors) => {
                let section_order: Vec<_> = sections
                    .iter()
                    .map(|(entity, _, _, _, id)| {
                        id.map(|id| id.0.clone())
                            .unwrap_or_else(|| format!("{entity:?}"))
                    })
                    .collect();
                for graph_error in errors {
                    error!(
                        "ship {root:?} has an invalid link-point graph; section order \
                         {section_order:?}: {graph_error:?}"
                    );
                }
            }
        }

        for (section_index, (section, ..)) in sections.iter().enumerate() {
            let connected = neighbors[section_index]
                .iter()
                .map(|neighbor| sections[*neighbor].0)
                .collect();
            commands.entity(*section).insert(ConnectedTo(connected));
        }
    }
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
/// which flows through the generic disable and root-destruction path. Roots carry no
/// `ConnectedTo` and are never leaves, so root destruction is a separate integrity-core hop;
/// the meshless root is then
/// despawned and the ship dies (its `PlayerSpaceshipMarker` is removed, reverting the camera
/// and clearing the HUDs).
///
/// The bubbled amount is clamped to what actually landed on the section by the health layer,
/// propagates `min(amount, section.current)`, not the raw hit. That is why overkill on one
/// section cannot kill the ship (a 1000-damage hit on a 100 hp section costs the root 100, not
/// 1000), while the last-section case still works - there the aggregate
/// equals that lone section, so the clamped amount is exactly enough to zero the root.
fn aggregate_ship_health(
    mut commands: Commands,
    q_root: Query<
        (
            Entity,
            Option<&Health>,
            Option<&Children>,
            Has<HealthZeroMarker>,
        ),
        (With<IntegrityRoot>, With<SpaceshipRootMarker>),
    >,
    q_section_health: Query<&Health, (With<SectionMarker>, Without<IntegrityRoot>)>,
) {
    for (root, root_health, children, already_zero) in &q_root {
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

        // NOTE: structural death backstop (the 0-HP ghost).
        // `HealthZeroMarker` only ever comes from the damage path (nova's
        // `on_damage`), so a ship that loses its last section WITHOUT a
        // final bubble reaching the root (a direct destroy, a detach, any
        // future scripted removal) would sit here forever as an unmarked
        // 0-HP hull. The recompute is the one place that always sees "no
        // living sections", so it owns the backstop: mark the root and let
        // the ordinary disable -> destroy chain take it. The previous-frame
        // `max > 0` guard means "this root has HAD living sections" - a
        // mid-spawn root whose sections have not landed yet is not executed
        // at birth.
        let had_sections = root_health.is_some_and(|health| health.max > 0.0);
        if !already_zero && had_sections && current <= 0.0 {
            debug!(
                "aggregate_ship_health: root {root:?} has no living sections \
                 and no zero marker; inserting the structural death backstop"
            );
            commands.entity(root).try_insert(HealthZeroMarker);
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

/// Physics-level tests for link-point graph publication at Avian's real `ColliderOf` seam.
#[cfg(test)]
mod physics_tests {
    use avian3d::prelude::*;
    use bevy_rand::prelude::*;
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;

    fn integrity_physics_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(ShipIntegrityPlugin);
        app.finish();
        app
    }

    /// Spawn a ship section entity (as `base_section` does: `SectionMarker` + cuboid collider
    /// + health/density) at a grid position, parented to `root`.
    fn spawn_section_with_points(
        app: &mut App,
        root: Entity,
        at: Vec3,
        link_points: Vec<LinkPoint>,
    ) -> Entity {
        app.world_mut()
            .spawn((
                ChildOf(root),
                SectionMarker,
                Transform::from_translation(at),
                SectionLinkPoints(link_points),
                ConnectedTo::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id()
    }

    fn spawn_section(app: &mut App, root: Entity, at: Vec3) -> Entity {
        spawn_section_with_points(app, root, at, unit_cube_link_points())
    }

    fn neighbors(app: &App, entity: Entity) -> Vec<Entity> {
        app.world().get::<ConnectedTo>(entity).unwrap().0.clone()
    }

    #[test]
    fn a_ship_builds_adjacency_from_link_point_mates() {
        // Explicit cube sockets reproduce the existing three-cell line graph.
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

    #[test]
    fn graph_build_uses_the_complete_section_spawn_batch() {
        let mut app = App::new();
        app.add_systems(Update, build_ship_integrity_graph);

        let root = app.world_mut().spawn(SpaceshipRootMarker).id();
        let left = spawn_section(&mut app, root, Vec3::ZERO);
        let right = spawn_section(&mut app, root, Vec3::X * 2.0);
        let bridge = spawn_section(&mut app, root, Vec3::X);

        app.update();

        assert_eq!(neighbors(&app, left), vec![bridge]);
        assert_eq!(neighbors(&app, right), vec![bridge]);
        let bridge_neighbors = neighbors(&app, bridge);
        assert_eq!(bridge_neighbors.len(), 2);
        assert!(bridge_neighbors.contains(&left));
        assert!(bridge_neighbors.contains(&right));
    }

    #[test]
    fn link_points_connect_sections_at_non_grid_distances() {
        let mut app = integrity_physics_app();
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();
        let left = spawn_section_with_points(
            &mut app,
            root,
            Vec3::ZERO,
            vec![LinkPoint {
                id: "out".to_string(),
                position: Vec3::X,
                normal: Vec3::X,
            }],
        );
        let right = spawn_section_with_points(
            &mut app,
            root,
            Vec3::X * 2.0,
            vec![LinkPoint {
                id: "in".to_string(),
                position: Vec3::NEG_X,
                normal: Vec3::NEG_X,
            }],
        );

        settle(&mut app);

        assert_eq!(neighbors(&app, left), vec![right]);
        assert_eq!(neighbors(&app, right), vec![left]);
    }

    #[test]
    fn adjacent_sections_without_link_points_do_not_gain_distance_edges() {
        let mut app = integrity_physics_app();
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
            ))
            .id();
        let left = spawn_section_with_points(&mut app, root, Vec3::ZERO, Vec::new());
        let right = spawn_section_with_points(&mut app, root, Vec3::X, Vec::new());

        settle(&mut app);

        assert!(neighbors(&app, left).is_empty());
        assert!(neighbors(&app, right).is_empty());
    }

    /// When a section is gone, the body's mass, center of mass and angular
    /// inertia must follow the
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
        app.add_plugins(ShipIntegrityPlugin);
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
        // zero it and kill the whole ship.
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

    /// Regression: overkill on ONE section must not kill the whole ship.
    /// A 1000-damage hit on a 100 hp section used to
    /// propagate its full amount to the root aggregate (200 -> -800 -> zeroed),
    /// dragging an otherwise-healthy ship through disable -> destroy. With the
    /// overkill clamp, the root is charged only the section's remaining 100, so the
    /// other section and the ship root survive.
    #[test]
    fn overkill_on_one_section_does_not_kill_the_ship() {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(ShipIntegrityPlugin);
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
            .spawn((RigidBody::Dynamic, Transform::default(), IntegrityRoot))
            .id();
        let node = app
            .world_mut()
            .spawn((
                ChildOf(body),
                Collider::sphere(1.0),
                ConnectedTo::default(),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id();

        settle(&mut app);

        assert!(app.world().get::<IntegrityRoot>(body).is_some());
        assert_eq!(neighbors(&app, node), Vec::<Entity>::new());
    }
}

/// The ghost-ship boundary rig: a playtest saw an enemy "survive" its
/// shootdown as an empty 0-HP hull. Root death depends
/// on the fatal hit's bubble reaching the root with a nonzero amount
/// (HealthZeroMarker comes ONLY from `on_damage`), while the aggregate
/// recompute writes marker-less zeros - these tests walk every path a ship
/// can reach "all sections dead" and assert the root actually dies
/// (despawns) within a frame budget. Cases that were never buggy stay as
/// pins (null-result-becomes-a-pin).
#[cfg(test)]
mod ghost_ship_tests {
    use avian3d::prelude::*;
    use bevy_rand::prelude::*;
    use nova_events::prelude::{EntityTypeName, GameEvent};
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;

    /// Records lifecycle events so unified defeat and physical destruction remain distinct.
    #[derive(Resource, Default)]
    struct FiredEvents(Vec<&'static str>);

    fn ghost_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(ShipIntegrityPlugin);
        // The destroy path's debris observers need material assets and the
        // global rng even in a headless run.
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.init_resource::<FiredEvents>();
        app.add_observer(|event: On<GameEvent>, mut fired: ResMut<FiredEvents>| {
            fired.0.push(event.name());
        });
        app.finish();
        app
    }

    fn destroy_events(app: &App) -> usize {
        app.world()
            .resource::<FiredEvents>()
            .0
            .iter()
            .filter(|name| **name == "ondestroyed")
            .count()
    }

    fn spawn_ship(app: &mut App, section_count: usize) -> (Entity, Vec<Entity>) {
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                EntityId::new("rig_ship"),
                EntityTypeName::new("spaceship"),
            ))
            .id();
        let sections = (0..section_count)
            .map(|i| {
                app.world_mut()
                    .spawn((
                        ChildOf(root),
                        SectionMarker,
                        Transform::from_translation(Vec3::X * i as f32),
                        SectionLinkPoints(unit_cube_link_points()),
                        ConnectedTo::default(),
                        Collider::cuboid(1.0, 1.0, 1.0),
                        ColliderDensity(1.0),
                        Health::new(100.0),
                    ))
                    .id()
            })
            .collect();
        settle(app);
        (root, sections)
    }

    fn hit(app: &mut App, target: Entity, amount: f32) {
        app.world_mut().trigger(HealthApplyDamage {
            entity: target,
            source: None,
            amount,
        });
    }

    /// True when the root died all the way: entity gone (the meshless
    /// despawn leg of IntegrityDestroyMarker).
    fn root_dead(app: &mut App, root: Entity, budget: usize) -> bool {
        for _ in 0..budget {
            if !app.world().entities().contains(root) {
                return true;
            }
            app.update();
        }
        !app.world().entities().contains(root)
    }

    /// The canonical kill: sections die one at a time to exact hits; the
    /// last bubble must take the root with it.
    #[test]
    fn killing_every_section_kills_the_ship() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 2);

        hit(&mut app, sections[0], 100.0);
        for _ in 0..5 {
            app.update();
        }
        hit(&mut app, sections[1], 100.0);

        assert!(
            root_dead(&mut app, root, 10),
            "all sections dead by damage, ship root must die (no 0-HP ghost)"
        );
        assert_eq!(
            destroy_events(&app),
            1,
            "the root's OnDestroyed fires exactly once (review R1.2)"
        );
    }

    /// Both sections take fatal hits in the SAME frame (a blast co-hit):
    /// the bubbles land back to back before any recompute.
    #[test]
    fn simultaneous_fatal_hits_kill_the_ship() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 2);

        hit(&mut app, sections[0], 100.0);
        hit(&mut app, sections[1], 100.0);

        assert!(
            root_dead(&mut app, root, 10),
            "same-frame fatal hits on every section must kill the ship"
        );
    }

    /// The last living section takes TWO hits in one frame (per-collider
    /// multi-hit): the second bubble is swallowed (amount = 0) by the
    /// already-zero section; the first must still have done the job.
    #[test]
    fn double_hit_on_the_last_section_kills_the_ship() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 2);

        hit(&mut app, sections[0], 100.0);
        for _ in 0..5 {
            app.update();
        }
        hit(&mut app, sections[1], 100.0);
        hit(&mut app, sections[1], 100.0);

        assert!(
            root_dead(&mut app, root, 10),
            "a swallowed second bubble must not save the ship"
        );
    }

    /// Sustained small-arms fire (the playtest's actual shape: turret rounds
    /// with typed-resistance fractions): many sub-lethal hits alternating
    /// across sections, one hit per frame, until everything is dead.
    #[test]
    fn many_small_hits_kill_the_ship() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 2);

        // 3.7 never divides 100 evenly, so every section death is a
        // fractional-residue kill (float-accumulation shape).
        for i in 0..60 {
            hit(&mut app, sections[i % 2], 3.7);
            app.update();
        }

        assert!(
            root_dead(&mut app, root, 20),
            "sustained fractional fire must kill the ship, not leave a ghost"
        );
    }

    /// The structural hole: the last section is REMOVED without the damage
    /// path (direct destroy - the shape of any future detach/scripted
    /// removal). The aggregate recomputes to zero, but no bubble ever
    /// reaches the root, so nothing marks it - the reported 0-HP ghost.
    #[test]
    fn last_section_destroyed_without_damage_still_kills_the_ship() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 2);

        hit(&mut app, sections[0], 100.0);
        for _ in 0..5 {
            app.update();
        }
        // Bypass health entirely: destroy the survivor the way a scripted
        // removal / detach would.
        app.world_mut()
            .entity_mut(sections[1])
            .insert(IntegrityDestroyMarker);

        assert!(
            root_dead(&mut app, root, 10),
            "a ship with no living sections is dead, however the last one went"
        );
        assert_eq!(
            destroy_events(&app),
            1,
            "the backstop kills the root exactly once, not zero, not twice \
             (review R1.2)"
        );
    }

    /// Damage landing on the ROOT body directly is overwritten by the next
    /// recompute (the aggregate mirrors sections, nothing else); the ship
    /// must still die exactly once when its sections then go - the
    /// interleave the plan promised (review R1.2 restored it).
    #[test]
    fn direct_root_damage_interleaved_with_the_recompute_still_kills_cleanly() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 2);

        hit(&mut app, root, 50.0);
        for _ in 0..3 {
            app.update(); // recompute overwrites the direct dent
        }
        assert_eq!(
            app.world().get::<Health>(root).unwrap().current,
            200.0,
            "delivery guard: the recompute owns the root's number again"
        );

        hit(&mut app, sections[0], 100.0);
        for _ in 0..5 {
            app.update();
        }
        hit(&mut app, sections[1], 100.0);

        assert!(
            root_dead(&mut app, root, 10),
            "the interleaved direct dent must not confuse the kill"
        );
        assert_eq!(destroy_events(&app), 1, "exactly one OnDestroyed");
    }
}
