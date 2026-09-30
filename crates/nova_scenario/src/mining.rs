//! Mining: the player's short-range beam that cuts ore out of a
//! travel-locked asteroid and drops it as cargo canisters.
//!
//! While the player holds `mine` ([`MiningHeld`]), the beam pulses once at
//! once and then every [`MINING_PULSE_SECS`] of game time. A pulse needs a
//! travel-locked asteroid whose kind yields ore ([`ore_for_asteroid_kind`])
//! and whose collider surface is within [`MINING_REACH`] of a player-ship
//! collider. It takes a sphere of [`MINING_RADIUS_CELLS`] field cells out of
//! the rock's [`AsteroidField`] where the beam meets solid material, and owes
//! one ore item per corner that sphere flipped from solid to empty. A corner a
//! hit or an earlier pulse already emptied flips nothing, so a combat carve
//! never yields ore and a pulse into an empty crater yields none.
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

use std::collections::VecDeque;

use avian3d::{
    collision::collider::contact_query::{closest_points, distance, ClosestPoints},
    prelude::{
        AngularVelocity, Collider, ColliderOf, LinearVelocity, SpatialQuery, SpatialQueryFilter,
    },
};
use bevy::prelude::*;
use nova_events::prelude::*;
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

/// `MiningPlugin`, its system set, the ore rule, the pulse event and the
/// mining constants.
pub mod prelude {
    pub use super::{
        ore_for_asteroid_kind, MinedCanisterQueue, MinedOre, MiningPlugin, MiningPulse,
        MiningRefusalType, MiningSystems, MINED_CANISTER_MESH, MINING_PULSE_SECS,
        MINING_RADIUS_CELLS, MINING_REACH,
    };
}

/// How far the beam reaches: from the nearest player-ship collider to the
/// rock's collider surface. Provisional.
pub const MINING_REACH: Meters = Meters(100.0);

/// Game seconds between pulses while the key is held. Provisional.
pub const MINING_PULSE_SECS: f32 = 1.0;

/// The radius of one pulse's sphere, in cells of the rock's own field.
/// Provisional.
pub const MINING_RADIUS_CELLS: f32 = 1.5;

/// The one generic canister model every mined canister wears. A path under
/// the asset root, as the base bundle's `self://gltf/cargo_canister_cuboid.glb`
/// resolves; a failed load panics.
pub const MINED_CANISTER_MESH: &str = "base/gltf/cargo_canister_cuboid.glb#Scene0";

/// How far off the mined surface a canister is born, in engine units.
const MINED_CANISTER_OFFSET: f32 = 1.0;

/// How fast a new canister drifts off the rock, in engine units per second,
/// on top of the rock's own motion at the birth point.
const MINED_CANISTER_SPEED: f32 = 0.2;

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
    /// No travel lock.
    NoLock,
    /// The travel lock is not an asteroid with a field node.
    NotAsteroid,
    /// The asteroid's kind yields no ore.
    Barren,
    /// No player-ship collider is within [`MINING_REACH`] of the rock.
    OutOfReach,
}

/// One pulse of the player's mining beam, triggered on the player ship.
///
/// `Ok` carries the corners the pulse flipped: the ore it owes, paid once
/// the rock's remesh lands. It is zero while the rock's field seeds or
/// remeshes, and when the beam met no material.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct MiningPulse {
    /// The player ship.
    pub entity: Entity,
    /// What the pulse did.
    pub outcome: Result<u32, MiningRefusalType>,
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

/// Seconds until the held beam's next pulse; on the player ship while the key
/// is held.
#[derive(Component, Clone, Copy, Debug)]
struct MiningPulseClock(f32);

/// Marks the entity a rock that ran out of material hands its queue to. It
/// carries the rock's last pose and despawns once the queue drains.
#[derive(Component, Clone, Copy, Debug)]
struct MinedOreDrop;

