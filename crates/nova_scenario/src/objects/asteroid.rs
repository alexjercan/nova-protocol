use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_common_systems::prelude::*;
use bevy_rand::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_events::prelude::*;
use nova_gameplay::prelude::*;
use rand::Rng;

pub mod prelude {
    pub use super::{
        asteroid_scenario_object, AsteroidConfig, AsteroidMarker, AsteroidPlugin, AsteroidRadius,
        AsteroidRenderMesh, AsteroidSurfaceGravity, AsteroidTexture, ASTEROID_TYPE_NAME,
    };
}

pub const ASTEROID_TYPE_NAME: &str = "asteroid";

#[derive(Clone, Debug)]
pub struct AsteroidConfig {
    pub radius: f32,
    pub texture: Handle<Image>,
    pub health: f32,
    /// Per-body gravity override, u/s^2 at the surface. `Some` always makes
    /// this asteroid a gravity well at that strength (subject to the
    /// [`GravitySettings::max_surface_gravity`] cap), even below the radius
    /// threshold; `None` falls back to the global rule (a default-strength
    /// well when `radius >= GravitySettings::min_well_radius`, none
    /// otherwise).
    pub surface_gravity: Option<f32>,
}

pub fn asteroid_scenario_object(config: AsteroidConfig) -> impl Bundle {
    debug!("asteroid_scenario_object: config {:?}", config);

    (
        AsteroidMarker,
        EntityTypeName::new(ASTEROID_TYPE_NAME),
        AsteroidTexture(config.texture),
        AsteroidRadius(config.radius),
        AsteroidHealth(config.health),
        AsteroidSurfaceGravity(config.surface_gravity),
        // The lock scanner sees a rock in proportion to its size: field
        // rocks only lock up close, big bodies from afar (well sources
        // are range-free in the targeting gate anyway).
        LockSignature(config.radius),
        // BodyRadius (the surface the GOTO standoff and the orbit band
        // measure from) is NOT authored here: the noise-displaced mesh
        // reaches past the nominal radius, so insert_asteroid_collider
        // derives it from the generated collider's outermost vertex.
    )
}

#[derive(Component, Clone, Debug, Reflect)]
pub struct AsteroidMarker;

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidTexture(pub Handle<Image>);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidRenderMesh(pub Mesh);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidRadius(pub f32);

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidHealth(pub f32);

/// The scenario's authored gravity for this asteroid (see
/// [`AsteroidConfig::surface_gravity`]). Consumed by
/// `insert_asteroid_gravity_well` when the asteroid spawns.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct AsteroidSurfaceGravity(pub Option<f32>);

/// Marks an asteroid root whose collider/health node has been destroyed, so its
/// now-empty `RigidBody` husk is despawned next frame (see `despawn_asteroid_husk`).
#[derive(Component, Clone, Debug, Default, Reflect)]
struct AsteroidHuskDespawn;

pub struct AsteroidPlugin {
    pub render: bool,
}

impl Plugin for AsteroidPlugin {
    fn build(&self, app: &mut App) {
        debug!("AsteroidPlugin: build");

        // The gravity layer normally initializes this (NovaGravityPlugin);
        // init here too so the asteroid observer works in scenario-only apps.
        app.init_resource::<GravitySettings>();

        app.add_observer(insert_asteroid_collider);
        app.add_observer(insert_asteroid_gravity_well);
        app.add_observer(on_asteroid_node_destroyed);
        app.add_systems(Update, despawn_asteroid_husk);
        if self.render {
            app.add_observer(insert_asteroid_render);
        }
    }
}

/// When an asteroid's collider/health node is destroyed, mark the asteroid root for
/// despawn. An asteroid is a `RigidBody::Dynamic` parent whose `Collider` + `Health`
/// live on a child node; once that node explodes and despawns, the parent is an empty
/// dynamic body with no collider - avian then logs "has no mass or inertia" and the
/// invisible husk lingers until the scenario unloads. Marking (rather than despawning
/// here) defers the despawn to `despawn_asteroid_husk` so the destruction observers -
/// which spawn the explosion fragments and despawn the node - all run first.
fn on_asteroid_node_destroyed(
    add: On<Add, IntegrityDestroyMarker>,
    mut commands: Commands,
    q_node: Query<&ChildOf, With<IntegrityDestroyMarker>>,
    q_asteroid: Query<(), With<AsteroidMarker>>,
) {
    let Ok(ChildOf(parent)) = q_node.get(add.entity) else {
        return;
    };
    if q_asteroid.contains(*parent) {
        trace!(
            "on_asteroid_node_destroyed: marking asteroid husk {:?}",
            parent
        );
        commands.entity(*parent).try_insert(AsteroidHuskDespawn);
    }
}

