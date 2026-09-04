//! What KIND of world a planetoid is: the authored type, the biomes each type
//! may draw from, and the seeded draw that turns one into the other.
//!
//! This module owns the CONTENT of a planet surface - names, colours,
//! roughness, the elevation and latitude a band claims. `planet_surface` owns
//! the SHAPE the bands are painted on and the material that paints them.
//! Nothing here touches rendering, so the palette can be unit-tested without a
//! GPU.
//!
//! # No blending, on purpose
//!
//! A band is chosen by a hard threshold and the last matching band wins. There
//! is no cross-fade between a plain and an upland, and no coastline gradient.
//! The edges are broken up in the shader by warping the elevation the
//! threshold reads, which is one multiply rather than a second palette lookup -
//! see `assets/shaders/planet_surface.wgsl`. Blending is the next round's
//! problem, not this one's.

use bevy::prelude::*;
use nova_events::prelude::*;

/// Planet types, their biome palettes, the authored [`PlanetConfig`], and the
/// seeded [`PlanetSurface`] a type and a seed resolve to.
pub mod prelude {
    pub use super::{
        Biome, BiomeSlot, PlanetBand, PlanetConfig, PlanetDetail, PlanetSurface, PlanetType,
        PLANET_BAND_LIMIT,
    };
}

/// How many bands one planet's shading may carry.
///
/// The shader holds them in a fixed-size uniform array, so this is a hard
/// ceiling rather than a guideline: a type authoring more bands than this is a
/// content bug the constructor catches. Six is what the busiest palette
/// (a temperate world: ocean, shore, plain, forest, upland, cap) needs, and a
/// palette that wants more probably wants blending instead.
pub const PLANET_BAND_LIMIT: usize = 6;

/// One authored surface look: what it is called, what colour it is, how rough
/// it is, and how much it glows.
///
/// Authored in sRGB because that is what a colour picker shows; the material
/// converts to linear once, at the uniform boundary, because Bevy's PBR
/// fragment does no colour conversion of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biome {
    /// What a reader calls this band. Shown by the example's readout, and the
    /// only handle content has on a band the seed chose.
    pub name: &'static str,
    /// The band's flat colour, in sRGB.
    pub color: Color,
    /// Perceptual roughness, 0 (mirror) to 1 (chalk). A dust plain is near 0.9.
    ///
    /// Floored around 0.3 even for water and ice. A body's shading normal here
    /// varies per facet and again per grain cell, so a band glossy enough to
    /// hold a tight specular lobe catches a different slice of that lobe at
    /// every pixel and reads as sparkle rather than as a sea.
    pub roughness: f32,
    /// Emissive strength multiplied into the band's own colour.
    ///
    /// On Bevy's emissive scale, not on the base-colour scale: the default
    /// `Exposure::BLENDER` multiplies lit surfaces by about 0.001 and emissive
    /// bypasses exposure entirely, so a glow that reads starts in the tens.
    /// Zero for every band that is not molten.
    pub glow: f32,
}

impl Biome {
    /// A plain, unlit band.
    pub const fn rock(name: &'static str, color: Color, roughness: f32) -> Self {
        Self {
            name,
            color,
            roughness,
            glow: 0.0,
        }
    }

    /// A band that emits light of its own colour.
    pub const fn molten(name: &'static str, color: Color, roughness: f32, glow: f32) -> Self {
        Self {
            name,
            color,
            roughness,
            glow,
        }
    }
}

/// One band's slot in a type's palette: where the band starts and which biomes
/// the seed may fill it with.
///
/// Slots are authored low to high and the LAST matching slot wins, so a polar
/// cap is written last with a [`latitude_floor`](Self::latitude_floor) and
/// claims everything above that latitude whatever the elevation there is -
/// which is what puts sea ice on a temperate world's poles for free.
#[derive(Clone, Copy, Debug)]
pub struct BiomeSlot {
    /// Where the band starts, as a fraction of the planet's own height range:
    /// 0 is that planet's deepest point and 1 its highest.
    ///
    /// A fraction and not a length because the range is normalized per planet
    /// (see [`PlanetSurface`]), which is what makes every authored band appear
    /// on every seed instead of only on the lucky ones.
    pub floor: f32,
    /// The lowest absolute latitude this band claims, 0 (the equator, so
    /// everywhere) to 1 (the pole). Only a cap band sets this.
    pub latitude_floor: f32,
    /// What the seed picks this slot's biome from. A one-entry list is a band
    /// that never varies.
    pub choices: &'static [Biome],
}

impl BiomeSlot {
    /// A band claiming everything from `floor` upward, at any latitude.
    pub const fn band(floor: f32, choices: &'static [Biome]) -> Self {
        Self {
            floor,
            latitude_floor: 0.0,
            choices,
        }
    }

