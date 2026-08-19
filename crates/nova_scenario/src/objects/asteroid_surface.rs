//! How a rock is TEXTURED, and how its silhouette is shaped.
//!
//! Two art problems with one cause between them, and both are about the mesh
//! being asked to carry information it is bad at carrying.
//!
//! # The texture: sampled by position, not by UV
//!
//! An asteroid's mesh UVs are planar PER TRIANGLE - each face projects the
//! texture into its own plane from its own first vertex - so the texture
//! restarted at every triangle edge and its scale followed the triangle's size.
//! A rock read as quilted, and a REMESHED rock wore a visibly different texture
//! scale from an uncarved one beside it, because a surface-nets triangle is not
//! the size of a subdivision triangle.
//!
//! [`AsteroidSurfaceMaterial`] samples the rock texture by POSITION in the
//! body's own space, blended across three axis projections. No UVs are
//! consulted, so there is nothing to seam and nothing that changes when the
//! triangles do - which is what makes a carved rock and a pristine one the same
//! material rather than two that happen to share a texture.
//!
//! # The silhouette: a rock is not a sphere with growths on it
//!
//! [`PlanetHeight`](super::asteroid::PlanetHeight) is a planet generator: a
//! twenty-five-permutation-table graph of continents, mountains, badlands and
//! rivers, and its output is NON-NEGATIVE. Displacing a sphere by it can only
//! push material outward, so wherever the noise bottoms out the original unit
//! sphere shows through untouched. That is exactly what a rock built this way
//! looks like - a ball with lumps stuck on it - and no amount of tuning the
//! amplitude fixes it, because the problem is the missing sign.
//!
//! [`RockHeight`] is the replacement: a handful of octaves of plain fBm,
//! SIGNED, so the surface is eaten into as much as it is grown out of, plus a
//! per-rock anisotropic stretch so a rock has a long axis. It is also about
//! twenty times cheaper to sample, which is what makes seeding a carve field
//! affordable enough to do at spawn.

use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

/// `RockHeight`, `RockHeightNoise`, `AsteroidSurfaceMaterial` and
/// `AsteroidSurfaceMaterialExt`.
pub mod prelude {
    pub use super::{
        AsteroidSurfaceMaterial, AsteroidSurfaceMaterialExt, RockHeight, RockHeightNoise,
    };
}

/// How many texture repeats one world unit of rock gets.
///
/// Tuned so a field rock a few units across shows the grain at roughly the
/// scale the old per-triangle projection did on a mid-sized triangle, which is
/// what keeps this from reading as a different material rather than as the same
/// one stopping being quilted.
pub const ROCK_TEXTURE_TILING: f32 = 0.35;

/// How hard the three triplanar projections cut over to each other.
///
/// High enough that a face pointing between two axes does not read as a
/// cross-fade of two blurs, low enough that the changeover is not a visible
/// line. Nothing here is subtle at this texture scale.
pub const ROCK_TEXTURE_SHARPNESS: f32 = 4.0;

/// The rock material: a standard PBR material surfaced triplanar-ly.
pub type AsteroidSurfaceMaterial = ExtendedMaterial<StandardMaterial, AsteroidSurfaceMaterialExt>;

/// The triplanar extension's own bindings.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct AsteroidSurfaceMaterialExt {
    /// Texture repeats per unit of the body's own local space.
    #[uniform(100)]
    pub tiling: f32,
    /// How hard the three projections cut over to each other.
    #[uniform(100)]
    pub sharpness: f32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b1: u32,
    #[cfg(target_arch = "wasm32")]
    #[uniform(100)]
    _webgl2_padding_16b2: u32,
    /// The rock texture, bound HERE rather than as the standard material's
    /// `base_color_texture`: the standard sampler would read it through the
    /// mesh UVs, which is the thing this exists to stop doing.
    #[texture(101)]
    #[sampler(102)]
    pub texture: Option<Handle<Image>>,
}