/// Despawn asteroid roots whose node was destroyed last frame, clearing the empty
/// `RigidBody` husk (and silencing avian's mass/inertia warning).
fn despawn_asteroid_husk(mut commands: Commands, q_husk: Query<Entity, With<AsteroidHuskDespawn>>) {
    for husk in &q_husk {
        trace!("despawn_asteroid_husk: despawning {:?}", husk);
        commands.entity(husk).try_despawn();
    }
}

/// Designate qualifying asteroids as gravity wells (spike
/// docs/spikes/20260709-193147-gravity-wells-orbital-mechanics.md): an
/// authored [`AsteroidSurfaceGravity`] always makes a well at that strength;
/// otherwise big rocks (nominal radius at or above
/// [`GravitySettings::min_well_radius`]) get a default-strength well and the
/// small field rocks stay flat space. Strength and SOI derive through
/// [`GravityWell::from_surface_gravity`], which also applies the escapability
/// cap - from the GEOMETRIC [`BodyRadius`] the collider observer derives,
/// not the nominal designation radius: the noise mesh reaches several
/// times past the nominal sphere, and a well whose SOI/fade were sized on
/// the nominal radius cannot contain an orbit band above the real surface
/// (the 2026-07-10 "no stable band" regression). Triggering on
/// `On<Add, BodyRadius>` is what sequences this after the collider
/// derivation; qualification stays keyed on the nominal radius (the
/// designation intent, seed-independent). The well goes on the asteroid
/// root - which never carries `GravityAffected`, so wells stay one-way
/// and the field cannot clump - and the source is put on rails
/// (`RigidBody::Static`, overriding the base scenario bundle's Dynamic):
/// a well that rams, blasts, or recoil could shove around would drag its
/// SOI and every orbit in it along (spike option B, "bodies on rails").
/// Small well-less rocks stay dynamic.
fn insert_asteroid_gravity_well(
    add: On<Add, BodyRadius>,
    mut commands: Commands,
    settings: Res<GravitySettings>,
    q_asteroid: Query<
        (&AsteroidRadius, &BodyRadius, &AsteroidSurfaceGravity),
        With<AsteroidMarker>,
    >,
) {
    let entity = add.entity;
    // BodyRadius on non-asteroid entities is legitimate (any sized
    // scenario object); only designated rocks become wells.
    let Ok((radius, body_radius, authored)) = q_asteroid.get(entity) else {
        return;
    };

    let surface_gravity = match **authored {
        Some(gravity) => gravity,
        None if **radius >= settings.min_well_radius => settings.default_surface_gravity,
        None => return,
    };

    commands.entity(entity).insert((
        GravityWell::from_surface_gravity(surface_gravity, **body_radius, &settings),
        RigidBody::Static,
    ));
}

