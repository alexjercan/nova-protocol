//! A docked lootable hull stripped whole: the running strip, the order its
//! sections come off in, and the removal each one ends in.
//!
//! The rules that plan a strip are [`plan_hull_strip`] and
//! [`plan_section_yield`]; this module adds the order and the two ship-side
//! pieces a strip needs. [`HullStripJob`] sits on the stripping ship while
//! the strip runs, so the Recycle tab and the HUD can show its progress.
//! [`next_strip_section`] picks the section that comes off next: the one
//! farthest from the berth port, so the rest of the hull stays joined to the
//! berth at every step, and the berth last. When a section's timer ends, a
//! [`SectionStripRequest`] takes it off its hull. The berth is not taken off
//! that way: the stripping ship lets go of the dock and the hull despawns.
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
//! [`plan_hull_strip`]: nova_gameplay::prelude::plan_hull_strip
//! [`plan_section_yield`]: nova_gameplay::prelude::plan_section_yield

use std::collections::{BTreeMap, VecDeque};

use bevy::prelude::*;
use nova_gameplay::prelude::{
    ConnectedTo, Health, HullStripPauseType, SectionMarker, SpaceshipRootMarker,
};

/// The strip job, the strip order and the removal request.
pub mod prelude {
    pub use super::{next_strip_section, HullStripJob, HullStripStep, SectionStripRequest};
}

/// A whole-hull strip in progress, on the root of the ship that strips.
///
/// Inserted by the Recycle tab when a strip is planned and removed when the
/// hull is gone or the strip stops. Not saved: a save mid-strip loses the
/// running section's timer, and every section already taken stays taken.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct HullStripJob {
    /// The docked hull being stripped.
    pub partner: Entity,
    /// The section coming off now; `None` until the next one is picked.
    pub step: Option<HullStripStep>,
    /// Game seconds of the sections already taken.
    pub done_secs: f32,
    /// Game seconds the whole strip was planned to take.
    pub total_secs: f32,
    /// Why the strip holds at a finished section, while it does.
    #[reflect(ignore)]
    pub paused: Option<HullStripPauseType>,
    /// Sections taken so far.
    pub taken: u32,
}

/// The section a [`HullStripJob`] is taking off now.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct HullStripStep {
    /// The section, a child of the job's partner.
    pub section: Entity,
    /// Its design section id on the partner.
    pub section_id: String,
    /// Its pane code on the partner (`PDC-1`).
    pub code: String,
    /// Game seconds it has run.
    pub elapsed_secs: f32,
    /// Game seconds it takes.
    pub duration_secs: f32,
}

impl HullStripJob {
    /// A strip of `partner` planned to take `total_secs`, with nothing taken.
    pub fn new(partner: Entity, total_secs: f32) -> Self {
        Self {
            partner,
            step: None,
            done_secs: 0.0,
            total_secs,
            paused: None,
            taken: 0,
        }
    }

    /// The share of the whole strip done, in `[0, 1]`.
    pub fn progress(&self) -> f32 {
        if self.total_secs <= 0.0 {
            return 1.0;
        }
        let running = self.step.as_ref().map_or(0.0, |step| step.elapsed_secs);
        ((self.done_secs + running) / self.total_secs).clamp(0.0, 1.0)
    }

    /// Game seconds left on the plan, never below zero.
    pub fn remaining_secs(&self) -> f32 {
        let running = self.step.as_ref().map_or(0.0, |step| step.elapsed_secs);
        (self.total_secs - self.done_secs - running).max(0.0)
    }
}

impl HullStripStep {
    /// Whether the section's timer has run out.
    pub fn is_done(&self) -> bool {
        self.elapsed_secs >= self.duration_secs
    }
}

/// Asks to take section `entity` off its hull after a strip: raised by the
/// Recycle tab once the section's parts are in the stripping ship's hold.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct SectionStripRequest {
    /// The section to take off.
    pub entity: Entity,
}

/// The live section a whole-hull strip takes next: the one farthest from
/// `berth` in hops over [`ConnectedTo`], so every section left is still
/// joined to the berth. A section the berth cannot reach goes first. Ties go
/// to the lowest `key`, the caller's order. The berth comes last, once it is
/// the only section left; `None` when `graph` is empty.
///
/// `graph` lists every live section of the hull with its order key and its
/// neighbours; a neighbour not listed is ignored.
pub fn next_strip_section<'a, K: Ord + Copy>(
    berth: Entity,
    graph: impl IntoIterator<Item = (Entity, K, &'a [Entity])>,
) -> Option<Entity> {
    let graph: BTreeMap<Entity, (K, &[Entity])> = graph
        .into_iter()
        .map(|(section, key, neighbours)| (section, (key, neighbours)))
        .collect();
    let mut hops: BTreeMap<Entity, u32> = BTreeMap::new();
    if graph.contains_key(&berth) {
        hops.insert(berth, 0);
        let mut queue = VecDeque::from([berth]);
        while let Some(section) = queue.pop_front() {
            let next = hops[&section] + 1;
            for neighbour in graph[&section].1 {
                if graph.contains_key(neighbour) && !hops.contains_key(neighbour) {
                    hops.insert(*neighbour, next);
                    queue.push_back(*neighbour);
                }
            }
        }
    }
    graph
        .iter()
        .max_by(|(a, (a_key, _)), (b, (b_key, _))| {
            let distance = |section: &Entity| hops.get(section).copied().unwrap_or(u32::MAX);
            distance(a).cmp(&distance(b)).then_with(|| b_key.cmp(a_key))
        })
        .map(|(section, _)| *section)
}

/// Take a stripped section off its hull: prune it from its neighbours'
/// [`ConnectedTo`], lower the hull's pinned health maximum by the section's
/// pool, and despawn it with its art.
///
/// # Panics
///
/// On an entity that is not a live section of a ship root: the Recycle tab
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

    /// A chain berth-A-B with a spur C off A and a stray D the berth cannot
    /// reach: D goes first, then the farthest (B), then A's tie with C by
    /// key, and the berth last. After each pick the rest still reaches the
    /// berth.
    #[test]
    fn a_strip_peels_the_hull_from_the_far_end_and_takes_the_berth_last() {
        let e = entities(5);
        let (berth, a, b, c, d) = (e[0], e[1], e[2], e[3], e[4]);
        let mut graph: Vec<(Entity, u32, Vec<Entity>)> = vec![
            (berth, 0, vec![a]),
            (a, 1, vec![berth, b, c]),
            (b, 2, vec![a]),
            (c, 3, vec![a]),
            (d, 4, vec![]),
        ];
        let mut order = Vec::new();
        while let Some(next) =
            next_strip_section(berth, graph.iter().map(|(s, k, n)| (*s, *k, n.as_slice())))
        {
            order.push(next);
            graph.retain(|(s, ..)| *s != next);
        }
        assert_eq!(order, vec![d, b, c, a, berth]);
        assert_eq!(
            next_strip_section(berth, std::iter::empty::<(Entity, u32, &[Entity])>()),
            None
        );
    }
}
