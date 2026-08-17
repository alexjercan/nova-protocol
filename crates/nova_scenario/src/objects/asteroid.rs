//! The asteroid scenario object: config, spawn bundle, mesh and texture
//! selection, collider and gravity well, and the husk despawn after a break.
//!
//! Radius drives mass, gravity and collider together, so the scenario author
//! sets one number rather than four that can disagree.
//!
//! Touch this module when changing what an authored asteroid spawns as.

use avian3d::prelude::*;
use bevy::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_events::prelude::{CommandsGameEventExt, *};
use nova_gameplay::prelude::*;
use nova_hud::prelude::*;
use nova_ship::prelude::*;

use super::asteroid_surface::prelude::{
    AsteroidSurfaceMaterial, AsteroidSurfaceMaterialExt, RockHeight,
};

/// The asteroid scenario object and its config, the radius, mass, mesh and texture components, the
/// geometric-factor bounds and `AsteroidPlugin`.
pub mod prelude {
    pub use super::{
        asteroid_scenario_object, asteroid_seed_from_id, AsteroidConfig, AsteroidInvulnerable,
        AsteroidMarker, AsteroidMass, AsteroidPlugin, AsteroidRadius, AsteroidRenderMesh,
        AsteroidSeed, AsteroidTexture, PlanetHeight, PlanetHeightNoise,
        ASTEROID_GEOMETRIC_FACTOR_MAX, ASTEROID_GEOMETRIC_FACTOR_MIN, ASTEROID_TYPE_NAME,
    };
}

/// The scenario/modding RON type name for an asteroid object.
pub const ASTEROID_TYPE_NAME: &str = "asteroid";

/// The scenario/modding RON surface for an asteroid object: a noise-generated
/// rock with health, textures, impact/destroy sounds, and optional gravity and
/// lock-signature overrides. Passed to [`asteroid_scenario_object`] to build the
/// asteroid-root bundle.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AsteroidConfig {
    /// Nominal radius (world units); drives well qualification and mesh scale.
    pub radius: f32,
    /// Surface texture. Authored as an asset path; resolved to a live handle
    /// at spawn time (see `insert_asteroid_render`).
    pub texture: AssetRef<Image>,
    /// Hit points; ignored when `invulnerable` is set.
    pub health: f32,
    /// The sound a hit on this rock plays. Authorable asset ref;
    /// AUTHORED-OR-SILENT. Snapshotted into [`ImpactDestroySounds`] on the
    /// asteroid parent (the audio observers walk up from the Health node).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub impact_sound: Option<AssetRef<AudioSource>>,
    /// The sound this rock's destruction plays; same rules.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub destroy_sound: Option<AssetRef<AudioSource>>,
    /// Mass parameter (`mu`, u^3/s^2) making this body a gravity well:
    /// `a = mu / r^2`, and the sphere of influence is where that decays to
    /// [`GravitySettings::soi_cutoff_accel`]. `Some` always makes this
    /// asteroid a well (subject to the
    /// [`GravitySettings::max_surface_gravity`] cap), even below the radius
    /// threshold; `None` falls back to the global rule
    /// ([`GravitySettings::default_mass`] when
    /// `radius >= GravitySettings::min_well_radius`, no well otherwise).
    ///
    /// Tune this by the SOI you want, not by a number that means anything on
    /// its own: mass is invisible in game, the sphere of influence is what a
    /// pilot feels. `mu = soi_cutoff_accel * soi^2`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub mass: Option<f32>,
    /// An invulnerable body gets its collider WITHOUT a health node:
    /// nothing can disable or destroy it, so its gravity well can never
    /// die mid-scenario (playtest 2026-07-12 finding 6 - a tutorial
    /// planetoid shot to death takes the whole orbit beat with it).
    /// `health` is ignored when set.
    pub invulnerable: bool,
    /// Radar signature override; `None` = the radius (a rock locks in
    /// proportion to its size). A scenario body meant to be designated from
    /// afar authors what it needs.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lock_signature: Option<f32>,
    /// Silhouette seed for the noise mesh. `Some` pins the generated shape -
    /// and with it the derived geometric `BodyRadius` - so content that
    /// authors clearances around this rock (patrol lanes, orbit gates) holds
    /// on every load. `None` derives one from the object's own id
    /// ([`asteroid_seed_from_id`]): still a different silhouette per rock, but
    /// the SAME one on every load, where it used to be a fresh draw from the
    /// global RNG per spawn. `ScatterObjects` fills this deterministically
    /// from its own seed, so scattered fields are stable without authoring
    /// per-rock seeds.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub seed: Option<u32>,
}

