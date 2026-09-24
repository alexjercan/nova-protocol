//! The clustered world's environment: three continuous noise fields that say
//! what kind of space a point is in.
//!
//! Example-owned, like the generator that reads it. The three fields are
//! INDEPENDENT - each has its own seed off the world seed and its own sample
//! origin - so a point can be rich in material and busy with traffic at once.
//! The generator reads the fields TOGETHER: a chance or a kind mix takes more
//! than one field, so where two fields overlap the world is different from
//! where either field is high alone.
//!
//! PURE: a reading is a function of the world seed and the position only.

use noise::{Fbm, MultiFractal, NoiseFn, Perlin};
use nova_protocol::prelude::*;
use nova_world::prelude::*;

/// One of the three environment fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvironmentField {
    /// How much rock and metal the space holds. Drives asteroid-rich,
    /// rock-only and planet-heavy groups, their rock counts and the
    /// background rock.
    MaterialDensity,
    /// How much ice and carbon the material is. Drives the rock and planet
    /// kind mix.
    Volatiles,
    /// How much traffic the space has seen. Drives derelict hulls, takes the
    /// worlds, and clears the background rock.
    HumanActivity,
}

impl EnvironmentField {
    /// Every field, in the order a reading and a legend list them.
    pub const ALL: [Self; 3] = [Self::MaterialDensity, Self::Volatiles, Self::HumanActivity];

    /// The field's slug: its seed domain and what a log calls it.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::MaterialDensity => "material_density",
            Self::Volatiles => "volatiles",
            Self::HumanActivity => "human_activity",
        }
    }

    /// What a readout or a legend calls the field.
    pub const fn label(self) -> &'static str {
        match self {
            Self::MaterialDensity => "material density",
            Self::Volatiles => "volatiles",
            Self::HumanActivity => "human activity",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::MaterialDensity => 0,
            Self::Volatiles => 1,
            Self::HumanActivity => 2,
        }
    }

    /// Where this field is read from, relative to the world.
    ///
    /// Perlin is identically zero on its own integer grid, one gridpoint a
    /// 160 km wavelength. The background rock reads the fields at cell
    /// centres, on multiples of 32 km, which meet that grid at every fifth
    /// cell; a group reads them at its jittered anchor. An offset that is not
    /// a multiple of 32 km on any axis keeps every cell centre off a
    /// gridpoint. A different offset per field also keeps the three fields
    /// from sharing their zero crossings.
    const fn origin(self) -> Meters3 {
        match self {
            Self::MaterialDensity => Meters3::new(91_000.0, 57_000.0, 131_000.0),
            Self::Volatiles => Meters3::new(-43_000.0, 119_000.0, 23_000.0),
            Self::HumanActivity => Meters3::new(67_000.0, -101_000.0, -77_000.0),
        }
    }
}

/// The wavelength of every field: how far apart two independent readings are.
///
/// 160 km, the width of the 5x5x5 window at 32 km cells: one window holds
/// about one rise and one fall of each field, so a flight across it walks
/// out of one kind of space and into another.
const FIELD_WAVELENGTH: Meters = Meters(160_000.0);

/// How many octaves each field sums. Three, so a region has an interior and a
/// ragged edge while the finest octave is still wider than a cell.
const FIELD_OCTAVES: usize = 3;

/// The raw reading that maps to the ends of `[0, 1]`.
///
/// Three octaves of Perlin read far inside their nominal `[-1, 1]`. Mapping
/// `+/-FIELD_SPAN` onto `[0, 1]` spreads the readings the field actually
/// takes across the whole range; a reading past it is clamped.
const FIELD_SPAN: f32 = 0.35;

/// A reading at or above this counts as HIGH for the biome label.
const BIOME_HIGH: f32 = 0.6;

/// The three fields, assembled once and then sampled.
///
/// `Fbm::new` seeds a permutation table per octave, so a caller builds this
/// once per cell or per paint and not once per sample.
pub struct EnvironmentFields([Fbm<Perlin>; 3]);

impl EnvironmentFields {
    /// The fields this world seed opens.
    pub fn new(world_seed: u32) -> Self {
        Self(EnvironmentField::ALL.map(|field| {
            // `build_sources` seeds octave `n` with `seed + n`, so a seed
            // within `octaves` of the ceiling overflows in a build with
            // overflow checks. The hash is uniform over the whole range.
            let seed = Fnv32::new()
                .write(&world_seed.to_le_bytes())
                .write(b"environment")
                .write(field.slug().as_bytes())
                .finish()
                % (u32::MAX - FIELD_OCTAVES as u32);
            Fbm::<Perlin>::new(seed)
                .set_frequency(1.0 / f64::from(FIELD_WAVELENGTH.get()))
                .set_octaves(FIELD_OCTAVES)
        }))
    }