impl AsteroidSurfaceMaterialExt {
    /// The extension a rock wearing `texture` wants.
    #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(
            clippy::needless_update,
            reason = "the webgl2 padding fields exist only on wasm32, and there this update is what fills them"
        )
    )]
    pub fn new(texture: Handle<Image>) -> Self {
        Self {
            tiling: ROCK_TEXTURE_TILING,
            sharpness: ROCK_TEXTURE_SHARPNESS,
            texture: Some(texture),
            ..default()
        }
    }
}

impl MaterialExtension for AsteroidSurfaceMaterialExt {
    fn fragment_shader() -> ShaderRef {
        "shaders/asteroid_surface.wgsl".into()
    }
}

/// How many octaves of noise shape a rock.
///
/// Four, against the planet generator's fourteen-octave continents plus six
/// more graphs layered on them. A rock has no continents: it wants a big lumpy
/// base, a couple of octaves of bite, and nothing else. The cost difference is
/// the point as much as the look - this is what a carve field is seeded from.
const ROCK_OCTAVES: usize = 4;

/// The base frequency of the rock noise, in cycles per unit of DIRECTION.
///
/// About one and a half lobes across the sphere at the base octave, so a rock
/// has a few big masses rather than a uniform crust of bumps.
const ROCK_FREQUENCY: f64 = 1.4;

/// How much each octave shrinks against the one before it.
const ROCK_PERSISTENCE: f64 = 0.55;
/// How much each octave's frequency grows against the one before it.
const ROCK_LACUNARITY: f64 = 2.3;

/// The radius a rock sits at before the noise moves it, in units of the mesh's
/// own unit space.
///
/// NOT 1.0, and that is a compatibility constraint rather than an art choice.
/// The old generator displaced a unit sphere outward by up to 5, so a rock's
/// real extent has always been several times its authored radius - a spread
/// pinned as `ASTEROID_GEOMETRIC_FACTOR_MIN..MAX` (3.5-6.0) and cited by
/// campaign clearances, editor placement refusal and orbit gates. Rebasing the
/// generator at 1.0 would shrink every rock in the game by about four times and
/// invalidate all of it. So the SHAPE changes here and the SIZE does not.
const ROCK_BASE: f32 = 4.0;

/// How far the surface moves from [`ROCK_BASE`], in the same units.
///
/// SIGNED: the surface runs from `base - amplitude` to `base + amplitude`, so
/// the noise cuts in as readily as it grows out. That sign is the whole
/// difference between a rock and a ball with lumps on it - the old generator's
/// output was non-negative, so wherever its noise bottomed out the original
/// sphere showed through untouched, and no amount of amplitude tuning could
/// have hidden it.
const ROCK_AMPLITUDE: f32 = 1.6;

/// The narrowest a rock's short axis gets against its long one.
///
/// A real rock is not round, and a generator that only perturbs a sphere makes
/// a field of identical potatoes. The stretch is per-seed, so rocks differ in
/// PROPORTION and not only in surface detail.
///
/// It only ever narrows: the stretch is normalized so the longest axis is the
/// unstretched one, which keeps a rock's furthest reach - the number
/// `ASTEROID_GEOMETRIC_FACTOR_MAX` bounds - where the noise alone put it.
const ROCK_STRETCH_MIN: f32 = 0.62;

/// The least a rock's radius may be, as a fraction of [`ROCK_BASE`]. A guard,
/// not a shape: fBm is not strictly bounded, and a rock may not turn inside
/// out however the noise falls.
const ROCK_RADIUS_FLOOR: f32 = 0.25;

/// How many directions [`RockHeightNoise::reach`] measures a rock's reach over.
///
/// fBm has no closed-form extremes, so the reach is sampled, and a sampled
/// extreme is only as good as the spread is fine. Measured against a 200k
/// spread over 64 seeds, the worst a sampled peak fell short of the real one
/// was 6.4% at 512 directions and 2.3% at 2048; the convergence is slow (about
/// one over the root of the count), so paying for the next step down buys
/// little. 2048 is 6% of what seeding a field costs.
const ROCK_EXTENT_SAMPLES: usize = 2048;