/// The silhouette seed an asteroid gets when its config authors none: a stable
/// hash (FNV-1a) of the scenario object's own id.
///
/// Derived rather than drawn from the global RNG so the rock is BUILT IN THE
/// SAME COMMAND BATCH as its body - see [`asteroid_scenario_object`] for why
/// that matters - and stable rather than fresh per spawn, which is what a
/// re-run capture and a reloaded save both want. Ids are unique within a
/// scenario, so rocks in a field still differ from each other.
pub fn asteroid_seed_from_id(id: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in id.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Build the whole asteroid onto `entity`: the root (marker, radius, sounds,
/// lock signature, body) AND its collider/health node, from one
/// [`AsteroidConfig`] and a resolved silhouette `seed`.
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
/// for a handful of its rocks every run (task 20260817-091716).
///
/// The seed is resolved by the CALLER because the mesh is generated here: an
/// authored seed wins, and an unseeded rock derives one from its id through
/// [`asteroid_seed_from_id`] rather than the global RNG, which a bundle built
/// inside a command has no access to.
pub fn asteroid_scenario_object(entity: &mut EntityCommands, config: AsteroidConfig, seed: u32) {
    debug!("asteroid_scenario_object: config {:?} seed {seed}", config);

    // The ROCK generator, not the planet one: signed, so the surface is cut
    // into as well as grown out of, and stretched per seed so a rock has a long
    // axis. See `asteroid_surface` for why a planet generator made every rock
    // look like a ball with lumps on it.
    let rock = RockHeight::default().with_seed(seed).sampler();
    let mesh = TriangleMeshBuilder::new_octahedron(3)
        .apply_noise(&rock)
        .build();
    let collider = Collider::trimesh_from_mesh(&mesh).unwrap_or(Collider::sphere(1.0));

    // The true geometric radius, from the collider volume itself: the
    // noise displaces the unit sphere's vertices OUTWARD (PlanetHeight is
    // non-negative), so the rock's real edge sits past the nominal radius
    // - sometimes far past. Everything that measures from the surface
    // (GOTO standoff, orbit clearance) reads this derived BodyRadius, not
    // the designation radius (2026-07-10 playtest: "still stops too
    // close"). The child mesh is unit-scale, scaled by `radius` on its
    // Transform, so the world extent is radius * the outermost vertex.
    let unit_extent = mesh_max_vertex_radius(&mesh).max(1.0);
    let radius = config.radius;

    entity.insert((
        AsteroidMarker,
        EntityTypeName::new(ASTEROID_TYPE_NAME),
        AsteroidTexture(config.texture),
        AsteroidRadius(radius),
        AsteroidHealth(config.health),
        ImpactDestroySounds {
            impact: config.impact_sound.clone(),
            destroy: config.destroy_sound.clone(),
        },
        AsteroidInvulnerable(config.invulnerable),
        AsteroidMass(config.mass),
        AsteroidSeed(seed),
        // The lock scanner sees a rock in proportion to its size: field
        // rocks only lock up close, big bodies from afar (well sources
        // are range-free in the targeting gate anyway). An authored
        // override wins (the shakedown derelict).
        LockSignature(config.lock_signature.unwrap_or(radius)),
        // Asteroids are worth scoping in the target inset (a physical combat
        // body, unlike a nav beacon), so flag them zoomable.
        InsetZoomable,
        RigidBody::Dynamic,
        // Physics advances Transform only on fixed ticks (64 Hz by default);
        // everything watched by the render-rate camera must interpolate between
        // them or it stair-steps. A field rock drifts and tumbles under the
        // smoothed chase camera, so it needs this even though the well sources
        // insert_asteroid_gravity_well puts on rails do not.
        TransformInterpolation,
        // The DERIVED surface, not the designation radius. Its `Add` is also
        // what sequences the gravity well after this build.
        BodyRadius(radius * unit_extent),
    ));

    entity.with_children(|parent| {
        let mut node = parent.spawn((
            Transform::from_scale(Vec3::splat(radius)),
            AsteroidRenderMesh(mesh),
            collider,
            ConnectedTo::default(),
            ColliderDensity(1.0),
            Visibility::Inherited,
        ));
        if !config.invulnerable {
            // An invulnerable rock gets NO Health: the integrity pipeline has
            // nothing to deplete, so the body (and its well) cannot be
            // destroyed. Health + ExplodableEntity is the rest of
            // `destructible_body`, whose density and visibility every node
            // above already carries.
            //
            // `DamageMarks` rides with them, on THIS node rather than on the
            // root: the node is where the mesh and collider are, it is drawn in
            // unit space, and a mark recorded here lands in that same space -
            // which is the frame `AsteroidField` carves in. It also means an
            // invulnerable rock is not carvable, which is the right answer for
            // a body whose whole job is to still be there at the end.
            node.insert((
                Health::new(config.health),
                ExplodableEntity,
                DamageMarks::default(),
            ));
        }
    });
}

/// Marks an asteroid root (a `RigidBody` parent whose collider/health live on a
/// child node). Inserted by `asteroid_scenario_object`; the asteroid observers
/// key on it to derive the collider, gravity well, and destruction handling.
#[derive(Component, Clone, Debug, Reflect)]
#[require(IntegrityRoot)]
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
/// from [`AsteroidConfig::radius`]. Drives well qualification and mesh scale;
/// the true geometric extent is the separately derived `BodyRadius`.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidRadius(pub f32);

/// The asteroid's authored hit points (from [`AsteroidConfig::health`]), carried
/// on the root and used to build the collider node's `Health` for destructible
/// bodies. Ignored for invulnerable asteroids.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidHealth(pub f32);

/// See [`AsteroidConfig::invulnerable`].
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidInvulnerable(pub bool);

/// The scenario's authored mass parameter for this asteroid (see
/// [`AsteroidConfig::mass`]). Consumed by `insert_asteroid_gravity_well`
/// when the asteroid spawns.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidMass(pub Option<f32>);

/// The RESOLVED silhouette seed this rock was generated from: the authored
/// [`AsteroidConfig::seed`] when there is one, otherwise the id-derived
/// [`asteroid_seed_from_id`]. Carried on the root so a reader can tell which
/// silhouette it is looking at.
#[derive(Component, Clone, Copy, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidSeed(pub u32);

/// Marks an asteroid root whose collider/health node has been destroyed, so its
/// now-empty `RigidBody` husk is despawned next frame (see `despawn_asteroid_husk`).
#[derive(Component, Clone, Debug, Default, Reflect)]
struct AsteroidHuskDespawn;

/// The asteroid scenario object: generates a noise-displaced rock, derives its
/// collider and (for big/authored bodies) a gravity well, and handles its
/// destruction. `render` gates the visible mesh; physics and gravity apply
/// regardless.
/// Seeds [`GravitySettings`], adds the collider/gravity-well/node-destroyed
/// observers plus the `Update` husk-despawn system, and (when `render`) the
/// render-insert observer.
pub struct AsteroidPlugin {
    /// Whether to add the render-insert observer that builds the visible mesh (false for headless tools).
    pub render: bool,
}

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        debug!("AsteroidPlugin: build");

        // NOTE: the gravity layer normally initializes this (NovaGravityPlugin);
        // init here too so the asteroid observer works in scenario-only apps.
        app.init_resource::<GravitySettings>();

        app.add_observer(insert_asteroid_gravity_well);
        app.add_observer(on_asteroid_node_destroyed);
        app.add_systems(Update, despawn_asteroid_husk);
        if self.render {
            // The triplanar rock material, which is what a rock is drawn with
            // whether or not it has ever been carved.
            app.add_plugins(MaterialPlugin::<AsteroidSurfaceMaterial>::default());
            app.add_observer(insert_asteroid_render);
        }
    }
}

