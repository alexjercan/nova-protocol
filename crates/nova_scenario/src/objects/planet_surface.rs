//! The SHAPE of a planetoid and the material that shades it.
//!
//! `planet_type` owns what a world is made of; this module owns the surface it
//! is made of it ON - a signed height field, the displaced sphere that carries
//! it, and the [`PlanetSurfaceMaterial`] that paints the bands.
//!
//! # Why a big rock reads as a grey repeat
//!
//! A "planetoid" today is an asteroid with a big radius, so it wears
//! [`AsteroidSurfaceMaterial`](super::asteroid_surface::AsteroidSurfaceMaterial):
//! one rock photo, projected triplanar-ly at
//! [`ROCK_TEXTURE_TILING`](super::asteroid_surface::ROCK_TEXTURE_TILING)
//! repeats per world unit. That constant is tuned for a rock a few units
//! across. The menu planetoid is 200 m of nominal radius, which the rock
//! generator reaches about four times past, so the same tile repeats hundreds
//! of times across the body. Every repeat is the same handful of grey pixels,
//! and the eye reads the whole thing as one flat grey crust with a pattern in
//! it. The scale is what breaks, not the texture.
//!
//! So this material samples NO texture. Colour comes from a palette keyed on
//! elevation and latitude, and the variation inside a band comes from a
//! procedural field defined on the body's own direction. A field on the
//! direction has no tile, so there is nothing to repeat however big the body
//! gets.
//!
//! # Why the sphere is displaced and not only shaded
//!
//! Bands alone paint a map onto a ball: the limb stays a perfect circle and
//! the terminator stays a clean arc, and both give the ball away. Displacing
//! the sphere by the same field the bands read costs one pass over the
//! vertices and buys a ragged limb, self-shadowing relief across the
//! terminator, and mountains that are actually where the mountain colour is.
//!
//! The displacement is SIGNED - the surface is cut into as much as it is grown
//! out of - for the reason
//! [`asteroid_surface`](super::asteroid_surface) documents at length: a
//! non-negative height field leaves the base sphere showing wherever it
//! bottoms out.

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin, RidgedMulti};

use super::planet_type::prelude::*;

/// The planet material and its extension, the height field, the mesh builder,
/// and `PlanetSurfacePlugin`.
pub mod prelude {
    pub use super::{
        planet_mesh, PlanetShape, PlanetShapeNoise, PlanetSurfaceMaterial,
        PlanetSurfaceMaterialExt, PlanetSurfacePlugin, PlanetVisual, PLANET_EDITOR_SUBDIVISIONS,
        PLANET_SUBDIVISIONS, PLANET_SUBDIVISIONS_MAX,
    };
}

/// The planet material: a standard PBR material whose base colour, roughness
/// and emissive are decided per pixel by a banded palette.
pub type PlanetSurfaceMaterial = ExtendedMaterial<StandardMaterial, PlanetSurfaceMaterialExt>;

/// Icosphere subdivisions a planet is meshed at by default.
///
/// Bevy's icosphere counts subdivisions as POINTS ADDED PER EDGE, not as
/// recursion depth, so the vertex count is `10 * (n + 1)^2 + 2` and its own
/// doc's "a good default is 5" is 362 vertices - a fine ball and a hopeless
/// planet. 48 is 24,010 triangles, a facet about 1.3 degrees of arc across:
/// 18 m on an 800 m body, which is where a mountain range stops being facets.
pub const PLANET_SUBDIVISIONS: u32 = 48;

/// The most subdivisions a planet may be meshed at.
///
/// Bevy's icosphere builder refuses past 65,535 vertices, which
/// `10 * (n+1)^2 + 2` crosses at 80, so this is not a taste limit: it is where
/// the builder starts returning an error. Clamped rather than reported, because
/// a caller asking for more detail than a mesh can hold wants the most
/// available, not a failure.
pub const PLANET_SUBDIVISIONS_MAX: u32 = 79;

/// What an EDITOR preview meshes at.
///
/// Coarser than [`PLANET_SUBDIVISIONS`] on purpose. A preview body is rebuilt
/// every time a creator edits a field the body is drawn from, so the mesh has
/// to be cheap to make; the palette, the seed and the silhouette all read the
/// same at this density, and only the limb loses a little smoothness.
pub const PLANET_EDITOR_SUBDIVISIONS: u32 = 24;

