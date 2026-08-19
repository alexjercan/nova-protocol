//! Ship-specific adapter for the generic integrity graph and lifecycle.
//!
//! Change this module when ship structure publication or aggregate health semantics change.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use avian3d::prelude::{mass_properties::MassPropertySystems, *};
use bevy::prelude::*;
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;

use super::link_points::prelude::*;

/// Ship graph publication, disabled-section behavior, and aggregate health.
pub mod prelude {
    pub use super::{
        ShipIntegrityPlugin, ShipWreckFragmentMarker, StructuralCollapseMarker,
        StructuralCollapseThreshold, DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD,
    };
}

/// The structural-collapse fraction a ship gets when nothing is authored.
///
/// Five percent of the built hull is the final wreckage floor. Physical
/// severing handles major hull loss without executing a still-capable half ship;
/// this threshold only prevents a tiny command shard from lingering forever.
pub const DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD: f32 = 0.05;

/// Relative speed applied to each severed component before momentum balancing.
const SEVER_SEPARATION_SPEED: f32 = 1.0;

/// The fraction of a ship's PINNED maximum health below which the hull comes
/// apart and the whole ship is destroyed - see [`aggregate_ship_health`].
///
/// `0.0` means "only a ship with no living sections at all dies", which is the
/// degenerate case the rule grew out of. Values are clamped to `0..=1` by
/// [`StructuralCollapseThreshold::new`]; a ship with no threshold component
/// collapses at [`DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct StructuralCollapseThreshold(pub f32);

impl Default for StructuralCollapseThreshold {
    fn default() -> Self {
        Self(DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD)
    }
}

impl StructuralCollapseThreshold {
    /// The threshold for `fraction` of the pinned hull, clamped to `0..=1`.
    /// The floor is load-bearing: a negative threshold is unreachable even at
    /// zero health, which would bring back the 0-HP ghost the rule exists to
    /// kill. `0.0` is how an author says "dismantle it completely".
    pub fn new(fraction: f32) -> Self {
        Self(fraction.clamp(0.0, 1.0))
    }
}

/// Marks a ship root that has fallen below its [`StructuralCollapseThreshold`]
/// and is now tearing itself apart - see [`cascade_structural_collapse`].
///
/// Latched: a wreck does not recover, and latching is also what keeps the
/// per-frame collapse test from re-marking the same root forever.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct StructuralCollapseMarker {
    /// Sections still standing at the previous cascade tick, `None` before the
    /// first one. The cascade's progress signal - see the no-progress override.
    standing: Option<usize>,
}

/// An inert, persistent compound body made from structure severed from a ship.
/// It is not a spaceship and owns no control, allegiance, or scenario identity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct ShipWreckFragmentMarker;

#[derive(Clone, Debug)]
struct PendingSeverCut {
    cut_offsets_from_com: Vec<Vec3>,
    old_origin_world: Vec3,
    old_rotation: Quat,
    old_com_local: Vec3,
    old_linear_velocity: Vec3,
    old_angular_velocity: Vec3,
}

#[derive(Resource, Default)]
struct PendingSeverRoots(BTreeMap<Entity, PendingSeverCut>);

#[derive(Clone, Debug)]
struct PendingSeverBatch {
    bodies: Vec<Entity>,
    origin_world: Vec3,
    rotation: Quat,
    cut_origin_world: Vec3,
    old_com_world: Vec3,
    old_linear_velocity: Vec3,
    old_angular_velocity: Vec3,
}

#[derive(Resource, Default)]
struct PendingSeverMotion(Vec<PendingSeverBatch>);

/// Adapts section-based ships to the generic gameplay integrity pipeline.
pub struct ShipIntegrityPlugin;

impl Plugin for ShipIntegrityPlugin {
    fn build(&self, app: &mut App) {
        debug!("ShipIntegrityPlugin: build");

        app.register_type::<LinkPoint>();
        app.register_type::<SectionLinkPoints>();
        app.register_type::<StructuralCollapseThreshold>();
        app.register_type::<StructuralCollapseMarker>();
        app.register_type::<ShipWreckFragmentMarker>();
        app.init_resource::<PendingSeverRoots>();
        app.init_resource::<PendingSeverMotion>();
        app.add_observer(on_section_disable);
        app.add_observer(queue_depleted_section_sever);
        app.add_systems(Update, build_ship_integrity_graph.before(IntegritySystems));
        // Chained: the cascade reads the collapse marker the aggregate writes,
        // and the ordering edge is what gets the marker applied in between.
        app.add_systems(
            Update,
            (aggregate_ship_health, cascade_structural_collapse)
                .chain()
                .in_set(IntegritySystems),
        );
        app.add_systems(
            Update,
            (sever_disconnected_structures, cleanup_empty_wreck_fragments)
                .chain()
                .after(IntegritySystems),
        );
        app.add_systems(
            FixedPostUpdate,
            (recompute_pending_sever_mass, apply_pending_sever_motion)
                .chain()
                .after(MassPropertySystems::UpdateComputedMassProperties)
                .before(PhysicsSystems::StepSimulation),
        );
    }
}

/// Directly depleted sections die regardless of graph degree. Healthy sections
/// disabled by structural collapse retain the leaf-first peel.
fn on_section_disable(
    add: On<Add, IntegrityDisabledMarker>,
    mut commands: Commands,
    q_section: Query<
        (Has<HealthZeroMarker>, Has<IntegrityLeafMarker>),
        (With<SectionMarker>, With<IntegrityDisabledMarker>),
    >,
) {
    let entity = add.entity;
    let Ok((depleted, leaf)) = q_section.get(entity) else {
        return;
    };

    if depleted && !leaf {
        debug!("on_section_disable: depleted interior section {entity:?} destroyed");
        commands.entity(entity).try_insert(IntegrityDestroyMarker);
    } else if !depleted && !leaf {
        trace!("on_section_disable: collapse disabled interior section {entity:?}");
        commands.entity(entity).try_insert(SectionInactiveMarker);
    }
}