/// When an asteroid's collider/health node is destroyed, mark the asteroid root
/// for despawn AND fire the scenario's OnDestroyed under the ROOT's id. An
/// asteroid is a `RigidBody::Dynamic` parent whose `Collider` + `Health` live
/// on a child node; once that node explodes and despawns, the parent is an
/// empty body with no collider, and the invisible husk would linger until the
/// scenario unloads. Marking (rather than despawning here) defers the despawn
/// to `despawn_asteroid_husk`
/// so the destruction observers - which spawn the explosion fragments and
/// despawn the node - all run first.
///
/// The EVENT must fire from here: the integrity pipeline's own bridge
/// (nova_gameplay explode.rs `on_destroyed_entity`) reads `EntityId` off the
/// MARKED entity, and for asteroids the marked entity is the id-less child
/// node - so no asteroid ever fired OnDestroyed, and the shakedown's derelict
/// beat soft-locked on a kill the script never heard about. Putting the marker
/// on the root instead would fire the bridge but ALSO trip the meshless
/// insta-despawn observer, racing the fragment spawn this deferral protects.
fn on_asteroid_node_destroyed(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_node: Query<&ChildOf, With<IntegrityDestroyMarker>>,
    q_asteroid: Query<(Option<&EntityId>, Option<&EntityTypeName>), With<AsteroidMarker>>,
) {
    let Ok(ChildOf(parent)) = q_node.get(add.entity) else {
        return;
    };
    if let Ok((id, type_name)) = q_asteroid.get(*parent) {
        trace!(
            "on_asteroid_node_destroyed: marking asteroid husk {:?}",
            parent
        );
        // On rails in the same insert: the node carried the rock's only
        // collider, so from here until the reaper the root is a dynamic body
        // with no mass, and avian says so once per rock. Static is what a
        // body awaiting removal already is (task 20260817-091716).
        commands
            .entity(*parent)
            .try_insert((AsteroidHuskDespawn, RigidBody::Static));
        // Editor previews carry no scenario id: husk cleanup still runs,
        // only the event is skipped.
        if let (Some(id), Some(type_name)) = (id, type_name) {
            commands.fire::<OnDestroyedEvent>(OnDestroyedEventInfo {
                id: id.to_string(),
                type_name: type_name.to_string(),
            });
        }
    }
}

