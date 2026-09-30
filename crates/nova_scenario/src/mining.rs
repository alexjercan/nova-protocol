//! Mining: a ship's mining sections cut ore out of its travel-locked asteroid
//! and drop it as cargo canisters.
//!
//! Each live mining section pulses on its own clock while its emitter is fully
//! deployed ([`MiningEmitter::is_deployed`]): at once, then every authored
//! `pulse_interval_seconds` of game time. Its checks run in
//! [`MiningRefusalType`] order. The ship's travel lock must be an asteroid
//! whose kind yields ore ([`ore_for_asteroid_kind`]). The rock's collider must
//! be within the section's `reach` of the emitter face, and the ray straight
//! out of that face ([`mining_emitter_face`]) must meet it within `reach`. The
//! ray is cast against the locked rock alone, so nothing else stops or takes
//! the beam. A pulse takes a sphere of `carve_radius_cells` field cells out of
//! the rock's [`AsteroidField`] where the beam meets solid material, and owes
//! one ore item per corner that sphere flipped from solid to empty. A corner a
//! hit or an earlier pulse already emptied flips nothing, so a combat carve
//! never yields ore and a pulse into an empty crater yields none. A refused
//! pulse changes nothing.
//!
//! While a deployed emitter's checks pass, [`MiningBeamHit`] holds where its
//! beam meets the rock, and a rendered app draws the beam to that point.
//!
//! An untouched rock has no field. The first pulse asks for one with
//! [`AsteroidFieldSeedRequest`] and takes nothing; later pulses wait while
//! the field seeds or while a remesh is in flight, so no pulse writes a field
//! that a finished remesh would replace.
//!
//! Owed ore stays on the rock node as [`MinedOre`] until
//! [`AsteroidRemeshed`] says the carve is drawn and collided with. Then it
//! becomes whole canisters of at most `CARGO_CANISTER_MAX_MASS_G`, queued
//! first in, first out in a [`MinedCanisterQueue`] on the same node. The queue
//! drops one canister at a time, one engine unit off the mined surface and
//! drifting away from it, and only while a canister-sized box there touches no
//! collider and no other canister. A rock that runs out of material hands its
//! queue to a drop entity at its last pose, which drains the same way.
//!
//! Nothing here is saved. Owed ore and queued canisters belong to the rock, or
//! to its drop, and a sector retirement or scenario unload discards them with
//! it; a retired rock comes back pristine.

use std::collections::{HashMap, VecDeque};

use avian3d::prelude::{
    AngularVelocity, Collider, LinearVelocity, SpatialQuery, SpatialQueryFilter,
};
use bevy::{light::NotShadowCaster, prelude::*};
use nova_gameplay::prelude::*;
use nova_ship::prelude::*;

use crate::{
    loader::prelude::ScenarioScopedMarker,
    objects::{
        asteroid::AsteroidMarker,
        asteroid_carve::{
            AsteroidField, AsteroidFieldSeedRequest, AsteroidFieldSeeding, AsteroidRemesh,
            AsteroidRemeshed,
        },
        asteroid_kind::{
            AsteroidKind, AsteroidKindId, KIND_CARBON, KIND_ICE, KIND_METAL, KIND_ROCK,
        },
    },
};

/// `MiningPlugin`, its system set, the ore rule, the pulse event, the beam
/// hit and the mined canister model.
pub mod prelude {
    pub use super::{
        ore_for_asteroid_kind, MinedCanisterQueue, MinedOre, MiningBeamHit, MiningPlugin,
        MiningPulse, MiningRefusalType, MiningSystems, MINED_CANISTER_MESH,
    };
}

/// The one generic canister model every mined canister wears. A path under
/// the asset root, as the base bundle's `self://gltf/cargo_canister_cuboid.glb`
/// resolves; a failed load panics.
pub const MINED_CANISTER_MESH: &str = "base/gltf/cargo_canister_cuboid.glb#Scene0";

/// How far off the mined surface a canister is born, in engine units.
const MINED_CANISTER_OFFSET: f32 = 1.0;

/// How fast a new canister drifts off the rock, in engine units per second,
/// on top of the rock's own motion at the birth point.
const MINED_CANISTER_SPEED: f32 = 0.2;

/// The drawn beam's radius, in engine units: about the emitter lens. Art only.
const MINING_BEAM_RADIUS: f32 = 0.04;

/// The ore a pulse into an asteroid of `kind` yields, or `None` for a kind
/// that holds none, such as `plain`, the unshaded control rock.
pub fn ore_for_asteroid_kind(kind: &AsteroidKindId) -> Option<ItemType> {
    match kind.as_str() {
        KIND_ROCK => Some(ItemType::StoneOre),
        KIND_METAL => Some(ItemType::IronOre),
        KIND_ICE => Some(ItemType::WaterIce),
        KIND_CARBON => Some(ItemType::CarbonOre),
        _ => None,
    }
}

/// Why a pulse takes nothing, in check order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiningRefusalType {
    /// The emitter's ship has no travel lock.
    NoLock,
    /// The travel lock is not an asteroid with a field node.
    NotAsteroid,
    /// The asteroid's kind yields no ore.
    Barren,
    /// The rock's collider is farther than the section's `reach` from the
    /// emitter face.
    OutOfReach,
    /// The rock is within reach, but the ray out of the emitter face misses
    /// it within reach.
    OffTarget,
}

