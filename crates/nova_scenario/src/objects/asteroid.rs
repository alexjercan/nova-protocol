//! The asteroid scenario object: config, spawn bundle, mesh and texture
//! selection, collider, and gravity opt-in.
//!
//! Radius drives the mesh, collider and radar signature together, so the
//! scenario author sets one number rather than three that can disagree.
//!
//! Touch this module when changing what an authored asteroid spawns as.

use avian3d::prelude::*;
// Bevy's platform Instant, not std's - `std::time::Instant::now` panics
// on wasm32-unknown-unknown, which this crate ships to.
use bevy::{platform::time::Instant, prelude::*};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use nova_hud::prelude::*;
use nova_ship::prelude::*;

use super::{
    asteroid_carve::{
        pristine_rock_mesh, AsteroidField, AsteroidFieldSeedRequest, AsteroidFieldSeeding,
        AsteroidFieldSnapshot, AsteroidRemesh,
    },
    asteroid_kind::prelude::{asteroid_kind_look, AsteroidKind, AsteroidKindId, ASTEROID_KINDS},
    asteroid_surface::prelude::{AsteroidSurfaceMaterial, AsteroidSurfaceMaterialExt},
};
use crate::mining::prelude::{MinedCanisterQueue, MinedOre};

/// The asteroid scenario object and its config, the prepared-geometry half of
/// the spawn, the radius, mass, mesh and texture components, the
/// geometric-factor bounds, the freeze/thaw pair and `AsteroidPlugin`.
pub mod prelude {
    pub use super::{
        asteroid_scenario_object, asteroid_scenario_object_prepared, asteroid_seed_from_id,
        freeze_asteroid, prepare_asteroid_geometry, prepare_frozen_asteroid, thaw_asteroid,
        AsteroidConfig, AsteroidMarker, AsteroidPlugin, AsteroidRadius, AsteroidRenderMesh,
        AsteroidSeed, AsteroidTexture, FrozenAsteroid, PlanetHeight, PlanetHeightNoise,
        PreparedAsteroid, ASTEROID_GEOMETRIC_FACTOR_MAX, ASTEROID_GEOMETRIC_FACTOR_MIN,
    };
}

/// The scenario/modding RON surface for an asteroid object: a noise-generated
/// rock with geometry-owned durability, textures, sounds, initial velocity,
/// and an optional lock-signature override. Passed to
/// [`asteroid_scenario_object`] to build the asteroid-root bundle.
///
/// Every asteroid is carvable and destructible; that is the type's rule, not
/// an authored field. A body that must survive the scenario is a planet.
///
/// STRICT: an unknown key is a load error. A file still carrying the removed
/// `invulnerable:` gets a refusal naming the key.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AsteroidConfig {
    /// Nominal radius; drives mesh scale.
    pub radius: Meters,
    /// Initial velocity in meters per second on each axis. Required; zero is
    /// an intentional stationary start that remains affected by gravity.
    pub initial_velocity: MetersPerSecond3,
    /// Surface texture. Authored as an asset path; resolved to a live handle
    /// at spawn time (see `insert_asteroid_render`).
    #[reflect(ignore)]
    pub texture: AssetRef<Image>,
    /// What this rock IS - one open id, snapshotted into [`AsteroidKind`] on
    /// the asteroid parent.
    ///
    /// It selects the surface shading through
    /// [`asteroid_kind_look`](super::asteroid_kind::asteroid_kind_look), so an
    /// `ice` body is drawn as ice, and it is where an ore yield attaches when
    /// mining exists. What a round SOUNDS like against it is not here and is
    /// not authored: every kind is [`ImpactSurface::Rock`], because the sample
    /// library knows plate and stone and an ice body is stone.
    ///
    /// REQUIRED, and checked. The base kinds are [`ASTEROID_KINDS`]: `rock`,
    /// `metal`, `ice`, `carbon` and the `plain` control. There is no default
    /// and no fallback - a rock that does not say what it is fails to
    /// deserialize, and one that names a kind nobody ships is a lint error and
    /// a loud refusal at spawn. A body this big in the frame does not get to
    /// be a shrug.
    pub kind: AsteroidKindId,
    /// The sound this rock's destruction plays. Authorable asset ref;
    /// AUTHORED-OR-SILENT, snapshotted into [`DestroySound`] on the same
    /// parent. Per-target, unlike the hit voice: a rock breaking up is one
    /// event whatever broke it.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    #[reflect(ignore)]
    pub destroy_sound: Option<AssetRef<AudioSource>>,
    /// Radar signature override; `None` = the radius (a rock locks in
    /// proportion to its size). A scenario body meant to be designated from
    /// afar authors what it needs. Lock range is
    /// [`signature_range_per_unit`](nova_ship::prelude::TargetingSettings::signature_range_per_unit)
    /// times this - a plain ratio, so 30 reads as 30 m of range per meter of
    /// signature.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lock_signature: Option<Meters>,
    /// Silhouette seed for the noise mesh. `Some` pins the generated shape -
    /// and with it the derived geometric `BodyRadius` - so content that
    /// authors clearances around this rock (patrol lanes, orbit gates) holds
    /// on every load. `None` derives one from the object's own id
    /// ([`asteroid_seed_from_id`]): a different silhouette per rock, and the SAME
    /// one on every load, which a draw from the global RNG per spawn cannot
    /// promise. `ScatterObjects` fills this deterministically
    /// from its own seed, so scattered fields are stable without authoring
    /// per-rock seeds.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub seed: Option<u32>,
}

/// The silhouette seed an asteroid gets when its config authors none: a stable
/// [`Fnv32`] hash of the scenario object's own id.
///
/// Derived rather than drawn from the global RNG so the rock is BUILT IN THE
/// SAME COMMAND BATCH as its body - see [`asteroid_scenario_object`] for why
/// that matters - and stable rather than fresh per spawn, which is what a
/// re-run capture and a reloaded save both want. Ids are unique within a
/// scenario, so rocks in a field still differ from each other.
pub fn asteroid_seed_from_id(id: &str) -> u32 {
    Fnv32::new().write(id.as_bytes()).finish()
}

/// What every rock returns to a scanner before its size is counted, in meters.
///
/// A pebble is not invisible - it is a solid body in a vacuum and a scanner is
/// not a camera - so the model has a floor, and the floor is what makes a
/// 10 m fragment a close-range contact instead of nothing at all.
const ROCK_SIGNATURE_BASE: Meters = Meters(100.0);

/// Signature per meter of true geometric radius. Half: stone reflects poorly
/// next to a hull full of running machinery, so a rock has to be twice a
/// ship's size to answer as loudly.
const ROCK_SIGNATURE_PER_RADIUS: f32 = 0.5;

/// A rock's radar signature from its true geometric size, world units in and
/// world units out.
///
/// Engine units both ways: the caller has the derived `BodyRadius` off the
/// meshed collider, and [`LockSignature`] is compared against an avian
/// position every frame.
fn rock_lock_signature(body_radius: f32) -> f32 {
    ROCK_SIGNATURE_BASE.to_engine() + ROCK_SIGNATURE_PER_RADIUS * body_radius.max(0.0)
}