    /// Read all three fields at `position`, each in `[0, 1]`.
    ///
    /// # Errors
    ///
    /// [`SectorFault::InvalidGeometry`] for a position that is not finite, and
    /// [`SectorFault::Generation`] for a reading that is not finite. A `NaN`
    /// compared against a chance is silently false, which would make a region
    /// of the world quietly empty.
    pub fn sample(&self, position: Meters3) -> Result<Environment, SectorFault> {
        if !position.get().is_finite() {
            return Err(SectorFault::InvalidGeometry {
                id: "environment_sample".to_string(),
            });
        }
        let mut values = [0.0; 3];
        for field in EnvironmentField::ALL {
            let point = (position + field.origin()).get();
            let raw = self.0[field.index()].get([
                f64::from(point.x),
                f64::from(point.y),
                f64::from(point.z),
            ]) as f32;
            if !raw.is_finite() {
                let at = position.get();
                return Err(SectorFault::Generation {
                    id: format!("environment_{}", field.slug()),
                    field: "reading",
                    value: format!("{raw} at {:.0} {:.0} {:.0} m", at.x, at.y, at.z),
                });
            }
            values[field.index()] = (raw / FIELD_SPAN * 0.5 + 0.5).clamp(0.0, 1.0);
        }
        Ok(Environment {
            material_density: values[0],
            volatiles: values[1],
            human_activity: values[2],
        })
    }
}

/// The three field readings at one point, each in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Environment {
    /// [`EnvironmentField::MaterialDensity`].
    pub material_density: f32,
    /// [`EnvironmentField::Volatiles`].
    pub volatiles: f32,
    /// [`EnvironmentField::HumanActivity`].
    pub human_activity: f32,
}

impl Environment {
    /// One field's reading.
    pub fn get(self, field: EnvironmentField) -> f32 {
        match field {
            EnvironmentField::MaterialDensity => self.material_density,
            EnvironmentField::Volatiles => self.volatiles,
            EnvironmentField::HumanActivity => self.human_activity,
        }
    }

    /// A name for the kind of space this is. DISPLAY ONLY: no generation
    /// decision reads it, because a label is a hard edge and the fields are
    /// continuous.
    pub fn biome(self) -> &'static str {
        let material = self.material_density >= BIOME_HIGH;
        let volatile = self.volatiles >= BIOME_HIGH;
        let busy = self.human_activity >= BIOME_HIGH;
        match (material, volatile, busy) {
            (true, _, true) => "worked belt",
            (true, true, false) => "ice belt",
            (true, false, false) => "rock belt",
            (false, _, true) => "traffic lane",
            (false, true, false) => "volatile haze",
            (false, false, false) => "open space",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-finite position refuses rather than reading as some value: a
    /// `NaN` reading would compare false against every chance.
    #[test]
    fn a_non_finite_position_is_refused() {
        let fields = EnvironmentFields::new(20_260_922);
        for position in [
            Meters3::new(f32::NAN, 0.0, 0.0),
            Meters3::new(0.0, f32::INFINITY, 0.0),
            Meters3::new(0.0, 0.0, f32::NEG_INFINITY),
        ] {
            assert!(
                matches!(
                    fields.sample(position),
                    Err(SectorFault::InvalidGeometry { .. })
                ),
                "{position:?} must refuse"
            );
        }
    }

    /// Every reading is in `[0, 1]`, over a lattice that spans several
    /// wavelengths.
    #[test]
    fn every_reading_is_inside_zero_to_one() {
        let fields = EnvironmentFields::new(20_260_922);
        for x in -8..=8 {
            for z in -8..=8 {
                let position = Meters3::new(x as f32 * 37_000.0, 11_000.0, z as f32 * 37_000.0);
                let reading = fields
                    .sample(position)
                    .unwrap_or_else(|fault| panic!("{position:?}: {fault}"));
                for field in EnvironmentField::ALL {
                    let value = reading.get(field);
                    assert!(
                        (0.0..=1.0).contains(&value),
                        "{} read {value} at {position:?}",
                        field.slug()
                    );
                }
            }
        }
    }
}