/// One pulse of one mining section's beam, triggered on that section.
///
/// `Ok` carries the corners the pulse flipped: the ore it owes, paid once
/// the rock's remesh lands. It is zero while the rock's field seeds or
/// remeshes, and when the beam met no material.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct MiningPulse {
    /// The mining section that pulsed.
    pub entity: Entity,
    /// What the pulse did.
    pub outcome: Result<u32, MiningRefusalType>,
}

/// Where a deployed emitter's beam meets its ship's locked ore rock, in world
/// space. On the mining section while every pulse check passes; absent
/// otherwise.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MiningBeamHit {
    /// The point on the rock's collider the beam reaches.
    pub at: Vec3,
}

/// Ore a rock node owes for corners mined since its last validated remesh.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MinedOre {
    /// The ore the rock's kind yields.
    item: ItemType,
    /// Corners flipped, one item each.
    corners: u32,
    /// The last pulse's surface point, in the node's local frame.
    at: Vec3,
    /// The outward direction there, in the node's local rotation.
    normal: Vec3,
}

impl MinedOre {
    /// Add `later`'s corners and take its surface point.
    fn absorb(&mut self, later: MinedOre) {
        self.corners += later.corners;
        self.at = later.at;
        self.normal = later.normal;
    }
}

/// Canisters waiting to be born off a mined rock, first in, first out. On a
/// rock node, or on the drop entity a rock that ran out of material leaves.
/// Never empty: the ejector removes the queue with its last canister.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MinedCanisterQueue {
    waiting: VecDeque<MinedCanister>,
}

impl MinedCanisterQueue {
    /// The canisters still waiting, front first.
    pub fn canisters(&self) -> impl Iterator<Item = &CargoCanister> {
        self.waiting.iter().map(|waiting| &waiting.canister)
    }
}

/// One waiting canister and where it leaves from, in its owner's frame.
#[derive(Clone, Debug, PartialEq)]
struct MinedCanister {
    canister: CargoCanister,
    at: Vec3,
    normal: Vec3,
}

/// Seconds until a deployed emitter's next pulse; on the mining section while
/// it is deployed.
#[derive(Component, Clone, Copy, Debug)]
struct MiningPulseClock(f32);

/// Marks the entity a rock that ran out of material hands its queue to. It
/// carries the rock's last pose and despawns once the queue drains.
#[derive(Component, Clone, Copy, Debug)]
struct MinedOreDrop;

/// Marks the drawn beam, a child of its mining section.
#[derive(Component, Clone, Copy, Debug)]
struct MiningBeamLaser;

/// The queries the pulse reads a locked rock through.
type RockQuery<'w, 's> =
    Query<'w, 's, (&'static AsteroidKind, &'static Children), With<AsteroidMarker>>;
type RockNodeQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Collider,
        &'static GlobalTransform,
        Option<&'static mut AsteroidField>,
        Option<&'static MinedOre>,
        Has<AsteroidRemesh>,
        Has<AsteroidFieldSeeding>,
    ),
    With<DamageMarks>,
>;

/// Where a beam meets an ore rock.
struct BeamAim {
    /// The rock's field node.
    node: Entity,
    /// The ore the rock yields.
    item: ItemType,
    /// Where the ray meets the rock's collider, in world space.
    at: Vec3,
    /// The collider's outward normal there.
    normal: Vec3,
}

/// Aim every deployed emitter, publish its [`MiningBeamHit`], and pulse the
/// ones whose clock is due. Owed ore from several pulses into one rock in one
/// frame is merged before it is written.
#[expect(
    clippy::type_complexity,
    reason = "each emitter with its config, pose, clock and hit"
)]
fn pulse_mining_beams(
    time: Res<Time>,
    mut commands: Commands,
    mut q_emitters: Query<
        (
            Entity,
            &ChildOf,
            &MiningEmitter,
            &MiningSectionConfigHelper,
            &SectionCollider,
            &GlobalTransform,
            Option<&mut MiningPulseClock>,
            Option<&MiningBeamHit>,
        ),
        (With<MiningSectionMarker>, Without<SectionInactiveMarker>),
    >,
    q_inactive: Query<
        Entity,
        (
            With<SectionInactiveMarker>,
            Or<(With<MiningPulseClock>, With<MiningBeamHit>)>,
        ),
    >,
    q_locks: Query<&TravelLock>,
    q_rocks: RockQuery,
    mut q_nodes: RockNodeQuery,
) {
    for section in &q_inactive {
        commands
            .entity(section)
            .remove::<(MiningPulseClock, MiningBeamHit)>();
    }
    let mut owed: HashMap<Entity, MinedOre> = HashMap::new();
    for (section, &ChildOf(ship), emitter, config, collider, frame, clock, hit) in &mut q_emitters {
        if !emitter.is_deployed() {
            if clock.is_some() || hit.is_some() {
                commands
                    .entity(section)
                    .remove::<(MiningPulseClock, MiningBeamHit)>();
            }
            continue;
        }
        let (_, rotation, position) = frame.to_scale_rotation_translation();
        let (origin, direction) = mining_emitter_face(position, rotation, *collider);
        let aim = aim_beam(
            q_locks.get(ship).ok(),
            origin,
            direction,
            config.reach.to_engine(),
            &q_rocks,
            &q_nodes,
        );
        match &aim {
            Ok(aim) => {
                let now = MiningBeamHit { at: aim.at };
                if hit != Some(&now) {
                    commands.entity(section).insert(now);
                }
            }
            Err(_) => {
                if hit.is_some() {
                    commands.entity(section).remove::<MiningBeamHit>();
                }
            }
        }

        let interval = config.pulse_interval_seconds;
        let due = match clock {
            Some(mut clock) => {
                clock.0 -= time.delta_secs();
                let due = clock.0 <= 0.0;
                if due {
                    clock.0 += interval;
                }
                due
            }
            None => {
                commands.entity(section).insert(MiningPulseClock(interval));
                true
            }
        };
        if !due {
            continue;
        }
        let outcome = aim.map(|aim| {
            carve(
                &aim,
                direction,
                config.carve_radius_cells,
                &mut commands,
                &mut q_nodes,
                &mut owed,
            )
        });
        match outcome {
            Ok(corners) => debug!("pulse_mining_beams: {section:?} flipped {corners} corner(s)"),
            Err(refusal) => info!("pulse_mining_beams: {section:?} refused: {refusal:?}"),
        }
        commands.trigger(MiningPulse {
            entity: section,
            outcome,
        });
    }
    for (node, ore) in owed {
        commands.entity(node).insert(ore);
    }
}