/// The world-free half of an asteroid: its meshed silhouette, the hull
/// collided against it, and the geometric extent derived from it.
///
/// GEOMETRY only. The [`AsteroidConfig`] is not carried: the caller passes it
/// beside this value at spawn, where [`asteroid_scenario_object_prepared`]
/// refuses a pair prepared for another seed or radius. A prepared planet is
/// the other shape - it carries its config with its visual.
///
/// This is where a rock's spawn cost lives. It needs no `World`, no assets and
/// no commands, so a caller that cannot afford it inside a frame - a streamed
/// world bringing up a whole sector - can produce one on
/// `AsyncComputeTaskPool` and hand the result to
/// [`asteroid_scenario_object_prepared`]. [`asteroid_scenario_object`] is the
/// same work done inline.
///
/// The fields are private and the `seed` and `radius` it was prepared FOR
/// travel with it, because the three geometries are one answer to one
/// question: a rock given someone else's hull would be collided against a
/// shape nobody can see.
#[derive(Debug)]
pub struct PreparedAsteroid {
    /// The silhouette seed this geometry answers for.
    seed: u32,
    /// The nominal radius this geometry answers for.
    radius: Meters,
    /// The pristine rock mesh, in unit space - the collider node scales it by
    /// the nominal radius.
    mesh: Mesh,
    /// The convex hull of `mesh`.
    collider: Collider,
    /// `mesh`'s outermost vertex radius, floored at the unit sphere.
    unit_extent: f32,
    /// The surface a carved frozen rock thaws to, and its trimesh. Only
    /// [`prepare_frozen_asteroid`] sets it; a fresh rock spawns pristine.
    carved: Option<(Mesh, Option<Collider>)>,
}

/// Mesh, hull and geometric extent for one rock.
///
/// PURE: the same `seed` and `radius` give the same geometry, on any thread,
/// in any order, with nothing live. That is what makes it safe to run on a
/// worker.
///
/// Meshed from the rock's own carve field, so an untouched rock and a cratered
/// one are the same shape at the same facet density. See `asteroid_carve` for
/// why building the shipped mesh a second way was a visible pop on the first
/// hit, and `asteroid_surface` for why a planet generator made every rock look
/// like a ball with lumps on it.
///
/// A pristine rock is a noise-displaced ball, so its HULL is what to collide
/// against until something puts a hole in it, and carving is what buys the
/// exact surface back - `carve_surface` rebuilds this collider from the holed
/// mesh. Same laziness `seed_asteroid_fields` already applies to the carve
/// grid, for a sharper reason: avian sleeps only TOUCHING contact pairs, so
/// two belt rocks whose AABBs overlap and whose surfaces never meet stay in
/// the ACTIVE contact set forever and are re-manifolded every step, asleep or
/// not. Trimesh against trimesh is the most expensive manifold parry can be
/// asked for - over the editor sandbox's field the same 52 never-touching
/// pairs cost 21.9 ms a step as trimeshes and 0.10 ms as hulls.
pub fn prepare_asteroid_geometry(seed: u32, radius: Meters) -> PreparedAsteroid {
    let started = Instant::now();
    // Engine boundary: the rock is meshed and collided in world units.
    let mesh = pristine_rock_mesh(seed, radius.to_engine());
    let collider = Collider::convex_hull_from_mesh(&mesh).unwrap_or(Collider::sphere(1.0));
    // The true geometric radius, from the meshed surface itself: a rock's
    // shape function is based several times out from the unit sphere
    // (`ROCK_BASE`), so its real edge sits far past the nominal radius.
    // Everything that measures from the surface (GOTO standoff, orbit
    // clearance) reads the derived BodyRadius, not the designation radius
    // (2026-07-10 playtest: "still stops too close").
    let unit_extent = mesh_max_vertex_radius(&mesh).max(1.0);
    trace!(
        "prepare_asteroid_geometry: seed {seed} at radius {:.1} m meshed and hulled in {:.1} ms",
        radius.get(),
        started.elapsed().as_secs_f32() * 1000.0
    );
    PreparedAsteroid {
        seed,
        radius,
        mesh,
        collider,
        unit_extent,
        carved: None,
    }
}

/// [`prepare_asteroid_geometry`] for a frozen rock, plus the carved surface
/// and trimesh its field meshes to when it was carved.
///
/// PURE, like its pristine half, so a sector worker builds it: the carved
/// geometry is rebuilt from the field rather than kept, which is what lets
/// a frozen rock leave the process.
pub fn prepare_frozen_asteroid(rock: &FrozenAsteroid) -> PreparedAsteroid {
    let mut geometry = prepare_asteroid_geometry(rock.seed, rock.radius);
    geometry.carved = rock.carved.as_ref().map(|carved| match carved {
        CarvedState::Settled(snapshot) | CarvedState::PendingRemesh(snapshot) => {
            snapshot.geometry()
        }
    });
    geometry
}

/// Build the whole asteroid onto `entity`: its root (marker, radius, sounds,
/// initial velocity, lock signature, body) AND its collider/carve node, from
/// one [`AsteroidConfig`] and a resolved silhouette `seed`.
///
/// Prepares the geometry inline and hands it straight to
/// [`asteroid_scenario_object_prepared`], so an authored scenario object
/// spawns in one call and in one command batch. A caller that wants the
/// meshing off the frame prepares first.
///
/// Takes `EntityCommands` rather than returning a bundle, unlike its sibling
/// scenario objects, because the collider node has to land in the SAME command
/// batch as `RigidBody`. avian computes a body's mass twice: once from an
/// `Add<RigidBody>` observer, when no collider is linked yet and the answer is
/// therefore ZERO, and again when the collider link (`ColliderOf`, itself a
/// deferred insert) arrives. A rock whose node was inserted by a LATER observer
/// spent a whole extra command hop in between, and any physics tick that landed
/// in that hop saw a dynamic body with no mass - which is exactly what avian's
/// "has no mass or inertia" warning reports, and it is what the arena logged
/// for a handful of its rocks every run.
///
/// The seed is resolved by the CALLER because the mesh is generated from it:
/// an authored seed wins, and an unseeded rock derives one from its id through
/// [`asteroid_seed_from_id`] rather than the global RNG, which a bundle built
/// inside a command has no access to.
pub fn asteroid_scenario_object(entity: &mut EntityCommands, config: AsteroidConfig, seed: u32) {
    assert!(
        config.initial_velocity.is_finite(),
        "asteroid_scenario_object: initial velocity {:?} is not finite",
        config.initial_velocity
    );
    let geometry = prepare_asteroid_geometry(seed, config.radius);
    asteroid_scenario_object_prepared(entity, config, seed, geometry);
}