/// Remember the owning structure and cut point before destruction despawns the
/// section. One later partition handles every section destroyed in the frame.
fn queue_depleted_section_sever(
    add: On<Add, HealthZeroMarker>,
    mut pending: ResMut<PendingSeverRoots>,
    q_section: Query<(&ChildOf, &ColliderTransform, &ColliderMassProperties), With<SectionMarker>>,
    q_root: Query<(&Position, &Rotation, &LinearVelocity, &AngularVelocity)>,
    q_fixtures: Query<
        (&ColliderOf, &ColliderTransform, &ColliderMassProperties),
        Without<SectionMarker>,
    >,
) {
    let Ok((&ChildOf(root), collider_transform, mass_properties)) = q_section.get(add.entity)
    else {
        return;
    };
    let Ok((position, rotation, linear, angular)) = q_root.get(root) else {
        return;
    };
    let mut total_mass = 0.0;
    let mut weighted_center = Vec3::ZERO;
    for (ChildOf(parent), transform, properties) in &q_section {
        if *parent != root || properties.mass <= f32::EPSILON {
            continue;
        }
        total_mass += properties.mass;
        weighted_center += transform.transform_point(properties.center_of_mass) * properties.mass;
    }
    for (collider_of, transform, properties) in &q_fixtures {
        if collider_of.body != root || properties.mass <= f32::EPSILON {
            continue;
        }
        total_mass += properties.mass;
        weighted_center += transform.transform_point(properties.center_of_mass) * properties.mass;
    }
    let old_com_local = if total_mass > f32::EPSILON {
        weighted_center / total_mass
    } else {
        Vec3::ZERO
    };
    let cut_local = collider_transform.transform_point(mass_properties.center_of_mass);
    let cut_offset = cut_local - old_com_local;
    if let Some(cut) = pending.0.get_mut(&root) {
        cut.cut_offsets_from_com.push(cut_offset);
        return;
    }
    pending.0.insert(
        root,
        PendingSeverCut {
            cut_offsets_from_com: vec![cut_offset],
            old_origin_world: position.0,
            old_rotation: rotation.0,
            old_com_local,
            old_linear_velocity: **linear,
            old_angular_velocity: **angular,
        },
    );
}