/// How many octaves shape a planet's continents.
///
/// More than a rock's four ([`ROCK_OCTAVES`](super::asteroid_surface)): a rock
/// wants a few big masses, and a planet wants a landmass that still has
/// coastline detail when a ship is a few hundred meters off it.
const CONTINENT_OCTAVES: usize = 6;

/// Continent frequency, in cycles per unit of DIRECTION. About one lobe across
/// a hemisphere at the base octave, so a planet gets a handful of landmasses
/// rather than a uniform speckle.
const CONTINENT_FREQUENCY: f64 = 1.1;

/// How much each continent octave shrinks against the one before it.
const CONTINENT_PERSISTENCE: f64 = 0.5;

/// How much each continent octave's frequency grows against the one before it.
const CONTINENT_LACUNARITY: f64 = 2.15;

/// How many octaves the mountain ridges carry.
const MOUNTAIN_OCTAVES: usize = 4;

/// Mountain frequency: several times the continents', so ranges run across a
/// landmass instead of being one.
const MOUNTAIN_FREQUENCY: f64 = 2.8;

/// How much of the height range the ridged mountain term may claim.
///
/// The ridges are MASKED by the continent height, so they build on high ground
/// and leave basins alone - which is what makes a range read as a range rather
/// than as noise laid over the whole ball.
const MOUNTAIN_WEIGHT: f32 = 0.55;

/// Where the mountain mask reaches full strength, in raw continent units. Below
/// zero it is off entirely.
const MOUNTAIN_MASK_TOP: f32 = 0.35;

/// How many directions the height range is measured over.
///
/// The range is normalized per planet so every authored band appears on every
/// seed; a sampled range is only as good as its spread is fine. 4,096 is a
/// fraction of the cost of meshing the body and lands the range within a
/// percent or so - and `height01` clamps, so the handful of points outside the
/// sampled range flatten into the top or bottom band rather than escaping it.
const RANGE_SAMPLES: usize = 4096;

/// The parameters one planet's shape is drawn from.
#[derive(Clone, Copy, Debug)]
pub struct PlanetShape {
    /// The terrain noise seed. Planets with the same seed are the same planet.
    pub seed: u32,
    /// How far the surface moves from the mean radius, as a fraction of it.
    pub relief: f32,
    /// The height fraction below which the surface flattens to a true sphere.
    pub sea_level: Option<f32>,
}

impl From<&PlanetSurface> for PlanetShape {
    fn from(surface: &PlanetSurface) -> Self {
        Self {
            seed: surface.shape_seed,
            relief: surface.relief,
            sea_level: surface.sea_level,
        }
    }
}

/// One planet's assembled height function, with its range already measured.
pub struct PlanetShapeNoise {
    continents: Fbm<Perlin>,
    mountains: RidgedMulti<Perlin>,
    /// The lowest raw elevation measured over [`RANGE_SAMPLES`] directions.
    low: f32,
    /// The measured range, never zero.
    span: f32,
    sea_level: Option<f32>,
    radius_min: f32,
    radius_span: f32,
}

impl PlanetShapeNoise {
    /// Build the height function `shape` describes and measure its range.
    ///
    /// The noise graphs are assembled ONCE and then sampled: `Fbm::new` seeds a
    /// permutation table per octave, so rebuilding the graph per sample would
    /// cost far more than sampling it.
    pub fn new(shape: &PlanetShape) -> Self {
        let continents = Fbm::<Perlin>::new(octave_safe_seed(shape.seed, CONTINENT_OCTAVES))
            .set_frequency(CONTINENT_FREQUENCY)
            .set_persistence(CONTINENT_PERSISTENCE)
            .set_lacunarity(CONTINENT_LACUNARITY)
            .set_octaves(CONTINENT_OCTAVES);
        let mountains = RidgedMulti::<Perlin>::new(octave_safe_seed(
            shape.seed.wrapping_add(0x9e37_79b9),
            MOUNTAIN_OCTAVES,
        ))
        .set_frequency(MOUNTAIN_FREQUENCY)
        .set_octaves(MOUNTAIN_OCTAVES);

        let mut noise = Self {
            continents,
            mountains,
            low: 0.0,
            span: 1.0,
            sea_level: shape.sea_level,
            radius_min: 1.0 - shape.relief,
            radius_span: 2.0 * shape.relief,
        };

        let (low, high) = sphere_spread(RANGE_SAMPLES)
            .map(|direction| noise.raw(direction))
            .fold((f32::MAX, f32::MIN), |(low, high), value| {
                (low.min(value), high.max(value))
            });
        noise.low = low;
        noise.span = (high - low).max(1e-4);
        noise
    }