    /// A polar cap: any elevation, from `latitude_floor` to the pole.
    pub const fn cap(latitude_floor: f32, choices: &'static [Biome]) -> Self {
        Self {
            floor: 0.0,
            latitude_floor,
            choices,
        }
    }
}

/// How broken-up a type's surface reads, over and above its palette.
///
/// Every figure is a fraction or a frequency in cycles per unit of DIRECTION,
/// so none of them scales with the planet: a dust world reads the same at a
/// 200 m planetoid and a 2 km one, which is the failure the shipped rock
/// texture has (see the round record).
#[derive(Clone, Copy, Debug)]
pub struct PlanetDetail {
    /// How far the shader may move the elevation a band threshold reads, as a
    /// fraction of the height range. This is what turns a contour line into a
    /// coastline; zero draws the bands as clean topographic rings.
    pub warp: f32,
    /// The frequency of that warp. Low is a few big incursions, high is a
    /// ragged edge.
    pub warp_frequency: f32,
    /// How much the band colour varies within itself, as a fraction.
    pub grain: f32,
    /// The frequency of that variation. This is the close-up texture and the
    /// reason the surface needs no image.
    pub grain_frequency: f32,
    /// How hard the same grain field bends the shading normal. The close pass
    /// reads as terrain rather than as paint because of this term.
    pub bump: f32,
}

/// The kinds of world content may author.
///
/// Archetypes, not places: the fiction is not the Solar System, so a type says
/// what a world is MADE of and leaves what it is called to the scenario. Each
/// carries a palette ([`Self::slots`]), a default relief, an optional sea
/// level, and its detail parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlanetType {
    /// Airless grey stone: high relief, nothing to erode it, no cap.
    BarrenRock,
    /// A rust and ochre dust world with thin polar frost.
    DustWorld,
    /// Frozen through: blue fissures, an ice plain where a sea would be, a
    /// glaring cap.
    IceWorld,
    /// Young and molten: basalt and ash over glowing rift valleys.
    Volcanic,
    /// Choked in haze: sulphur creams, low contrast, relief crushed flat.
    Greenhouse,
    /// The Earth-like one, kept to a single entry in six: ocean, shore,
    /// vegetation, upland, ice cap.
    Temperate,
}

impl PlanetType {
    /// Every type, in the order the example's keys walk them.
    pub const ALL: [Self; 6] = [
        Self::BarrenRock,
        Self::DustWorld,
        Self::IceWorld,
        Self::Volcanic,
        Self::Greenhouse,
        Self::Temperate,
    ];

    /// The type's name, for a readout or a label.
    pub const fn name(self) -> &'static str {
        match self {
            Self::BarrenRock => "barren rock",
            Self::DustWorld => "dust world",
            Self::IceWorld => "ice world",
            Self::Volcanic => "volcanic",
            Self::Greenhouse => "greenhouse",
            Self::Temperate => "temperate",
        }
    }

    /// The palette, low band first and any cap last.
    pub const fn slots(self) -> &'static [BiomeSlot] {
        match self {
            Self::BarrenRock => BARREN_ROCK_SLOTS,
            Self::DustWorld => DUST_WORLD_SLOTS,
            Self::IceWorld => ICE_WORLD_SLOTS,
            Self::Volcanic => VOLCANIC_SLOTS,
            Self::Greenhouse => GREENHOUSE_SLOTS,
            Self::Temperate => TEMPERATE_SLOTS,
        }
    }

    /// How far the surface moves from the authored radius, as a fraction of
    /// it, when the config authors no relief of its own.
    ///
    /// Openly exaggerated. Real relief is a rounding error on a planet's
    /// radius - Everest is 0.14% of Earth's - and a body modelled that
    /// honestly has no silhouette at all. These run 2% to 6%, which reads as
    /// mountains at the limb without turning the body into a rock.
    pub const fn relief(self) -> f32 {
        match self {
            Self::BarrenRock => 0.055,
            Self::DustWorld => 0.050,
            Self::IceWorld => 0.045,
            Self::Volcanic => 0.060,
            // The haze is the point: a greenhouse has weather, not terrain.
            Self::Greenhouse => 0.020,
            Self::Temperate => 0.050,
        }
    }

    /// The height fraction below which the surface flattens to a true sphere,
    /// if this type has a sea at all.
    ///
    /// Flattening is what makes an ocean read as an ocean: a sea painted onto
    /// a displaced surface still has hills in it. The lowest band of a type
    /// with a sea is therefore the sea, and the next band starts just above
    /// the flattened level.
    /// Exhaustive on purpose: a new type has to STATE whether it has a sea.
    /// A catch-all here would hand every future world a dry surface without
    /// anyone deciding that.
    pub const fn sea_level(self) -> Option<f32> {
        match self {
            Self::IceWorld => Some(0.30),
            Self::Temperate => Some(0.42),
            Self::BarrenRock | Self::DustWorld | Self::Volcanic | Self::Greenhouse => None,
        }
    }

    /// The type's warp, grain and bump parameters.
    pub const fn detail(self) -> PlanetDetail {
        match self {
            Self::BarrenRock => PlanetDetail {
                warp: 0.10,
                warp_frequency: 7.0,
                grain: 0.20,
                grain_frequency: 90.0,
                bump: 0.45,
            },
            Self::DustWorld => PlanetDetail {
                warp: 0.09,
                warp_frequency: 5.5,
                grain: 0.16,
                grain_frequency: 70.0,
                bump: 0.35,
            },
            Self::IceWorld => PlanetDetail {
                warp: 0.07,
                warp_frequency: 6.5,
                grain: 0.12,
                grain_frequency: 60.0,
                bump: 0.30,
            },
            Self::Volcanic => PlanetDetail {
                warp: 0.12,
                warp_frequency: 8.0,
                grain: 0.22,
                grain_frequency: 100.0,
                bump: 0.50,
            },
            // Soft and banded: the haze has no edges to break up.
            Self::Greenhouse => PlanetDetail {
                warp: 0.14,
                warp_frequency: 3.0,
                grain: 0.10,
                grain_frequency: 30.0,
                bump: 0.10,
            },
            Self::Temperate => PlanetDetail {
                warp: 0.08,
                warp_frequency: 6.0,
                grain: 0.14,
                grain_frequency: 55.0,
                bump: 0.30,
            },
        }
    }
}