/// The checks of [`MiningRefusalType`] in order, for a beam leaving `origin`
/// along `direction` and reaching `reach` engine units.
fn aim_beam(
    lock: Option<&TravelLock>,
    origin: Vec3,
    direction: Vec3,
    reach: f32,
    q_rocks: &RockQuery,
    q_nodes: &RockNodeQuery,
) -> Result<BeamAim, MiningRefusalType> {
    let rock = lock
        .and_then(|lock| lock.0)
        .ok_or(MiningRefusalType::NoLock)?;
    let (kind, children) = q_rocks
        .get(rock)
        .map_err(|_| MiningRefusalType::NotAsteroid)?;
    let node = children
        .iter()
        .find(|child| q_nodes.contains(*child))
        .ok_or(MiningRefusalType::NotAsteroid)?;
    let item = ore_for_asteroid_kind(kind).ok_or(MiningRefusalType::Barren)?;
    let (rock_collider, frame, ..) = q_nodes.get(node).expect("the node was found in this query");
    let (_, rock_rotation, rock_position) = frame.to_scale_rotation_translation();
    if rock_collider.distance_to_point(rock_position, rock_rotation, origin, true) > reach {
        return Err(MiningRefusalType::OutOfReach);
    }
    let (distance, normal) = rock_collider
        .cast_ray(rock_position, rock_rotation, origin, direction, reach, true)
        .ok_or(MiningRefusalType::OffTarget)?;
    // A face already inside the rock meets it at distance zero, where the
    // collider reports no normal: the beam's own back-direction stands in.
    let normal = normal.try_normalize().unwrap_or(-direction);
    Ok(BeamAim {
        node,
        item,
        at: origin + direction * distance,
        normal,
    })
}

/// Carve one pulse into the rock `aim` meets: ask for a field if it has none,
/// wait out a seed or a remesh, else march a sphere of `radius_cells` along
/// the beam `direction` until it flips solid corners. Returns the corners
/// flipped and owes them in `owed`.
fn carve(
    aim: &BeamAim,
    direction: Vec3,
    radius_cells: f32,
    commands: &mut Commands,
    q_nodes: &mut RockNodeQuery,
    owed: &mut HashMap<Entity, MinedOre>,
) -> u32 {
    let (_, frame, field, mined, remeshing, seeding) = q_nodes
        .get_mut(aim.node)
        .expect("the node was found in this query");
    let Some(mut field) = field else {
        if !seeding {
            commands.entity(aim.node).insert(AsteroidFieldSeedRequest);
        }
        return 0;
    };
    if remeshing {
        return 0;
    }

    let (_, rock_rotation, _) = frame.to_scale_rotation_translation();
    let to_local = frame.affine().inverse();
    let at = to_local.transform_point3(aim.at);
    let local_normal = (rock_rotation.inverse() * aim.normal).normalize_or_zero();
    let local_direction = (rock_rotation.inverse() * direction).normalize_or_zero();
    let cell = field.solid().cell_size();
    let radius = radius_cells * cell;
    // The collider a pristine rock carries is the convex hull of its surface,
    // so the hit can stand over a hollow. March the sphere along the beam,
    // one cell a step, until it meets solid material. An empty step flips
    // nothing and removes nothing.
    let steps = (2.0 * field.solid().half_extent() / cell).ceil() as u32;
    let corners = (0..=steps)
        .map(|step| at + local_direction * (step as f32 * cell))
        .find_map(|centre| {
            let flipped = field.mine(centre, radius);
            (flipped > 0).then_some(flipped)
        })
        .unwrap_or(0);
    if corners == 0 {
        return 0;
    }
    let pulse = MinedOre {
        item: aim.item,
        corners,
        at,
        normal: local_normal,
    };
    match owed.get_mut(&aim.node) {
        Some(pending) => pending.absorb(pulse),
        None => {
            let mut total = mined.copied().unwrap_or(MinedOre {
                corners: 0,
                ..pulse
            });
            total.absorb(pulse);
            owed.insert(aim.node, total);
        }
    }
    commands.trigger(CarveSpew {
        entity: aim.node,
        at: aim.at,
        radius: radius * frame.scale().max_element(),
        kind: DamageType::Kinetic,
    });
    corners
}