    /// The raw, unnormalized elevation along `direction`.
    fn raw(&self, direction: Vec3) -> f32 {
        let at = [
            f64::from(direction.x),
            f64::from(direction.y),
            f64::from(direction.z),
        ];
        let continent = self.continents.get(at) as f32;
        // Ridged noise peaks near 1 and floors near -1; the half-shift makes
        // the term additive so a range only ever builds ground up.
        let ridge = (self.mountains.get(at) as f32).mul_add(0.5, 0.5);
        let mask = (continent / MOUNTAIN_MASK_TOP).clamp(0.0, 1.0);
        continent + ridge * mask * MOUNTAIN_WEIGHT
    }

    /// Where the surface stands along `direction`, as a fraction of the height
    /// range: 0 is this planet's deepest point, 1 its highest.
    ///
    /// Normalized against the MEASURED range rather than the noise's nominal
    /// one, which is what makes every authored band appear on every seed. A
    /// sea flattens everything below its level onto one exact value, so an
    /// ocean is a true sphere patch and not a hilly one painted blue.
    pub fn height01(&self, direction: Vec3) -> f32 {
        let normalized = ((self.raw(direction) - self.low) / self.span).clamp(0.0, 1.0);
        match self.sea_level {
            Some(sea) => normalized.max(sea),
            None => normalized,
        }
    }

    /// Where the surface stands along `direction`, in the mesh's own unit
    /// space (the body's mean radius is 1).
    pub fn radius(&self, direction: Vec3) -> f32 {
        self.radius_min + self.radius_span * self.height01(direction)
    }

    /// The radius of this planet's deepest point, in unit space. The material
    /// needs it to recover the elevation from a fragment's position.
    pub fn radius_min(&self) -> f32 {
        self.radius_min
    }

    /// The distance between this planet's deepest and highest points, in unit
    /// space.
    pub fn radius_span(&self) -> f32 {
        self.radius_span
    }
}

/// A seed the `noise` fractal generators can safely build `octaves` from.
///
/// They seed octave `n` with `seed + n` in `u32`, so a seed within `octaves` of
/// the ceiling panics on overflow in any build with overflow checks. A planet's
/// shape seed comes off a hash and is uniform over the whole range, so this is
/// reachable by an unlucky seed rather than only by a test poking `u32::MAX`.
fn octave_safe_seed(seed: u32, octaves: usize) -> u32 {
    seed % (u32::MAX - octaves as u32)
}

/// An even spread of `count` directions over the sphere - a Fibonacci lattice,
/// so no axis or pole is favoured.
fn sphere_spread(count: usize) -> impl Iterator<Item = Vec3> {
    (0..count).map(move |step| {
        let height = 1.0 - 2.0 * (step as f32 + 0.5) / count as f32;
        let ring = (1.0 - height * height).max(0.0).sqrt();
        let turn = step as f32 * 2.399_963_2;
        Vec3::new(ring * turn.cos(), height, ring * turn.sin()).normalize_or(Vec3::Y)
    })
}