/// Pulse the beam of every player ship holding the key.
#[expect(
    clippy::type_complexity,
    reason = "the player, the locked rock, its field node and every ship collider"
)]
fn pulse_mining_beam(
    time: Res<Time>,
    mut commands: Commands,
    mut q_player: Query<
        (Entity, &TravelLock, Option<&mut MiningPulseClock>),
        (With<PlayerSpaceshipMarker>, With<MiningHeld>),
    >,
    q_idle: Query<Entity, (With<MiningPulseClock>, Without<MiningHeld>)>,
    q_rocks: Query<(&AsteroidKind, &Children), With<AsteroidMarker>>,
    mut q_nodes: Query<
        (
            &Collider,
            &GlobalTransform,
            Option<&mut AsteroidField>,
            Option<&mut MinedOre>,
            Has<AsteroidRemesh>,
            Has<AsteroidFieldSeeding>,
        ),
        With<DamageMarks>,
    >,
    q_colliders: Query<(&Collider, &ColliderOf, &GlobalTransform)>,
) {
    for ship in &q_idle {
        commands.entity(ship).remove::<MiningPulseClock>();
    }
    for (ship, lock, clock) in &mut q_player {
        let due = match clock {
            Some(mut clock) => {
                clock.0 -= time.delta_secs();
                let due = clock.0 <= 0.0;
                if due {
                    clock.0 += MINING_PULSE_SECS;
                }
                due
            }
            None => {
                commands
                    .entity(ship)
                    .insert(MiningPulseClock(MINING_PULSE_SECS));
                true
            }
        };
        if !due {
            continue;
        }
        let outcome = pulse(
            ship,
            lock,
            &mut commands,
            &q_rocks,
            &mut q_nodes,
            &q_colliders,
        );
        match outcome {
            Ok(corners) => debug!("pulse_mining_beam: {ship:?} flipped {corners} corner(s)"),
            Err(refusal) => info!("pulse_mining_beam: {ship:?} refused: {refusal:?}"),
        }
        commands.trigger(MiningPulse {
            entity: ship,
            outcome,
        });
    }
}