/// Despawn asteroid roots whose node was destroyed last frame, clearing the
/// empty `RigidBody` husk. The husk is already on rails by then (see
/// `on_asteroid_node_destroyed`); this is what actually removes it.
fn despawn_asteroid_husk(mut commands: Commands, q_husk: Query<Entity, With<AsteroidHuskDespawn>>) {
    for husk in &q_husk {
        trace!("despawn_asteroid_husk: despawning {:?}", husk);
        commands.entity(husk).try_despawn();
    }
}

/// Designate qualifying asteroids as gravity wells: an authored
/// [`AsteroidMass`] always makes a well at that mass; otherwise big rocks
/// (nominal radius at or above [`GravitySettings::min_well_radius`]) get a
/// [`GravitySettings::default_mass`] well and the small field rocks stay flat
/// space. Strength and SOI derive from the mass alone through
/// [`GravityWell::from_mass`]; the GEOMETRIC [`BodyRadius`] the collider
/// observer derives is passed rather than the nominal designation radius
/// because it is the real surface - what the pull clamps at and what the
/// escapability cap is measured against (a well sized on the nominal radius
/// cannot contain an orbit band above the real surface - the 2026-07-10 "no
/// stable band" regression). Triggering on
/// `On<Add, BodyRadius>` is what sequences this after the collider derivation;
/// qualification stays keyed on the nominal radius (the designation intent,
/// seed-independent). The well goes on the asteroid root - which never carries
/// `GravityAffected`, so wells stay one-way and the field cannot clump - and
/// the source is put on rails (`RigidBody::Static`, overriding the asteroid
/// bundle's Dynamic): a well that rams, blasts, or recoil could shove
/// around would drag its SOI and every orbit in it along (spike option B,
/// "bodies on rails"). Small well-less rocks stay dynamic.
fn insert_asteroid_gravity_well(
    add: On<Add, BodyRadius>,
    mut commands: Commands,
    settings: Res<GravitySettings>,
    q_asteroid: Query<(&AsteroidRadius, &BodyRadius, &AsteroidMass), With<AsteroidMarker>>,
) {
    let entity = add.entity;
    // BodyRadius on non-asteroid entities is legitimate (any sized
    // scenario object); only designated rocks become wells.
    let Ok((radius, body_radius, authored)) = q_asteroid.get(entity) else {
        return;
    };

    let mu = match **authored {
        Some(mass) => mass,
        None if **radius >= settings.min_well_radius => settings.default_mass,
        None => return,
    };

    commands.entity(entity).insert((
        GravityWell::from_mass(mu, **body_radius, &settings),
        RigidBody::Static,
    ));
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
    mut plain: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    q_render: Query<(&AsteroidRenderMesh, &ChildOf)>,
    q_asteroid: Query<&AsteroidTexture, With<AsteroidMarker>>,
) {
    let entity = add.entity;
    trace!("insert_asteroid_render: entity {:?}", entity);

    let Ok((render_mesh, ChildOf(asteroid))) = q_render.get(entity) else {
        error!(
            "insert_asteroid_render: entity {:?} not found in q_render",
            entity
        );
        return;
    };

    let Ok(texture) = q_asteroid.get(*asteroid) else {
        error!(
            "insert_asteroid_render: entity {:?} not found in q_asteroid",
            entity
        );
        return;
    };

    let mesh = (**render_mesh).clone();
    // The texture goes to the EXTENSION, not to `base_color_texture`: the
    // standard sampler would read it through the mesh UVs, and reading it
    // through the mesh UVs is exactly what made a rock look quilted and made a
    // carved rock wear a different texture scale from an uncarved one. The
    // standard material keeps its tint, which the extension multiplies into.
    let image = texture.resolve(&asset_server);
    let material = AsteroidSurfaceMaterial {
        base: StandardMaterial::default(),
        extension: AsteroidSurfaceMaterialExt::new(image.clone()),
    };

    commands.entity(entity).insert((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(material)),
        // What this rock's pieces are drawn with when it breaks. The triplanar
        // material above is NOT it: that shader samples by the body's own local
        // position, and a fragment is a new body with a new origin, so it would
        // draw the rock's grain from the wrong place. A plainly-mapped copy of
        // the same texture keeps the pieces looking like the rock they came off
        // without pretending they are still part of it.
        FragmentMaterial(plain.add(StandardMaterial {
            base_color_texture: Some(image),
            ..default()
        })),
    ));
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
    use super::*;

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
    /// generator: sweep the production noise + mesh path (the exact
    /// pipeline asteroid_scenario_object runs) across a spread of seeds
    /// and require every factor inside the exported bounds. Content
    /// authored against the derived geometry (the shakedown orbit gate)
    /// cites these consts; a noise retune that widens the real range
    /// fails HERE instead of soft-locking a scenario in the field.
    #[test]
    fn geometric_factor_bounds_hold_across_seeds() {
        let mut lowest = f32::MAX;
        let mut highest = 0.0f32;
        for i in 0..256u32 {
            // Spread the sampled seeds across the u32 space (production
            // seeds come from rng.next_u32(), not small integers).
            let seed = i.wrapping_mul(2654435761);
            let planet = PlanetHeight::default().with_seed(seed).sampler();
            let mesh = TriangleMeshBuilder::new_octahedron(3)
                .apply_noise(&planet)
                .build();
            let factor = mesh_max_vertex_radius(&mesh).max(1.0);
            lowest = lowest.min(factor);
            highest = highest.max(factor);
            assert!(
                (ASTEROID_GEOMETRIC_FACTOR_MIN..=ASTEROID_GEOMETRIC_FACTOR_MAX).contains(&factor),
                "seed {seed}: factor {factor} outside the exported bounds \
                 [{ASTEROID_GEOMETRIC_FACTOR_MIN}, {ASTEROID_GEOMETRIC_FACTOR_MAX}]"
            );
        }
        eprintln!("geometric factor sweep: observed [{lowest}, {highest}] across 256 seeds");
    }

    fn husk_app() -> App {
        let mut app = App::new();
        app.add_observer(on_asteroid_node_destroyed);
        app.add_systems(Update, despawn_asteroid_husk);
        app
    }

    /// The husk stops being simulated the moment it is marked, not when it is
    /// finally reaped: its only collider left with the node, so a dynamic
    /// husk is a massless body avian warns about for the whole gap
    /// (task 20260817-091716).
    #[test]
    fn a_marked_husk_is_on_rails_before_the_reaper_runs() {
        let mut app = App::new();
        // Observer only: the reaper is deliberately NOT registered, so this
        // reads the husk exactly in the gap the warning was emitted from.
        app.add_observer(on_asteroid_node_destroyed);
        let asteroid = app
            .world_mut()
            .spawn((AsteroidMarker, RigidBody::Dynamic))
            .id();
        let node = app.world_mut().spawn(ChildOf(asteroid)).id();

        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert_eq!(
            app.world().get::<RigidBody>(asteroid),
            Some(&RigidBody::Static),
            "a husk awaiting the reaper must not be a dynamic body"
        );
    }

    #[test]
    fn destroying_an_asteroid_node_despawns_the_husk() {
        // The collider/health node is a child of the asteroid root; destroying it must
        // take the now-empty RigidBody husk with it.
        let mut app = husk_app();
        let asteroid = app.world_mut().spawn(AsteroidMarker).id();
        let node = app.world_mut().spawn(ChildOf(asteroid)).id();

        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert!(
            !app.world().entities().contains(asteroid),
            "the asteroid husk should be despawned when its node is destroyed"
        );
    }

    /// Destroying the node fires the scenario's OnDestroyed under the ROOT's
    /// id, through the real handler pipeline.
    #[test]
    fn destroying_an_asteroid_node_fires_on_destroyed_for_the_root() {
        use nova_events::prelude::{EventHandler, GameEventsPlugin};
        use nova_gameplay::prelude::GameObjectives;

        use crate::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(GameEventsPlugin::<NovaEventWorld>::default());
        app.init_resource::<NovaEventWorld>();
        app.init_resource::<GameObjectives>();
        app.add_observer(on_asteroid_node_destroyed);
        app.add_systems(Update, despawn_asteroid_husk);

        let mut handler =
            EventHandler::<NovaEventWorld>::from(crate::events::EventConfig::OnDestroyed);
        // Filter on BOTH the root's id and its type: the asteroid_field
        // scenario (example 03) counts kills by type_name alone, so the
        // fired info's type is part of the contract this pin owns.
        handler.add_filter(EventFilterConfig::Entity(EntityFilterConfig {
            id: Some("derelict".to_string()),
            type_name: Some(ASTEROID_TYPE_NAME.to_string()),
            ..Default::default()
        }));
        handler.add_action(EventActionConfig::VariableSet(VariableSetActionConfig {
            key: "hulk_down".to_string(),
            expression: VariableExpressionNode::new_term(VariableTermNode::new_factor(
                VariableFactorNode::new_literal(VariableLiteral::Boolean(true)),
            )),
        }));
        app.world_mut().spawn(handler);

        let asteroid = app
            .world_mut()
            .spawn((
                AsteroidMarker,
                EntityId::new("derelict".to_string()),
                EntityTypeName::new(ASTEROID_TYPE_NAME),
            ))
            .id();
        let node = app.world_mut().spawn(ChildOf(asteroid)).id();

        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDestroyMarker);
        app.update();
        app.update();

        assert!(
            matches!(
                app.world()
                    .resource::<NovaEventWorld>()
                    .get_variable("hulk_down"),
                Some(VariableLiteral::Boolean(true))
            ),
            "the node's death must reach a handler filtered on the ROOT's id"
        );
    }

    fn gravity_app() -> App {
        let mut app = App::new();
        app.init_resource::<GravitySettings>();
        app.add_observer(insert_asteroid_gravity_well);
        app
    }

    /// Every rock, root AND collider node, in ONE command batch - the whole
    /// point of the builder taking `EntityCommands`. A body that reached a
    /// physics tick before its node landed spent that tick massless
    /// (task 20260817-091716).
    #[test]
    fn the_collider_node_lands_in_the_same_batch_as_the_body() {
        let mut app = App::new();
        let asteroid = spawn_rock(&mut app, rock(20.0, None), 7);

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

    #[test]
    fn body_radius_derives_from_the_generated_collider() {
        // The noise-displaced mesh reaches past the nominal radius, so
        // the geometric BodyRadius is derived from the actual collider
        // volume (outermost vertex), never authored (2026-07-10 playtest:
        // GOTO "still stops too close" when measured from the nominal
        // sphere).
        let mut app = App::new();
        let asteroid = spawn_rock(&mut app, rock(20.0, None), 4242);

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
        let first = spawn_rock(&mut app, rock(10.0, None), 7);
        let second = spawn_rock(&mut app, rock(10.0, None), 7);
        let other = spawn_rock(&mut app, rock(10.0, None), 8);

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

    #[test]
    fn the_well_derives_from_the_geometric_radius() {
        // The well observer, triggered by the BodyRadius the builder derives
        // from the mesh, sizes the well on the GEOMETRIC radius - a well
        // sized on the nominal sphere cannot contain an orbit band above
        // the real surface (2026-07-10 "no stable band" regression).
        let mut app = App::new();
        app.init_resource::<GravitySettings>();
        app.add_observer(insert_asteroid_gravity_well);

        let settings = GravitySettings::default();
        let asteroid = spawn_rock(&mut app, rock(20.0, Some(45_000.0)), 11);
        app.update();

        let derived = app
            .world()
            .get::<BodyRadius>(asteroid)
            .map(|r| **r)
            .expect("derived BodyRadius");
        let well = app
            .world()
            .get::<GravityWell>(asteroid)
            .expect("designated rock well");
        assert_eq!(well.body_radius, derived);
        // The well is built on the GEOMETRIC radius, but only its surface
        // clamp cares: the mass sets mu and the SOI outright, so both are the
        // same numbers on every mesh seed.
        assert_eq!(well.mu, 45_000.0);
        assert_eq!(
            well.soi_radius,
            (45_000.0f32 / settings.soi_cutoff_accel).sqrt()
        );
    }

    /// An invulnerable body's collider node carries NO Health - the
    /// integrity pipeline has nothing to deplete, so the body (and its
    /// well) cannot die mid-scenario (playtest 2026-07-12 finding 6). A
    /// destructible one keeps Health; both keep the collider so physics
    /// and lock targeting are unchanged.
    #[test]
    fn invulnerable_asteroids_get_no_health_node() {
        let mut app = App::new();

        let spawn = |app: &mut App, invulnerable: bool| -> Entity {
            let mut config = rock(20.0, Some(45_000.0));
            config.health = 2000.0;
            config.invulnerable = invulnerable;
            spawn_rock(app, config, 3)
        };
        let tough = spawn(&mut app, true);
        let normal = spawn(&mut app, false);

        let child_of = |app: &mut App, root: Entity| -> Entity {
            app.world()
                .get::<Children>(root)
                .and_then(|children| children.iter().next())
                .expect("the builder spawns the node child")
        };
        let tough_node = child_of(&mut app, tough);
        let normal_node = child_of(&mut app, normal);

        assert!(
            app.world().get::<Collider>(tough_node).is_some(),
            "invulnerable bodies still collide"
        );
        assert!(
            app.world().get::<Health>(tough_node).is_none(),
            "no Health on an invulnerable body's node - nothing to deplete"
        );
        assert!(
            app.world().get::<Health>(normal_node).is_some(),
            "delivery guard: a destructible body's node does carry Health"
        );
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

    /// The authored config every rock test starts from.
    fn rock(radius: f32, mass: Option<f32>) -> AsteroidConfig {
        AsteroidConfig {
            impact_sound: None,
            destroy_sound: None,
            radius,
            texture: AssetRef::default(),
            health: 100.0,
            mass,
            invulnerable: false,
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

    /// The derived geometric surface the well is sized on.
    fn body_radius(app: &App, entity: Entity) -> f32 {
        app.world()
            .get::<BodyRadius>(entity)
            .map(|radius| **radius)
            .expect("derived BodyRadius")
    }

    #[test]
    fn a_big_rock_gets_a_default_well_and_a_field_rock_gets_none() {
        let mut app = gravity_app();
        let settings = GravitySettings::default();
        let big = spawn_rock(&mut app, rock(20.0, None), 21);
        let small = spawn_rock(&mut app, rock(2.0, None), 22);
        app.update();

        // The default mass, through the same constructor the observer uses,
        // measured against the rock's OWN derived surface: qualification is
        // on the nominal radius, the well's numbers are not.
        let well = app.world().get::<GravityWell>(big).expect("big rock well");
        let expected =
            GravityWell::from_mass(settings.default_mass, body_radius(&app, big), &settings);
        assert_eq!(well.mu, expected.mu);
        assert_eq!(well.soi_radius, expected.soi_radius);
        assert!(
            app.world().get::<GravityWell>(small).is_none(),
            "field rocks below the radius threshold stay flat space"
        );

        // The lock scanner sees every rock in proportion to its size.
        assert_eq!(
            app.world().get::<LockSignature>(big).map(|s| **s),
            Some(20.0)
        );
        assert_eq!(
            app.world().get::<LockSignature>(small).map(|s| **s),
            Some(2.0)
        );

        // Well sources go on rails so nothing can shove an SOI around;
        // well-less rocks keep the base bundle's dynamic body.
        assert_eq!(
            app.world().get::<RigidBody>(big),
            Some(&RigidBody::Static),
            "a well source must be static"
        );
        assert_eq!(
            app.world().get::<RigidBody>(small),
            Some(&RigidBody::Dynamic)
        );
    }

    #[test]
    fn an_authored_mass_overrides_the_threshold_and_is_capped() {
        let mut app = gravity_app();
        let settings = GravitySettings::default();
        // Authored well on a rock below the threshold: still a well.
        let small = spawn_rock(&mut app, rock(2.0, Some(4.0)), 31);
        // Authored mass beyond the guardrail: capped, not honored.
        let hot = spawn_rock(&mut app, rock(20.0, Some(500_000.0)), 32);
        app.update();

        let small_well = app
            .world()
            .get::<GravityWell>(small)
            .expect("authored well");
        assert_eq!(small_well.mu, 4.0);
        let hot_well = app.world().get::<GravityWell>(hot).expect("capped well");
        // The cap is surface gravity over the rock's real surface, so it
        // reads the derived radius rather than the nominal one.
        let surface = body_radius(&app, hot);
        assert_eq!(
            hot_well.mu,
            settings.max_surface_gravity * surface * surface
        );
    }

    #[test]
    fn destroying_a_non_asteroid_node_leaves_its_parent() {
        // A destroyed node whose parent is not an asteroid (e.g. a ship section under a
        // ship root) must not despawn its parent - the ship dies through its own path.
        let mut app = husk_app();
        let parent = app.world_mut().spawn_empty().id();
        let node = app.world_mut().spawn(ChildOf(parent)).id();

        app.world_mut()
            .entity_mut(node)
            .insert(IntegrityDestroyMarker);
        app.update();

        assert!(
            app.world().entities().contains(parent),
            "a non-asteroid parent must not be despawned by the husk cleanup"
        );
    }
}
