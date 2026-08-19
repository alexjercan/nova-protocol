//! The visual side of destruction: a destroyed [`ExplodableEntity`] DETACHES.
//! It stops being part of its parent, becomes a body of its own carrying the
//! art and the collider it already had, takes a kick and a spin, and despawns
//! on a timer. Reacts to the destruction events fired by the integrity glue
//! rather than deciding when something dies.
//!
//! # Nothing computes geometry when something dies
//!
//! A death moves entities and clones a collider handle. That is the whole
//! mechanism, and it is why this module has no fragment budget and no spawn
//! queue: there is nothing left to ration.
//!
//! What was here instead was a random-plane slicer, and it was both the largest
//! single item in the death frame and wrong three ways at once - every plane
//! passed through the WORLD ORIGIN at every recursion level, so a piece sitting
//! off to one side was cut by a plane outside itself and came back as a sliver
//! or as an uncut lump flying nowhere, and it drew from the thread RNG, so no
//! two deaths were alike. Fixing it would have cost the same milliseconds.
//!
//! # The finale is decided ONCE
//!
//! Destructibility is SEMANTIC - it is [`ExplodableEntity`] plus a destroy
//! marker, and never where a `Mesh3d` happens to sit. A section carries health,
//! a collider and the marker on a gameplay root while its art hangs off
//! `SectionRenderOf` descendants under a gltf `WorldAssetRoot`, so anything
//! gated on `Mesh3d` at the gameplay entity found nothing on every section ever
//! shipped. Detaching asks about neither: the children come across whatever
//! they are, and the COLLIDER is what says a body can be one.
//!
//! # It takes its plates and its greebles down with it
//!
//! Every direct child moves, not just the meshed ones. A section's skin plates
//! and their greebles are parented to it, so they ride the wreck out still
//! bolted on, which is both what a piece breaking off a hull looks like and the
//! cheaper of the two options - detaching each of them as a body of its own
//! multiplies the piece count by everything a section wears.
//!
//! # Born inside the body it left
//!
//! A piece starts exactly where it was, which is inside the collider of the
//! structure it was part of. A dynamic body spawned interpenetrating another is
//! a problem the solver fixes by shoving them apart, hard. So it takes the same
//! [`ChunkGrace`] a carved rock chunk takes: kinematic and colliderless until it
//! has drifted clear. See [`chunk`](super::chunk).
//!
//! Touch this module to change how wrecks come apart (kick, spin, lifetime).
//! Health, disable, and destroy bookkeeping lives in the generic
//! [`core`](super::core) integrity layer; structure owners publish its graph.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_rand::prelude::*;
use nova_events::prelude::*;
use rand::RngExt;

use super::components::prelude::*;
use crate::{
    integrity::{chunk::prelude::ChunkGrace, neutralize::prelude::DefeatedMarker},
    lifetime::TempEntity,
    prelude::SpaceshipRootMarker,
};

/// `ExplodableEntity` and `DetachedPieceMarker`.
pub mod prelude {
    pub use super::{DetachedPieceMarker, ExplodableEntity};
}

/// How long a detached piece survives before it despawns.
///
/// Pieces are dynamic rigid bodies and nothing despawns them otherwise until
/// scenario teardown; an unattended scene that keeps destroying things (a menu
/// backdrop, a long mission) accumulates physics bodies without bound. Long
/// enough to watch the wreck drift apart, short enough to keep the body count
/// flat.
const PIECE_LIFETIME_SECS: f32 = 30.0;

/// How fast a piece is pushed away from the middle of the structure, in world
/// units per second, on top of whatever that structure was already doing.
const PIECE_KICK: std::ops::Range<f32> = 2.0..5.0;

/// How fast a piece tumbles as it leaves, in radians per second.
///
/// The tumble is what carries the read. A whole section coming off is one rigid
/// shape, so what tells destruction from disassembly is that it leaves spinning
/// rather than sliding.
const PIECE_SPIN: std::ops::Range<f32> = 1.0..4.0;

/// Marker component to indicate that an entity can be exploded.
#[derive(Component, Clone, Debug, Default, Reflect)]
pub struct ExplodableEntity;

/// Tags a piece that came off a destroyed [`ExplodableEntity`]: the body it was
/// part of, now a body of its own, wearing the art it already had.
///
/// It NAMES the body it left, which is already despawned by the time anything
/// sees this. That is what lets an observer attribute wreckage to the death that
/// produced it: deaths overlap - a collapsing structure destroys every section
/// it has left in one frame - so counting bodies over a window blames one death
/// for another's pieces.
#[derive(Component, Debug, Clone, Reflect)]
pub struct DetachedPieceMarker(pub Entity);