/// Build the whole asteroid onto `entity` from geometry someone else already
/// prepared. See [`asteroid_scenario_object`] for why this takes
/// `EntityCommands` and for where `seed` comes from.
///
/// # Panics
///
/// When `geometry` was not prepared for this `seed` and this
/// `config.radius`. The alternative is a rock drawn as one shape and collided
/// as another, which nothing downstream can detect. Also when
/// `config.initial_velocity` is not finite. Invalid physics state is refused
/// here even if a Rust caller skipped content lint.
pub fn asteroid_scenario_object_prepared(
    entity: &mut EntityCommands,
    config: AsteroidConfig,
    seed: u32,
    geometry: PreparedAsteroid,
) {
    assert!(
        config.initial_velocity.is_finite(),
        "asteroid_scenario_object_prepared: initial velocity {:?} is not finite",
        config.initial_velocity
    );
    trace!(
        "asteroid_scenario_object_prepared: config {:?} seed {seed}",
        config
    );
    assert!(
        geometry.seed == seed && geometry.radius == config.radius,
        "asteroid_scenario_object_prepared: geometry was prepared for seed {} at {} m, \
         not seed {seed} at {} m",
        geometry.seed,
        geometry.radius.get(),
        config.radius.get()
    );
    let PreparedAsteroid {
        mesh,
        collider,
        unit_extent,
        carved,
        ..
    } = geometry;
    assert!(
        carved.is_none(),
        "asteroid_scenario_object_prepared: seed {seed} was prepared as a frozen carved rock; \
         thaw it with thaw_asteroid"
    );

    // The child mesh is unit-scale, scaled by `radius` on its Transform, so
    // the world extent is radius * the outermost vertex.
    let radius = config.radius.to_engine();

    // One resolved id for the two components that carry it: what the rock
    // sounds like and what it is drawn as can never disagree, because there is
    // nothing for them to disagree about.
    let kind = config.kind.clone();

    entity.insert((
        AsteroidMarker,
        EntityTypeName::new(ASTEROID_TYPE_NAME),
        // A rock throws rock. Without this the shared carve debris is ship
        // plate, and shooting an asteroid sprayed hot gunmetal chips off it.
        CarveDebris::Rock,
        AsteroidTexture(config.texture),
        AsteroidRadius(radius),
        DestroySound(config.destroy_sound.clone()),
        // Nested so the bundle stays inside the 15-element tuple limit. The
        // two are NOT one fact: the kind is how a rock is shaded, the surface
        // is what a round bites into, and every kind bites the same.
        (AsteroidKind(kind), ImpactSurface::Rock),
        AsteroidSeed(seed),
        // What the rock returns to a scanner: a floor every rock clears
        // plus half its true geometric size, so a field pebble is a
        // close-range contact and a belt body is a landmark. Half, not all:
        // stone is a poor reflector next to a hull full of running
        // machinery. An authored override wins (the shakedown derelict).
        LockSignature(
            config
                .lock_signature
                .map_or(rock_lock_signature(radius * unit_extent), Meters::to_engine),
        ),
        // Asteroids are worth scoping in the target inset (a physical combat
        // body, unlike a nav beacon), so flag them zoomable.
        InsetZoomable,
        RigidBody::Dynamic,
        LinearVelocity(config.initial_velocity.to_engine()),
        GravityAffected,
        // Physics advances Transform only on fixed ticks (64 Hz by default);
        // everything watched by the render-rate camera must interpolate between
        // them or it stair-steps. A field rock drifts and tumbles under the
        // smoothed chase camera, so it needs this interpolation.
        TransformInterpolation,
        // The DERIVED surface, not the designation radius.
        BodyRadius(radius * unit_extent),
    ));

    entity.with_children(|parent| {
        parent.spawn((
            Transform::from_scale(Vec3::splat(radius)),
            AsteroidRenderMesh(mesh),
            collider,
            // Rock stops radio. A lock is a radio link, so this hull is what
            // takes the lock scanner's line of sight away - on the COLLIDER,
            // because that is what the scanner's ray meets.
            RadarOccluder,
            ConnectedTo::default(),
            ColliderDensity(1.0),
            Visibility::Inherited,
            // The field is the rock's only durability. `DamageMarks` rides on
            // THIS node because its mesh and collider use the same unit space.
            // Collision events are explicit now that no Health component opts
            // the collider into the generic ram-damage observer.
            DamageMarks::default(),
            CollisionEventsEnabled,
        ));
    });
}

/// Marks an asteroid root (a `RigidBody` parent whose collider/field live on a
/// child node). Inserted by `asteroid_scenario_object` for asteroid consumers.
#[derive(Component, Clone, Debug, Reflect)]
pub struct AsteroidMarker;

/// The asteroid's surface texture ref (from [`AsteroidConfig::texture`]),
/// carried on the root; `insert_asteroid_render` resolves it to a live handle
/// for the generated mesh's material.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidTexture(#[reflect(ignore)] pub AssetRef<Image>);

/// The noise-generated asteroid mesh, placed on the collider child by
/// [`asteroid_scenario_object`]; `insert_asteroid_render` keys on its `Add` to
/// build the rendered `Mesh3d` + material.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidRenderMesh(pub Mesh);

/// The asteroid's authored NOMINAL radius (world units), carried on the root
/// from [`AsteroidConfig::radius`]. Drives mesh scale and carving;
/// the true geometric extent is the separately derived `BodyRadius`.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidRadius(pub f32);

/// The RESOLVED silhouette seed this rock was generated from: the authored
/// [`AsteroidConfig::seed`] when there is one, otherwise the id-derived
/// [`asteroid_seed_from_id`]. Carried on the root so a reader can tell which
/// silhouette it is looking at.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidSeed(pub u32);

/// A streamed asteroid's whole state while its sector is frozen: everything a
/// [`thaw_asteroid`] spawn needs to put back the SAME rock - the one the
/// player left carved, marked and owed ore the way they left it - rather than
/// a fresh one at the same seed.
///
/// Built by [`freeze_asteroid`]; consumed once by [`thaw_asteroid`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FrozenAsteroid {
    seed: u32,
    radius: Meters,
    kind: AsteroidKindId,
    texture: AssetRef<Image>,
    destroy_sound: Option<AssetRef<AudioSource>>,
    lock_signature: f32,
    body_radius: f32,
    damage_marks: DamageMarks,
    carved: Option<CarvedState>,
    pending_seed: bool,
    mined_ore: Option<MinedOre>,
    mined_queue: Option<MinedCanisterQueue>,
}

impl FrozenAsteroid {
    /// The silhouette seed its pristine geometry is prepared from.
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The nominal radius its pristine geometry is prepared for.
    pub fn radius(&self) -> Meters {
        self.radius
    }
}

/// The carved field a frozen rock's node held, and whether its last remesh
/// had already landed.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum CarvedState {
    /// The field's mesh and collider were already validated, drawn and
    /// collided with; nothing is in flight.
    Settled(AsteroidFieldSnapshot),
    /// A remesh job was in flight when the rock froze. The dropped job cost
    /// nothing real - every mark is already subtracted into the live field in
    /// place (see `asteroid_carve::carve_asteroid_fields`), so only the mesh
    /// and collider swap was still pending - and thaw only has to make
    /// `carve_asteroid_fields` queue that swap again.
    PendingRemesh(AsteroidFieldSnapshot),
}

/// Carries a thawed carved rock's surface onto its freshly spawned node.
///
/// `insert_asteroid_render`'s `Add<AsteroidRenderMesh>` observer draws this
/// mesh instead of the pristine one, so a carved rock thaws drawn as it froze
/// instead of popping back to its pristine silhouette for a frame. It has to
/// ride in the SAME spawn bundle as `AsteroidRenderMesh`: the observer fires
/// as that bundle lands, before any later, separately-queued `Mesh3d` insert
/// would apply, so a later insert loses the race rather than winning it.
/// Consumed and removed by the observer the same flush.
#[derive(Component, Clone, Debug, Deref, DerefMut)]
struct ThawedCarvedMesh(Mesh);

/// Snapshot an asteroid's whole live state so its sector can despawn it and
/// [`thaw_asteroid`] can put the same rock back later.
///
/// # Panics
///
/// When `asteroid` is not an [`AsteroidMarker`] root, or has no collider
/// node - both mean the caller resolved the wrong entity.
pub fn freeze_asteroid(world: &World, asteroid: Entity) -> Result<FrozenAsteroid, UnsettledBody> {
    assert!(
        world.get::<AsteroidMarker>(asteroid).is_some(),
        "freeze_asteroid: {asteroid:?} is not an AsteroidMarker"
    );
    let node = world
        .get::<Children>(asteroid)
        .and_then(|children| {
            children
                .iter()
                .find(|child| world.get::<DamageMarks>(*child).is_some())
        })
        .unwrap_or_else(|| panic!("freeze_asteroid: {asteroid:?} has no collider node"));

    if world.get::<IntegrityDestroyMarker>(node).is_some() {
        return Err(UnsettledBody {
            reason: "freeze_asteroid: the node is exhausting",
        });
    }

    let seed = world
        .get::<AsteroidSeed>(asteroid)
        .expect("freeze_asteroid: an AsteroidMarker root carries AsteroidSeed")
        .0;
    let radius = Meters::from_engine(
        world
            .get::<AsteroidRadius>(asteroid)
            .expect("freeze_asteroid: an AsteroidMarker root carries AsteroidRadius")
            .0,
    );
    let kind = world
        .get::<AsteroidKind>(asteroid)
        .expect("freeze_asteroid: an AsteroidMarker root carries AsteroidKind")
        .0
        .clone();
    let texture = world
        .get::<AsteroidTexture>(asteroid)
        .expect("freeze_asteroid: an AsteroidMarker root carries AsteroidTexture")
        .0
        .clone();
    let destroy_sound = world
        .get::<DestroySound>(asteroid)
        .expect("freeze_asteroid: an AsteroidMarker root carries DestroySound")
        .0
        .clone();
    let lock_signature = world
        .get::<LockSignature>(asteroid)
        .expect("freeze_asteroid: an AsteroidMarker root carries LockSignature")
        .0;
    let body_radius = world
        .get::<BodyRadius>(asteroid)
        .expect("freeze_asteroid: an AsteroidMarker root carries BodyRadius")
        .0;
    let damage_marks = world
        .get::<DamageMarks>(node)
        .expect("freeze_asteroid: the node carries DamageMarks")
        .clone();

    let pending_seed = world.get::<AsteroidFieldSeedRequest>(node).is_some()
        || world.get::<AsteroidFieldSeeding>(node).is_some();
    let carved = if pending_seed {
        None
    } else {
        world.get::<AsteroidField>(node).map(|field| {
            if world.get::<AsteroidRemesh>(node).is_some() {
                CarvedState::PendingRemesh(field.snapshot())
            } else {
                CarvedState::Settled(field.snapshot())
            }
        })
    };
    Ok(FrozenAsteroid {
        seed,
        radius,
        kind,
        texture,
        destroy_sound,
        lock_signature,
        body_radius,
        damage_marks,
        carved,
        pending_seed,
        mined_ore: world.get::<MinedOre>(node).copied(),
        mined_queue: world.get::<MinedCanisterQueue>(node).cloned(),
    })
}

