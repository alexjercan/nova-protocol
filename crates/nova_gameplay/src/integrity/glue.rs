//! Section-specific "glue" between the generic integrity core (in `plugin.rs`) and the
//! spaceship sections. These systems know about `SectionMarker` and the ship hierarchy; the
//! integrity core itself only deals with generic nodes ([`ConnectedTo`]) and roots
//! ([`IntegrityRoot`]). Keeping them here stops the core from depending on sections.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;

use super::components::*;
use crate::prelude::{SectionInactiveMarker, SectionMarker, SpaceshipRootMarker};

pub(super) struct IntegrityGluePlugin;

impl Plugin for IntegrityGluePlugin {
    fn build(&self, app: &mut App) {
        debug!("IntegrityGluePlugin: build");

        app.add_observer(on_section_disable);
        app.add_observer(build_integrity_relations);
        app.add_systems(
            Update,
            aggregate_ship_health.in_set(super::plugin::IntegritySystems),
        );
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