pub(super) struct ExplodablePlugin;

impl Plugin for ExplodablePlugin {
    fn build(&self, app: &mut App) {
        debug!("ExplodablePlugin: build");

        app.add_observer(on_add_explodable_entity);
        app.add_observer(on_destroyed_entity);
        app.add_observer(detach_destroyed_body);
        app.add_observer(despawn_destroyed_that_does_not_detach);
    }
}

/// A unit vector drawn off the seeded stream, uniformly over the sphere.
fn random_unit_vector(rng: &mut impl RngExt) -> Vec3 {
    let height: f32 = rng.random_range(-1.0..1.0);
    let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
    let ring = (1.0 - height * height).max(0.0).sqrt();
    Vec3::new(ring * angle.cos(), ring * angle.sin(), height).normalize_or(Vec3::Y)
}

/// The centre, drift and spin of the nearest body at or above `entity` that is
/// moving, or `None` when nothing in the chain is.
///
/// A section carries no velocity of its own: it is a child of the ship's rigid
/// body, and avian keeps the velocity there. So a piece's inheritance is a
/// walk, not a lookup.
fn moving_body(
    entity: Entity,
    q_parents: &Query<&ChildOf>,
    q_motion: &Query<(&GlobalTransform, &LinearVelocity, Option<&AngularVelocity>)>,
) -> Option<(Vec3, Vec3, Vec3)> {
    let mut current = entity;
    loop {
        if let Ok((frame, linear, angular)) = q_motion.get(current) {
            return Some((
                frame.translation(),
                linear.0,
                angular.map_or(Vec3::ZERO, |angular| angular.0),
            ));
        }
        current = q_parents.get(current).ok()?.0;
    }
}

/// Despawn a destroyed entity that [`detach_destroyed_body`] does not own.
///
/// The detach despawns everything it runs on, which is every destroyed
/// [`ExplodableEntity`] that is not an [`IntegrityRoot`]. This reaper covers the
/// rest, and it must: a zero-health node left standing keeps its collider live
/// and its capabilities working.
///
/// Two kinds reach it. Destructible things that are not explodable - skin plates
/// and other fixtures, which come off with the section they are bolted to - and
/// integrity roots, which the detach deliberately skips. Recursive, so a
/// section's gltf children go with it; `try_despawn` because a sibling reaction
/// can have taken the entity already.
fn despawn_destroyed_that_does_not_detach(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_destroyed: Query<(Has<ExplodableEntity>, Has<IntegrityRoot>), With<IntegrityDestroyMarker>>,
) {
    let entity = add.entity;
    let Ok((explodable, is_root)) = q_destroyed.get(entity) else {
        return;
    };
    if explodable && !is_root {
        // The detach owns this one and despawns it once its piece is standing.
        return;
    }

    trace!(
        "despawn_destroyed_that_does_not_detach: despawning {:?}",
        entity
    );
    commands.entity(entity).try_despawn();
}

fn on_add_explodable_entity(
    add: On<Add, ExplodableEntity>,
    mut commands: Commands,
    q_explode: Query<&ChildOf, With<ExplodableEntity>>,
) {
    let entity = add.entity;
    trace!("on_add_explodable_entity: entity {:?}", entity);

    let Ok(ChildOf(parent)) = q_explode.get(entity) else {
        return;
    };

    debug!(
        "on_add_explodable_entity: entity {:?} is child of {:?}, adding ExplodableEntity to parent",
        entity, *parent
    );

    commands.entity(*parent).insert(ExplodableEntity);
}

fn on_destroyed_entity(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_info: Query<
        (
            &EntityId,
            &EntityTypeName,
            Has<SpaceshipRootMarker>,
            Has<DefeatedMarker>,
        ),
        With<IntegrityDestroyMarker>,
    >,
) {
    let entity = add.entity;
    trace!("on_destroyed_entity: entity {:?}", entity);

    let Ok((id, type_name, is_ship, already_defeated)) = q_info.get(entity) else {
        return;
    };

    debug!(
        "on_destroyed_entity: entity {:?} destroyed (id: {:?}, type: {:?})",
        entity, id, type_name
    );
    let id = id.to_string();
    let type_name = type_name.to_string();
    if is_ship && !already_defeated {
        // Persist the exact-once guard through the rest of this frame. This
        // also prevents neutralization detection from reporting a second
        // defeat if destruction and section loss converge in one update.
        // try_insert: the reaper above observes this same Add event and observer
        // order is unspecified, so its queued despawn can land before this
        // insert; a plain insert then panics at apply time. A root despawned
        // mid-batch cannot be defeated twice, so losing the marker is harmless -
        // the events below still fire either way.
        commands.entity(entity).try_insert(DefeatedMarker);
        commands.fire::<OnDefeatedEvent>(OnDefeatedEventInfo {
            id: id.clone(),
            type_name: type_name.clone(),
        });
    }
    commands.fire::<OnDestroyedEvent>(OnDestroyedEventInfo { id, type_name });
}