/// The scenario surface for a planet: what kind of world it is, which one of
/// that kind, how big it is, and the gameplay it carries.
///
/// Deliberately shaped like [`AsteroidConfig`](super::asteroid::AsteroidConfig)
/// - radius in meters, an optional seed that pins the body across loads, a
/// mass, an invulnerable flag and a lock signature - so an author who can
/// place a rock can place a planet.
///
/// The one field that does NOT carry over is `texture`: a planet samples no
/// texture at all. Its whole surface comes from the type and the seed.
#[derive(Clone, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlanetConfig {
    /// Mean radius: the REAL size of the body, not a designation.
    ///
    /// The surface runs from `radius * (1 - relief)` to `radius * (1 + relief)`
    /// and the derived `BodyRadius` is the outer figure, so everything measured
    /// from the surface - the gravity well's clamp, the SOI, an orbit ring, a
    /// GOTO standoff - follows this number closely. An asteroid's `radius` is
    /// the opposite: a designation its noise mesh reaches about five times
    /// past. Porting a rock to a planet therefore means porting its DERIVED
    /// body radius, not the number in its config.
    pub radius: Meters,
    /// What kind of world this is.
    pub planet_type: PlanetType,
    /// Which world of that kind: the seed picks the biome in every slot, the
    /// cap latitude, the palette's tint, and the terrain itself.
    ///
    /// REQUIRED, and deliberately. A planet is a landmark an author frames a
    /// scene around, so which world it is cannot be a number the engine
    /// happens to pick. Omitting it fails the load rather than drawing a
    /// house world.
    pub seed: u32,
    /// How far the surface stands off the mean radius, at the highest point.
    /// `None` takes the type's own default ([`PlanetType::relief`]).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub relief: Option<Meters>,
    /// Override the type's sea level, as a fraction of the height range.
    /// `Some(0.0)` drains a sea; `None` takes the type's own.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub sea_level: Option<f32>,
    /// Well STRENGTH: the mass parameter making this body a gravity well.
    /// Same meaning and same units as
    /// [`AsteroidConfig::mass`](super::asteroid::AsteroidConfig::mass), and
    /// `None` falls back to the same global rule. Tune it by the sphere of
    /// influence you want, not by a number that means anything on its own.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub mass: Option<f32>,
    /// Whether weapons fire leaves this body alone.
    ///
    /// A planet a chapter is authored around must still be there at the end of
    /// it, so every authored planet today sets this. An invulnerable body also
    /// keeps its well for the whole scenario, because nothing can carve it.
    pub invulnerable: bool,
    /// Override how loud this body reads to the lock scanner. `None` is the
    /// mean radius, so a planet locks from proportionally far out.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub lock_signature: Option<Meters>,
}

impl PlanetConfig {
    /// The simplest planet an author can write: a type, a radius and a seed.
    /// Everything else is an override with a per-type default behind it.
    pub fn new(planet_type: PlanetType, radius: Meters, seed: u32) -> Self {
        Self {
            radius,
            planet_type,
            seed,
            relief: None,
            sea_level: None,
            mass: None,
            invulnerable: false,
            lock_signature: None,
        }
    }