/// Rebuild a frozen asteroid's root and collider node onto `entity`, exactly
/// as [`freeze_asteroid`] found them: its carved field (or its still-pristine
/// one), its damage marks, its owed ore, and - for a rock that had been
/// carved - the surface and trimesh its field meshes to.
///
/// `geometry` comes from [`prepare_frozen_asteroid`], off-thread: the rock's
/// pristine geometry for the frozen seed and radius, and for a carved rock
/// the surface and trimesh rebuilt from its field, which replace the pristine
/// mesh and hull the same way a live carve does on its first hit.
///
/// Inserts no velocity: the caller already knows the rock's last
/// `LinearVelocity`/`AngularVelocity` and inserts them itself.
///
/// # Panics
///
/// When `geometry` was not prepared for `frozen`'s own seed and radius - the
/// same guard [`asteroid_scenario_object_prepared`] makes, for the same
/// reason: a rock drawn as one shape and collided as another is a defect
/// nothing downstream can detect. Also when `geometry` disagrees with
/// `frozen` about whether the rock was carved, and when a carved field meshes
/// to no usable trimesh: the live carve keeps its prior collider then, and a
/// thawed rock has none to keep.
pub fn thaw_asteroid(
    entity: &mut EntityCommands,
    frozen: FrozenAsteroid,
    geometry: PreparedAsteroid,
) {
    assert!(
        geometry.seed == frozen.seed && geometry.radius == frozen.radius,
        "thaw_asteroid: geometry was prepared for seed {} at {} m, not seed {} at {} m",
        geometry.seed,
        geometry.radius.get(),
        frozen.seed,
        frozen.radius.get(),
    );
    let FrozenAsteroid {
        seed,
        radius,
        kind,
        texture,
        destroy_sound,
        lock_signature,
        body_radius,
        damage_marks,
        carved,
        pending_seed,
        mined_ore,
        mined_queue,
    } = frozen;
    let PreparedAsteroid {
        mesh,
        collider: pristine_collider,
        carved: carved_geometry,
        ..
    } = geometry;
    assert_eq!(
        carved.is_some(),
        carved_geometry.is_some(),
        "thaw_asteroid: seed {seed} geometry was prepared for a rock carved {}, not carved {}",
        carved_geometry.is_some(),
        carved.is_some(),
    );
    let (node_collider, carved_mesh) = match carved_geometry {
        Some((surface, Some(collider))) => (collider, Some(surface)),
        Some((_, None)) => panic!(
            "thaw_asteroid: seed {seed}'s carved field meshes to no usable trimesh, and a \
             thawed rock has no prior collider to keep"
        ),
        None => (pristine_collider, None),
    };
    let radius_engine = radius.to_engine();

    entity.insert((
        AsteroidMarker,
        EntityTypeName::new(ASTEROID_TYPE_NAME),
        CarveDebris::Rock,
        AsteroidTexture(texture),
        AsteroidRadius(radius_engine),
        DestroySound(destroy_sound),
        (AsteroidKind(kind), ImpactSurface::Rock),
        AsteroidSeed(seed),
        LockSignature(lock_signature),
        InsetZoomable,
        RigidBody::Dynamic,
        GravityAffected,
        TransformInterpolation,
        BodyRadius(body_radius),
    ));

    let mut node_entity = Entity::PLACEHOLDER;
    entity.with_children(|parent| {
        node_entity = match carved_mesh {
            Some(surface) => parent
                .spawn((
                    Transform::from_scale(Vec3::splat(radius_engine)),
                    AsteroidRenderMesh(mesh),
                    node_collider,
                    RadarOccluder,
                    ConnectedTo::default(),
                    ColliderDensity(1.0),
                    Visibility::Inherited,
                    damage_marks,
                    CollisionEventsEnabled,
                    ThawedCarvedMesh(surface),
                ))
                .id(),
            None => parent
                .spawn((
                    Transform::from_scale(Vec3::splat(radius_engine)),
                    AsteroidRenderMesh(mesh),
                    node_collider,
                    RadarOccluder,
                    ConnectedTo::default(),
                    ColliderDensity(1.0),
                    Visibility::Inherited,
                    damage_marks,
                    CollisionEventsEnabled,
                ))
                .id(),
        };
    });

    let mut commands = entity.commands();
    let mut node = commands.entity(node_entity);
    if pending_seed {
        node.insert(AsteroidFieldSeedRequest);
    }
    match carved {
        Some(CarvedState::Settled(snapshot)) => {
            node.insert(snapshot.restored());
        }
        Some(CarvedState::PendingRemesh(snapshot)) => {
            node.insert(snapshot.restored_pending_remesh());
        }
        None => {}
    }
    if let Some(ore) = mined_ore {
        node.insert(ore);
    }
    if let Some(queue) = mined_queue {
        node.insert(queue);
    }
}

/// The asteroid scenario object: generates a noise-displaced rock and derives
/// its collider. Asteroids opt in to gravity as dynamic bodies. Geometry-owned
/// destruction lives in `asteroid_carve`; `render` gates the visible mesh.
pub struct AsteroidPlugin {
    /// Whether to add the render-insert observer that builds the visible mesh (false for headless tools).
    pub render: bool,
}

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        trace!("AsteroidPlugin: build");

        if self.render {
            // The triplanar rock material, which is what a rock is drawn with
            // whether or not it has ever been carved.
            app.add_plugins(MaterialPlugin::<AsteroidSurfaceMaterial>::default());
            app.add_observer(insert_asteroid_render);
        }
    }
}

/// Bounds on the unit-mesh geometric factor: how far the noise-displaced
/// asteroid mesh reaches past its nominal unit sphere, across seeds. The
/// derived `BodyRadius` is `nominal_radius * factor`, and everything sized
/// from it (SOI = 8x, orbit ring = 1.5x) inherits the spread - so CONTENT
/// that authors distances around a designated body (the shakedown orbit
/// gate, beacon placement) must hold across this whole range, not one
/// observed seed. Pinned by the seed-sweep test below
/// (`geometric_factor_bounds_hold_across_seeds`): observed [3.70, 5.64]
/// across 256 seeds spread over the u32 space; the consts carry margin on
/// both sides. Widen only with the sweep's numbers in hand.
pub const ASTEROID_GEOMETRIC_FACTOR_MIN: f32 = 3.5;
/// Upper bound of the geometric factor (see [`ASTEROID_GEOMETRIC_FACTOR_MIN`]).
pub const ASTEROID_GEOMETRIC_FACTOR_MAX: f32 = 6.0;