/// THE finale: take the destroyed body off its parent and let it tumble away.
///
/// An [`IntegrityRoot`] is excluded. `ExplodableEntity` propagates to parents
/// (see [`on_add_explodable_entity`]), so a root carries it too - and a root is
/// by definition the entity whose descendants are its integrity NODES rather
/// than its own art. Detaching it would take the whole structure off at once and
/// bypass every node's own death.
///
/// The test is `IntegrityRoot` rather than `SpaceshipRootMarker` because there
/// is more than one kind of root and the others are not spaceships: a severed
/// wreck body carries `ShipWreckFragmentMarker` and no ship marker at all, and
/// an asteroid's root is a husk whose collider, health and mesh live on a child
/// node. Naming the concept instead of one of its cases also keeps this layer
/// free of anything `nova_ship` owns.
///
/// A body with NO collider cannot become one, so it leaves nothing behind and
/// says so. That is a deliberate refusal rather than a gap: there used to be a
/// burst of generic cubes on this branch, and the trouble with it was not that
/// it looked bad, it was that it looked like SOMETHING - so a body that had
/// silently failed to come apart was indistinguishable from one that had come
/// apart badly. The `system_destruction_finale` range asserts this branch never runs on
/// shipped content.
///
/// Despawns the body on every path. It is the only thing that despawns a
/// destroyed explodable, so an early return leaves a zero-health wreck standing
/// with a live collider.
fn detach_destroyed_body(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_dying: Query<
        (
            &GlobalTransform,
            Option<&Collider>,
            Option<&Children>,
            Option<&Name>,
        ),
        (
            With<ExplodableEntity>,
            With<IntegrityDestroyMarker>,
            Without<IntegrityRoot>,
        ),
    >,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_motion: Query<(&GlobalTransform, &LinearVelocity, Option<&AngularVelocity>)>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let entity = add.entity;
    trace!("detach_destroyed_body: entity {:?}", entity);

    let Ok((frame, collider, children, name)) = q_dying.get(entity) else {
        return;
    };

    let Some(collider) = collider.cloned() else {
        error!(
            "detach_destroyed_body: {entity:?} died with no collider to detach as \
             - nothing was left behind"
        );
        commands.entity(entity).try_despawn();
        return;
    };

    let transform = frame.compute_transform();
    // How the structure was moving, so its pieces leave with it. A section is a
    // child of the rigid body rather than the body itself, so this walks up for
    // the nearest thing that has a velocity at all.
    let motion = moving_body(entity, &q_parents, &q_motion);
    let (centre, drift) =
        motion.map_or((transform.translation, Vec3::ZERO), |(at, linear, spin)| {
            // v + omega x r. Without the second term a ship dying at speed leaves
            // its pieces hanging where it was hit while the wreck flies out from
            // under them, which reads as debris being spawned rather than shed.
            (at, linear + spin.cross(transform.translation - at))
        });
    // Outward from the middle of the structure, so a ship comes apart instead of
    // every piece sliding the same way.
    let kick = random_unit_vector(&mut rng);
    let away =
        Dir3::new(transform.translation - centre).unwrap_or(Dir3::new(kick).unwrap_or(Dir3::Y));

    let piece = commands
        .spawn((
            DetachedPieceMarker(entity),
            Name::new(match name {
                Some(name) => format!("Wreck of {name}"),
                None => format!("Wreck of {entity}"),
            }),
            transform,
            Visibility::Visible,
            // Kinematic and colliderless to begin with - it is standing inside
            // the structure it was bolted to. `ChunkGrace` makes it physical
            // once it has drifted clear.
            RigidBody::Kinematic,
            LinearVelocity(drift + away * rng.random_range(PIECE_KICK)),
            AngularVelocity(random_unit_vector(&mut rng) * rng.random_range(PIECE_SPIN)),
            ChunkGrace::new(collider),
            TempEntity(PIECE_LIFETIME_SECS),
        ))
        .id();

    // The art comes WITH it, which is the whole mechanism: nothing is meshed,
    // sliced or copied, the children simply belong to the piece now and keep the
    // local transforms and materials they already had.
    //
    // Their COLLIDERS do not come with them. avian attaches every collider under
    // a rigid body to that body, and a section wears a plate per face and a
    // handful of greebles per plate, each carrying a compound of its own - so a
    // wreck that kept them is a body with dozens of shapes on it, times every
    // section a collapse kills at once. Measured on the arena's worst frame, that
    // is the difference between 85 ms and 736 ms. The section's own collider is
    // the shape the wreck is, and it is enough to fly into.
    if let Some(children) = children {
        let mut stack: Vec<Entity> = children.iter().collect();
        for child in &stack {
            commands.entity(*child).insert(ChildOf(piece));
        }
        while let Some(node) = stack.pop() {
            commands.entity(node).try_remove::<Collider>();
            if let Ok(grandchildren) = q_children.get(node) {
                stack.extend(grandchildren.iter());
            }
        }
    }

    debug!("detach_destroyed_body: {entity:?} left as {piece:?}");
    // Queued after the reparent, so the children are no longer descendants by
    // the time the recursive despawn reads them.
    commands.entity(entity).try_despawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity::chunk::prelude::CarvedChunkPlugin;

    #[derive(Resource, Default)]
    struct FiredEvents(Vec<&'static str>);

    fn destruction_event_app() -> App {
        let mut app = App::new();
        app.init_resource::<FiredEvents>();
        app.add_observer(on_destroyed_entity);
        app.add_observer(|event: On<GameEvent>, mut fired: ResMut<FiredEvents>| {
            fired.0.push(event.name());
        });
        app
    }

    #[test]
    fn direct_ship_destruction_defeats_before_it_destroys() {
        let mut app = destruction_event_app();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("ship"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
            ))
            .id();

        app.world_mut()
            .entity_mut(ship)
            .insert(IntegrityDestroyMarker);

        assert_eq!(
            app.world().resource::<FiredEvents>().0,
            [OnDefeatedEvent::name(), OnDestroyedEvent::name()]
        );
        assert!(
            app.world().entity(ship).contains::<DefeatedMarker>(),
            "direct destruction persists the exact-once guard"
        );
    }

    /// Bevy gives no ordering between two observers of one Add event, so the
    /// reaper can queue the root's despawn BEFORE this observer's
    /// `DefeatedMarker` insert. Model that exact queue shape: a despawn already
    /// queued ahead of the observer pass. The marker insert must tolerate the
    /// dead root (a plain insert panics at command-apply time and takes down the
    /// whole frame - the 2026-08-13 "The Raid" crash), and the defeat/destroy
    /// events must still reach the scenario.
    #[test]
    fn defeat_survives_a_sibling_observer_despawning_the_root() {
        let mut app = destruction_event_app();
        let ship = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                EntityId::new("raider"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
            ))
            .id();

        app.world_mut().commands().entity(ship).try_despawn();
        app.world_mut()
            .entity_mut(ship)
            .insert(IntegrityDestroyMarker);

        assert!(!app.world().entities().contains(ship));
        assert_eq!(
            app.world().resource::<FiredEvents>().0,
            [OnDefeatedEvent::name(), OnDestroyedEvent::name()],
            "the scenario still hears a kill whose root died mid-batch"
        );
    }

    #[test]
    fn destroying_an_already_defeated_wreck_does_not_defeat_it_twice() {
        let mut app = destruction_event_app();
        let wreck = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                DefeatedMarker,
                EntityId::new("wreck"),
                EntityTypeName::new(SPACESHIP_TYPE_NAME),
            ))
            .id();

        app.world_mut()
            .entity_mut(wreck)
            .insert(IntegrityDestroyMarker);

        assert_eq!(
            app.world().resource::<FiredEvents>().0,
            [OnDestroyedEvent::name()]
        );
    }

    /// The whole finale, wired the way the game wires it.
    fn finale_app(seed: u64) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(seed.to_ne_bytes()));
        app.add_plugins(CarvedChunkPlugin);
        app.add_plugins(ExplodablePlugin);
        app
    }

    /// A body that draws the way every shipped section draws: nothing on the
    /// gameplay entity, art two levels down, and the authored collider on the
    /// gameplay entity itself. Returns the gameplay root and its art node.
    fn body_drawing_through_descendants(app: &mut App, parts: usize) -> (Entity, Entity) {
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::from(Cuboid::new(1.0, 1.0, 1.0)));

        let root = app
            .world_mut()
            .spawn((
                ExplodableEntity,
                Collider::cuboid(1.0, 1.0, 1.0),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        // The `SectionRenderOf` child a section spawns, standing in for the
        // gltf scene root; the meshes hang under IT, not under the root.
        let art = app
            .world_mut()
            .spawn((
                ChildOf(root),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        for part in 0..parts {
            let offset = Vec3::X * part as f32;
            app.world_mut().spawn((
                ChildOf(art),
                Mesh3d(mesh.clone()),
                Transform::from_translation(offset),
                GlobalTransform::from_translation(offset),
            ));
        }
        (root, art)
    }

    /// Every detached body on the field.
    fn wreckage(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .query_filtered::<Entity, With<DetachedPieceMarker>>()
            .iter(app.world())
            .collect()
    }

    /// THE mechanism. A section's art is not meshed, sliced or copied: the
    /// entities that were drawing it belong to the wreck now, which is what
    /// makes a death cost nothing.
    #[test]
    fn a_dead_body_takes_its_own_art_with_it() {
        let mut app = finale_app(7);
        let (body, art) = body_drawing_through_descendants(&mut app, 3);

        app.world_mut()
            .entity_mut(body)
            .insert(IntegrityDestroyMarker);
        app.update();

        let pieces = wreckage(&mut app);
        assert_eq!(pieces.len(), 1, "a section leaves exactly one body");
        let piece = pieces[0];
        assert_eq!(
            app.world().get::<ChildOf>(art).map(|parent| parent.0),
            Some(piece),
            "the art it was drawing with belongs to the wreck now"
        );
        assert!(
            app.world().get::<TempEntity>(piece).is_some(),
            "and the wreck despawns on a timer rather than accumulating"
        );
        assert!(
            !app.world().entities().contains(body),
            "the dead section itself is gone"
        );
    }

    /// A wreck is ONE shape: the section's own. avian attaches every collider
    /// under a rigid body to that body, and a section wears a plate per face
    /// with greebles on each, so a wreck that kept their colliders is a compound
    /// of dozens - times every section a collapse kills at once. That was
    /// measured at 736 ms against 85 ms on the arena's worst frame.
    #[test]
    fn a_wreck_carries_only_the_shape_the_section_was() {
        let mut app = finale_app(7);
        let (body, art) = body_drawing_through_descendants(&mut app, 2);
        // A skin plate with its own collider, the way a section wears one.
        let plate = app
            .world_mut()
            .spawn((
                ChildOf(body),
                Collider::cuboid(1.0, 0.1, 1.0),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        // And a greeble bolted to the plate, which is where they hang.
        app.world_mut().spawn((
            ChildOf(plate),
            Collider::cuboid(0.2, 0.2, 0.2),
            Transform::default(),
            GlobalTransform::IDENTITY,
        ));

        app.world_mut()
            .entity_mut(body)
            .insert(IntegrityDestroyMarker);
        app.update();

        let piece = *wreckage(&mut app).first().expect("the section detached");
        let colliders = app
            .world_mut()
            .query_filtered::<(), With<Collider>>()
            .iter(app.world())
            .count();
        assert_eq!(
            colliders, 0,
            "nothing under the wreck may keep a collider before the grace lands"
        );
        assert!(
            app.world().get::<ChunkGrace>(piece).is_some(),
            "and the one shape it does get is still waiting on the grace"
        );
        assert_eq!(
            app.world().get::<ChildOf>(art).map(|parent| parent.0),
            Some(piece),
            "delivery guard: the art still came across"
        );
    }

    /// A body with no collider cannot become a body of its own. It leaves
    /// NOTHING and is still taken off the field: there used to be a burst of
    /// generic cubes here, and it made a silent failure look like a working
    /// finale.
    #[test]
    fn a_body_with_no_collider_leaves_nothing_and_is_still_reaped() {
        let mut app = finale_app(7);
        let body = app
            .world_mut()
            .spawn((
                ExplodableEntity,
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();

        app.world_mut()
            .entity_mut(body)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert_eq!(wreckage(&mut app).len(), 0, "there was no body to detach");
        assert!(
            !app.world().entities().contains(body),
            "and it is despawned rather than left standing at zero health"
        );
    }

    /// A root carries `ExplodableEntity` by propagation, and its descendants are
    /// whole integrity NODES rather than its own art. Detaching it would take
    /// the entire structure off in one piece and rob every node of its own
    /// death.
    ///
    /// Run over BOTH kinds of root, because the exclusion used to name
    /// `SpaceshipRootMarker` and there is more than one root: a severed wreck
    /// body carries `ShipWreckFragmentMarker` and no ship marker at all.
    #[test]
    fn no_kind_of_root_detaches_the_structure_hanging_off_it() {
        for ship in [true, false] {
            let mut app = finale_app(7);
            let (root, _) = body_drawing_through_descendants(&mut app, 2);
            // An IntegrityRoot either way; only one of the two is a spaceship.
            app.world_mut().entity_mut(root).insert(IntegrityRoot);
            if ship {
                app.world_mut().entity_mut(root).insert(SpaceshipRootMarker);
            }

            app.world_mut()
                .entity_mut(root)
                .insert(IntegrityDestroyMarker);
            app.update();

            assert_eq!(
                wreckage(&mut app).len(),
                0,
                "a root leaves nothing of its own (spaceship: {ship})"
            );
            assert!(
                !app.world().entities().contains(root),
                "but the reaper still takes it - nothing else would (spaceship: {ship})"
            );
        }
    }

    /// A body's wreck leaves WITH it. Without the inherited motion a ship dying
    /// at speed leaves its pieces hanging where it was hit while the wreck
    /// carries on out from under them, which reads as debris being spawned
    /// rather than shed.
    ///
    /// The velocity is read at the SHIP, not at the section: a section is a
    /// child of the rigid body and has none of its own.
    #[test]
    fn a_bodys_piece_leaves_with_the_body() {
        let mut app = finale_app(7);
        let ship = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::IDENTITY,
                LinearVelocity(Vec3::Z * 40.0),
            ))
            .id();
        let (body, _) = body_drawing_through_descendants(&mut app, 1);
        app.world_mut().entity_mut(body).insert(ChildOf(ship));

        app.world_mut()
            .entity_mut(body)
            .insert(IntegrityDestroyMarker);
        app.update();

        let pieces = wreckage(&mut app);
        assert_eq!(pieces.len(), 1, "delivery guard: it came apart");
        let velocity = app
            .world()
            .get::<LinearVelocity>(pieces[0])
            .expect("a wreck carries a velocity")
            .0;
        assert!(
            velocity.z > 30.0,
            "a piece must carry the ship's drift, not just its own kick: {velocity}"
        );
    }

    /// Destructible but not explodable: a skin plate comes off with the section
    /// it is bolted to rather than becoming a body. Nothing in the finale claims
    /// it, so the reaper must, or it stands at zero health with a live collider.
    #[test]
    fn a_destructible_that_is_not_explodable_is_still_reaped() {
        let mut app = finale_app(7);
        let plate = app
            .world_mut()
            .spawn((Transform::default(), GlobalTransform::IDENTITY))
            .id();

        app.world_mut()
            .entity_mut(plate)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert!(!app.world().entities().contains(plate));
        assert_eq!(wreckage(&mut app).len(), 0, "and it does not detach");
    }

    /// The kick and the tumble come off the SEEDED stream, so the same death
    /// plays the same way twice. The slicer this replaced drew from the thread
    /// RNG, which made every section death a different one.
    #[test]
    fn the_same_death_tumbles_the_same_way_twice() {
        let leaving = |seed: u64| {
            let mut app = finale_app(seed);
            let (body, _) = body_drawing_through_descendants(&mut app, 1);
            app.world_mut()
                .entity_mut(body)
                .insert(IntegrityDestroyMarker);
            app.update();
            let piece = *wreckage(&mut app).first().expect("the section detached");
            (
                app.world().get::<LinearVelocity>(piece).unwrap().0,
                app.world().get::<AngularVelocity>(piece).unwrap().0,
            )
        };

        let (velocity, spin) = leaving(20260818);
        assert_eq!(
            (velocity, spin),
            leaving(20260818),
            "the same seed must produce the same death"
        );
        assert!(
            spin.length() > 0.5,
            "delivery guard: a wreck leaves tumbling, not sliding ({spin})"
        );
        assert_ne!(
            (velocity, spin),
            leaving(4242),
            "delivery guard: the tumble is drawn from the stream at all"
        );
    }
}