    /// This config as a fixed body: a gravity well of `mass` that weapons fire
    /// leaves alone. The shape every authored planetoid takes today.
    pub fn anchored(mut self, mass: f32) -> Self {
        self.mass = Some(mass);
        self.invulnerable = true;
        self
    }

    /// The outer surface radius: what the derived `BodyRadius` becomes, and so
    /// what the gravity well, the SOI and an orbit ring are measured from.
    pub fn body_radius(&self) -> Meters {
        Meters(self.radius.get() * (1.0 + self.relief_fraction()))
    }

    /// The relief this config asks for, as a fraction of the radius.
    ///
    /// The authored relief is a HEIGHT in meters and the generator needs a
    /// fraction, so this is the only place that division happens. A
    /// non-positive radius would make it meaningless; `check_planet` in the
    /// scenario lint rejects one before it reaches here.
    pub fn relief_fraction(&self) -> f32 {
        match self.relief {
            Some(relief) => relief.get() / self.radius.get(),
            None => self.planet_type.relief(),
        }
    }
}

/// One resolved band: the biome the seed drew for a slot, tinted, with the
/// thresholds it claims.
#[derive(Clone, Copy, Debug)]
pub struct PlanetBand {
    /// The drawn biome's name.
    pub name: &'static str,
    /// The band's colour, already in LINEAR space.
    ///
    /// Converted here rather than in the shader because Bevy's PBR fragment
    /// does no conversion of its own - an sRGB triple handed to a uniform is
    /// the washed-out-colours bug the round-3 research names.
    pub color: LinearRgba,
    /// See [`Biome::roughness`].
    pub roughness: f32,
    /// See [`Biome::glow`].
    pub glow: f32,
    /// See [`BiomeSlot::floor`].
    pub floor: f32,
    /// See [`BiomeSlot::latitude_floor`].
    pub latitude_floor: f32,
}

/// One planet's resolved surface: the bands it wears and the numbers its shape
/// is generated from.
///
/// The whole reproducible unit. A type and a seed give exactly this, on every
/// load and in every process, and this is all the mesh generator and the
/// material need.
#[derive(Clone, Debug)]
pub struct PlanetSurface {
    /// The type this was drawn from.
    pub planet_type: PlanetType,
    /// The seed it was drawn with.
    pub seed: u32,
    /// The drawn bands, low first, any cap last.
    pub bands: Vec<PlanetBand>,
    /// The seed the terrain noise is built from. Derived from [`Self::seed`]
    /// rather than equal to it, so two planets whose seeds differ by one do
    /// not wear neighbouring terrain.
    pub shape_seed: u32,
    /// See [`PlanetType::sea_level`]. Already resolved against the config.
    pub sea_level: Option<f32>,
    /// How far the surface stands off the mean radius, as a fraction.
    pub relief: f32,
    /// The type's detail parameters, carried so the material has one thing to
    /// read.
    pub detail: PlanetDetail,
}

/// How far the seed may tint a whole palette, as a fraction.
///
/// Applied to every band at once so a planet reads as one world rather than as
/// a bag of unrelated colours. Small: past about a tenth a dust world starts
/// drifting off its own type.
const PALETTE_TINT: f32 = 0.07;

/// How far the seed may lighten or darken one band against its siblings.
const BAND_VALUE_JITTER: f32 = 0.06;

/// How far the seed may move an authored cap latitude.
const CAP_LATITUDE_JITTER: f32 = 0.07;