/// Give every new live emitter its drawn beam, hidden.
fn insert_mining_beam_laser(
    add: On<Add, MiningEmitter>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut art: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    let (mesh, material) = art
        .get_or_insert_with(|| {
            (
                meshes.add(Cylinder::new(MINING_BEAM_RADIUS, 1.0)),
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.15, 0.85, 0.5),
                    emissive: LinearRgba::rgb(0.6, 3.4, 2.0),
                    unlit: true,
                    ..default()
                }),
            )
        })
        .clone();
    commands.spawn((
        Name::new("Mining Beam"),
        MiningBeamLaser,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        NotShadowCaster,
        Transform::default(),
        Visibility::Hidden,
        ChildOf(add.entity),
    ));
}

/// Stretch each drawn beam from its emitter face to its [`MiningBeamHit`], or
/// hide it when the section has none.
fn draw_mining_beams(
    mut q_lasers: Query<(&ChildOf, &mut Transform, &mut Visibility), With<MiningBeamLaser>>,
    q_sections: Query<(&GlobalTransform, &SectionCollider, Option<&MiningBeamHit>)>,
) {
    for (&ChildOf(section), mut transform, mut visibility) in &mut q_lasers {
        let Ok((frame, collider, hit)) = q_sections.get(section) else {
            continue;
        };
        let Some(hit) = hit else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let (face, _) = mining_emitter_face(Vec3::ZERO, Quat::IDENTITY, *collider);
        let target = frame.affine().inverse().transform_point3(hit.at);
        let Some(direction) = (target - face).try_normalize() else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let length = face.distance(target);
        *transform = Transform {
            translation: (face + target) * 0.5,
            rotation: Quat::from_rotation_arc(Vec3::Y, direction),
            scale: Vec3::new(1.0, length, 1.0),
        };
        visibility.set_if_neq(Visibility::Inherited);
    }
}

/// Turn the ore a rock owes into queued canisters once its carve is drawn and
/// collided with; hand an exhausted rock's queue to a drop at its last pose.
fn release_mined_ore(
    remeshed: On<AsteroidRemeshed>,
    mut commands: Commands,
    mut q_nodes: Query<(
        &GlobalTransform,
        &ChildOf,
        Option<&MinedOre>,
        Option<&mut MinedCanisterQueue>,
    )>,
    q_parents: Query<&ChildOf>,
    q_frames: Query<&GlobalTransform>,
) {
    let node = remeshed.entity;
    let Ok((frame, &ChildOf(root), owed, queue)) = q_nodes.get_mut(node) else {
        return;
    };
    let mut waiting = queue.map_or_else(VecDeque::new, |queue| queue.waiting.clone());
    if let Some(owed) = owed {
        let per_canister = CARGO_CANISTER_MAX_MASS_G / owed.item.mass_g();
        let mut rest = owed.corners;
        while rest > 0 {
            let count = rest.min(per_canister);
            waiting.push_back(MinedCanister {
                canister: CargoCanister::new(owed.item, count),
                at: owed.at,
                normal: owed.normal,
            });
            rest -= count;
        }
        commands.entity(node).remove::<MinedOre>();
    }
    if waiting.is_empty() {
        return;
    }
    let queue = MinedCanisterQueue { waiting };
    if !remeshed.exhausted {
        commands.entity(node).insert(queue);
        return;
    }
    // The rock is despawned after this: its queue moves to a drop that keeps
    // the rock's last pose, under the rock's own parent so a sector retirement
    // still takes it.
    let parent = q_parents.get(root).ok().map(|&ChildOf(parent)| parent);
    let local = match parent.and_then(|parent| q_frames.get(parent).ok()) {
        Some(parent_frame) => frame.reparented_to(parent_frame),
        None => frame.compute_transform(),
    };
    let mut drop = commands.spawn((
        Name::new("Mined Ore Drop"),
        MinedOreDrop,
        ScenarioScopedMarker,
        queue,
        local,
        *frame,
    ));
    if let Some(parent) = parent {
        drop.insert(ChildOf(parent));
    }
}

/// Drop the front canister of every queue whose birth point is clear: no
/// collider inside a canister-sized box there and no canister centre within a
/// canister diagonal of it. One canister per queue per frame; a blocked queue
/// waits.
#[expect(
    clippy::type_complexity,
    reason = "each queue with its owner's pose and the rock's motion"
)]
fn eject_mined_canisters(
    mut commands: Commands,
    spatial: SpatialQuery,
    mut q_queues: Query<(
        Entity,
        &mut MinedCanisterQueue,
        &GlobalTransform,
        Has<MinedOreDrop>,
        Option<&ChildOf>,
    )>,
    q_motion: Query<(&GlobalTransform, &LinearVelocity, &AngularVelocity)>,
    q_canisters: Query<&GlobalTransform, With<CargoCanister>>,
) {
    let size = CARGO_CANISTER_SIZE;
    let shape = Collider::cuboid(size.x, size.y, size.z);
    let mut born: Vec<Vec3> = Vec::new();
    for (owner, mut queue, frame, drop, child_of) in &mut q_queues {
        let front = queue
            .waiting
            .front()
            .expect("a MinedCanisterQueue is never empty");
        let (_, rotation, _) = frame.to_scale_rotation_translation();
        let normal = rotation * front.normal;
        let birth = frame.transform_point(front.at) + normal * MINED_CANISTER_OFFSET;
        let facing = Quat::from_rotation_arc(Vec3::X, normal);
        let crowded = q_canisters
            .iter()
            .map(GlobalTransform::translation)
            .chain(born.iter().copied())
            .any(|centre| centre.distance(birth) < size.length());
        if crowded
            || !spatial
                .shape_intersections(&shape, birth, facing, &SpatialQueryFilter::default())
                .is_empty()
        {
            continue;
        }
        let rock_velocity = match (drop, child_of) {
            (false, Some(&ChildOf(root))) => {
                q_motion
                    .get(root)
                    .map_or(Vec3::ZERO, |(pose, linear, angular)| {
                        rigid_body_point_velocity(linear.0, angular.0, pose.translation(), birth)
                    })
            }
            _ => Vec3::ZERO,
        };
        let MinedCanister { canister, .. } =
            queue.waiting.pop_front().expect("the front was read above");
        commands.spawn((
            cargo_canister(
                canister,
                Transform::from_translation(birth).with_rotation(facing),
                rock_velocity + normal * MINED_CANISTER_SPEED,
                AssetRef::from(MINED_CANISTER_MESH),
            ),
            ScenarioScopedMarker,
        ));
        born.push(birth);
        if queue.waiting.is_empty() {
            if drop {
                commands.entity(owner).despawn();
            } else {
                commands.entity(owner).remove::<MinedCanisterQueue>();
            }
        }
    }
}