/// One pulse of `ship`'s beam: the checks in [`MiningRefusalType`] order,
/// then the carve.
#[expect(
    clippy::type_complexity,
    reason = "the queries of pulse_mining_beam, passed through"
)]
fn pulse(
    ship: Entity,
    lock: &TravelLock,
    commands: &mut Commands,
    q_rocks: &Query<(&AsteroidKind, &Children), With<AsteroidMarker>>,
    q_nodes: &mut Query<
        (
            &Collider,
            &GlobalTransform,
            Option<&mut AsteroidField>,
            Option<&mut MinedOre>,
            Has<AsteroidRemesh>,
            Has<AsteroidFieldSeeding>,
        ),
        With<DamageMarks>,
    >,
    q_colliders: &Query<(&Collider, &ColliderOf, &GlobalTransform)>,
) -> Result<u32, MiningRefusalType> {
    let rock = lock.0.ok_or(MiningRefusalType::NoLock)?;
    let (kind, children) = q_rocks
        .get(rock)
        .map_err(|_| MiningRefusalType::NotAsteroid)?;
    let node = children
        .iter()
        .find(|child| q_nodes.contains(*child))
        .ok_or(MiningRefusalType::NotAsteroid)?;
    let item = ore_for_asteroid_kind(kind).ok_or(MiningRefusalType::Barren)?;

    let (rock_collider, frame, field, mined, remeshing, seeding) = q_nodes
        .get_mut(node)
        .expect("the node was found in this query");
    let (_, rock_rotation, rock_position) = frame.to_scale_rotation_translation();
    let reach = MINING_REACH.to_engine();
    let (surface, normal) = q_colliders
        .iter()
        .filter(|(_, of, _)| of.body == ship)
        .filter_map(|(collider, _, pose)| {
            let (_, rotation, position) = pose.to_scale_rotation_translation();
            distance(
                collider,
                position,
                rotation,
                rock_collider,
                rock_position,
                rock_rotation,
            )
            .ok()
            .filter(|gap| *gap <= reach)
            .map(|gap| (gap, collider, position, rotation))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .and_then(|(_, collider, position, rotation)| {
            beam_contact(
                (collider, position, rotation),
                (rock_collider, rock_position, rock_rotation),
                reach,
            )
        })
        .ok_or(MiningRefusalType::OutOfReach)?;

    let Some(mut field) = field else {
        if !seeding {
            commands.entity(node).insert(AsteroidFieldSeedRequest);
        }
        return Ok(0);
    };
    if remeshing {
        return Ok(0);
    }

    let to_local = frame.affine().inverse();
    let at = to_local.transform_point3(surface);
    let local_normal = (rock_rotation.inverse() * normal).normalize_or_zero();
    let cell = field.solid().cell_size();
    let radius = MINING_RADIUS_CELLS * cell;
    // The collider a pristine rock carries is the convex hull of its surface,
    // so the hull point can stand over a hollow. March the sphere inward along
    // the beam, one cell a step, until it meets solid material. An empty step
    // flips nothing and removes nothing.
    let steps = (2.0 * field.solid().half_extent() / cell).ceil() as u32;
    let corners = (0..=steps)
        .map(|step| at - local_normal * (step as f32 * cell))
        .find_map(|centre| {
            let flipped = field.mine(centre, radius);
            (flipped > 0).then_some(flipped)
        })
        .unwrap_or(0);
    if corners == 0 {
        return Ok(0);
    }
    match mined {
        Some(mut owed) => {
            owed.corners += corners;
            owed.at = at;
            owed.normal = local_normal;
        }
        None => {
            commands.entity(node).insert(MinedOre {
                item,
                corners,
                at,
                normal: local_normal,
            });
        }
    }
    commands.trigger(CarveSpew {
        entity: node,
        at: surface,
        radius: radius * frame.scale().max_element(),
        kind: DamageType::Kinetic,
    });
    Ok(corners)
}

/// Where the beam from the nearest ship collider meets the rock, in world
/// space, and the outward direction there; `None` past `reach`.
fn beam_contact(
    (ship, ship_position, ship_rotation): (&Collider, Vec3, Quat),
    (rock, rock_position, rock_rotation): (&Collider, Vec3, Quat),
    reach: f32,
) -> Option<(Vec3, Vec3)> {
    let outward = |surface: Vec3, from: Vec3| {
        (from - surface)
            .try_normalize()
            .or_else(|| (surface - rock_position).try_normalize())
            .unwrap_or(Vec3::Y)
    };
    match closest_points(
        ship,
        ship_position,
        ship_rotation,
        rock,
        rock_position,
        rock_rotation,
        reach,
    )
    .ok()?
    {
        ClosestPoints::WithinMargin(on_ship, on_rock) => Some((on_rock, outward(on_rock, on_ship))),
        // Touching: the rock's surface nearest the ship collider's centre.
        ClosestPoints::Intersecting => {
            let (surface, _) =
                rock.project_point(rock_position, rock_rotation, ship_position, false);
            Some((surface, outward(surface, ship_position)))
        }
        ClosestPoints::OutsideMargin => None,
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

/// System set for the beam pulse and the canister ejector, on `Update`. It
/// needs no order against the asteroid carve systems: a pulse only changes a
/// field with no remesh in flight, and the carve system starts that remesh in
/// the same frame or the next.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MiningSystems;

/// Adds the player's mining beam, the ore release and the canister ejector.
#[derive(Default, Clone, Debug)]
pub struct MiningPlugin {
    /// Whether the mined canister model is loaded and checked (false on
    /// headless rigs, which draw no canister).
    pub render: bool,
}

impl Plugin for MiningPlugin {
    fn build(&self, app: &mut App) {
        trace!("MiningPlugin: build");

        app.add_observer(release_mined_ore);
        app.add_systems(
            Update,
            (pulse_mining_beam, eject_mined_canisters)
                .chain()
                .in_set(MiningSystems),
        );
        if self.render {
            app.add_systems(Update, check_mined_canister_mesh);
        }
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::*;
    use nova_gameplay::test_support::{settle, unfinished_integrity_physics_app};

    use super::*;
    use crate::{
        actions::scoped_entities, objects::asteroid_kind::KIND_PLAIN, prelude::*,
        test_support::drain_spawns,
    };

    /// Frames a seed, remesh or drain may take before the test calls it hung.
    /// A cap that names a hang, not a budget.
    const FRAME_CAP: usize = 20_000;

    /// Every pulse outcome, in order.
    #[derive(Resource, Default)]
    struct Pulses(Vec<Result<u32, MiningRefusalType>>);

    impl Pulses {
        fn paid(&self) -> u32 {
            self.0.iter().filter_map(|outcome| outcome.ok()).sum()
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
            MiningPlugin { render: false },
        ));
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.init_resource::<Pulses>();
        app.init_resource::<Remeshes>();
        app.add_observer(|pulse: On<MiningPulse>, mut pulses: ResMut<Pulses>| {
            pulses.0.push(pulse.outcome);
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

    /// A one-collider player ship `gap` engine units off the rock collider's
    /// `+X` bound, travel-locked on `root`.
    fn spawn_player(app: &mut App, root: Entity, node: Entity, gap: f32) -> Entity {
        let aabb = *app
            .world()
            .get::<ColliderAabb>(node)
            .expect("the field node carries the rock's collider");
        let centre = aabb.center();
        let ship = app
            .world_mut()
            .spawn((
                PlayerSpaceshipMarker,
                TravelLock(Some(root)),
                RigidBody::Kinematic,
                Collider::sphere(1.0),
                Transform::from_xyz(aabb.max.x + gap + 1.0, centre.y, centre.z),
            ))
            .id();
        settle(app);
        ship
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

    /// A held beam on an untouched rock asks for its field and takes nothing,
    /// then owes one stone ore per flipped corner. No canister exists before a
    /// validated remesh, every canister holds only stone ore, and at every
    /// frame the ore owed, queued and drifting equals what the pulses reported.
    #[test]
    fn mined_ore_is_paid_per_flipped_corner_only_after_the_remesh_lands() {
        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_ROCK);
        let ship = spawn_player(&mut app, root, node, 5.0);
        app.world_mut().entity_mut(ship).insert(MiningHeld);

        app.update();
        assert_eq!(
            app.world().resource::<Pulses>().0,
            [Ok(0)],
            "the first pulse on an untouched rock only asks for its field"
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

        let paid = app.world().resource::<Pulses>().paid();
        assert!(paid > 0, "the held beam never took material");
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
    /// and past reach, and a refused pulse asks for no field.
    #[test]
    fn a_pulse_refuses_with_no_lock_on_barren_rock_and_out_of_reach() {
        fn pulse_once(app: &mut App, ship: Entity) -> Result<u32, MiningRefusalType> {
            app.world_mut().entity_mut(ship).remove::<MiningHeld>();
            app.update();
            app.world_mut().entity_mut(ship).insert(MiningHeld);
            app.update();
            *app.world()
                .resource::<Pulses>()
                .0
                .last()
                .expect("a held beam pulses at once")
        }

        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_PLAIN);
        let ship = spawn_player(&mut app, root, node, 5.0);

        assert_eq!(pulse_once(&mut app, ship), Err(MiningRefusalType::Barren));
        app.world_mut().entity_mut(ship).insert(TravelLock(None));
        assert_eq!(pulse_once(&mut app, ship), Err(MiningRefusalType::NoLock));
        assert!(app.world().get::<AsteroidFieldSeedRequest>(node).is_none());
        assert!(app.world().get::<AsteroidField>(node).is_none());

        let mut app = mining_app();
        let (root, node) = spawn_rock(&mut app, KIND_ROCK);
        let reach = MINING_REACH.to_engine();
        let ship = spawn_player(&mut app, root, node, reach + 5.0);
        assert_eq!(
            pulse_once(&mut app, ship),
            Err(MiningRefusalType::OutOfReach)
        );
        assert!(app.world().get::<AsteroidFieldSeedRequest>(node).is_none());
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