impl PlanetSurface {
    /// Draw the surface a config asks for.
    ///
    /// Every draw comes off one [`SeedStream`], in a fixed order, so the same
    /// seed is the same planet - and adding a slot to a palette reshuffles only
    /// the draws AFTER it, which is the property that lets a type grow without
    /// invalidating the seeds already authored against its earlier slots.
    pub fn generate(config: &PlanetConfig) -> Self {
        let planet_type = config.planet_type;
        let seed = config.seed;
        let mut stream = SeedStream::new(seed);

        let shape_seed = stream.next_u32();
        let tint = Vec3::new(
            1.0 + stream.signed() * PALETTE_TINT,
            1.0 + stream.signed() * PALETTE_TINT,
            1.0 + stream.signed() * PALETTE_TINT,
        );

        // A hard assert, not a debug one, and no truncation behind it. The
        // palettes are a built-in table, so overflowing the shader's band
        // array is a programming error in THIS crate - and silently dropping
        // the cap off a world in release is precisely the kind of quiet
        // fallback that makes a look bug unfindable.
        let slots = planet_type.slots();
        assert!(
            slots.len() <= PLANET_BAND_LIMIT,
            "{} authors {} bands, past the {PLANET_BAND_LIMIT} the shader holds",
            planet_type.name(),
            slots.len()
        );

        let bands = slots
            .iter()
            .map(|slot| {
                let biome = stream.pick(slot.choices);
                let value = 1.0 + stream.signed() * BAND_VALUE_JITTER;
                let latitude_floor = match slot.latitude_floor > 0.0 {
                    true => (slot.latitude_floor + stream.signed() * CAP_LATITUDE_JITTER)
                        .clamp(0.05, 0.98),
                    false => 0.0,
                };
                let linear = LinearRgba::from(biome.color);
                PlanetBand {
                    name: biome.name,
                    color: LinearRgba::new(
                        (linear.red * tint.x * value).clamp(0.0, 1.0),
                        (linear.green * tint.y * value).clamp(0.0, 1.0),
                        (linear.blue * tint.z * value).clamp(0.0, 1.0),
                        1.0,
                    ),
                    roughness: biome.roughness,
                    glow: biome.glow,
                    floor: slot.floor,
                    latitude_floor,
                }
            })
            .collect();

        Self {
            planet_type,
            seed,
            bands,
            shape_seed,
            sea_level: config.sea_level.or_else(|| planet_type.sea_level()),
            relief: config.relief_fraction(),
            detail: planet_type.detail(),
        }
    }

    /// A one-line description of what the seed drew, for a readout or a log.
    pub fn summary(&self) -> String {
        let bands: Vec<&str> = self.bands.iter().map(|band| band.name).collect();
        format!(
            "{} seed {} - {}",
            self.planet_type.name(),
            self.seed,
            bands.join(", ")
        )
    }
}

/// A deterministic stream of draws from one seed.
///
/// FNV-1a with a shift-xor finalizer, the hash this crate already derives
/// asteroid seeds and axis stretches with. Not a general PRNG and not trying
/// to be one: it has to be identical in every process and on every platform,
/// which an integer hash is and a floating-point generator is not.
struct SeedStream(u32);

impl SeedStream {
    fn new(seed: u32) -> Self {
        Self(0x811c_9dc5 ^ seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0100_0193);
        self.0 ^= self.0 >> 15;
        self.0
    }

    /// The next draw in `[0, 1)`.
    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    /// The next draw in `[-1, 1)`.
    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// One entry of `choices`. Panics on an empty slot, which is an authoring
    /// error rather than a runtime condition.
    /// One draw from a slot's choices. A slot with no choices is a hole in
    /// the palette table, so it fails here rather than drawing a house biome.
    fn pick<'choices, T>(&mut self, choices: &'choices [T]) -> &'choices T {
        assert!(
            !choices.is_empty(),
            "a biome slot offers no choices; every slot must name at least one"
        );
        &choices[self.next_u32() as usize % choices.len()]
    }
}

// ---------------------------------------------------------------------------
// The palettes. Authored low band first, any cap last.
// ---------------------------------------------------------------------------

const BARREN_ROCK_SLOTS: &[BiomeSlot] = &[
    BiomeSlot::band(
        0.0,
        &[
            Biome::rock("mare basalt", Color::srgb(0.22, 0.22, 0.24), 0.95),
            Biome::rock("shadow plain", Color::srgb(0.18, 0.19, 0.21), 0.95),
        ],
    ),
    BiomeSlot::band(
        0.30,
        &[
            Biome::rock("regolith", Color::srgb(0.42, 0.40, 0.36), 0.92),
            Biome::rock("grey dust", Color::srgb(0.46, 0.45, 0.43), 0.92),
        ],
    ),
    BiomeSlot::band(
        0.58,
        &[
            Biome::rock("highland", Color::srgb(0.56, 0.53, 0.48), 0.88),
            Biome::rock("pale highland", Color::srgb(0.62, 0.59, 0.54), 0.88),
        ],
    ),
    BiomeSlot::band(
        0.82,
        &[Biome::rock(
            "ridge chalk",
            Color::srgb(0.72, 0.70, 0.64),
            0.80,
        )],
    ),
];