/// Panic when the mined canister model fails to load: a canister that draws
/// nothing hides ore in plain sight.
fn check_mined_canister_mesh(
    asset_server: Res<AssetServer>,
    mut handle: Local<Option<Handle<WorldAsset>>>,
    mut loaded: Local<bool>,
) {
    if *loaded {
        return;
    }
    let handle =
        handle.get_or_insert_with(|| AssetRef::from(MINED_CANISTER_MESH).resolve(&asset_server));
    match asset_server.load_state(handle.id()) {
        bevy::asset::LoadState::Loaded => *loaded = true,
        bevy::asset::LoadState::Failed(error) => {
            panic!("mined canister mesh {MINED_CANISTER_MESH} failed to load: {error}")
        }
        _ => {}
    }
}

/// System set for the beam pulse, the drawn beam and the canister ejector, on
/// `Update` after [`MiningSectionSystems`], whose deploy state the pulse reads.
/// It needs no order against the asteroid carve systems: a pulse only changes
/// a field with no remesh in flight, and the carve system starts that remesh
/// in the same frame or the next.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MiningSystems;

/// Adds the mining sections' beams, the ore release and the canister ejector.
#[derive(Default, Clone, Debug)]
pub struct MiningPlugin {
    /// Whether the beam is drawn and the mined canister model is loaded and
    /// checked (false on headless rigs, which draw neither).
    pub render: bool,
}