/// The outermost vertex distance of a mesh, in its local space: the
/// radius of the smallest origin-centered sphere containing the collider
/// volume. Zero for a mesh without positions. Pure for unit testing.
fn mesh_max_vertex_radius(mesh: &Mesh) -> f32 {
    use bevy::render::mesh::VertexAttributeValues;
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(positions)) => positions
            .iter()
            .map(|p| Vec3::from_array(*p).length())
            .fold(0.0, f32::max),
        _ => 0.0,
    }
}

fn insert_asteroid_render(
    add: On<Add, AsteroidRenderMesh>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<AsteroidSurfaceMaterial>>,
    asset_server: Res<AssetServer>,
    q_render: Query<(&AsteroidRenderMesh, &ChildOf, Option<&ThawedCarvedMesh>)>,
    q_asteroid: Query<(&AsteroidTexture, &AsteroidKind, &AsteroidSeed), With<AsteroidMarker>>,
) {
    let entity = add.entity;
    trace!("insert_asteroid_render: entity {:?}", entity);

    let Ok((render_mesh, ChildOf(asteroid), frozen_mesh)) = q_render.get(entity) else {
        error!(
            "insert_asteroid_render: entity {:?} not found in q_render",
            entity
        );
        return;
    };

    let Ok((texture, kind, seed)) = q_asteroid.get(*asteroid) else {
        error!(
            "insert_asteroid_render: entity {:?} not found in q_asteroid",
            entity
        );
        return;
    };

    // The texture goes to the EXTENSION, not to `base_color_texture`: the
    // standard sampler would read it through the mesh UVs, and reading it
    // through the mesh UVs is exactly what made a rock look quilted and made a
    // carved rock wear a different texture scale from an uncarved one. The
    // standard material keeps its tint, which the extension multiplies into.
    //
    // The material is built ONCE per body and the carve path re-applies the
    // same handle to a remeshed rock and to every piece it throws, so the kind
    // and the per-body jitter survive being shot at without anything having to
    // rebuild them.
    // No house look for an id nobody ships. The lint, the spawn actions and the
    // world validator all refuse one before the rock exists; this is the
    // backstop for a factory caller that skipped them. The rock keeps its
    // collider and gets no mesh, and the log names the body and the id.
    let Some(look) = asteroid_kind_look(kind) else {
        error!(
            "insert_asteroid_render: asteroid {:?} is made of '{}', which is not a kind. \
             Author one of {:?}.",
            *asteroid, **kind, ASTEROID_KINDS
        );
        return;
    };

    let image = texture.resolve(&asset_server);
    let material = AsteroidSurfaceMaterial {
        base: StandardMaterial::default(),
        extension: AsteroidSurfaceMaterialExt::new(image, &look, **seed),
    };

    // A thawed carved rock carries its carved surface rather than the
    // pristine mesh `render_mesh` names - see `ThawedCarvedMesh`. Consumed
    // here so a later `Add<AsteroidRenderMesh>` (there never is one, but
    // nothing should rely on that) does not draw a stale surface.
    let mesh_handle = match frozen_mesh {
        Some(carved) => meshes.add((**carved).clone()),
        None => meshes.add((**render_mesh).clone()),
    };
    commands
        .entity(entity)
        .insert((Mesh3d(mesh_handle), MeshMaterial3d(materials.add(material))));
    if frozen_mesh.is_some() {
        commands.entity(entity).remove::<ThawedCarvedMesh>();
    }
}

/// Planet seed. Change this to generate a different planet.
const CURRENT_SEED: u32 = 0;

/// Scale of the planet. Change this to zoom in or out.
const ZOOM_SCALE: f64 = 0.1;

/// Frequency of the planet's continents. Higher frequency produces
/// smaller, more numerous continents. This value is measured in radians.
const CONTINENT_FREQUENCY: f64 = 1.0;

/// Lacunarity of the planet's continents. Changing this value produces
/// slightly different continents. For the best results, this value should
/// be random, but close to 2.0.
const CONTINENT_LACUNARITY: f64 = 2.208984375;

/// Lacunarity of the planet's mountains. Changing the value produces
/// slightly different mountains. For the best results, this value should
/// be random, but close to 2.0.
const MOUNTAIN_LACUNARITY: f64 = 2.142578125;

/// Lacunarity of the planet's hills. Changing this value produces
/// slightly different hills. For the best results, this value should be
/// random, but close to 2.0.
const HILLS_LACUNARITY: f64 = 2.162109375;

/// Lacunarity of the planet's plains. Changing this value produces
/// slightly different plains. For the best results, this value should be
/// random, but close to 2.0.
const PLAINS_LACUNARITY: f64 = 2.314453125;

/// Lacunarity of the planet's badlands. Changing this value produces
/// slightly different badlands. For the best results, this value should
/// be random, but close to 2.0.
const BADLANDS_LACUNARITY: f64 = 2.212890625;

/// Specifies the "twistiness" of the mountains.
const MOUNTAINS_TWIST: f64 = 1.0;

/// Specifies the "twistiness" of the hills.
const HILLS_TWIST: f64 = 1.0;

/// Specifies the "twistiness" of the badlands.
const BADLANDS_TWIST: f64 = 1.0;

/// Specifies the planet's sea level. This value must be between -1.0
/// (minimum planet elevation) and +1.0 (maximum planet elevation).
const SEA_LEVEL: f64 = 0.0;

/// Specifies the level on the planet in which continental shelves appear.
/// This value must be between -1.0 (minimum planet elevation) and +1.0
/// (maximum planet elevation), and must be less than `SEA_LEVEL`.
const SHELF_LEVEL: f64 = -0.375;

/// Determines the amount of mountainous terrain that appears on the
/// planet. Values range from 0.0 (no mountains) to 1.0 (all terrain is
/// covered in mountains). Mountains terrain will overlap hilly terrain.
/// Because the badlands terrain may overlap parts of the mountainous
/// terrain, setting `MOUNTAINS_AMOUNT` to 1.0 may not completely cover the
/// terrain in mountains.
const MOUNTAINS_AMOUNT: f64 = 0.5;

/// Determines the amount of hilly terrain that appears on the planet.
/// Values range from 0.0 (no hills) to 1.0 (all terrain is covered in
/// hills). This value must be less than `MOUNTAINS_AMOUNT`. Because the
/// mountains terrain will overlap parts of the hilly terrain, and the
/// badlands terrain may overlap parts of the hilly terrain, setting
/// `HILLS_AMOUNT` to 1.0 may not completely cover the terrain in hills.
const HILLS_AMOUNT: f64 = (1.0 + MOUNTAINS_AMOUNT) / 2.0;

/// Determines the amount of badlands terrain that covers the planet.
/// Values range from 0.0 (no badlands) to 1.0 (all terrain is covered in
/// badlands). Badlands terrain will overlap any other type of terrain.
const BADLANDS_AMOUNT: f64 = 0.3125;

/// Offset to apply to the terrain type definition. Low values (< 1.0)
/// cause the rough areas to appear only at high elevations. High values
/// (> 2.0) cause the rough areas to appear at any elevation. The
/// percentage of rough areas on the planet are independent of this value.
const TERRAIN_OFFSET: f64 = 1.0;

/// Specifies the amount of "glaciation" on the mountains. This value
/// should be close to 1.0 and greater than 1.0.
const MOUNTAIN_GLACIATION: f64 = 1.375;

/// Scaling to apply to the base continent elevations, in planetary
/// elevation units.
const CONTINENT_HEIGHT_SCALE: f64 = (1.0 - SEA_LEVEL) / 4.0;

/// Maximum depth of the rivers, in planetary elevation units.
const RIVER_DEPTH: f64 = 0.0234375;