fn insert_asteroid_collider(
    add: On<Add, AsteroidMarker>,
    mut commands: Commands,
    q_asteroid: Query<(&AsteroidRadius, &AsteroidHealth), With<AsteroidMarker>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let entity = add.entity;
    trace!("insert_asteroid_render: entity {:?}", entity);

    let Ok((radius, health)) = q_asteroid.get(entity) else {
        error!(
            "insert_asteroid_render: entity {:?} not found in q_asteroid",
            entity
        );
        return;
    };

    let planet = PlanetHeight::default().with_seed(rng.next_u32());
    let mesh = TriangleMeshBuilder::new_octahedron(3)
        .apply_noise(&planet)
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
    commands
        .entity(entity)
        .insert(BodyRadius(**radius * unit_extent));

    commands.entity(entity).insert((children![(
        Transform::from_scale(Vec3::splat(**radius)),
        AsteroidRenderMesh(mesh.clone()),
        collider,
        destructible_body(**health, 1.0),
        // destructible_body (bevy_common_systems) is Health + density + visibility; add
        // ExplodableEntity so the asteroid enters nova's explode pipeline on destruction.
        ExplodableEntity,
    )],));
}

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
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    let material = StandardMaterial {
        base_color_texture: Some((**texture).clone()),
        ..default()
    };

    commands.entity(entity).insert((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(material)),
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

#[derive(Resource, Clone, Copy, Debug)]
pub struct PlanetHeight {
    pub seed: u32,
    pub zoom_scale: f64,
    pub continent_frequency: f64,
    pub continent_lacunarity: f64,
    pub mountain_lacunarity: f64,
    pub hills_lacunarity: f64,
    pub plains_lacunarity: f64,
    pub badlands_lacunarity: f64,
    pub mountains_twist: f64,
    pub hills_twist: f64,
    pub badlands_twist: f64,
    pub sea_level: f64,
    pub shelf_level: f64,
    pub mountains_amount: f64,
    pub hills_amount: f64,
    pub badlands_amount: f64,
    pub terrain_offset: f64,
    pub mountain_glaciation: f64,
    pub continent_height_scale: f64,
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
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    pub fn get_point(&self, point: Vec3) -> f64 {
        _ = self.mountain_lacunarity; // Silence unused warning
        _ = self.hills_lacunarity; // Silence unused warning
        _ = self.plains_lacunarity; // Silence unused warning
        _ = self.badlands_lacunarity; // Silence unused warning
        _ = self.mountains_twist; // Silence unused warning
        _ = self.hills_twist; // Silence unused warning
        _ = self.badlands_twist; // Silence unused warning
        _ = self.shelf_level; // Silence unused warning
        _ = self.mountain_glaciation; // Silence unused warning
        _ = self.river_depth; // Silence unused warning
        _ = self.terrain_offset; // Silence unused warning
        _ = self.hills_amount; // Silence unused warning
        _ = self.mountains_amount; // Silence unused warning
        _ = self.badlands_amount; // Silence unused warning
        _ = self.continent_height_scale; // Silence unused warning

        // Example taken from
        // <https://github.com/Razaekel/noise-rs/blob/develop/examples/complexplanet.rs>

        // 1: [Continent module]: This FBM module generates the continents. This
        // noise function has a high number of octaves so that detail is visible at
        // high zoom levels.
        let base_continent_def_fb0 = Fbm::<Perlin>::new(self.seed)
            .set_frequency(self.continent_frequency)
            .set_persistence(0.5)
            .set_lacunarity(self.continent_lacunarity)
            .set_octaves(14);

        // 2: [Continent-with-ranges module]: Next, a curve module modifies the
        // output value from the continent module so that very high values appear
        // near sea level. This defines the positions of the mountain ranges.
        let base_continent_def_cu = noise::Curve::new(base_continent_def_fb0)
            .add_control_point(-2.0000 + self.sea_level, -1.625 + self.sea_level)
            .add_control_point(-1.0000 + self.sea_level, -1.375 + self.sea_level)
            .add_control_point(0.0000 + self.sea_level, -0.375 + self.sea_level)
            .add_control_point(0.0625 + self.sea_level, 0.125 + self.sea_level)
            .add_control_point(0.1250 + self.sea_level, 0.250 + self.sea_level)
            .add_control_point(0.2500 + self.sea_level, 1.000 + self.sea_level)
            .add_control_point(0.5000 + self.sea_level, 0.250 + self.sea_level)
            .add_control_point(0.7500 + self.sea_level, 0.250 + self.sea_level)
            .add_control_point(1.0000 + self.sea_level, 0.500 + self.sea_level)
            .add_control_point(2.0000 + self.sea_level, 0.500 + self.sea_level);

        // 3: [Carver module]: This higher-frequency BasicMulti module will be
        // used by subsequent noise functions to carve out chunks from the
        // mountain ranges within the continent-with-ranges module so that the
        // mountain ranges will not be completely impassible.
        let base_continent_def_fb1 = Fbm::<Perlin>::new(self.seed + 1)
            .set_frequency(self.continent_frequency * 4.34375)
            .set_persistence(0.5)
            .set_lacunarity(self.continent_lacunarity)
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

        let x = point.x as f64 * self.zoom_scale;
        let y = point.y as f64 * self.zoom_scale;
        let z = point.z as f64 * self.zoom_scale;

        let noise = base_continent_def.get([x, y, z]);
        ((noise + 1.0) * 0.5) * 5.0
    }
}

impl NoiseFn<f64, 3> for PlanetHeight {
    fn get(&self, point: [f64; 3]) -> f64 {
        let vec = Vec3::new(point[0] as f32, point[1] as f32, point[2] as f32);
        self.get_point(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn husk_app() -> App {
        let mut app = App::new();
        app.add_observer(on_asteroid_node_destroyed);
        app.add_systems(Update, despawn_asteroid_husk);
        app
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

    fn gravity_app() -> App {
        let mut app = App::new();
        app.init_resource::<GravitySettings>();
        app.add_observer(insert_asteroid_gravity_well);
        app
    }

    #[test]
    fn body_radius_derives_from_the_generated_collider() {
        // The noise-displaced mesh reaches past the nominal radius, so
        // the geometric BodyRadius is derived from the actual collider
        // volume (outermost vertex), never authored (2026-07-10 playtest:
        // GOTO "still stops too close" when measured from the nominal
        // sphere).
        let mut app = App::new();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.add_observer(insert_asteroid_collider);
        // Let the entropy plugin spawn the global rng before the
        // asteroid observer needs it.
        app.update();

        let asteroid = spawn_asteroid(&mut app, 20.0, None);
        app.update();

        let derived = app
            .world()
            .get::<BodyRadius>(asteroid)
            .map(|r| **r)
            .expect("the collider observer derives BodyRadius");
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
    fn the_well_derives_from_the_geometric_radius() {
        // The full observer chain: the collider observer derives
        // BodyRadius from the mesh, and the well observer (triggered by
        // that insert) sizes the well on the GEOMETRIC radius - a well
        // sized on the nominal sphere cannot contain an orbit band above
        // the real surface (2026-07-10 "no stable band" regression).
        let mut app = App::new();
        app.init_resource::<GravitySettings>();
        app.add_plugins(EntropyPlugin::<WyRand>::default());
        app.add_observer(insert_asteroid_collider);
        app.add_observer(insert_asteroid_gravity_well);
        app.update();

        let settings = GravitySettings::default();
        let asteroid = spawn_asteroid_underived(&mut app, 20.0, Some(6.0));
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
        assert_eq!(well.soi_radius, settings.soi_factor * derived);
        assert_eq!(well.mu, 6.0 * derived * derived);
    }

    /// The raw scenario bundle without the test stand-in BodyRadius, for
    /// tests that run the real collider derivation.
    fn spawn_asteroid_underived(
        app: &mut App,
        radius: f32,
        surface_gravity: Option<f32>,
    ) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                asteroid_scenario_object(AsteroidConfig {
                    radius,
                    texture: Handle::default(),
                    health: 100.0,
                    surface_gravity,
                }),
            ))
            .id()
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

    /// Spawn an asteroid the way the scenario does: the base bundle's
    /// dynamic rigid body plus the asteroid components, minus render bits.
    fn spawn_asteroid(app: &mut App, radius: f32, surface_gravity: Option<f32>) -> Entity {
        app.world_mut()
            .spawn((
                RigidBody::Dynamic,
                asteroid_scenario_object(AsteroidConfig {
                    radius,
                    texture: Handle::default(),
                    health: 100.0,
                    surface_gravity,
                }),
                // In the real pipeline the collider observer derives this
                // from the generated mesh; the well tests stand in with a
                // unit extent so mu/SOI expectations stay exact.
                BodyRadius(radius),
            ))
            .id()
    }

    #[test]
    fn a_big_rock_gets_a_default_well_and_a_field_rock_gets_none() {
        let mut app = gravity_app();
        let settings = GravitySettings::default();
        let big = spawn_asteroid(&mut app, 20.0, None);
        let small = spawn_asteroid(&mut app, 2.0, None);
        app.update();

        let well = app.world().get::<GravityWell>(big).expect("big rock well");
        assert_eq!(well.mu, settings.default_surface_gravity * 400.0);
        assert_eq!(well.soi_radius, settings.soi_factor * 20.0);
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
    fn an_authored_surface_gravity_overrides_the_threshold_and_is_capped() {
        let mut app = gravity_app();
        let settings = GravitySettings::default();
        // Authored well on a rock below the threshold: still a well.
        let small = spawn_asteroid(&mut app, 2.0, Some(1.0));
        // Authored strength beyond the guardrail: capped, not honored.
        let hot = spawn_asteroid(&mut app, 20.0, Some(50.0));
        app.update();

        let small_well = app
            .world()
            .get::<GravityWell>(small)
            .expect("authored well");
        assert_eq!(small_well.mu, 1.0 * 4.0);
        let hot_well = app.world().get::<GravityWell>(hot).expect("capped well");
        assert_eq!(hot_well.mu, settings.max_surface_gravity * 400.0);
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