/// Mesh a planet: a unit icosphere displaced by `shape`, with normals rebuilt
/// from the height field itself.
///
/// An ICOsphere, not the UV sphere `compare_planets` uses. A UV sphere's
/// facets shrink to nothing at its poles, which a displacement makes obvious -
/// the poles pinch and the relief there is meshed a hundred times finer than
/// at the equator. An icosphere's facets are near enough uniform everywhere,
/// and this material reads no UVs, so the one thing a UV sphere was for (an
/// equirect map) does not apply.
///
/// Normals come from finite differences on the height field rather than from
/// the meshed triangles. The difference is at the seams: an icosphere shares
/// its vertices, but a face-averaged normal still depends on which triangles
/// happen to meet at a vertex, while a field difference depends only on the
/// direction - so two planets meshed at different subdivisions light the same.
pub fn planet_mesh(shape: &PlanetShapeNoise, subdivisions: u32) -> Mesh {
    let subdivisions = subdivisions.clamp(1, PLANET_SUBDIVISIONS_MAX);
    let mut mesh = Sphere::new(1.0)
        .mesh()
        .ico(subdivisions)
        .expect("an icosphere at or below PLANET_SUBDIVISIONS_MAX always fits the vertex limit");

    let Some(positions) = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|values| values.as_float3())
        .map(<[[f32; 3]]>::to_vec)
    else {
        error!("planet_mesh: the icosphere builder returned no positions");
        return mesh;
    };

    // Half a facet. An icosahedron edge subtends about 1.107 rad and carries
    // `subdivisions + 1` facets, so this is close enough to measure the local
    // slope and far enough not to be measuring float noise.
    let epsilon = 0.55 / (subdivisions + 1) as f32;

    let mut displaced = Vec::with_capacity(positions.len());
    let mut normals = Vec::with_capacity(positions.len());
    for position in positions {
        let direction = Vec3::from_array(position).normalize_or(Vec3::Y);
        let point = direction * shape.radius(direction);

        // A right-handed tangent frame whose cross product is the outward
        // direction, so the differenced normal comes out pointing outward
        // without a sign test. Which tangent is picked does not matter - the
        // normal is the same to the order of the difference - so the branch
        // that keeps the pick well-conditioned near an axis is free.
        let reference = match direction.z.abs() < 0.9 {
            true => Vec3::Z,
            false => Vec3::X,
        };
        let tangent = reference.cross(direction).normalize_or(Vec3::X);
        let bitangent = direction.cross(tangent);

        let along = (direction + tangent * epsilon).normalize_or(direction);
        let across = (direction + bitangent * epsilon).normalize_or(direction);
        let normal = (along * shape.radius(along) - point)
            .cross(across * shape.radius(across) - point)
            .normalize_or(direction);

        displaced.push(point.to_array());
        normals.push(normal.to_array());
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, displaced);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh
}

/// One band as the shader reads it.
///
/// Two `vec4`s and nothing else. Every field of this uniform is 16-byte
/// aligned by construction, so the WebGL2 alignment rule the asteroid material
/// pads by hand for is satisfied without padding fields - there is no scalar
/// in the layout to strand.
#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct PlanetBandUniform {
    /// Linear rgb, with the emissive multiplier in `w`.
    pub color: Vec4,
    /// Roughness, height floor, latitude floor, and one spare.
    pub surface: Vec4,
}

/// A planet's whole palette and shape, as the shader reads it.
#[derive(Clone, Debug, ShaderType)]
pub struct PlanetSurfaceUniform {
    /// The bands, low first and any cap last. Entries past
    /// [`shape`](Self::shape)`.z` are never read.
    pub bands: [PlanetBandUniform; PLANET_BAND_LIMIT],
    /// Deepest radius, the range between deepest and highest, how many bands
    /// are live, and one spare - all in the mesh's own unit space.
    pub shape: Vec4,
    /// Warp amount, warp frequency, grain amount, grain frequency.
    pub detail: Vec4,
    /// Normal-bump strength, the noise seed, and two spares.
    pub extra: Vec4,
}

impl Default for PlanetSurfaceUniform {
    fn default() -> Self {
        Self {
            bands: [PlanetBandUniform::default(); PLANET_BAND_LIMIT],
            shape: Vec4::new(1.0, 1.0, 1.0, 0.0),
            detail: Vec4::ZERO,
            extra: Vec4::ZERO,
        }
    }
}

/// The planet extension's own bindings: one uniform, no textures.
///
/// No texture is the whole point. An image would have to tile to cover a body
/// kilometres across, and a tile at that scale is the grey repeat this module
/// exists to replace.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct PlanetSurfaceMaterialExt {
    /// The palette and shape this planet is shaded from.
    #[uniform(100)]
    pub surface: PlanetSurfaceUniform,
}

impl PlanetSurfaceMaterialExt {
    /// The extension a planet with this drawn surface and this shape wants.
    pub fn new(surface: &PlanetSurface, shape: &PlanetShapeNoise) -> Self {
        let mut bands = [PlanetBandUniform::default(); PLANET_BAND_LIMIT];
        for (slot, band) in bands.iter_mut().zip(&surface.bands) {
            *slot = PlanetBandUniform {
                color: Vec4::new(band.color.red, band.color.green, band.color.blue, band.glow),
                surface: Vec4::new(band.roughness, band.floor, band.latitude_floor, 0.0),
            };
        }

        let detail = surface.detail;
        Self {
            surface: PlanetSurfaceUniform {
                bands,
                shape: Vec4::new(
                    shape.radius_min(),
                    shape.radius_span(),
                    surface.bands.len() as f32,
                    0.0,
                ),
                detail: Vec4::new(
                    detail.warp,
                    detail.warp_frequency,
                    detail.grain,
                    detail.grain_frequency,
                ),
                // The seed reaches the shader as a float because a uniform
                // `vec4` is what the layout is; it is small enough to be exact.
                extra: Vec4::new(detail.bump, (surface.shape_seed % 4096) as f32, 0.0, 0.0),
            },
        }
    }
}