/// The parameter set for the Perlin-FBM terrain noise that displaces an
/// asteroid's unit sphere (seed plus the continent/mountain/hills/etc tuning
/// constants). Also a `NoiseFn`; [`asteroid_scenario_object`] builds a
/// per-asteroid `PlanetHeight::default().with_seed(..)` to shape the mesh.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlanetHeight {
    /// Noise seed; a different seed yields a different rock (see `CURRENT_SEED`).
    pub seed: u32,
    /// Sample-space scale: zooms the noise in or out (see `ZOOM_SCALE`).
    pub zoom_scale: f64,
    /// Continent frequency (radians): higher yields smaller, more numerous continents.
    pub continent_frequency: f64,
    /// Continent lacunarity; best near 2.0 (see `CONTINENT_LACUNARITY`).
    pub continent_lacunarity: f64,
    /// Mountain lacunarity; best near 2.0 (see `MOUNTAIN_LACUNARITY`).
    pub mountain_lacunarity: f64,
    /// Hills lacunarity; best near 2.0 (see `HILLS_LACUNARITY`).
    pub hills_lacunarity: f64,
    /// Plains lacunarity; best near 2.0 (see `PLAINS_LACUNARITY`).
    pub plains_lacunarity: f64,
    /// Badlands lacunarity; best near 2.0 (see `BADLANDS_LACUNARITY`).
    pub badlands_lacunarity: f64,
    /// "Twistiness" of the mountains (see `MOUNTAINS_TWIST`).
    pub mountains_twist: f64,
    /// "Twistiness" of the hills (see `HILLS_TWIST`).
    pub hills_twist: f64,
    /// "Twistiness" of the badlands (see `BADLANDS_TWIST`).
    pub badlands_twist: f64,
    /// Sea level in planet elevation units, -1.0 to +1.0 (see `SEA_LEVEL`).
    pub sea_level: f64,
    /// Elevation where continental shelves appear; below `sea_level` (see `SHELF_LEVEL`).
    pub shelf_level: f64,
    /// Fraction of terrain covered in mountains, 0.0 to 1.0 (see `MOUNTAINS_AMOUNT`).
    pub mountains_amount: f64,
    /// Fraction of terrain covered in hills, below `mountains_amount` (see `HILLS_AMOUNT`).
    pub hills_amount: f64,
    /// Fraction of terrain covered in badlands, 0.0 to 1.0 (see `BADLANDS_AMOUNT`).
    pub badlands_amount: f64,
    /// Offset to the terrain-type definition (see `TERRAIN_OFFSET`).
    pub terrain_offset: f64,
    /// Mountain "glaciation" amount, close to and above 1.0 (see `MOUNTAIN_GLACIATION`).
    pub mountain_glaciation: f64,
    /// Scaling of base continent elevations, in planet elevation units (see `CONTINENT_HEIGHT_SCALE`).
    pub continent_height_scale: f64,
    /// Maximum river depth, in planet elevation units (see `RIVER_DEPTH`).
    pub river_depth: f64,
}

impl Default for PlanetHeight {
    fn default() -> Self {
        PlanetHeight {
            seed: CURRENT_SEED,
            zoom_scale: ZOOM_SCALE,
            continent_frequency: CONTINENT_FREQUENCY,
            continent_lacunarity: CONTINENT_LACUNARITY,
            mountain_lacunarity: MOUNTAIN_LACUNARITY,
            hills_lacunarity: HILLS_LACUNARITY,
            plains_lacunarity: PLAINS_LACUNARITY,
            badlands_lacunarity: BADLANDS_LACUNARITY,
            mountains_twist: MOUNTAINS_TWIST,
            hills_twist: HILLS_TWIST,
            badlands_twist: BADLANDS_TWIST,
            sea_level: SEA_LEVEL,
            shelf_level: SHELF_LEVEL,
            mountains_amount: MOUNTAINS_AMOUNT,
            hills_amount: HILLS_AMOUNT,
            badlands_amount: BADLANDS_AMOUNT,
            terrain_offset: TERRAIN_OFFSET,
            mountain_glaciation: MOUNTAIN_GLACIATION,
            continent_height_scale: CONTINENT_HEIGHT_SCALE,
            river_depth: RIVER_DEPTH,
        }
    }
}

impl PlanetHeight {
    /// Return a copy of these parameters with the noise seed replaced.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Build the sampler these parameters describe.
    ///
    /// The graph is assembled ONCE here and then sampled, which is the whole
    /// point of the split: `Fbm::new` seeds a permutation table per octave, and
    /// this graph carries 25 of them, so assembling it per sample cost ~80 us a
    /// vertex - about 125 ms for one 1536-vertex rock and ~11 s for the 88 rocks
    /// a chapter scatters, all inside the single frame that spawns them.
    pub fn sampler(&self) -> PlanetHeightNoise {
        PlanetHeightNoise::new(self)
    }
}

/// The assembled Perlin-FBM graph for one [`PlanetHeight`] parameter set, and
/// the only thing that samples it.
///
/// Build one per asteroid ([`PlanetHeight::sampler`]) and hand it to
/// `TriangleMeshBuilder::apply_noise`. Not [`Clone`] or [`Send`]: the graph ends
/// in a `noise::Cache`, whose last-sample memo is a `Cell`.
pub struct PlanetHeightNoise {
    /// The assembled graph, boxed because its concrete type is a ~7-layer
    /// nesting of `noise` combinators that no signature wants to name.
    graph: Box<dyn NoiseFn<f64, 3>>,
    /// Sample-space scale, applied to the point before the graph sees it.
    zoom_scale: f64,
}

impl PlanetHeightNoise {
    fn new(params: &PlanetHeight) -> Self {
        _ = params.mountain_lacunarity; // Silence unused warning
        _ = params.hills_lacunarity; // Silence unused warning
        _ = params.plains_lacunarity; // Silence unused warning
        _ = params.badlands_lacunarity; // Silence unused warning
        _ = params.mountains_twist; // Silence unused warning
        _ = params.hills_twist; // Silence unused warning
        _ = params.badlands_twist; // Silence unused warning
        _ = params.shelf_level; // Silence unused warning
        _ = params.mountain_glaciation; // Silence unused warning
        _ = params.river_depth; // Silence unused warning
        _ = params.terrain_offset; // Silence unused warning
        _ = params.hills_amount; // Silence unused warning
        _ = params.mountains_amount; // Silence unused warning
        _ = params.badlands_amount; // Silence unused warning
        _ = params.continent_height_scale; // Silence unused warning

        let (seed, sea_level) = (params.seed, params.sea_level);
        let continent_frequency = params.continent_frequency;
        let continent_lacunarity = params.continent_lacunarity;

        // Example taken from
        // <https://github.com/Razaekel/noise-rs/blob/develop/examples/complexplanet.rs>

        // 1: [Continent module]: This FBM module generates the continents. This
        // noise function has a high number of octaves so that detail is visible at
        // high zoom levels.
        let base_continent_def_fb0 = Fbm::<Perlin>::new(seed)
            .set_frequency(continent_frequency)
            .set_persistence(0.5)
            .set_lacunarity(continent_lacunarity)
            .set_octaves(14);

        // 2: [Continent-with-ranges module]: Next, a curve module modifies the
        // output value from the continent module so that very high values appear
        // near sea level. This defines the positions of the mountain ranges.
        let base_continent_def_cu = noise::Curve::new(base_continent_def_fb0)
            .add_control_point(-2.0000 + sea_level, -1.625 + sea_level)
            .add_control_point(-1.0000 + sea_level, -1.375 + sea_level)
            .add_control_point(0.0000 + sea_level, -0.375 + sea_level)
            .add_control_point(0.0625 + sea_level, 0.125 + sea_level)
            .add_control_point(0.1250 + sea_level, 0.250 + sea_level)
            .add_control_point(0.2500 + sea_level, 1.000 + sea_level)
            .add_control_point(0.5000 + sea_level, 0.250 + sea_level)
            .add_control_point(0.7500 + sea_level, 0.250 + sea_level)
            .add_control_point(1.0000 + sea_level, 0.500 + sea_level)
            .add_control_point(2.0000 + sea_level, 0.500 + sea_level);

        // 3: [Carver module]: This higher-frequency BasicMulti module will be
        // used by subsequent noise functions to carve out chunks from the
        // mountain ranges within the continent-with-ranges module so that the
        // mountain ranges will not be completely impassible.
        let base_continent_def_fb1 = Fbm::<Perlin>::new(seed + 1)
            .set_frequency(continent_frequency * 4.34375)
            .set_persistence(0.5)
            .set_lacunarity(continent_lacunarity)
            .set_octaves(11);

        // 4: [Scaled-carver module]: This scale/bias module scales the output
        // value from the carver module such that it is usually near 1.0. This
        // is required for step 5.
        let base_continent_def_sb = noise::ScaleBias::new(base_continent_def_fb1)
            .set_scale(0.375)
            .set_bias(0.625);

        // 5: [Carved-continent module]: This minimum-value module carves out
        // chunks from the continent-with-ranges module. it does this by ensuring
        // that only the minimum of the output values from the scaled-carver
        // module and the continent-with-ranges module contributes to the output
        // value of this subgroup. Most of the time, the minimum value module will
        // select the output value from the continent-with-ranges module since the
        // output value from the scaled-carver is usually near 1.0. Occasionally,
        // the output from the scaled-carver module will be less than the output
        // value from the continent-with-ranges module, so in this case, the output
        // value from the scaled-carver module is selected.
        let base_continent_def_mi = noise::Min::new(base_continent_def_sb, base_continent_def_cu);

        // 6: [Clamped-continent module]: Finally, a clamp module modifies the
        // carved continent module to ensure that the output value of this subgroup
        // is between -1.0 and 1.0.
        let base_continent_def_cl = noise::Clamp::new(base_continent_def_mi).set_bounds(-1.0, 1.0);

        // 7: [Base-continent-definition subgroup]: Caches the output value from
        // the clamped-continent module.
        let base_continent_def = noise::Cache::new(base_continent_def_cl);

        Self {
            graph: Box::new(base_continent_def),
            zoom_scale: params.zoom_scale,
        }
    }