/// How far [`RockHeightNoise::reach`] pushes its sampled floor DOWN.
///
/// The measured worst overshoot at [`ROCK_EXTENT_SAMPLES`] is 6.0%: the near
/// extreme sits in the kink where the axis stretch changes which axis is
/// nearest, and a spread that steps over the kink misses it. Ten per cent is
/// that with margin, and it costs only a slightly smaller sphere of samples the
/// field gets to skip.
const ROCK_REACH_SLACK_NEAR: f32 = 0.10;

/// How far [`RockHeightNoise::reach`] pushes its sampled ceiling UP.
///
/// The measured worst undershoot at [`ROCK_EXTENT_SAMPLES`] is 2.3%. This is
/// the expensive end - it sizes the carve field's whole domain, so every
/// percent here is a percent of coarseness on every rock in the game - and the
/// dangerous one, because a rock reaching past its domain is CLIPPED FLAT
/// against the grid wall. Five per cent is twice the measured worst.
const ROCK_REACH_SLACK_FAR: f32 = 0.05;

/// The parameters one rock's shape is drawn from.
///
/// The asteroid counterpart to [`PlanetHeight`](super::asteroid::PlanetHeight),
/// and deliberately a fraction of it: see the module docs for why a planet
/// generator makes a bad rock.
#[derive(Clone, Copy, Debug)]
pub struct RockHeight {
    /// The noise seed. Rocks with the same seed are the same rock.
    pub seed: u32,
    /// How far the surface moves from the unit sphere, as a fraction of it.
    pub amplitude: f32,
}

impl Default for RockHeight {
    fn default() -> Self {
        Self {
            seed: 0,
            amplitude: ROCK_AMPLITUDE,
        }
    }
}

impl RockHeight {
    /// These parameters with the seed replaced.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Build the sampler these parameters describe.
    ///
    /// Assembled ONCE and then sampled, for the reason
    /// [`PlanetHeight::sampler`](super::asteroid::PlanetHeight::sampler)
    /// documents: `Fbm::new` seeds a permutation table per octave, so building
    /// the graph per sample would cost far more than sampling it.
    pub fn sampler(&self) -> RockHeightNoise {
        RockHeightNoise::new(self)
    }
}

/// One rock's assembled shape function.
pub struct RockHeightNoise {
    noise: Fbm<Perlin>,
    amplitude: f32,
    /// The per-rock stretch, applied to the DIRECTION before the noise sees it
    /// and to the radius after: a rock with a long axis, rather than a sphere
    /// wearing a different set of bumps.
    stretch: Vec3,
}

impl RockHeightNoise {
    fn new(params: &RockHeight) -> Self {
        let noise = Fbm::<Perlin>::new(octave_safe_seed(params.seed))
            .set_frequency(ROCK_FREQUENCY)
            .set_persistence(ROCK_PERSISTENCE)
            .set_lacunarity(ROCK_LACUNARITY)
            .set_octaves(ROCK_OCTAVES);
        Self {
            noise,
            amplitude: params.amplitude,
            // The UNCLAMPED seed: the stretch is our own hash and has no such
            // ceiling, so two rocks whose seeds differ only up near it still
            // come out different shapes.
            stretch: seed_stretch(params.seed),
        }
    }

    /// How far the surface stands from the origin along `direction`, in the
    /// mesh's own unit space.
    ///
    /// `direction` is expected normalized. The result is always positive, so
    /// there is no direction in which a rock turns inside out however the noise
    /// falls.
    pub fn radius(&self, direction: Vec3) -> f32 {
        // The noise is sampled on the STRETCHED direction, so a rock's surface
        // detail stretches with its proportions instead of sitting on top of
        // them at a constant scale and giving the stretch away.
        let at = direction * self.stretch;
        let noise = self
            .noise
            .get([f64::from(at.x), f64::from(at.y), f64::from(at.z)]) as f32;
        // How much of the base this direction gets: 1 along the long axis,
        // down to `ROCK_STRETCH_MIN` along the shortest.
        let along = self
            .stretch
            .dot(direction.abs())
            .clamp(ROCK_STRETCH_MIN, 1.0);
        (ROCK_BASE * along + noise * self.amplitude).max(ROCK_BASE * ROCK_RADIUS_FLOOR)
    }