impl MaterialExtension for PlanetSurfaceMaterialExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/planet_surface.wgsl".into()
    }
}

/// Everything a planet needs to be spawned: the mesh, the material, and the
/// drawn surface the two came from.
///
/// The one call a caller makes.
/// [`planet_scenario_object`](super::planet::planet_scenario_object) parks one
/// of these on the render child until an observer can reach `Assets`.
#[derive(Clone, Debug)]
pub struct PlanetVisual {
    /// The displaced icosphere, in unit space. Scale it by the config's radius.
    pub mesh: Mesh,
    /// The material shading it.
    pub material: PlanetSurfaceMaterial,
    /// What the seed drew, for a label or a log.
    pub surface: PlanetSurface,
}

impl PlanetVisual {
    /// Draw the surface, build the height field, mesh it, and dress it.
    pub fn build(config: &PlanetConfig, subdivisions: u32) -> Self {
        let surface = PlanetSurface::generate(config);
        let shape = PlanetShapeNoise::new(&PlanetShape::from(&surface));
        let mesh = planet_mesh(&shape, subdivisions);
        let material = PlanetSurfaceMaterial {
            // White and rough: the extension writes the real colour and
            // roughness per pixel, and multiplies into whatever tint the base
            // carries - so a tinted base still tints.
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 1.0,
                metallic: 0.0,
                ..default()
            },
            extension: PlanetSurfaceMaterialExt::new(&surface, &shape),
        };
        Self {
            mesh,
            material,
            surface,
        }
    }
}

/// Registers the planet material's render pipeline.
///
/// NOT added by
/// [`ScenarioObjectsPlugin`](super::ScenarioObjectsPlugin): this round adds a
/// look, not a scenario object, so nothing an authored scenario spawns changes.
/// An app that wants planets adds this itself.
pub struct PlanetSurfacePlugin;