    /// Sample the terrain-height noise at a point on the unit sphere.
    pub fn get_point(&self, point: Vec3) -> f64 {
        let x = point.x as f64 * self.zoom_scale;
        let y = point.y as f64 * self.zoom_scale;
        let z = point.z as f64 * self.zoom_scale;

        let noise = self.graph.get([x, y, z]);
        ((noise + 1.0) * 0.5) * 5.0
    }
}

impl NoiseFn<f64, 3> for PlanetHeightNoise {
    fn get(&self, point: [f64; 3]) -> f64 {
        let vec = Vec3::new(point[0] as f32, point[1] as f32, point[2] as f32);
        self.get_point(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::{super::asteroid_kind::prelude::KIND_ROCK, *};

    /// A REUSED sampler answers every point the way a fresh one would.
    ///
    /// The graph ends in a `noise::Cache`, which memoises the LAST point it was
    /// asked about. That memo never mattered while the graph was rebuilt per
    /// sample; now that one graph answers 1536 vertices, a revisited point must
    /// still come back with its own value rather than a neighbour's - so sample
    /// A, then B, then A again, and require the two A's to agree with a fresh
    /// sampler's A.
    #[test]
    fn a_reused_sampler_answers_every_point_the_same() {
        let sampler = PlanetHeight::default().with_seed(4242).sampler();
        let (a, b) = (Vec3::new(0.3, -0.7, 0.65), Vec3::new(-0.9, 0.1, 0.42));

        let first = sampler.get_point(a);
        let between = sampler.get_point(b);
        let again = sampler.get_point(a);

        assert_ne!(first, between, "delivery guard: the two points differ");
        assert_eq!(first, again, "a revisited point keeps its own value");
        assert_eq!(
            first,
            PlanetHeight::default()
                .with_seed(4242)
                .sampler()
                .get_point(a),
            "a reused sampler agrees with a fresh one"
        );
    }

    /// Pin ASTEROID_GEOMETRIC_FACTOR_MIN/MAX against the real mesh
    /// generator: sweep the production mesh path (the exact pipeline
    /// asteroid_scenario_object runs) across a spread of seeds and require
    /// every factor inside the exported bounds. Content authored against the
    /// derived geometry (the shakedown orbit gate) cites these consts; a noise
    /// retune that widens the real range fails HERE instead of soft-locking a
    /// scenario in the field.
    ///
    /// Fewer seeds than the sweep it replaced: meshing a rock from its field
    /// costs a hundred times what displacing an octahedron did, and the
    /// analytic sweep in `asteroid_surface` covers the noise across seeds at
    /// full width. This one is here to catch the MESH losing reach against the
    /// function it is meshing.
    ///
    /// Swept at BOTH ends of the size range, because a rock's grid is now sized
    /// in world units: the smallest rock is meshed on the coarsest grid the
    /// floor allows and the biggest on the cap, and a coarse grid is exactly
    /// what loses a peak. One radius would only pin one resolution.
    #[test]
    fn geometric_factor_bounds_hold_across_seeds() {
        for radius in [0.8f32, 5.0] {
            let mut lowest = f32::MAX;
            let mut highest = 0.0f32;
            for i in 0..12u32 {
                // Spread the sampled seeds across the u32 space (production
                // seeds are FNV hashes of object ids, not small integers).
                let seed = i.wrapping_mul(2654435761);
                let factor = mesh_max_vertex_radius(&pristine_rock_mesh(seed, radius)).max(1.0);
                lowest = lowest.min(factor);
                highest = highest.max(factor);
                assert!(
                    (ASTEROID_GEOMETRIC_FACTOR_MIN..=ASTEROID_GEOMETRIC_FACTOR_MAX)
                        .contains(&factor),
                    "seed {seed} at radius {radius}: factor {factor} outside the exported \
                     bounds [{ASTEROID_GEOMETRIC_FACTOR_MIN}, {ASTEROID_GEOMETRIC_FACTOR_MAX}]"
                );
            }
            eprintln!(
                "geometric factor sweep at radius {radius}: observed [{lowest}, {highest}] \
                 across 12 seeds"
            );
        }
    }

    /// Every rock, root AND collider node, in ONE command batch - the whole
    /// point of the builder taking `EntityCommands`. A body that reached a
    /// physics tick before its node landed spent that tick massless.
    #[test]
    fn the_collider_node_lands_in_the_same_batch_as_the_body() {
        let mut app = App::new();
        let asteroid = spawn_rock(&mut app, rock(Meters(200.0)), 7);

        // No update() anywhere: everything below is true the moment the
        // spawning batch has been applied.
        assert!(
            app.world().get::<RigidBody>(asteroid).is_some(),
            "the body is on the root"
        );
        let node = app
            .world()
            .get::<Children>(asteroid)
            .and_then(|children| children.iter().next())
            .expect("the collider node is a child of the root");
        assert!(
            app.world().get::<Collider>(node).is_some(),
            "the collider is on the node, in the same batch as the body"
        );
        assert!(
            app.world().get::<BodyRadius>(asteroid).is_some(),
            "the derived surface lands with the body too"
        );
    }

    /// Rock is cover. The marker has to ride the collider node, because the
    /// lock scanner's ray meets colliders and never the body root.
    #[test]
    fn a_rock_hull_stops_the_radar() {
        let mut app = App::new();
        let asteroid = spawn_rock(&mut app, rock(Meters(200.0)), 7);

        let node = app
            .world()
            .get::<Children>(asteroid)
            .and_then(|children| children.iter().next())
            .expect("the collider node is a child of the root");
        assert!(
            app.world().get::<RadarOccluder>(node).is_some(),
            "a rock has to take a lock's line of sight away"
        );
        assert!(
            app.world().get::<RadarOccluder>(asteroid).is_none(),
            "the root a lock NAMES must not be what hides things behind it"
        );
    }

    /// A pristine rock collides as a HULL. avian sleeps only TOUCHING contact
    /// pairs, so a belt's never-touching neighbours stay in the active set and
    /// are re-manifolded on every step for as long as the scene is loaded -
    /// which trimesh against trimesh cannot pay for. Carving is what buys the
    /// exact surface back, on the rocks that have actually been shot.
    #[test]
    fn a_pristine_rock_collides_as_a_hull() {
        let mut app = App::new();
        let asteroid = spawn_rock(&mut app, rock(Meters(40.0)), 20_260_819);

        let node = app
            .world()
            .get::<Children>(asteroid)
            .and_then(|children| children.iter().next())
            .expect("the collider node is a child of the root");
        let collider = app
            .world()
            .get::<Collider>(node)
            .expect("the node carries a collider");
        assert!(
            collider.shape().as_convex_polyhedron().is_some(),
            "an unshot rock must collide as a hull, not as its drawn surface"
        );
    }

    #[test]
    fn body_radius_derives_from_the_generated_collider() {
        // The noise-displaced mesh reaches past the nominal radius, so
        // the geometric BodyRadius is derived from the actual collider
        // volume (outermost vertex), never authored (2026-07-10 playtest:
        // GOTO "still stops too close" when measured from the nominal
        // sphere).
        let mut app = App::new();
        let asteroid = spawn_rock(&mut app, rock(Meters(200.0)), 4242);

        let derived = app
            .world()
            .get::<BodyRadius>(asteroid)
            .map(|r| **r)
            .expect("the builder derives BodyRadius");
        assert!(
            derived >= 20.0,
            "the noise only displaces outward, got {derived}"
        );
        assert!(
            derived < 20.0 * 7.0,
            "sanity: bounded by the max noise elevation, got {derived}"
        );
    }

    #[test]
    fn a_seed_pins_the_silhouette_and_a_different_one_moves_it() {
        // Same seed, same rock: the derived geometric BodyRadius is what
        // authored clearances (patrol lanes, orbit gates) are measured
        // against, so it must not drift run to run.
        let mut app = App::new();
        let first = spawn_rock(&mut app, rock(Meters(100.0)), 7);
        let second = spawn_rock(&mut app, rock(Meters(100.0)), 7);
        let other = spawn_rock(&mut app, rock(Meters(100.0)), 8);

        let radius_of = |app: &App, entity: Entity| -> f32 {
            app.world()
                .get::<BodyRadius>(entity)
                .map(|r| **r)
                .expect("derived BodyRadius")
        };
        assert_eq!(
            radius_of(&app, first),
            radius_of(&app, second),
            "one seed, one silhouette"
        );
        assert_ne!(
            radius_of(&app, first),
            radius_of(&app, other),
            "delivery guard: another seed is another rock"
        );
    }

    /// An unseeded rock derives its silhouette from its own id, so it is a
    /// different rock from its neighbour and the SAME rock on the next load.
    #[test]
    fn an_unseeded_rock_derives_a_stable_seed_from_its_id() {
        assert_eq!(
            asteroid_seed_from_id("field_rock_3"),
            asteroid_seed_from_id("field_rock_3"),
            "the same id is the same rock on every load"
        );
        assert_ne!(
            asteroid_seed_from_id("field_rock_3"),
            asteroid_seed_from_id("field_rock_4"),
            "neighbours in a field are not clones"
        );
    }

    /// No asteroid carries a health kill gate. Every rock accepts marks.
    #[test]
    fn asteroid_geometry_is_the_only_durability() {
        let mut app = App::new();
        let root = spawn_rock(&mut app, rock(Meters(200.0)), 3);
        let node = app
            .world()
            .get::<Children>(root)
            .and_then(|children| children.iter().next())
            .expect("the builder spawns the node child");

        assert!(app.world().get::<Health>(node).is_none());
        assert!(app.world().get::<DamageMarks>(node).is_some());
        assert!(
            app.world().get::<CollisionEventsEnabled>(node).is_some(),
            "healthless rocks still need ram collision events"
        );
    }

    /// Vulnerability is the body type's rule, so a file that still authors
    /// the removed flag is refused rather than read as a rock that ignores it.
    #[cfg(feature = "serde")]
    #[test]
    fn an_asteroid_refuses_the_removed_invulnerable_key() {
        let current = r#"(radius: 100.0, initial_velocity: (0.0, 0.0, 0.0), texture: "self://textures/asteroid.png", kind: "rock")"#;
        let stale = r#"(radius: 100.0, initial_velocity: (0.0, 0.0, 0.0), texture: "self://textures/asteroid.png", kind: "rock", invulnerable: true)"#;

        assert!(ron::from_str::<AsteroidConfig>(current).is_ok());
        let error = ron::from_str::<AsteroidConfig>(stale)
            .expect_err("a stale `invulnerable:` key must fail the load")
            .to_string();
        assert!(error.contains("invulnerable"), "{error}");
    }

    #[test]
    #[should_panic(expected = "geometry was prepared for seed")]
    fn prepared_geometry_refuses_a_different_seed() {
        let mut app = App::new();
        let radius = Meters(20.0);
        let geometry = prepare_asteroid_geometry(7, radius);
        let world = app.world_mut();
        let entity = world.spawn_empty().id();
        let mut commands = world.commands();
        let mut entity_commands = commands.entity(entity);
        asteroid_scenario_object_prepared(&mut entity_commands, rock(radius), 8, geometry);
    }

    #[test]
    fn mesh_max_vertex_radius_finds_the_outermost_vertex() {
        let mesh = TriangleMeshBuilder::new_octahedron(1).build();
        let max = mesh_max_vertex_radius(&mesh);
        assert!(
            (max - 1.0).abs() < 1e-4,
            "unit octahedron sphere, got {max}"
        );
    }

    /// The authored config every rock test starts from. The radius is
    /// authored in meters; the assertions below read avian and Bevy, so they
    /// are in world units - a 200 m rock is 20 of them.
    fn rock(radius: Meters) -> AsteroidConfig {
        AsteroidConfig {
            kind: KIND_ROCK.into(),
            destroy_sound: None,
            radius,
            initial_velocity: MetersPerSecond3::ZERO,
            texture: AssetRef::default(),
            seed: None,
            lock_signature: None,
        }
    }

    /// Spawn a rock exactly as the scenario spawn action does: one entity,
    /// one command batch, root and collider node together.
    fn spawn_rock(app: &mut App, config: AsteroidConfig, seed: u32) -> Entity {
        let world = app.world_mut();
        let entity = world.spawn_empty().id();
        {
            let mut commands = world.commands();
            let mut entity_commands = commands.entity(entity);
            asteroid_scenario_object(&mut entity_commands, config, seed);
        }
        world.flush();
        entity
    }

    #[test]
    fn asteroids_opt_into_gravity_as_dynamic_bodies() {
        let mut app = App::new();
        let mut config = rock(Meters(200.0));
        config.initial_velocity = MetersPerSecond3::new(0.0, 0.0, 2500.0);
        let asteroid = spawn_rock(&mut app, config, 21);

        assert_eq!(
            app.world().get::<RigidBody>(asteroid),
            Some(&RigidBody::Dynamic),
            "gravity-affected asteroids remain dynamic bodies"
        );
        assert!(
            app.world().get::<GravityAffected>(asteroid).is_some(),
            "the asteroid root opts into gravity"
        );
        assert_eq!(
            app.world().get::<LinearVelocity>(asteroid),
            Some(&LinearVelocity(Vec3::new(0.0, 0.0, 250.0))),
            "initial velocity is converted from meters per second to engine units"
        );
    }
}