const DUST_WORLD_SLOTS: &[BiomeSlot] = &[
    BiomeSlot::band(
        0.0,
        &[
            Biome::rock("basin floor", Color::srgb(0.42, 0.23, 0.15), 0.92),
            Biome::rock("dark basin", Color::srgb(0.35, 0.19, 0.13), 0.92),
        ],
    ),
    BiomeSlot::band(
        0.28,
        &[
            Biome::rock("rust plain", Color::srgb(0.61, 0.35, 0.20), 0.90),
            Biome::rock("ochre plain", Color::srgb(0.66, 0.42, 0.22), 0.90),
        ],
    ),
    BiomeSlot::band(
        0.55,
        &[
            Biome::rock("dust upland", Color::srgb(0.75, 0.54, 0.33), 0.88),
            Biome::rock("pale dust", Color::srgb(0.81, 0.63, 0.44), 0.88),
        ],
    ),
    BiomeSlot::band(
        0.80,
        &[Biome::rock(
            "oxide ridge",
            Color::srgb(0.85, 0.70, 0.54),
            0.85,
        )],
    ),
    BiomeSlot::cap(
        0.80,
        &[
            Biome::rock("frost cap", Color::srgb(0.87, 0.89, 0.91), 0.40),
            Biome::rock("dry-ice cap", Color::srgb(0.91, 0.93, 0.95), 0.35),
        ],
    ),
];

const ICE_WORLD_SLOTS: &[BiomeSlot] = &[
    BiomeSlot::band(
        0.0,
        &[
            Biome::rock("deep fissure", Color::srgb(0.11, 0.23, 0.30), 0.35),
            Biome::rock("dark ice", Color::srgb(0.13, 0.25, 0.31), 0.30),
        ],
    ),
    BiomeSlot::band(
        0.32,
        &[
            Biome::rock("ice sheet", Color::srgb(0.62, 0.78, 0.85), 0.34),
            Biome::rock("blue shelf", Color::srgb(0.56, 0.74, 0.82), 0.34),
        ],
    ),
    BiomeSlot::band(
        0.60,
        &[Biome::rock(
            "pressure ridge",
            Color::srgb(0.83, 0.90, 0.94),
            0.30,
        )],
    ),
    BiomeSlot::band(
        0.84,
        &[Biome::rock(
            "frost peak",
            Color::srgb(0.95, 0.97, 0.98),
            0.32,
        )],
    ),
    BiomeSlot::cap(
        0.88,
        &[Biome::rock("polar glare", Color::srgb(1.0, 1.0, 1.0), 0.30)],
    ),
];

const VOLCANIC_SLOTS: &[BiomeSlot] = &[
    BiomeSlot::band(
        0.0,
        &[
            Biome::molten("magma rift", Color::srgb(1.0, 0.35, 0.07), 0.60, 14.0),
            Biome::molten("lava lake", Color::srgb(1.0, 0.48, 0.12), 0.60, 20.0),
        ],
    ),
    BiomeSlot::band(
        0.22,
        &[
            Biome::rock("cooling crust", Color::srgb(0.29, 0.14, 0.09), 0.85),
            Biome::rock("cinder flat", Color::srgb(0.23, 0.12, 0.09), 0.85),
        ],
    ),
    BiomeSlot::band(
        0.46,
        &[Biome::rock(
            "basalt plain",
            Color::srgb(0.15, 0.14, 0.16),
            0.90,
        )],
    ),
    BiomeSlot::band(
        0.72,
        &[
            Biome::rock("ash slope", Color::srgb(0.42, 0.40, 0.38), 0.95),
            Biome::rock("sulphur slope", Color::srgb(0.48, 0.42, 0.29), 0.95),
        ],
    ),
    BiomeSlot::band(
        0.90,
        &[Biome::rock("ash peak", Color::srgb(0.60, 0.58, 0.55), 0.95)],
    ),
];

const GREENHOUSE_SLOTS: &[BiomeSlot] = &[
    BiomeSlot::band(
        0.0,
        &[
            Biome::rock("haze basin", Color::srgb(0.69, 0.60, 0.33), 0.70),
            Biome::rock("shadowed basin", Color::srgb(0.63, 0.55, 0.30), 0.70),
        ],
    ),
    BiomeSlot::band(
        0.35,
        &[
            Biome::rock("sulphur plain", Color::srgb(0.85, 0.75, 0.43), 0.65),
            Biome::rock("cream plain", Color::srgb(0.87, 0.79, 0.51), 0.65),
        ],
    ),
    BiomeSlot::band(
        0.68,
        &[
            Biome::rock("cloud deck", Color::srgb(0.93, 0.88, 0.66), 0.55),
            Biome::rock("pale deck", Color::srgb(0.95, 0.92, 0.77), 0.55),
        ],
    ),
];

