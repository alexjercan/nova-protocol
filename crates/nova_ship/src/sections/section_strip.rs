//! A section stripped off a docked lootable hull: the running strip and the
//! removal it ends in.
//!
//! The rule that plans a strip is [`plan_section_strip`]; this module adds
//! the structure facts it reads and the two ship-side pieces a strip needs.
//! [`strip_splits_hull`] says whether taking a section would leave the hull
//! in two pieces. [`SectionStripJob`] sits on the stripping ship while the
//! timer runs, so the Ship pane and the HUD can show its progress. When the
//! timer ends, a [`SectionStripRequest`] takes the section off its hull.
//!
//! A strip is not a destruction. The section does not go through
//! `IntegrityDestroyMarker`, which would fire the destruction burst,
//! `OnDestroyed`, a canister drop of its parts share and a floating wreck
//! piece. Its parts went to the stripping ship's hold, so the section and its
//! art simply leave: [`strip_section`] prunes it from its neighbours'
//! [`ConnectedTo`] as the destroy path does, lowers the hull's pinned health
//! maximum by its pool as a swap does, and despawns it. A stripped section is
//! then a design section with no live entity, as a destroyed one is, so a
//! freeze, a thaw and a save keep it stripped.
//!
//! [`plan_section_strip`]: nova_gameplay::prelude::plan_section_strip

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use nova_gameplay::prelude::{ConnectedTo, Health, SectionMarker, SpaceshipRootMarker};

/// The strip job, the split rule and the removal request.
pub mod prelude {
    pub use super::{strip_splits_hull, SectionStripJob, SectionStripRequest};
}

/// A strip in progress, on the root of the ship that strips.
///
/// Inserted by the Ship pane when a strip is planned and removed when it
/// finishes or is cancelled. Not saved: a save mid-strip loses only the
/// timer, and nothing has moved yet.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct SectionStripJob {
    /// The docked hull being stripped.
    pub partner: Entity,
    /// The section being stripped, a child of `partner`.
    pub section: Entity,
    /// The section's design section id on `partner`.
    pub section_id: String,
    /// The section's pane code on `partner` (`PDC-1`).
    pub code: String,
    /// Game seconds the strip has run.
    pub elapsed_secs: f32,
    /// Game seconds the strip takes.
    pub duration_secs: f32,
}

impl SectionStripJob {
    /// The share of the strip done, in `[0, 1]`.
    pub fn progress(&self) -> f32 {
        if self.duration_secs <= 0.0 {
            return 1.0;
        }
        (self.elapsed_secs / self.duration_secs).clamp(0.0, 1.0)
    }

    /// Game seconds left, never below zero.
    pub fn remaining_secs(&self) -> f32 {
        (self.duration_secs - self.elapsed_secs).max(0.0)
    }

    /// Whether the timer has run out.
    pub fn is_done(&self) -> bool {
        self.elapsed_secs >= self.duration_secs
    }
}

/// Asks to take section `entity` off its hull after a strip: raised by the
/// Ship pane once the strip's parts are in the stripping ship's hold.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SectionStripRequest {
    /// The section to take off.
    pub entity: Entity,
}

/// Whether removing `target` leaves the other live sections of its hull in
/// more than one connected piece. `graph` lists every live section of the
/// hull with its neighbours ([`ConnectedTo`]); a neighbour not listed is
/// ignored. A hull of one section, or of `target` alone, never splits.
pub fn strip_splits_hull<'a>(
    target: Entity,
    graph: impl IntoIterator<Item = (Entity, &'a [Entity])>,
) -> bool {
    let graph: BTreeMap<Entity, &[Entity]> = graph
        .into_iter()
        .filter(|(section, _)| *section != target)
        .collect();
    let Some(&start) = graph.keys().next() else {
        return false;
    };
    let mut reached = BTreeSet::from([start]);
    let mut stack = vec![start];
    while let Some(section) = stack.pop() {
        for neighbour in graph[&section] {
            if graph.contains_key(neighbour) && reached.insert(*neighbour) {
                stack.push(*neighbour);
            }
        }
    }
    reached.len() < graph.len()
}