impl Plugin for PlanetSurfacePlugin {
    fn build(&self, app: &mut App) {
        trace!("PlanetSurfacePlugin: build");

        app.add_plugins(MaterialPlugin::<PlanetSurfaceMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use bevy::render::mesh::VertexAttributeValues;
    use nova_events::prelude::*;

    use super::*;

    fn shape_of(planet_type: PlanetType, seed: u32) -> PlanetShapeNoise {
        let config = PlanetConfig::new(planet_type, Meters(800.0), seed);
        PlanetShapeNoise::new(&PlanetShape::from(&PlanetSurface::generate(&config)))
    }

    /// THE fix a rock already got and a planet still needs: the surface has to
    /// be cut INTO the sphere as well as grown out of it, or the base sphere
    /// shows through wherever the field bottoms out.
    #[test]
    fn a_planet_is_cut_into_as_well_as_grown_out_of() {
        let shape = shape_of(PlanetType::BarrenRock, 7);

        let mut inside = 0;
        let mut outside = 0;
        for direction in sphere_spread(400) {
            match shape.radius(direction) < 1.0 {
                true => inside += 1,
                false => outside += 1,
            }
        }

        assert!(
            inside > 40 && outside > 40,
            "a planet has to go both ways: {inside} in, {outside} out"
        );
    }

    /// Every authored band has to be REACHABLE, or a palette is decoration.
    /// The height range is normalized per planet for exactly this reason, so
    /// this is the test that pins the normalization.
    #[test]
    fn every_band_is_reached_on_every_type_and_seed() {
        for planet_type in PlanetType::ALL {
            for seed in [0u32, 3, 4242, 20_260_904] {
                let config = PlanetConfig::new(planet_type, Meters(800.0), seed);
                let surface = PlanetSurface::generate(&config);
                let shape = PlanetShapeNoise::new(&PlanetShape::from(&surface));

                let heights: Vec<f32> = sphere_spread(4000).map(|d| shape.height01(d)).collect();
                for band in &surface.bands {
                    // A cap claims latitude, not elevation, so its floor is 0
                    // and it is reached by construction.
                    if band.latitude_floor > 0.0 {
                        continue;
                    }
                    assert!(
                        heights.iter().any(|height| *height >= band.floor),
                        "{} seed {seed}: nothing reaches {} at {}",
                        planet_type.name(),
                        band.name,
                        band.floor
                    );
                }
            }
        }
    }

    /// A sea is a true sphere patch: everything under the sea level flattens
    /// onto one exact radius. A sea that still had hills in it would read as
    /// blue ground.
    #[test]
    fn a_sea_flattens_to_one_radius() {
        let shape = shape_of(PlanetType::Temperate, 11);
        let sea = PlanetType::Temperate
            .sea_level()
            .expect("a temperate world has a sea");

        let mut flat = 0;
        for direction in sphere_spread(2000) {
            let height = shape.height01(direction);
            assert!(height >= sea - 1e-6, "the sea leaked below its own level");
            if (height - sea).abs() < 1e-6 {
                flat += 1;
            }
        }
        assert!(flat > 100, "only {flat} of 2000 points are sea");
    }

    /// The material recovers a fragment's elevation from its distance to the
    /// centre, so the mesh's radii have to span exactly the range the uniform
    /// hands the shader. A mesh reaching past it would clamp into the top band.
    #[test]
    fn the_mesh_stays_inside_the_range_the_uniform_declares() {
        for planet_type in PlanetType::ALL {
            let shape = shape_of(planet_type, 5);
            let mesh = planet_mesh(&shape, 24);
            let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("a meshed planet has positions");
            };

            let low = shape.radius_min();
            let high = shape.radius_min() + shape.radius_span();
            for position in positions {
                let radius = Vec3::from_array(*position).length();
                assert!(
                    radius >= low - 1e-4 && radius <= high + 1e-4,
                    "{} meshed a vertex at {radius}, outside [{low}, {high}]",
                    planet_type.name()
                );
            }
        }
    }

    /// Normals come from the height field, so they point outward everywhere -
    /// an inward normal is a black facet in the render and it is the failure
    /// a hand-rolled tangent frame gets wrong.
    #[test]
    fn every_meshed_normal_points_outward() {
        let shape = shape_of(PlanetType::Volcanic, 2);
        let mesh = planet_mesh(&shape, 24);
        let (
            Some(VertexAttributeValues::Float32x3(positions)),
            Some(VertexAttributeValues::Float32x3(normals)),
        ) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL),
        )
        else {
            panic!("a meshed planet has positions and normals");
        };

        for (position, normal) in positions.iter().zip(normals) {
            let outward = Vec3::from_array(*position).normalize_or(Vec3::Y);
            let normal = Vec3::from_array(*normal);
            assert!(
                (normal.length() - 1.0).abs() < 1e-3,
                "a normal of length {}",
                normal.length()
            );
            assert!(
                outward.dot(normal) > 0.3,
                "a normal facing {} against the outward {outward}",
                normal
            );
        }
    }

    /// The reproducibility contract, end to end: the same config meshes the
    /// same body, not only the same palette.
    #[test]
    fn the_same_config_meshes_the_same_planet() {
        let config = PlanetConfig::new(PlanetType::IceWorld, Meters(800.0), 31);
        let once = PlanetVisual::build(&config, 24);
        let again = PlanetVisual::build(&config, 24);

        let (
            Some(VertexAttributeValues::Float32x3(first)),
            Some(VertexAttributeValues::Float32x3(second)),
        ) = (
            once.mesh.attribute(Mesh::ATTRIBUTE_POSITION),
            again.mesh.attribute(Mesh::ATTRIBUTE_POSITION),
        )
        else {
            panic!("a meshed planet has positions");
        };
        assert_eq!(first, second);
        assert_eq!(once.surface.summary(), again.surface.summary());
    }

    /// Meshing must not fail or silently degrade at the top of the supported
    /// range, and asking past it clamps rather than panicking.
    #[test]
    fn subdivisions_clamp_instead_of_failing() {
        let shape = shape_of(PlanetType::DustWorld, 1);
        let top = planet_mesh(&shape, PLANET_SUBDIVISIONS_MAX).count_vertices();
        let past = planet_mesh(&shape, PLANET_SUBDIVISIONS_MAX + 4).count_vertices();
        assert_eq!(top, past);
        // `10 * (n + 1)^2 + 2`, and one vertex past the builder's 65,535 limit
        // is an error rather than a coarser mesh - so this pins the ceiling.
        assert_eq!(top, 64_002, "the top subdivision meshes {top} vertices");
    }
}