const TEMPERATE_SLOTS: &[BiomeSlot] = &[
    BiomeSlot::band(
        0.0,
        &[
            Biome::rock("ocean", Color::srgb(0.09, 0.22, 0.31), 0.34),
            Biome::rock("deep sea", Color::srgb(0.06, 0.16, 0.23), 0.30),
        ],
    ),
    // Just above the sea level the surface flattens to, so the shore is the
    // first band the terrain can still climb through.
    BiomeSlot::band(
        0.425,
        &[
            Biome::rock("shore", Color::srgb(0.73, 0.66, 0.47), 0.70),
            Biome::rock("pale sand", Color::srgb(0.79, 0.73, 0.56), 0.70),
        ],
    ),
    BiomeSlot::band(
        0.48,
        &[
            Biome::rock("grass plain", Color::srgb(0.31, 0.44, 0.25), 0.80),
            Biome::rock("steppe", Color::srgb(0.49, 0.51, 0.28), 0.80),
            Biome::rock("savanna", Color::srgb(0.59, 0.56, 0.30), 0.80),
        ],
    ),
    BiomeSlot::band(
        0.64,
        &[
            Biome::rock("forest", Color::srgb(0.21, 0.31, 0.18), 0.85),
            Biome::rock("taiga", Color::srgb(0.20, 0.27, 0.23), 0.85),
        ],
    ),
    BiomeSlot::band(
        0.79,
        &[Biome::rock(
            "rock upland",
            Color::srgb(0.48, 0.44, 0.41),
            0.90,
        )],
    ),
    BiomeSlot::cap(
        0.86,
        &[Biome::rock("ice cap", Color::srgb(0.93, 0.96, 0.97), 0.30)],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The reproducibility contract the round is about: a type and a seed are
    /// one planet, in every process and on every load.
    #[test]
    fn the_same_type_and_seed_draw_the_same_planet() {
        for planet_type in PlanetType::ALL {
            for seed in [0u32, 1, 7, 4242, 20_260_904, u32::MAX] {
                let config = PlanetConfig::new(planet_type, Meters(200.0), seed);
                let once = PlanetSurface::generate(&config);
                let again = PlanetSurface::generate(&config);

                assert_eq!(once.shape_seed, again.shape_seed);
                assert_eq!(once.summary(), again.summary());
                for (left, right) in once.bands.iter().zip(&again.bands) {
                    assert_eq!(left.color, right.color, "{} seed {seed}", once.summary());
                    assert_eq!(left.latitude_floor, right.latitude_floor);
                }
            }
        }
    }

    /// A seed has to CHANGE something, or the config's headline knob is a lie.
    /// Checked across the types that author a choice, and over enough seeds
    /// that a two-way slot cannot pass by luck.
    #[test]
    fn different_seeds_draw_different_planets() {
        for planet_type in PlanetType::ALL {
            let summaries: Vec<String> = (0..24)
                .map(|seed| {
                    PlanetSurface::generate(&PlanetConfig::new(planet_type, Meters(200.0), seed))
                        .summary()
                })
                .collect();
            let distinct = summaries
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            assert!(
                distinct > 1,
                "{} draws one planet over 24 seeds",
                planet_type.name()
            );
        }
    }

    /// The shader holds a fixed-size band array, so no palette may outgrow it.
    #[test]
    fn no_palette_authors_more_bands_than_the_shader_holds() {
        for planet_type in PlanetType::ALL {
            let slots = planet_type.slots();
            assert!(
                !slots.is_empty() && slots.len() <= PLANET_BAND_LIMIT,
                "{} authors {} bands",
                planet_type.name(),
                slots.len()
            );
        }
    }

    /// Bands are matched last-wins, so an out-of-order palette would leave a
    /// band unreachable - it would be shadowed by the one authored after it.
    /// Latitude caps are exempt: a cap claims its latitude at any elevation
    /// and is authored last for exactly that reason.
    #[test]
    fn every_palette_is_authored_low_band_first() {
        for planet_type in PlanetType::ALL {
            let elevation: Vec<f32> = planet_type
                .slots()
                .iter()
                .filter(|slot| slot.latitude_floor == 0.0)
                .map(|slot| slot.floor)
                .collect();
            assert!(
                elevation.windows(2).all(|pair| pair[0] < pair[1]),
                "{} authors its bands out of order: {elevation:?}",
                planet_type.name()
            );
            assert_eq!(
                elevation.first(),
                Some(&0.0),
                "{}'s lowest band has to start at the floor",
                planet_type.name()
            );
        }
    }

    /// A cap is only ever the LAST slot: matching is last-wins, so a cap
    /// authored mid-palette would be overwritten by every band above it.
    #[test]
    fn a_polar_cap_is_always_the_last_band() {
        for planet_type in PlanetType::ALL {
            let slots = planet_type.slots();
            for (index, slot) in slots.iter().enumerate() {
                assert!(
                    slot.latitude_floor == 0.0 || index == slots.len() - 1,
                    "{} authors a cap at slot {index} of {}",
                    planet_type.name(),
                    slots.len()
                );
            }
        }
    }

    /// A type with a sea flattens everything below its sea level, so its
    /// second band must start ABOVE that level or it can never be reached.
    #[test]
    fn a_sea_leaves_room_for_the_band_above_it() {
        for planet_type in PlanetType::ALL {
            let Some(sea) = planet_type.sea_level() else {
                continue;
            };
            let next = planet_type
                .slots()
                .iter()
                .filter(|slot| slot.latitude_floor == 0.0)
                .nth(1)
                .expect("a type with a sea has a shore");
            assert!(
                next.floor > sea,
                "{}'s sea at {sea} drowns its next band at {}",
                planet_type.name(),
                next.floor
            );
        }
    }

    /// Relief is authored in meters against a radius in meters, and the
    /// generator has to read it as a fraction of that radius - not as a world
    /// unit, and not as a fraction already.
    #[test]
    fn authored_relief_reads_as_a_fraction_of_the_radius() {
        let config = PlanetConfig {
            relief: Some(Meters(40.0)),
            ..PlanetConfig::new(PlanetType::DustWorld, Meters(800.0), 0)
        };
        assert!((config.relief_fraction() - 0.05).abs() < 1e-6);

        let defaulted = PlanetConfig::new(PlanetType::DustWorld, Meters(800.0), 0);
        assert!((defaulted.relief_fraction() - PlanetType::DustWorld.relief()).abs() < 1e-6);
    }

    /// A planet that does not say what it is, or says something the build
    /// does not know, FAILS TO LOAD. There is no house type to fall back to.
    ///
    /// This is what buys the closed [`PlanetType`] enum its keep. An open
    /// string id would need a lint pass and a load-time check to say the same
    /// thing; a named variant gets it from the format, at the only point that
    /// can still tell the author which file was wrong.
    #[test]
    fn a_planet_must_name_a_type_the_build_knows() {
        let missing = r#"(radius: 800.0, seed: 1, invulnerable: true)"#;
        let unknown = r#"(radius: 800.0, planet_type: WaterWorld, seed: 1, invulnerable: true)"#;
        let unseeded = r#"(radius: 800.0, planet_type: DustWorld, invulnerable: true)"#;
        let unstated = r#"(radius: 800.0, planet_type: DustWorld, seed: 1)"#;

        for (authored, why) in [
            (missing, "a planet with no type"),
            (unknown, "a planet claiming a type the build does not have"),
            (unseeded, "a planet with no seed"),
            (
                unstated,
                "a planet that does not say whether it can be destroyed",
            ),
        ] {
            assert!(
                ron::from_str::<PlanetConfig>(authored).is_err(),
                "{why} must fail the load, not draw something"
            );
        }

        assert!(
            ron::from_str::<PlanetConfig>(
                r#"(radius: 800.0, planet_type: DustWorld, seed: 1, invulnerable: true)"#
            )
            .is_ok(),
            "the minimum honest planet still parses"
        );
    }

    /// The band colours reach the shader in LINEAR space. An sRGB triple in a
    /// uniform is the washed-out-colour bug the round-3 research names, and it
    /// is invisible in a screenshot until you compare two.
    #[test]
    fn band_colours_are_converted_to_linear() {
        let surface =
            PlanetSurface::generate(&PlanetConfig::new(PlanetType::Temperate, Meters(800.0), 0));
        let ocean = surface.bands[0].color;
        let authored = LinearRgba::from(Color::srgb(0.09, 0.22, 0.31));
        // Tinted, so not equal - but far below the sRGB values it came from.
        assert!(ocean.red < 0.05, "ocean red {} is not linear", ocean.red);
        assert!(
            (ocean.green - authored.green).abs() < 0.02,
            "ocean green {} drifted from {}",
            ocean.green,
            authored.green
        );
    }

    /// The authored RON shape, pinned: this is the config a scenario would
    /// carry, and the round record quotes it.
    #[cfg(feature = "serde")]
    #[test]
    fn a_planet_config_round_trips_through_ron() {
        let authored = r#"(
            radius: 800.0,
            planet_type: DustWorld,
            seed: 4242,
            invulnerable: true,
            relief: Some(40.0),
        )"#;
        let config: PlanetConfig = ron::from_str(authored).expect("authored planet config parses");

        assert_eq!(config.planet_type, PlanetType::DustWorld);
        assert_eq!(config.seed, 4242);
        assert!((config.radius.get() - 800.0).abs() < 1e-6);
        assert!((config.relief_fraction() - 0.05).abs() < 1e-6);
        assert_eq!(config.sea_level, None);

        let written = ron::to_string(&config).expect("a planet config serializes");
        let again: PlanetConfig = ron::from_str(&written).expect("and parses back");
        assert_eq!(again.planet_type, config.planet_type);
        assert_eq!(again.seed, config.seed);
    }
}