    /// The closest and furthest this rock's surface stands from its own centre.
    ///
    /// Two jobs. The far end sizes a carve field's domain - the grid has to
    /// contain the whole rock, because a surface reaching past the domain would
    /// be clipped flat against it. Both ends then let the field settle a point's
    /// SIGN without paying for the noise: inside the near reach every rock is
    /// solid, outside the far one every rock is empty, and the surface can only
    /// be in the shell between them. That shell is about a third of the grid, so
    /// two thirds of the samples never touch the noise.
    ///
    /// Sampled over [`ROCK_EXTENT_SAMPLES`] directions rather than solved - fBm
    /// has no closed form - and widened at each end by the measured worst the
    /// sampling misses by, so what comes back is a BOUND and not an estimate.
    pub fn reach(&self) -> (f32, f32) {
        let mut nearest = f32::MAX;
        let mut furthest = 0.0f32;
        for direction in sphere_spread(ROCK_EXTENT_SAMPLES) {
            let radius = self.radius(direction);
            nearest = nearest.min(radius);
            furthest = furthest.max(radius);
        }
        (
            nearest * (1.0 - ROCK_REACH_SLACK_NEAR),
            furthest * (1.0 + ROCK_REACH_SLACK_FAR),
        )
    }
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

/// A seed `Fbm` can safely build this many octaves from.
///
/// `noise`'s fractal generators seed octave `n` with `seed + n` and do it in
/// `u32`, so a seed within `octaves` of the ceiling panics on overflow in any
/// build with overflow checks. An asteroid's seed is an FNV hash of its
/// scenario id ([`asteroid_seed_from_id`](super::asteroid::asteroid_seed_from_id)),
/// which is uniform over the whole `u32` range - so this is reachable by
/// naming an object unluckily, not just by a test poking `u32::MAX`.
///
/// [`PlanetHeight`](super::asteroid::PlanetHeight) has the same exposure and
/// always has.
fn octave_safe_seed(seed: u32) -> u32 {
    seed % (u32::MAX - ROCK_OCTAVES as u32)
}

/// The axis stretch a rock with this seed wears.
///
/// Deterministic (FNV-1a, the hash asteroid seeds already use), and normalized
/// so the LONGEST axis is always 1: the stretch changes a rock's proportions
/// without changing how big it is, which is what keeps `AsteroidRadius` meaning
/// what it says.
fn seed_stretch(seed: u32) -> Vec3 {
    let mut hash: u32 = 0x811c_9dc5 ^ seed;
    let mut next = || {
        hash = hash.wrapping_mul(0x0100_0193);
        hash ^= hash >> 15;
        ROCK_STRETCH_MIN + (hash >> 16) as f32 / 65_536.0 * (1.0 - ROCK_STRETCH_MIN)
    };
    let stretch = Vec3::new(next(), next(), next());
    stretch / stretch.max_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE fix: the surface has to be cut INTO the sphere as well as grown out
    /// of it. A generator that only ever adds leaves the base sphere showing
    /// wherever the noise bottoms out, which is what made every rock read as a
    /// ball with lumps on it.
    #[test]
    fn a_rock_is_carved_into_as_well_as_grown_out_of() {
        let rock = RockHeight::default().with_seed(99).sampler();

        let mut inside = 0;
        let mut outside = 0;
        for direction in sphere_spread(400) {
            match rock.radius(direction) < ROCK_BASE {
                true => inside += 1,
                false => outside += 1,
            }
        }

        assert!(
            inside > 40 && outside > 40,
            "a rock has to go both ways: {inside} in, {outside} out"
        );
    }

    /// The SIZE contract, which is not this change's to break: campaign
    /// clearances, editor placement refusal and orbit gates all size rocks off
    /// `ASTEROID_GEOMETRIC_FACTOR_MIN..MAX`, so a rock's furthest reach has to
    /// stay inside those bounds however its shape moved.
    ///
    /// Pinned HERE as well as against the real mesh path, because this is
    /// where the numbers that decide it live.
    #[test]
    fn every_rock_reaches_inside_the_pinned_geometric_bounds() {
        use super::super::asteroid::{
            ASTEROID_GEOMETRIC_FACTOR_MAX, ASTEROID_GEOMETRIC_FACTOR_MIN,
        };

        for seed in [0u32, 1, 7, 99, 4242, 20260817, u32::MAX] {
            let rock = RockHeight::default().with_seed(seed).sampler();
            let furthest = sphere_spread(2000)
                .map(|direction| rock.radius(direction))
                .fold(0.0f32, f32::max);
            assert!(
                (ASTEROID_GEOMETRIC_FACTOR_MIN..=ASTEROID_GEOMETRIC_FACTOR_MAX).contains(&furthest),
                "seed {seed} reaches {furthest}, outside the pinned bounds"
            );
        }
    }

    /// The carve field settles a point's sign from [`RockHeightNoise::reach`]
    /// without asking the noise, so `reach` has to BOUND the surface rather
    /// than approximate it. A rock reaching past the far end would be clipped
    /// flat against the grid wall; one dipping inside the near end would be
    /// solid where it should be hollow.
    ///
    /// Checked against a spread forty times finer than the one `reach` samples,
    /// and a different lattice, so it is not being asked about its own points.
    #[test]
    fn reach_bounds_the_surface_in_every_direction() {
        for seed in [0u32, 1, 7, 99, 4242, 20260817, u32::MAX] {
            let rock = RockHeight::default().with_seed(seed).sampler();
            let (nearest, furthest) = rock.reach();
            for direction in sphere_spread(ROCK_EXTENT_SAMPLES * 40) {
                let radius = rock.radius(direction);
                assert!(
                    (nearest..=furthest).contains(&radius),
                    "seed {seed} reaches {radius} at {direction}, outside [{nearest}, {furthest}]"
                );
            }
        }
    }

    /// A rock may never turn inside out, however the noise falls.
    #[test]
    fn a_rock_always_has_a_positive_radius() {
        for seed in [0, 1, 7, 4242, u32::MAX] {
            let rock = RockHeight::default().with_seed(seed).sampler();
            for direction in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
                assert!(rock.radius(direction) > 0.0, "seed {seed} at {direction}");
            }
        }
    }

    /// Rocks differ in PROPORTION and not only in surface detail, which is what
    /// stops a field reading as a bag of identical potatoes.
    #[test]
    fn rocks_of_different_seeds_have_different_proportions() {
        let one = seed_stretch(1);
        let another = seed_stretch(20260817);

        assert_ne!(one, another);
        for stretch in [one, another] {
            assert!(
                (stretch.max_element() - 1.0).abs() < 1e-5,
                "the longest axis is the unit one: {stretch}"
            );
            assert!(
                stretch.min_element() >= ROCK_STRETCH_MIN - 1e-5,
                "a rock is not a needle: {stretch}"
            );
        }
    }

    /// The same seed is the same rock, on every load and in every process. A
    /// field whose rocks reshuffled would move every clearance authored around
    /// them.
    #[test]
    fn the_same_seed_is_the_same_rock() {
        let once = RockHeight::default().with_seed(31).sampler();
        let again = RockHeight::default().with_seed(31).sampler();
        for direction in [Vec3::X, Vec3::Y, Vec3::new(1.0, 2.0, 3.0).normalize()] {
            assert_eq!(once.radius(direction), again.radius(direction));
        }
    }
}