/// Take a stripped section off its hull: prune it from its neighbours'
/// [`ConnectedTo`], lower the hull's pinned health maximum by the section's
/// pool, and despawn it with its art.
///
/// # Panics
///
/// On an entity that is not a live section of a ship root: the Ship pane
/// raises the request only for a live section of the docked partner.
pub(crate) fn strip_section(
    request: On<SectionStripRequest>,
    mut commands: Commands,
    q_section: Query<(&ChildOf, &Health), With<SectionMarker>>,
    mut q_connected: Query<&mut ConnectedTo>,
    mut q_root: Query<&mut Health, (With<SpaceshipRootMarker>, Without<SectionMarker>)>,
) {
    let section = request.entity;
    let Ok((&ChildOf(root), health)) = q_section.get(section) else {
        panic!("strip_section: {section:?} is not a live section");
    };
    let pool = health.max;
    let neighbours = q_connected
        .get(section)
        .map(|connected| connected.0.clone())
        .unwrap_or_default();
    for neighbour in neighbours {
        if let Ok(mut connected) = q_connected.get_mut(neighbour) {
            connected.retain(|&node| node != section);
        }
    }
    let Ok(mut root_health) = q_root.get_mut(root) else {
        panic!("strip_section: section {section:?} hangs off {root:?}, which is not a ship root");
    };
    // The collapse share stays a share of the hull as it stands now, so a
    // stripped wreck does not tear itself apart for losing what was taken.
    root_health.max = (root_health.max - pool).max(0.0);
    debug!("strip_section: {section:?} leaves {root:?}");
    commands.entity(section).despawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entities(count: u32) -> Vec<Entity> {
        (1..=count)
            .map(Entity::from_raw_u32)
            .map(Option::unwrap)
            .collect()
    }

    /// A strip takes the section off: its neighbours forget it, the hull's
    /// pinned maximum drops by its pool, and it is gone with its children.
    #[test]
    fn a_stripped_section_leaves_its_neighbours_and_the_pinned_pool() {
        let mut app = App::new();
        app.add_observer(strip_section);
        let world = app.world_mut();
        let root = world.spawn((SpaceshipRootMarker, Health::new(300.0))).id();
        let spawn_section = |world: &mut World| {
            world
                .spawn((SectionMarker, Health::new(100.0), ChildOf(root)))
                .id()
        };
        let (a, b, c) = (
            spawn_section(world),
            spawn_section(world),
            spawn_section(world),
        );
        let art = world.spawn(ChildOf(c)).id();
        world.entity_mut(a).insert(ConnectedTo(vec![b, c]));
        world.entity_mut(b).insert(ConnectedTo(vec![a]));
        world.entity_mut(c).insert(ConnectedTo(vec![a]));

        world.trigger(SectionStripRequest { entity: c });
        world.flush();

        assert!(world.get_entity(c).is_err());
        assert!(world.get_entity(art).is_err());
        assert_eq!(world.get::<ConnectedTo>(a).unwrap().0, vec![b]);
        assert_eq!(world.get::<ConnectedTo>(b).unwrap().0, vec![a]);
        assert_eq!(world.get::<Health>(root).unwrap().max, 200.0);
    }

    /// A chain A-B-C splits when B goes and holds when an end goes; a ring
    /// holds whatever goes.
    #[test]
    fn a_strip_splits_the_hull_only_through_a_cut_section() {
        let e = entities(4);
        let (a, b, c, d) = (e[0], e[1], e[2], e[3]);
        let chain = [(a, vec![b]), (b, vec![a, c]), (c, vec![b])];
        let graph = || chain.iter().map(|(s, n)| (*s, n.as_slice()));
        assert!(strip_splits_hull(b, graph()));
        assert!(!strip_splits_hull(a, graph()));
        assert!(!strip_splits_hull(c, graph()));
        let ring = [
            (a, vec![b, d]),
            (b, vec![a, c]),
            (c, vec![b, d]),
            (d, vec![c, a]),
        ];
        for target in [a, b, c, d] {
            assert!(!strip_splits_hull(
                target,
                ring.iter().map(|(s, n)| (*s, n.as_slice()))
            ));
        }
        assert!(!strip_splits_hull(a, [(a, [].as_slice())]));
    }
}