/// Split a structure whose destroyed section disconnected its graph.
#[allow(clippy::type_complexity)]
fn sever_disconnected_structures(
    mut commands: Commands,
    mut pending: ResMut<PendingSeverRoots>,
    mut pending_motion: ResMut<PendingSeverMotion>,
    q_roots: Query<
        (
            Entity,
            &Children,
            &Position,
            &Rotation,
            &LinearVelocity,
            &AngularVelocity,
            Has<SpaceshipRootMarker>,
            Has<StructuralCollapseMarker>,
        ),
        (
            With<IntegrityRoot>,
            Or<(With<SpaceshipRootMarker>, With<ShipWreckFragmentMarker>)>,
        ),
    >,
    q_sections: Query<
        (
            &ConnectedTo,
            &Transform,
            &Health,
            Has<ControllerSectionMarker>,
            Has<SectionInactiveMarker>,
            Has<IntegrityDestroyMarker>,
        ),
        With<SectionMarker>,
    >,
) {
    let pending_roots = std::mem::take(&mut pending.0);
    for (root, cut) in pending_roots {
        let Ok((
            _,
            children,
            position,
            rotation,
            linear_velocity,
            angular_velocity,
            is_spaceship,
            collapsing,
        )) = q_roots.get(root)
        else {
            continue;
        };
        if collapsing {
            continue;
        }

        let mut sections: Vec<_> = children
            .iter()
            .filter(|child| {
                q_sections
                    .get(*child)
                    .is_ok_and(|(_, _, _, _, _, destroying)| !destroying)
            })
            .collect();
        sections.sort_by_key(|section| section.to_bits());
        if sections.len() < 2 {
            continue;
        }
        let section_set: BTreeSet<_> = sections.iter().copied().collect();

        let mut unseen = section_set.clone();
        let mut components = Vec::new();
        while let Some(start) = unseen.pop_first() {
            let mut component = vec![start];
            let mut queue = VecDeque::from([start]);
            while let Some(section) = queue.pop_front() {
                let Ok((connected, ..)) = q_sections.get(section) else {
                    continue;
                };
                for neighbor in connected.iter().copied() {
                    if section_set.contains(&neighbor) && unseen.remove(&neighbor) {
                        component.push(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
            component.sort_by_key(|section| section.to_bits());
            components.push(component);
        }
        if components.len() <= 1 {
            continue;
        }

        let rank = |component: &[Entity]| {
            let mut live_controllers = 0usize;
            let mut maximum_health = 0.0f32;
            for section in component {
                if let Ok((_, _, health, controller, inactive, _)) = q_sections.get(*section) {
                    maximum_health += health.max;
                    live_controllers +=
                        usize::from(controller && !inactive && health.current > 0.0);
                }
            }
            (live_controllers, maximum_health)
        };
        let retained = components
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                let (a_controllers, a_health) = rank(a);
                let (b_controllers, b_health) = rank(b);
                let controller_order = if is_spaceship {
                    a_controllers.cmp(&b_controllers)
                } else {
                    std::cmp::Ordering::Equal
                };
                controller_order
                    .then_with(|| a_health.total_cmp(&b_health))
                    // `max_by` keeps the later equal item; reverse the stable
                    // entity key so the lowest entity wins a complete tie.
                    .then_with(|| b[0].to_bits().cmp(&a[0].to_bits()))
            })
            .map(|(index, _)| index)
            .expect("a split has components");

        let mut bodies = vec![root];
        for (index, component) in components.iter().enumerate() {
            if index == retained {
                continue;
            }
            let fragment = commands
                .spawn((
                    Name::new("Severed Ship Wreck"),
                    ShipWreckFragmentMarker,
                    IntegrityRoot,
                    RigidBody::Dynamic,
                    Position(position.0),
                    Rotation(rotation.0),
                    Transform::from_translation(position.0).with_rotation(rotation.0),
                    Visibility::default(),
                    LinearVelocity(**linear_velocity),
                    AngularVelocity(**angular_velocity),
                    TransformInterpolation,
                ))
                .id();
            bodies.push(fragment);
            for section in component {
                let Ok((_, transform, ..)) = q_sections.get(*section) else {
                    continue;
                };
                commands.entity(*section).insert((
                    ChildOf(fragment),
                    *transform,
                    SectionInactiveMarker,
                ));
            }
        }

        let cut_offset = cut.cut_offsets_from_com.iter().copied().sum::<Vec3>()
            / cut.cut_offsets_from_com.len() as f32;
        let old_com_world = cut.old_origin_world + cut.old_rotation * cut.old_com_local;
        pending_motion.0.push(PendingSeverBatch {
            bodies,
            origin_world: cut.old_origin_world,
            rotation: cut.old_rotation,
            cut_origin_world: old_com_world + cut.old_rotation * cut_offset,
            old_com_world,
            old_linear_velocity: cut.old_linear_velocity,
            old_angular_velocity: cut.old_angular_velocity,
        });
        debug!(
            "sever_disconnected_structures: {root:?} split into {} bodies",
            components.len()
        );
    }
}

/// Recompute immediately because hierarchy changes can miss Avian's current
/// prepare pass when they land after its recomputation queue.
fn recompute_pending_sever_mass(
    pending: Res<PendingSeverMotion>,
    mut mass_properties: MassPropertyHelper,
) {
    for batch in &pending.0 {
        for body in &batch.bodies {
            mass_properties.update_mass_properties(*body);
        }
    }
}

/// Restore each new body's rigid point velocity after Avian computes its new
/// centre of mass, then add a momentum-neutral fracture kick.
#[allow(clippy::type_complexity)]
fn apply_pending_sever_motion(
    mut pending: ResMut<PendingSeverMotion>,
    mut bodies: ParamSet<(
        Query<(&ComputedCenterOfMass, &ComputedMass)>,
        Query<(
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut AngularVelocity,
        )>,
    )>,
) {
    let mut waiting = Vec::new();
    for batch in pending.0.drain(..) {
        let samples: Option<Vec<_>> = {
            let q_body = bodies.p0();
            batch
                .bodies
                .iter()
                .map(|body| {
                    let (center, mass) = q_body.get(*body).ok()?;
                    let com_world = batch.origin_world + batch.rotation * center.0;
                    let direction = (com_world - batch.cut_origin_world)
                        .try_normalize()
                        .unwrap_or(Vec3::X);
                    Some((*body, com_world, mass.value(), direction))
                })
                .collect()
        };
        let Some(samples) = samples else {
            waiting.push(batch);
            continue;
        };
        let total_mass: f32 = samples.iter().map(|(_, _, mass, _)| *mass).sum();
        if total_mass <= f32::EPSILON {
            waiting.push(batch);
            continue;
        }
        let mean_kick = samples
            .iter()
            .map(|(_, _, mass, direction)| *direction * (*mass * SEVER_SEPARATION_SPEED))
            .sum::<Vec3>()
            / total_mass;

        let mut q_motion = bodies.p1();
        for (body, com_world, _, direction) in samples {
            let Ok((mut position, mut rotation, mut linear, mut angular)) = q_motion.get_mut(body)
            else {
                continue;
            };
            **position = batch.origin_world;
            **rotation = batch.rotation;
            let point_velocity = batch.old_linear_velocity
                + batch
                    .old_angular_velocity
                    .cross(com_world - batch.old_com_world);
            **linear = point_velocity + direction * SEVER_SEPARATION_SPEED - mean_kick;
            **angular = batch.old_angular_velocity;
        }
    }
    pending.0 = waiting;
}

/// A fragment root has no aggregate health component, so remove it explicitly
/// after its last section has gone.
fn cleanup_empty_wreck_fragments(
    mut commands: Commands,
    q_fragments: Query<(Entity, Option<&Children>), With<ShipWreckFragmentMarker>>,
    q_sections: Query<(), With<SectionMarker>>,
) {
    for (fragment, children) in &q_fragments {
        let has_section = children
            .is_some_and(|children| children.iter().any(|child| q_sections.contains(child)));
        if !has_section {
            commands.entity(fragment).try_despawn();
        }
    }
}

/// Build each ship's authoritative section graph after its section spawn batch is complete.
///
/// An observer on Avian's `Add<ColliderOf>` sees colliders one at a time. A valid ship can be
/// disconnected halfway through that sequence, which briefly publishes an empty graph and
/// emits false errors. `Added<SectionLinkPoints>` is evaluated once per update, after all
/// section commands from the spawn observer have landed, so each affected root is derived once
/// from its complete authored batch.
///
/// `pub(crate)` because the derived skin hangs off the same edge and orders
/// itself after this: structure settles, then it is dressed.
pub(crate) fn build_ship_integrity_graph(
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

/// Keep each ship's aggregate health equal to the sum of its living section children over a
/// PINNED maximum, so the health HUD tracks real damage, and destroy a ship that has fallen
/// below its [`StructuralCollapseThreshold`].
///
/// Scoped to spaceship roots ([`SpaceshipRootMarker`]) on purpose: other [`IntegrityRoot`]s,
/// such as a lone asteroid, hold their [`Health`] on the collider body itself and have no
/// [`SectionMarker`] children to sum. Running this on them would just staple a meaningless
/// `Health { current: 0, max: 0 }` onto the root every frame. "Sum a ship's sections" only
/// makes sense for ships, so only ships are matched.
///
/// Sections are direct children of the ship root (which carries [`IntegrityRoot`]). This
/// recomputes the root's `current` every frame as the sum of its living sections. `max` is a
/// RUNNING MAXIMUM instead, because a destroyed section despawns and takes its share of the
/// sum with it: a live denominator makes the HP bar FILL UP as a ship is shot apart (150/1100
/// becomes 100/100 when a 1000-hp section dies) and makes any fraction of it rebound, so a
/// percentage threshold could never trip. A running maximum is also why this is not a
/// set-once pin: a ship's sections can land across several frames, and a first reading would
/// pin a half-assembled hull; taking the maximum every frame instead cannot rebound, and still
/// grows if a ship is ever repaired or extended.
///
/// Damage on a section also bubbles up to the root (`HealthApplyDamage` auto-propagates
/// through `ChildOf`) and the health layer clamps the bubbled amount to what actually landed,
/// `min(amount, section.current)` rather than the raw hit. That is why overkill on one section
/// cannot kill a ship (a 1000-damage hit on a 100 hp section costs the root 100, not 1000).
/// The recompute overwrites whatever that bubble left on the root, so the collapse rule below
/// - not the bubble - is what actually kills ships.
///
/// Roots carry no `ConnectedTo` and are never leaves, so root destruction is a separate
/// integrity-core hop; the meshless root is then despawned and the ship dies (its
/// `PlayerSpaceshipMarker` is removed, reverting the camera and clearing the HUDs).
fn aggregate_ship_health(
    mut commands: Commands,
    q_root: Query<
        (
            Entity,
            Option<&Health>,
            Option<&Children>,
            Has<HealthZeroMarker>,
            Has<StructuralCollapseMarker>,
            Option<&StructuralCollapseThreshold>,
        ),
        (With<IntegrityRoot>, With<SpaceshipRootMarker>),
    >,
    q_section_health: Query<&Health, (With<SectionMarker>, Without<IntegrityRoot>)>,
) {
    for (root, root_health, children, already_zero, already_collapsing, threshold) in &q_root {
        let mut current = 0.0;
        let mut living_max = 0.0;
        let mut standing = 0usize;
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(health) = q_section_health.get(child) {
                    current += health.current;
                    living_max += health.max;
                    standing += 1;
                }
            }
        }

        let pinned_max = root_health.map_or(0.0, |health| health.max);
        let max = pinned_max.max(living_max);

        // Structural collapse. Below its threshold of the hull it was built
        // with, a ship is wreckage - but the ROOT is not what dies of it.
        // Collapse hands the ship to `cascade_structural_collapse`, which
        // disables what is still standing so the ordinary disable -> destroy
        // chain peels it apart section by section; every one of them bursts
        // its debris on the way out instead of being despawned silently with
        // the root. Sections leaving the sum is what then walks `current` down
        // to zero, and zero is where the root dies - one frame at a time, but
        // still through exactly one destruction path.
        //
        // With nothing structural left the root is marked directly, which is
        // both the last hop of a collapse and the old structural-death backstop
        // (the 0-HP ghost) in its own right: `HealthZeroMarker` otherwise only
        // ever comes from the damage path (nova's `on_damage`), so a ship that
        // loses its last section WITHOUT a final bubble reaching the root (a
        // direct destroy, a detach, any future scripted removal) would sit here
        // forever as an unmarked 0-HP hull. The recompute is the one place that
        // always sees how much structure is left, so it owns the rule.
        //
        // The `pinned_max > 0` guard means "this root has HAD sections", and it
        // reads the PREVIOUS frame's write on purpose - a mid-spawn root whose
        // sections have not landed yet is not executed at birth.
        let fraction = threshold.copied().unwrap_or_default().0;
        if pinned_max > 0.0 && current <= max * fraction {
            if standing == 0 {
                if !already_zero {
                    debug!(
                        "aggregate_ship_health: root {root:?} has no structure left \
                         ({current} of {max}, threshold {fraction}); marking it destroyed"
                    );
                    commands.entity(root).try_insert(HealthZeroMarker);
                }
            } else if !already_collapsing {
                debug!(
                    "aggregate_ship_health: root {root:?} collapsed structurally \
                     ({current} of {max}, threshold {fraction}); peeling {standing} sections"
                );
                commands
                    .entity(root)
                    .try_insert(StructuralCollapseMarker::default());
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

/// Peel a collapsing ship apart from its extremities inward.
///
/// Every section still standing is disabled, and the generic core's existing
/// chain does the rest: a disabled LEAF is destroyed (which bursts its debris),
/// the prune turns its neighbours into leaves, and they follow on the next
/// frames. The ship comes apart over several frames instead of vanishing, and
/// each section dies through [`IntegrityDestroyMarker`] rather than being
/// despawned silently along with its root.
///
/// THE NO-PROGRESS OVERRIDE, which is not optional. Disabling a section does
/// not zero its health - only DESTRUCTION takes a section out of the root's
/// sum - so a shape with no leaves never drains. Four hulls mated in a ring
/// each keep two neighbours, none ever becomes a leaf, nothing is destroyed,
/// `current` never falls and the root never dies: an immortal disabled hulk,
/// the 0-HP ghost in a new costume. So the leaf rule is treated as what it is,
/// a preference for the ORDER a wreck comes apart in and not a correctness
/// requirement. A tick that disables nothing new AND does not see the standing
/// count fall is a stall, and the most leaf-like survivor is then destroyed
/// whatever its neighbours. Breaking one node out of a ring leaves a chain, so
/// the ordinary cascade takes over again and the peel is kept everywhere it is
/// possible. Do not simplify this into "destroy the leaves".
///
/// Progress is measured as the standing count FALLING rather than against a
/// frame budget, because the cascade's own gaps are irregular (a prune lands
/// one frame, the leaf derivation reads it the next) while a count that fell is
/// direct evidence that a section died. Sections already carrying
/// [`IntegrityDestroyMarker`] are left out of the count, so a destruction whose
/// despawn has not landed yet still reads as progress.
fn cascade_structural_collapse(
    mut commands: Commands,
    mut q_collapsing: Query<(&Children, &mut StructuralCollapseMarker)>,
    q_standing: Query<
        (Entity, Option<&ConnectedTo>, Has<IntegrityDisabledMarker>),
        (With<SectionMarker>, Without<IntegrityDestroyMarker>),
    >,
) {
    for (children, mut collapse) in &mut q_collapsing {
        let mut standing: Vec<_> = children
            .iter()
            .filter_map(|child| q_standing.get(child).ok())
            .collect();
        if standing.is_empty() {
            // Nothing left to peel; `aggregate_ship_health` takes the root.
            continue;
        }

        // The collapse test stays true for every frame of the cascade, so only
        // sections that are not disabled yet are touched.
        let mut disabled_any = false;
        for &(section, _, disabled) in &standing {
            if !disabled {
                commands.entity(section).try_insert(IntegrityDisabledMarker);
                disabled_any = true;
            }
        }

        let stalled = !disabled_any
            && collapse
                .standing
                .is_some_and(|previous| standing.len() >= previous);
        if collapse.standing != Some(standing.len()) {
            collapse.standing = Some(standing.len());
        }
        if !stalled {
            continue;
        }

        // Fewest neighbours first, so the forced kill still starts at the
        // closest thing this shape has to an extremity. Entity order only
        // breaks ties, and only so the peel is reproducible.
        standing.sort_by_key(|(section, connected, _)| {
            (
                connected.map_or(0, |connected| connected.len()),
                section.to_bits(),
            )
        });
        let (section, ..) = standing[0];
        debug!("cascade_structural_collapse: no leaf to peel, forcing {section:?} apart");
        commands.entity(section).try_insert(IntegrityDestroyMarker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An aggregate-only app: the recompute and the cascade with no
    /// destruction chain behind them, so markers land and stay put instead of
    /// despawning anything. The end-to-end shape is in `ghost_ship_tests`.
    fn aggregate_app() -> App {
        let mut app = App::new();
        app.add_systems(
            Update,
            (aggregate_ship_health, cascade_structural_collapse).chain(),
        );
        app
    }

    /// A ship root carrying `threshold` (`None` = no component at all, so the
    /// engine default applies) with one section per `(current, max)` pair.
    fn spawn_ship(
        app: &mut App,
        threshold: Option<f32>,
        sections: &[(f32, f32)],
    ) -> (Entity, Vec<Entity>) {
        let children: Vec<_> = sections
            .iter()
            .map(|(current, max)| {
                app.world_mut()
                    .spawn((
                        SectionMarker,
                        Health {
                            current: *current,
                            max: *max,
                        },
                    ))
                    .id()
            })
            .collect();
        let mut root = app.world_mut().spawn((IntegrityRoot, SpaceshipRootMarker));
        if let Some(fraction) = threshold {
            root.insert(StructuralCollapseThreshold::new(fraction));
        }
        let root = root.id();
        app.world_mut().entity_mut(root).add_children(&children);
        (root, children)
    }

    /// The ship has begun to come apart (its sections are being peeled).
    fn collapsing(app: &App, root: Entity) -> bool {
        app.world().get::<StructuralCollapseMarker>(root).is_some()
    }

    /// The ROOT itself is marked for death - the last hop, once nothing
    /// structural is left.
    fn root_marked_dead(app: &App, root: Entity) -> bool {
        app.world().get::<HealthZeroMarker>(root).is_some()
    }

    fn disabled(app: &App, section: Entity) -> bool {
        app.world()
            .get::<IntegrityDisabledMarker>(section)
            .is_some()
    }

    fn destroying(app: &App, section: Entity) -> bool {
        app.world().get::<IntegrityDestroyMarker>(section).is_some()
    }

    fn root_health(app: &App, root: Entity) -> (f32, f32) {
        let health = app.world().get::<Health>(root).unwrap();
        (health.current, health.max)
    }

    /// The reported bug: a destroyed section took its `max` out of the
    /// DENOMINATOR as well as the numerator, so a ship at 150/1100 read
    /// 100/100 and the HP bar appeared to FILL UP as it was shot apart.
    #[test]
    fn destroying_a_section_does_not_refill_the_hp_bar() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(&mut app, None, &[(50.0, 1000.0), (100.0, 100.0)]);

        app.update();
        assert_eq!(root_health(&app, root), (150.0, 1100.0));

        app.world_mut().entity_mut(sections[0]).despawn();
        app.update();

        assert_eq!(
            root_health(&app, root),
            (100.0, 1100.0),
            "the denominator is the hull the ship was BUILT with; only the numerator falls"
        );
    }

    /// The maximum is a RUNNING one, not a set-once pin: a ship whose sections
    /// land across several frames must end up with its whole hull in the
    /// denominator, not the part that happened to land first.
    #[test]
    fn a_section_landing_a_frame_late_raises_the_pinned_maximum() {
        let mut app = aggregate_app();
        let (root, _) = spawn_ship(&mut app, None, &[(100.0, 100.0)]);
        app.update();
        assert_eq!(root_health(&app, root), (100.0, 100.0));

        let late = app
            .world_mut()
            .spawn((SectionMarker, Health::new(1000.0)))
            .id();
        app.world_mut().entity_mut(root).add_children(&[late]);
        app.update();

        assert_eq!(root_health(&app, root), (1100.0, 1100.0));
    }

    /// Structural collapse: 100 hp of a pinned 1000 is under the authored
    /// quarter, so the ship starts coming apart - the surviving section is
    /// disabled and handed to the ordinary disable -> destroy chain. The ROOT
    /// outlives it and dies only once nothing structural is left.
    #[test]
    fn a_ship_below_its_collapse_threshold_starts_coming_apart() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(&mut app, Some(0.25), &[(100.0, 100.0), (900.0, 900.0)]);
        app.update();
        assert!(!collapsing(&app, root), "a fresh ship is not a wreck");

        app.world_mut().entity_mut(sections[1]).despawn();
        app.update();

        assert!(collapsing(&app, root));
        assert!(
            disabled(&app, sections[0]),
            "the survivor is disabled, not despawned with the root"
        );
        assert!(
            !root_marked_dead(&app, root),
            "the root waits for its sections to go first"
        );
        assert_eq!(root_health(&app, root), (100.0, 1000.0));
    }

    /// ...and a ship still carrying 30 percent of its hull keeps fighting.
    #[test]
    fn a_ship_just_above_its_collapse_threshold_survives() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(&mut app, Some(0.25), &[(300.0, 300.0), (700.0, 700.0)]);
        app.update();

        app.world_mut().entity_mut(sections[1]).despawn();
        app.update();

        assert!(!collapsing(&app, root), "300 of 1000 is above a quarter");
        assert!(!disabled(&app, sections[0]));
    }

    /// A root with no threshold component collapses at the engine default, so
    /// a ship spawned outside the scenario layer is not immortal.
    #[test]
    fn a_ship_with_no_authored_threshold_collapses_at_the_default() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(&mut app, None, &[(40.0, 40.0), (960.0, 960.0)]);
        app.update();

        app.world_mut().entity_mut(sections[1]).despawn();
        app.update();

        assert_eq!(
            DEFAULT_STRUCTURAL_COLLAPSE_THRESHOLD, 0.05,
            "the default leaves only the final five percent to collapse"
        );
        assert!(collapsing(&app, root));
    }

    /// Threshold 0 is the old structural-death backstop, unchanged: a ship
    /// whose last section is REMOVED without a damage bubble (a direct destroy,
    /// a detach, a scripted removal) dies instead of lingering as an unmarked
    /// 0-HP hull.
    #[test]
    fn a_ship_that_loses_its_last_section_dies_even_at_a_zero_threshold() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(&mut app, Some(0.0), &[(40.0, 40.0)]);
        app.update();
        assert!(!root_marked_dead(&app, root));

        app.world_mut().entity_mut(sections[0]).despawn();
        app.update();

        assert!(root_marked_dead(&app, root));
    }

    /// The collapse test is true on every frame of the cascade, so a section
    /// that is already disabled must not be marked again - and the root must
    /// not be re-marked either.
    #[test]
    fn a_collapsing_ship_is_not_re_marked_every_frame() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(&mut app, Some(0.25), &[(100.0, 100.0), (900.0, 900.0)]);
        app.update();
        app.world_mut().entity_mut(sections[1]).despawn();
        app.update();
        assert!(collapsing(&app, root));

        // A detector for genuinely new inserts: `On<Add, T>` fires only for a
        // component that was not there, so any count above zero from here is a
        // re-mark.
        #[derive(Resource, Default)]
        struct Remarks(usize);
        app.init_resource::<Remarks>();
        app.add_observer(
            |_: On<Add, IntegrityDisabledMarker>, mut remarks: ResMut<Remarks>| remarks.0 += 1,
        );
        app.add_observer(
            |_: On<Add, StructuralCollapseMarker>, mut remarks: ResMut<Remarks>| remarks.0 += 1,
        );

        for _ in 0..5 {
            app.update();
        }

        assert_eq!(app.world().resource::<Remarks>().0, 0);
    }

    /// THE HAZARD. A remnant with no leaves (a ring, or - as here - sections
    /// whose graph never made any of them a leaf) would disable itself and
    /// stop: disabling costs no health, so `current` never falls and the root
    /// never dies. The no-progress override destroys anyway.
    #[test]
    fn a_collapse_with_no_leaf_to_peel_forces_a_section_apart() {
        let mut app = aggregate_app();
        let (root, sections) = spawn_ship(
            &mut app,
            Some(0.25),
            &[(100.0, 100.0), (100.0, 100.0), (800.0, 800.0)],
        );
        // A triangle: every section keeps two neighbours, so nothing in it is
        // ever a leaf and the ordinary chain has nothing to start on.
        for (section, neighbors) in [
            (sections[0], [sections[1], sections[2]]),
            (sections[1], [sections[0], sections[2]]),
            (sections[2], [sections[0], sections[1]]),
        ] {
            app.world_mut()
                .entity_mut(section)
                .insert(ConnectedTo(neighbors.to_vec()));
        }
        app.update();

        // Under the threshold without losing a section: 200 of a pinned 1000,
        // all three still standing and still mated to each other.
        app.world_mut()
            .entity_mut(sections[2])
            .insert(Health::new(0.0));
        app.update();
        assert!(collapsing(&app, root));
        assert!(
            sections.iter().all(|section| !destroying(&app, *section)),
            "the first tick only disables - the leaf rule gets its chance"
        );

        app.update();
        assert_eq!(
            sections
                .iter()
                .filter(|section| destroying(&app, **section))
                .count(),
            1,
            "a stalled cascade forces ONE section apart, not the whole ring"
        );

        // ...and it keeps going until the ring is gone.
        for _ in 0..6 {
            app.update();
        }
        assert!(
            sections.iter().all(|section| destroying(&app, *section)),
            "the override drains a leafless shape completely"
        );
    }

    /// The birth guard: a root whose sections have not landed yet has no
    /// pinned maximum, so an empty hull is not executed for being empty.
    #[test]
    fn a_mid_spawn_root_with_no_sections_is_not_collapsed_at_birth() {
        let mut app = aggregate_app();
        let root = app
            .world_mut()
            .spawn((IntegrityRoot, SpaceshipRootMarker))
            .id();

        app.update();
        app.update();
        assert!(
            !collapsing(&app, root) && !root_marked_dead(&app, root),
            "an unbuilt ship is not a wreck"
        );

        let section = app
            .world_mut()
            .spawn((SectionMarker, Health::new(100.0)))
            .id();
        app.world_mut().entity_mut(root).add_children(&[section]);
        app.update();

        assert!(
            !collapsing(&app, root) && !root_marked_dead(&app, root),
            "its sections landed late, not never"
        );
        assert_eq!(root_health(&app, root), (100.0, 100.0));
    }

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
    fn a_depleted_non_leaf_section_is_destroyed() {
        let mut app = App::new();
        app.add_observer(on_section_disable);

        let section = app
            .world_mut()
            .spawn((SectionMarker, HealthZeroMarker))
            .id();
        app.world_mut()
            .entity_mut(section)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(
            app.world().get::<IntegrityDestroyMarker>(section).is_some(),
            "graph degree cannot keep a depleted ship section alive"
        );
        assert!(app.world().get::<SectionInactiveMarker>(section).is_none());
    }

    #[test]
    fn the_ship_adapter_leaves_a_depleted_leaf_to_the_generic_core() {
        let mut app = App::new();
        app.add_observer(on_section_disable);

        let section = app
            .world_mut()
            .spawn((SectionMarker, IntegrityLeafMarker, HealthZeroMarker))
            .id();
        app.world_mut()
            .entity_mut(section)
            .insert(IntegrityDisabledMarker);
        app.update();

        assert!(app.world().get::<SectionInactiveMarker>(section).is_none());
        assert!(
            app.world().get::<IntegrityDestroyMarker>(section).is_none(),
            "only the generic leaf observer may queue destruction"
        );
    }
}

/// Physics-level tests for link-point graph publication at Avian's real `ColliderOf` seam.
#[cfg(test)]
mod physics_tests {
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
    fn a_capital_scale_redundant_ring_stays_one_body_after_a_cut() {
        const SECTION_COUNT: usize = 128;
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(ShipIntegrityPlugin);
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
        let sections: Vec<_> = (0..SECTION_COUNT)
            .map(|index| {
                let angle = index as f32 * std::f32::consts::TAU / SECTION_COUNT as f32;
                spawn_section(
                    &mut app,
                    root,
                    Vec3::new(angle.cos(), angle.sin(), 0.0) * 25.0,
                )
            })
            .collect();
        settle(&mut app);
        for (index, section) in sections.iter().copied().enumerate() {
            app.world_mut().entity_mut(section).insert(ConnectedTo(vec![
                sections[(index + SECTION_COUNT - 1) % SECTION_COUNT],
                sections[(index + 1) % SECTION_COUNT],
            ]));
        }

        app.world_mut().trigger(HealthApplyDamage {
            entity: sections[0],
            source: None,
            amount: 100.0,
        });
        for _ in 0..3 {
            app.update();
        }

        assert!(!app.world().entities().contains(sections[0]));
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<ShipWreckFragmentMarker>>()
                .iter(app.world())
                .count(),
            0,
            "a redundant cut must not invent a wreck body"
        );
        for section in sections.into_iter().skip(1) {
            assert_eq!(app.world().get::<ColliderOf>(section).unwrap().body, root);
        }
    }

    #[test]
    fn destroying_an_interior_bridge_severs_a_physical_wreck() {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(ShipIntegrityPlugin);
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.finish();

        let old_linear = Vec3::new(5.0, 0.0, 0.0);
        let old_angular = Vec3::new(0.0, 0.0, 2.0);
        let root = app
            .world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                LinearVelocity(old_linear),
                AngularVelocity(old_angular),
            ))
            .id();
        let left = spawn_section(&mut app, root, Vec3::ZERO);
        app.world_mut()
            .entity_mut(left)
            .insert(ControllerSectionMarker);
        let bridge = spawn_section(&mut app, root, Vec3::X);
        let right = spawn_section(&mut app, root, Vec3::X * 2.0);
        let second_bridge = spawn_section(&mut app, root, Vec3::X * 3.0);
        let rear = spawn_section(&mut app, root, Vec3::X * 4.0);
        settle(&mut app);
        let old_com = app.world().get::<Position>(root).unwrap().0
            + app.world().get::<Rotation>(root).unwrap().0
                * app.world().get::<ComputedCenterOfMass>(root).unwrap().0;

        app.world_mut().trigger(HealthApplyDamage {
            entity: bridge,
            source: None,
            amount: 100.0,
        });
        app.update();
        // A dead section detaches as a physical body of its own. Remove that
        // separate wreckage before the next fixed pass so this test isolates
        // the sever motion contract itself.
        let debris: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<DetachedPieceMarker>>()
            .iter(app.world())
            .collect();
        for entity in debris {
            app.world_mut().entity_mut(entity).despawn();
        }
        app.update();

        assert!(
            !app.world().entities().contains(bridge),
            "the depleted non-leaf bridge must disappear"
        );
        let fragments: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<ShipWreckFragmentMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(fragments.len(), 1, "one detached component makes one wreck");
        let fragment = fragments[0];
        assert_eq!(
            app.world().get::<ColliderOf>(left).unwrap().body,
            root,
            "the controller component keeps ship identity"
        );
        assert_eq!(
            app.world().get::<ColliderOf>(right).unwrap().body,
            fragment,
            "Avian must assign the detached collider to its new body"
        );
        assert!(
            app.world().get::<SectionInactiveMarker>(right).is_some(),
            "a wreck section is inert"
        );
        assert!(
            app.world()
                .get::<Health>(right)
                .is_some_and(|health| health.current > 0.0),
            "an inert wreck section remains healthy and damageable"
        );

        let left_velocity = **app.world().get::<LinearVelocity>(root).unwrap();
        let right_velocity = **app.world().get::<LinearVelocity>(fragment).unwrap();
        let left_com = app.world().get::<Position>(root).unwrap().0
            + app.world().get::<Rotation>(root).unwrap().0
                * app.world().get::<ComputedCenterOfMass>(root).unwrap().0;
        let right_com = app.world().get::<Position>(fragment).unwrap().0
            + app.world().get::<Rotation>(fragment).unwrap().0
                * app.world().get::<ComputedCenterOfMass>(fragment).unwrap().0;
        let expected_relative_rotation = old_angular.cross(right_com - left_com);
        let fracture_relative = right_velocity - left_velocity - expected_relative_rotation;
        assert!(
            (fracture_relative.length() - 2.0 * SEVER_SEPARATION_SPEED).abs() < 0.2,
            "each side receives the accepted 1 u/s kick: {fracture_relative:?}"
        );
        let left_mass = app.world().get::<ComputedMass>(root).unwrap().value();
        let right_mass = app.world().get::<ComputedMass>(fragment).unwrap().value();
        let balanced =
            (left_velocity * left_mass + right_velocity * right_mass) / (left_mass + right_mass);
        let surviving_com =
            (left_com * left_mass + right_com * right_mass) / (left_mass + right_mass);
        let pre_kick_survivor_velocity = old_linear + old_angular.cross(surviving_com - old_com);
        assert!(
            (balanced - pre_kick_survivor_velocity).length() < 0.2,
            "the kick must preserve survivor momentum: balanced={balanced:?}, expected={pre_kick_survivor_velocity:?}, masses=({left_mass}, {right_mass}), velocities=({left_velocity:?}, {right_velocity:?})"
        );

        // A persistent wreck uses the same partition path on a later hit.
        app.world_mut().trigger(HealthApplyDamage {
            entity: second_bridge,
            source: None,
            amount: 100.0,
        });
        app.update();
        let debris: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<DetachedPieceMarker>>()
            .iter(app.world())
            .collect();
        for entity in debris {
            app.world_mut().entity_mut(entity).despawn();
        }
        for _ in 0..3 {
            app.update();
        }

        let fragment_roots: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<ShipWreckFragmentMarker>>()
            .iter(app.world())
            .collect();
        assert_eq!(fragment_roots.len(), 2, "a wreck can sever again");
        let right_body = app.world().get::<ColliderOf>(right).unwrap().body;
        let rear_body = app.world().get::<ColliderOf>(rear).unwrap().body;
        assert_ne!(right_body, rear_body, "the cut made two wreck bodies");

        // Removing the only section from the new wreck also removes its
        // otherwise-healthless root. Stable tie resolution decides which side
        // retained the first wreck identity.
        let (single_section, empty_root) = if right_body == fragment {
            (rear, rear_body)
        } else {
            (right, right_body)
        };
        app.world_mut().trigger(HealthApplyDamage {
            entity: single_section,
            source: None,
            amount: 100.0,
        });
        for _ in 0..3 {
            app.update();
        }
        assert!(
            !app.world().entities().contains(empty_root),
            "an empty wreck root must not persist"
        );
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
    use bevy_rand::prelude::*;
    use nova_events::prelude::{EntityTypeName, GameEvent, SPACESHIP_TYPE_NAME};
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;

    /// Records lifecycle events so unified defeat and physical destruction remain distinct.
    ///
    /// Only the ROOT of these rigs carries an `EntityId`/`EntityTypeName`, so
    /// these counts are the SHIP's own events. Real scenario sections carry
    /// ids too and fire an `ondestroyed` of their own as they come apart,
    /// exactly as they already do when a ship is dismantled by gunfire.
    #[derive(Resource, Default)]
    struct FiredEvents(Vec<&'static str>);

    /// Every entity that reached [`IntegrityDestroyMarker`], recorded at the
    /// insert - the sections themselves are despawned in the same command
    /// flush, so there is nothing left to inspect afterwards.
    #[derive(Resource, Default)]
    struct Destroyed(Vec<Entity>);

    fn ghost_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins(ShipIntegrityPlugin);
        // The destroy path's debris observers need material assets and the
        // global rng even in a headless run.
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.init_resource::<FiredEvents>();
        app.init_resource::<Destroyed>();
        app.add_observer(|event: On<GameEvent>, mut fired: ResMut<FiredEvents>| {
            fired.0.push(event.name());
        });
        app.add_observer(
            |add: On<Add, IntegrityDestroyMarker>, mut destroyed: ResMut<Destroyed>| {
                destroyed.0.push(add.entity);
            },
        );
        app.finish();
        app
    }

    fn count_events(app: &App, name: &str) -> usize {
        app.world()
            .resource::<FiredEvents>()
            .0
            .iter()
            .filter(|fired| **fired == name)
            .count()
    }

    fn destroy_events(app: &App) -> usize {
        count_events(app, "ondestroyed")
    }

    fn defeat_events(app: &App) -> usize {
        count_events(app, "ondefeated")
    }

    fn was_destroyed(app: &App, entity: Entity) -> bool {
        app.world().resource::<Destroyed>().0.contains(&entity)
    }

    fn destroyed_count(app: &App) -> usize {
        app.world().resource::<Destroyed>().0.len()
    }

    fn spawn_root(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                Transform::default(),
                SpaceshipRootMarker,
                EntityId::new("rig_ship"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
            ))
            .id()
    }

    fn spawn_section_at(app: &mut App, root: Entity, at: Vec3) -> Entity {
        app.world_mut()
            .spawn((
                ChildOf(root),
                SectionMarker,
                Transform::from_translation(at),
                SectionLinkPoints(unit_cube_link_points()),
                ConnectedTo::default(),
                Collider::cuboid(1.0, 1.0, 1.0),
                ColliderDensity(1.0),
                Health::new(100.0),
            ))
            .id()
    }

    fn spawn_ship(app: &mut App, section_count: usize) -> (Entity, Vec<Entity>) {
        let root = spawn_root(app);
        let sections = (0..section_count)
            .map(|i| spawn_section_at(app, root, Vec3::X * i as f32))
            .collect();
        settle(app);
        (root, sections)
    }

    /// Four unit cubes mated in a square: every section keeps two neighbours,
    /// so the graph has NO leaf and the ordinary disable -> destroy chain has
    /// nothing to start on.
    fn spawn_ring_ship(app: &mut App) -> (Entity, Vec<Entity>) {
        let root = spawn_root(app);
        let sections = [Vec3::ZERO, Vec3::X, Vec3::X + Vec3::Y, Vec3::Y]
            .into_iter()
            .map(|at| spawn_section_at(app, root, at))
            .collect::<Vec<_>>();
        settle(app);
        for section in &sections {
            assert_eq!(
                app.world().get::<ConnectedTo>(*section).unwrap().len(),
                2,
                "delivery guard: the rig must really be a ring"
            );
            assert!(
                app.world().get::<IntegrityLeafMarker>(*section).is_none(),
                "delivery guard: a ring has no leaves"
            );
        }
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

    /// Structural collapse end to end: a ship down to a twentieth of the hull
    /// it was built with dies with that last section still ALIVE, through the same
    /// disable -> destroy chain (one OnDestroyed, no second death path).
    ///
    /// The section still alive at the collapse is DESTROYED - it bursts its
    /// debris like any other - rather than being despawned in silence with the
    /// root, which is what it used to do.
    #[test]
    fn a_ship_below_its_collapse_threshold_dies_with_a_section_still_alive() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 20);

        for section in &sections[..18] {
            hit(&mut app, *section, 100.0);
            for _ in 0..5 {
                app.update();
            }
        }
        assert!(
            app.world().entities().contains(root),
            "a tenth of a hull still flies"
        );

        hit(&mut app, sections[18], 100.0);

        assert!(
            root_dead(&mut app, root, 10),
            "five percent of a hull is wreckage, not a ship"
        );
        assert!(
            !app.world().entities().contains(sections[19]),
            "the last living section goes with the hull it hung from"
        );
        assert!(
            was_destroyed(&app, sections[19]),
            "it is DESTROYED on the way out, not quietly despawned - that is \
             what makes it burst"
        );
        assert_eq!(destroy_events(&app), 1, "exactly one OnDestroyed");
        assert_eq!(defeat_events(&app), 1, "exactly one OnDefeated");
    }

    /// The peel is progressive: a chain-shaped remnant comes apart from both
    /// ends inward over several frames, not all at once in the collapse frame.
    #[test]
    fn a_collapsing_ship_peels_apart_over_several_frames() {
        let mut app = ghost_app();
        // Six sections in a line, each shot down to four percent: 24 of a pinned
        // 600 is under the default threshold with the whole chain still
        // standing, so the remnant that collapses is the entire ship.
        let (root, sections) = spawn_ship(&mut app, 6);
        for section in &sections {
            hit(&mut app, *section, 96.0);
        }

        let mut destroyed_per_frame = Vec::new();
        let mut seen = destroyed_count(&app);
        for _ in 0..12 {
            app.update();
            let total = destroyed_count(&app);
            destroyed_per_frame.push(total - seen);
            seen = total;
        }

        assert!(
            !app.world().entities().contains(root),
            "the peel has to finish the ship off"
        );
        for section in &sections {
            assert!(was_destroyed(&app, *section), "every section bursts");
        }
        let peeling_frames = destroyed_per_frame
            .iter()
            .filter(|destroyed| **destroyed > 0)
            .count();
        assert!(
            peeling_frames > 1,
            "a chain must come apart over several frames, not vanish in one: \
             {destroyed_per_frame:?}"
        );
        assert!(
            destroyed_per_frame.iter().all(|destroyed| *destroyed < 6),
            "no single frame may take the whole chain: {destroyed_per_frame:?}"
        );
    }

    /// THE HAZARD, the control half. Disabling every section of a RING is not
    /// enough on its own: nothing in it is ever a leaf, so the ordinary chain
    /// destroys nothing, no section leaves the root's sum, and the ship would
    /// hang there as an immortal disabled hulk. Same app, same rig as the test
    /// below - the only difference is that this ship never crosses its
    /// threshold, so the collapse cascade (and its no-progress override) never
    /// runs on it.
    #[test]
    fn the_leaf_rule_alone_cannot_take_a_ring_apart() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ring_ship(&mut app);
        for section in &sections {
            app.world_mut()
                .entity_mut(*section)
                .insert(IntegrityDisabledMarker);
        }

        for _ in 0..20 {
            app.update();
        }

        assert_eq!(
            destroyed_count(&app),
            0,
            "with no leaf to start on, the chain reaction never starts"
        );
        for section in &sections {
            assert!(
                app.world().entities().contains(*section),
                "every section of the ring is still standing"
            );
        }
        assert!(
            app.world().entities().contains(root),
            "and the root outlives them all - the hulk the override exists for"
        );
    }

    /// THE HAZARD, the real half: a ring-shaped remnant that DOES collapse is
    /// drained completely by the no-progress override and still fires its
    /// events exactly once.
    #[test]
    fn a_ring_shaped_remnant_still_collapses_completely() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ring_ship(&mut app);
        // 16 of a pinned 400 with all four sections alive and still mated.
        for section in &sections {
            hit(&mut app, *section, 96.0);
        }

        assert!(
            root_dead(&mut app, root, 20),
            "a leafless wreck must still die"
        );
        for section in &sections {
            assert!(
                was_destroyed(&app, *section),
                "every section of the ring is destroyed, none left behind"
            );
        }
        assert_eq!(defeat_events(&app), 1, "exactly one OnDefeated");
        assert_eq!(destroy_events(&app), 1, "exactly one OnDestroyed");
    }

    /// A ship whose sections are disabled progressively can still be a
    /// combatant for a few frames, so the neutralize path fires the unified
    /// defeat edge BEFORE the root dies. It must not fire a second time when
    /// it does.
    #[test]
    fn an_armed_ship_that_collapses_is_defeated_exactly_once() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 4);
        app.world_mut()
            .entity_mut(sections[1])
            .insert(TurretSectionMarker);
        for _ in 0..3 {
            app.update();
        }

        // 16 of a pinned 400, gun included: the ship collapses while it is
        // still armed.
        for section in &sections {
            hit(&mut app, *section, 96.0);
        }

        assert!(root_dead(&mut app, root, 20));
        assert_eq!(
            defeat_events(&app),
            1,
            "neutralization and destruction are the same defeat, counted once"
        );
        assert_eq!(destroy_events(&app), 1, "exactly one OnDestroyed");
    }

    /// A ship taken apart the ordinary way, never crossing its threshold, is
    /// untouched by any of this: healthy sections are not disabled behind its
    /// back, and it still dies exactly once when the last one goes.
    #[test]
    fn a_ship_dismantled_without_collapsing_is_unaffected() {
        let mut app = ghost_app();
        let (root, sections) = spawn_ship(&mut app, 3);
        app.world_mut()
            .entity_mut(root)
            .insert(StructuralCollapseThreshold::new(0.0));
        settle(&mut app);

        hit(&mut app, sections[0], 100.0);
        for _ in 0..5 {
            app.update();
        }

        assert!(
            app.world().get::<StructuralCollapseMarker>(root).is_none(),
            "two thirds of a hull is not a collapse at threshold 0"
        );
        for section in &sections[1..] {
            assert!(
                app.world()
                    .get::<IntegrityDisabledMarker>(*section)
                    .is_none(),
                "an undamaged section keeps working"
            );
            assert!(
                app.world().get::<Health>(*section).unwrap().current > 99.0,
                "and keeps its health"
            );
        }

        hit(&mut app, sections[1], 100.0);
        for _ in 0..5 {
            app.update();
        }
        hit(&mut app, sections[2], 100.0);

        assert!(root_dead(&mut app, root, 10));
        assert_eq!(destroy_events(&app), 1, "exactly one OnDestroyed");
        assert_eq!(defeat_events(&app), 1, "exactly one OnDefeated");
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