impl Plugin for MiningPlugin {
    fn build(&self, app: &mut App) {
        trace!("MiningPlugin: build");

        app.add_observer(release_mined_ore);
        app.configure_sets(Update, MiningSystems.after(MiningSectionSystems));
        app.add_systems(
            Update,
            (pulse_mining_beams, eject_mined_canisters)
                .chain()
                .in_set(MiningSystems),
        );
        if self.render {
            app.add_observer(insert_mining_beam_laser);
            app.add_systems(
                Update,
                (
                    draw_mining_beams
                        .after(pulse_mining_beams)
                        .in_set(MiningSystems),
                    check_mined_canister_mesh,
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::*;
    use nova_events::prelude::*;
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::{
        actions::scoped_entities, objects::asteroid_kind::KIND_PLAIN, prelude::*,
        test_support::drain_spawns,
    };

    /// Frames a seed, remesh or drain may take before the test calls it hung.
    /// A cap that names a hang, not a budget.
    const FRAME_CAP: usize = 20_000;

    /// Every pulse, in order: the section that pulsed and what it did.
    #[derive(Resource, Default)]
    struct Pulses(Vec<(Entity, Result<u32, MiningRefusalType>)>);

    impl Pulses {
        fn paid(&self) -> u32 {
            self.0.iter().filter_map(|(_, outcome)| outcome.ok()).sum()
        }

        fn of(&self, section: Entity) -> Vec<Result<u32, MiningRefusalType>> {
            self.0
                .iter()
                .filter(|(each, _)| *each == section)
                .map(|(_, outcome)| *outcome)
                .collect()
        }
    }

    /// How many validated remeshes and exhaustions the rocks reported.
    #[derive(Resource, Default)]
    struct Remeshes(u32);

    fn mining_app() -> App {
        let mut app = unfinished_integrity_physics_app();
        app.add_plugins((
            AsteroidPlugin { render: false },
            AsteroidCarvePlugin { render: false },
            SectionAnimationPlugin,
            MiningSectionPlugin { render: false },
            MiningPlugin { render: false },
        ));
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<Pulses>();
        app.init_resource::<Remeshes>();
        app.add_observer(|pulse: On<MiningPulse>, mut pulses: ResMut<Pulses>| {
            pulses.0.push((pulse.entity, pulse.outcome));
        });
        app.add_observer(|_: On<AsteroidRemeshed>, mut remeshes: ResMut<Remeshes>| {
            remeshes.0 += 1;
        });
        app.finish();
        app
    }

    /// An authored rock of `kind` at the origin: its root and its field node.
    fn spawn_rock(app: &mut App, kind: &str) -> (Entity, Entity) {
        let authored = ScenarioObjectConfig {
            base: BaseScenarioObjectConfig {
                id: "rock".to_string(),
                name: "Rock".to_string(),
                position: Meters3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind: ScenarioObjectKind::Asteroid(AsteroidConfig {
                kind: kind.into(),
                destroy_sound: None,
                radius: Meters(200.0),
                texture: AssetRef::default(),
                mass: Some(45_000.0),
                seed: None,
                lock_signature: None,
            }),
        };
        authored.action(
            &mut app.world_mut().resource_mut::<NovaEventWorld>(),
            &GameEventInfo::default(),
        );
        drain_spawns(app.world_mut());
        settle(app);
        let [root] = scoped_entities(app.world_mut(), "rock")[..] else {
            panic!("the authored id must resolve to one rock");
        };
        let node = app
            .world()
            .get::<Children>(root)
            .expect("the rock has its field node")
            .iter()
            .find(|child| app.world().get::<DamageMarks>(*child).is_some())
            .expect("the field node takes marks");
        (root, node)
    }

    /// The provisional base emitter's stats, pulsing every `interval` seconds.
    fn emitter_config(interval: f32) -> MiningSectionConfig {
        MiningSectionConfig {
            render_mesh: AssetRef::default(),
            render_mesh_transform: None,
            reach: Meters(100.0),
            pulse_interval_seconds: interval,
            carve_radius_cells: 1.5,
        }
    }

    /// A player ship whose centre is `gap` engine units plus half a cell off
    /// the rock collider's `+X` bound, travel-locked on `root`.
    fn spawn_player(app: &mut App, root: Entity, node: Entity, gap: f32) -> Entity {
        let aabb = *app
            .world()
            .get::<ColliderAabb>(node)
            .expect("the field node carries the rock's collider");
        let centre = aabb.center();
        app.world_mut()
            .spawn((
                PlayerSpaceshipMarker,
                TravelLock(Some(root)),
                Transform::from_xyz(aabb.max.x + gap + 0.5, centre.y, centre.z),
                Visibility::default(),
            ))
            .id()
    }

    /// A one-cell emitter at `offset` on `ship` whose -Z face turns by `aim`
    /// about Y from facing -X, the rock's side. No tracks, so it deploys the
    /// frame its ship holds the key.
    fn spawn_emitter(app: &mut App, ship: Entity, offset: Vec3, aim: f32, interval: f32) -> Entity {
        let facing_rock = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let emitter = app
            .world_mut()
            .spawn((
                mining_section(emitter_config(interval)),
                SectionAnimations::default(),
                SectionCollider::Cuboid { size: Vec3::ONE },
                Transform::from_translation(offset)
                    .with_rotation(Quat::from_rotation_y(aim) * facing_rock),
                ChildOf(ship),
            ))
            .id();
        settle(app);
        emitter
    }

    /// Ore of `item` the world holds anywhere between a pulse and a hold:
    /// owed on a rock, queued for birth, or drifting in a canister.
    fn ore_in_world(world: &mut World, item: ItemType) -> u32 {
        let owed: u32 = world
            .query::<&MinedOre>()
            .iter(world)
            .filter(|owed| owed.item == item)
            .map(|owed| owed.corners)
            .sum();
        let queued: u32 = world
            .query::<&MinedCanisterQueue>()
            .iter(world)
            .flat_map(MinedCanisterQueue::canisters)
            .map(|canister| held(canister, item))
            .sum();
        owed + queued + drifting(world, item)
    }

    fn held(canister: &CargoCanister, item: ItemType) -> u32 {
        canister
            .stacks()
            .filter(|(each, _)| *each == item)
            .map(|(_, count)| count)
            .sum()
    }

    fn drifting(world: &mut World, item: ItemType) -> u32 {
        world
            .query::<&CargoCanister>()
            .iter(world)
            .map(|canister| held(canister, item))
            .sum()
    }

    fn canisters(world: &mut World) -> Vec<Entity> {
        world
            .query_filtered::<Entity, With<CargoCanister>>()
            .iter(world)
            .collect()
    }

    /// Two emitters held on an untouched rock each ask for its field and take
    /// nothing, then owe one stone ore per flipped corner, pulsing in the
    /// same frames. No canister exists before a validated remesh, every
    /// canister holds only stone ore, and at every frame the ore owed, queued
    /// and drifting equals what the pulses reported.
    #[test]
    fn mined_ore_is_paid_per_flipped_corner_only_after_the_remesh_lands() {
        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_ROCK);
        let ship = spawn_player(&mut app, root, node, 5.0);
        let upper = spawn_emitter(&mut app, ship, Vec3::Y, 0.0, 1.0);
        let lower = spawn_emitter(&mut app, ship, Vec3::NEG_Y, 0.0, 1.0);
        app.world_mut().entity_mut(ship).insert(MiningHeld);

        app.update();
        assert_eq!(
            app.world().resource::<Pulses>().0,
            [(upper, Ok(0)), (lower, Ok(0))],
            "the first pulses on an untouched rock only ask for its field"
        );

        let mut released = false;
        for _ in 0..FRAME_CAP {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
            let paid = app.world().resource::<Pulses>().paid();
            let world = app.world_mut();
            assert_eq!(ore_in_world(world, ItemType::StoneOre), paid);
            let drifting_now = canisters(world);
            if !drifting_now.is_empty() {
                assert!(
                    world.resource::<Remeshes>().0 > 0,
                    "a canister was born before any remesh landed"
                );
            }
            if paid > 0 && !released {
                world.entity_mut(ship).remove::<MiningHeld>();
                released = true;
            }
            let pending = world.query::<&MinedOre>().iter(world).count()
                + world.query::<&MinedCanisterQueue>().iter(world).count();
            if released && pending == 0 && !drifting_now.is_empty() {
                break;
            }
        }

        let pulses = app.world().resource::<Pulses>();
        let paid = pulses.paid();
        assert!(paid > 0, "the held beams never took material");
        for emitter in [upper, lower] {
            assert!(
                pulses
                    .of(emitter)
                    .iter()
                    .any(|outcome| outcome.is_ok_and(|corners| corners > 0)),
                "{emitter:?} never took material: {:?}",
                pulses.of(emitter)
            );
        }
        let world = app.world_mut();
        assert_eq!(drifting(world, ItemType::StoneOre), paid);
        for canister in canisters(world) {
            let held: Vec<_> = world
                .get::<CargoCanister>(canister)
                .unwrap()
                .stacks()
                .collect();
            assert!(
                held.iter().all(|(item, _)| *item == ItemType::StoneOre),
                "{held:?}"
            );
            assert!(world.get::<ScenarioScopedMarker>(canister).is_some());
        }
    }

    /// Three emitters on one held ship: one aimed at the rock, one turned
    /// away from it pulsing twice as often, and one destroyed. Each deployed
    /// emitter pulses on its own clock and aim: the aimed one takes material
    /// and carries a beam hit, the turned one refuses every pulse as off
    /// target and carries none, and the destroyed one never pulses. Released,
    /// neither pulses again and no hit is left.
    #[test]
    fn every_deployed_emitter_pulses_on_its_own_aim_and_clock() {
        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_ROCK);
        let ship = spawn_player(&mut app, root, node, 5.0);
        let aimed = spawn_emitter(&mut app, ship, Vec3::ZERO, 0.0, 1.0);
        let turned = spawn_emitter(&mut app, ship, Vec3::Y, std::f32::consts::PI, 0.5);
        let destroyed = spawn_emitter(&mut app, ship, Vec3::NEG_Y, 0.0, 1.0);
        app.world_mut()
            .entity_mut(destroyed)
            .insert(SectionInactiveMarker);
        app.world_mut().entity_mut(ship).insert(MiningHeld);

        for _ in 0..FRAME_CAP {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
            if app.world().resource::<Pulses>().paid() > 0 {
                break;
            }
        }
        let pulses = app.world().resource::<Pulses>();
        let from_aimed = pulses.of(aimed);
        let from_turned = pulses.of(turned);
        assert!(from_aimed.iter().all(Result::is_ok), "{from_aimed:?}");
        assert!(pulses.paid() > 0, "the aimed emitter never took material");
        assert!(
            from_turned
                .iter()
                .all(|outcome| *outcome == Err(MiningRefusalType::OffTarget)),
            "{from_turned:?}"
        );
        assert!(
            from_turned.len() > from_aimed.len(),
            "the faster clock pulsed no more: {} vs {}",
            from_turned.len(),
            from_aimed.len()
        );
        assert!(pulses.of(destroyed).is_empty());
        assert!(app.world().get::<MiningBeamHit>(aimed).is_some());
        assert!(app.world().get::<MiningBeamHit>(turned).is_none());
        assert!(app.world().get::<MiningBeamHit>(destroyed).is_none());

        app.world_mut().entity_mut(ship).remove::<MiningHeld>();
        let before = app.world().resource::<Pulses>().0.len();
        for _ in 0..120 {
            app.update();
        }
        assert_eq!(app.world().resource::<Pulses>().0.len(), before);
        assert!(app.world().get::<MiningBeamHit>(aimed).is_none());
    }

    /// A weapon's crater remeshes the rock like a pulse does, and pays
    /// nothing: no ore is owed, queued or drifting after it lands.
    #[test]
    fn a_combat_carve_remeshes_the_rock_and_pays_no_ore() {
        let mut app = mining_app();
        let (_, node) = spawn_rock(&mut app, KIND_ROCK);
        app.world_mut()
            .get_mut::<DamageMarks>(node)
            .expect("the node takes marks")
            .0
            .push(DamageMark {
                at: Vec3::X,
                radius: 0.3,
            });

        for _ in 0..FRAME_CAP {
            if app.world().resource::<Remeshes>().0 > 0 {
                break;
            }
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(
            app.world().resource::<Remeshes>().0 > 0,
            "delivery guard: the crater must remesh"
        );
        app.update();
        let world = app.world_mut();
        for item in [
            ItemType::StoneOre,
            ItemType::IronOre,
            ItemType::WaterIce,
            ItemType::CarbonOre,
        ] {
            assert_eq!(ore_in_world(world, item), 0, "{item:?}");
        }
    }

    /// A pulse refuses, in order, with no lock, on a rock that holds no ore,
    /// past reach, and aimed away within reach. A refused pulse asks for no
    /// field, owes nothing and leaves no beam hit.
    #[test]
    fn a_refused_pulse_changes_nothing() {
        fn pulse_once(
            app: &mut App,
            ship: Entity,
            emitter: Entity,
        ) -> Result<u32, MiningRefusalType> {
            app.world_mut().entity_mut(ship).remove::<MiningHeld>();
            app.update();
            app.world_mut().entity_mut(ship).insert(MiningHeld);
            app.update();
            *app.world()
                .resource::<Pulses>()
                .of(emitter)
                .last()
                .expect("a deployed emitter pulses at once")
        }
        fn untouched(app: &mut App, node: Entity, emitter: Entity) {
            let world = app.world();
            assert!(world.get::<AsteroidFieldSeedRequest>(node).is_none());
            assert!(world.get::<AsteroidField>(node).is_none());
            assert!(world.get::<MinedOre>(node).is_none());
            assert!(world.get::<MiningBeamHit>(emitter).is_none());
        }

        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_PLAIN);
        let ship = spawn_player(&mut app, root, node, 5.0);
        let emitter = spawn_emitter(&mut app, ship, Vec3::ZERO, 0.0, 1.0);
        assert_eq!(
            pulse_once(&mut app, ship, emitter),
            Err(MiningRefusalType::Barren)
        );
        app.world_mut().entity_mut(ship).insert(TravelLock(None));
        assert_eq!(
            pulse_once(&mut app, ship, emitter),
            Err(MiningRefusalType::NoLock)
        );
        untouched(&mut app, node, emitter);

        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_ROCK);
        let reach = emitter_config(1.0).reach.to_engine();
        let ship = spawn_player(&mut app, root, node, reach + 5.0);
        let emitter = spawn_emitter(&mut app, ship, Vec3::ZERO, 0.0, 1.0);
        assert_eq!(
            pulse_once(&mut app, ship, emitter),
            Err(MiningRefusalType::OutOfReach)
        );
        untouched(&mut app, node, emitter);

        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_ROCK);
        let ship = spawn_player(&mut app, root, node, 5.0);
        let emitter = spawn_emitter(&mut app, ship, Vec3::ZERO, std::f32::consts::PI, 1.0);
        assert_eq!(
            pulse_once(&mut app, ship, emitter),
            Err(MiningRefusalType::OffTarget)
        );
        untouched(&mut app, node, emitter);
    }

    /// A rock that runs out of material while its canisters' birth point is
    /// blocked hands them to a drop that keeps them all and waits. Once the
    /// point clears, the drop lets them out first in, first out, one at a
    /// time, and despawns with the last: all 45 ore, none lost.
    #[test]
    fn an_exhausted_rock_keeps_blocked_canisters_until_the_birth_point_clears() {
        let mut app = mining_app();
        let sector = app
            .world_mut()
            .spawn((Transform::from_xyz(3.0, 0.0, 0.0), Visibility::default()))
            .id();
        let root = app
            .world_mut()
            .spawn((Transform::default(), Visibility::default(), ChildOf(sector)))
            .id();
        let node = app
            .world_mut()
            .spawn((
                Transform::default(),
                Visibility::default(),
                ChildOf(root),
                MinedOre {
                    item: ItemType::IronOre,
                    corners: 45,
                    at: Vec3::new(0.5, 0.0, 0.0),
                    normal: Vec3::X,
                },
            ))
            .id();
        let birth = Vec3::new(4.5, 0.0, 0.0);
        let blocker = app
            .world_mut()
            .spawn((
                RigidBody::Static,
                Collider::sphere(0.5),
                Transform::from_translation(birth),
            ))
            .id();
        settle(&mut app);

        app.world_mut().trigger(AsteroidRemeshed {
            entity: node,
            exhausted: true,
        });
        app.world_mut().entity_mut(root).despawn();
        for _ in 0..120 {
            app.update();
        }

        let world = app.world_mut();
        assert!(
            canisters(world).is_empty(),
            "a blocked birth spawned a canister"
        );
        let drops: Vec<_> = world
            .query_filtered::<(Entity, &MinedCanisterQueue, &ChildOf, &GlobalTransform), With<MinedOreDrop>>()
            .iter(world)
            .map(|(drop, queue, child_of, frame)| {
                let counts: Vec<u32> = queue
                    .canisters()
                    .map(|canister| held(canister, ItemType::IronOre))
                    .collect();
                (drop, counts, child_of.parent(), frame.translation())
            })
            .collect();
        let [(drop, counts, parent, at)] = &drops[..] else {
            panic!("the exhausted rock must leave one drop: {drops:?}");
        };
        assert_eq!(counts, &[20, 20, 5]);
        assert_eq!(*parent, sector);
        assert!(at.distance(Vec3::new(3.0, 0.0, 0.0)) < 1e-5, "{at}");
        assert!(world.get::<ScenarioScopedMarker>(*drop).is_some());
        let drop = *drop;

        world.entity_mut(blocker).despawn();
        let mut born = Vec::new();
        for _ in 0..FRAME_CAP {
            app.update();
            let world = app.world_mut();
            for canister in canisters(world) {
                if !born.iter().any(|(each, _)| *each == canister) {
                    let count = held(
                        world.get::<CargoCanister>(canister).unwrap(),
                        ItemType::IronOre,
                    );
                    assert!(world.get::<ScenarioScopedMarker>(canister).is_some());
                    born.push((canister, count));
                }
            }
            if world.get_entity(drop).is_err() {
                break;
            }
        }
        assert!(
            app.world().get_entity(drop).is_err(),
            "the drop never drained"
        );
        let counts: Vec<u32> = born.iter().map(|(_, count)| *count).collect();
        assert_eq!(counts, [20, 20, 5], "first in, first out, one at a time");
    }
}
