//! The open world's civilizations: seeded identities whose influence overlaps
//! in 3D and says which of them a place's ships could come from.
//!
//! One civilization stands at each node of a coarse jittered lattice. Its
//! identity, name, living or extinct status, advancement and fleet-role
//! preference are a function of the world seed and the node alone, so the
//! same world has the same civilizations in any visit order. Influence falls
//! smoothly with 3D distance and reaches zero only at [`INFLUENCE_REACH`];
//! there is no territory border and no unclaimed pool.
//!
//! PURE, and read by debug diagnostics only: the live generator does not
//! select ships from it yet. Every constant here is provisional until the
//! diagnostic maps are reviewed.

use std::fmt;

use nova_events::prelude::{Meters, Meters3};
use nova_gameplay::prelude::{Fnv32, SeedStream};
use nova_world::prelude::*;

use crate::{clusters::ramp, environment::Environment};

/// The spacing of the civilization lattice: one civilization per node.
const CIVILIZATION_LATTICE: Meters = Meters(240_000.0);

/// How far a centroid may move off its node on each axis, in either direction.
const CIVILIZATION_JITTER: Meters = Meters(40_000.0);

/// The distance at which influence has fallen to one half.
const INFLUENCE_CORE: Meters = Meters(160_000.0);

/// The distance at and past which influence is zero.
const INFLUENCE_REACH: Meters = Meters(320_000.0);

/// The power the outer taper is raised to: `2 (R - C) / C`, the one exponent
/// that matches the inner parabola's slope at the core edge.
const INFLUENCE_TAPER: f32 = 2.0 * (INFLUENCE_REACH.0 - INFLUENCE_CORE.0) / INFLUENCE_CORE.0;

/// The centroid radius at which the advancement ramp reaches one.
const ADVANCEMENT_SATURATION: Meters = Meters(1_000_000.0);

/// The seeded advancement offset, drawn in `[-0.1, 0.1)`.
const ADVANCEMENT_VARIATION: f32 = 0.1;

/// The chance a civilization is still living.
const LIVING_CHANCE: f32 = 0.7;

/// The selection-weight ratio of a place straight outward from a centroid to
/// one straight inward at the same distance.
const OUTWARD_PREFERENCE: f32 = 4.0;

/// The lowest drawn role preference: each role draws `floor + [0, 1)`.
const ROLE_PREFERENCE_FLOOR: f32 = 0.5;

/// The chance one role is the civilization's specialty.
const SPECIALIST_CHANCE: f32 = 0.15;

/// How much a specialty multiplies that role's preference. A scavenger
/// specialist is the pirate-like case.
const SPECIALIST_FACTOR: f32 = 4.0;

/// The chance a civilization fields no scavengers, and separately the chance
/// it fields no armored ships. Civilian and industrial preference never reach
/// zero, so a civilization with no eligible weapon still has a buildable role.
const FIGHTER_ABSENT_CHANCE: f32 = 0.15;

/// The chance a name has three syllables rather than two.
const NAME_THREE_SYLLABLE_CHANCE: f32 = 0.5;

/// The syllables a name is composed from. Content: a change renames every
/// civilization in every world.
const NAME_SYLLABLES: [&str; 24] = [
    "an", "bel", "cor", "da", "el", "fen", "gar", "hal", "is", "jor", "ka", "lun", "mar", "nor",
    "os", "pra", "quel", "ren", "sol", "tev", "ur", "vas", "wen", "zo",
];

// Coverage: the farthest point from its nearest centroid is a lattice-gap
// corner with every jitter pulled away, `sqrt(3) * (L / 2 + J)`, 277 km. It
// must stay inside the reach, or some place has no civilization.
const _: () = assert!(
    3.0 * (CIVILIZATION_LATTICE.0 * 0.5 + CIVILIZATION_JITTER.0)
        * (CIVILIZATION_LATTICE.0 * 0.5 + CIVILIZATION_JITTER.0)
        < INFLUENCE_REACH.0 * INFLUENCE_REACH.0
);

// The taper exponent is above one, so influence meets zero with zero slope:
// `0 < C < 2R / 3`.
const _: () = assert!(INFLUENCE_CORE.0 > 0.0 && 3.0 * INFLUENCE_CORE.0 < 2.0 * INFLUENCE_REACH.0);

/// A civilization's stable machine identity: the world seed and its signed
/// lattice node.
///
/// Never a hash and never a name. Two nodes whose draw streams collide still
/// have distinct identities, and two civilizations may share a display name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilizationId {
    /// The world seed the civilization was drawn from.
    pub world_seed: u32,
    /// Its node on the civilization lattice.
    pub node: [i32; 3],
}

impl CivilizationId {
    /// The display name: two or three syllables, first letter capitalized.
    ///
    /// A pure function of the identity. Names repeat across a world, so a
    /// name must not be used to find a civilization.
    pub fn name(self) -> String {
        let mut stream = self.stream(b"name");
        let syllables = if stream.unit() < 1.0 - NAME_THREE_SYLLABLE_CHANCE {
            2
        } else {
            3
        };
        // Three syllable draws every time, so the syllable count does not
        // shift any later draw.
        let drawn: [&str; 3] = std::array::from_fn(|_| {
            let index = (stream.unit() * NAME_SYLLABLES.len() as f32) as usize;
            NAME_SYLLABLES[index.min(NAME_SYLLABLES.len() - 1)]
        });
        let joined = drawn[..syllables].concat();
        let mut letters = joined.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        })
    }

    /// The draw stream for one aspect of this civilization.
    ///
    /// Each aspect has its own stream, so retuning one aspect cannot move
    /// another: a new name inventory keeps every centroid and status.
    fn stream(self, aspect: &[u8]) -> SeedStream {
        SeedStream::new(
            Fnv32::new()
                .write(&self.world_seed.to_le_bytes())
                .write(b"civilization")
                .write(aspect)
                .write(&self.node[0].to_le_bytes())
                .write(&self.node[1].to_le_bytes())
                .write(&self.node[2].to_le_bytes())
                .finish(),
        )
    }
}

impl fmt::Display for CivilizationId {
    /// `civ_<x>_<y>_<z>@<seed>`, a negative index written `n<abs>`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "civ")?;
        for index in self.node {
            if index < 0 {
                write!(formatter, "_n{}", index.unsigned_abs())?;
            } else {
                write!(formatter, "_{index}")?;
            }
        }
        write!(formatter, "@{}", self.world_seed)
    }
}

/// Whether a civilization still fields intact ships or only left derelicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivilizationStatusType {
    /// Its ships are intact.
    Living,
    /// It left derelicts.
    Extinct,
}

/// The four closed ship roles a civilization fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipRoleType {
    /// Unarmed traffic.
    Civilian,
    /// Unarmed miners with cargo intake.
    Industrial,
    /// Rough, low-tier fighting ships.
    Scavenger,
    /// Equipped fighting ships.
    Armored,
}

impl ShipRoleType {
    /// Every role, in the order a role array holds them.
    pub const ALL: [Self; 4] = [
        Self::Civilian,
        Self::Industrial,
        Self::Scavenger,
        Self::Armored,
    ];

    /// What a readout or a legend calls the role.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Civilian => "civilian",
            Self::Industrial => "industrial",
            Self::Scavenger => "scavenger",
            Self::Armored => "armored",
        }
    }
}

/// How advancement rises with a centroid's distance from the world origin.
///
/// TODO(20260923-110307): delete once the owner picks the shipped curve from
/// the diagnostic maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdvancementCurveType {
    /// A clamped straight ramp to [`ADVANCEMENT_SATURATION`].
    Linear,
    /// A smoothstep to [`ADVANCEMENT_SATURATION`].
    Smoothstep,
}

/// One seeded civilization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Civilization {
    /// Its stable identity.
    pub id: CivilizationId,
    /// Where its influence is centred.
    pub centroid: Meters3,
    /// Whether it is living or extinct, drawn independently of advancement
    /// and distance.
    pub status: CivilizationStatusType,
    /// Its advancement in `[0, 1]`: the curve read at the centroid's distance
    /// from the origin, plus a seeded offset.
    pub advancement: f32,
    /// Its seeded preference for each role, in [`ShipRoleType::ALL`] order.
    /// Each is at least zero; only fighter roles can be zero.
    pub role_preference: [f32; 4],
}

impl Civilization {
    /// The weight of each role at a place with `environment`, in
    /// [`ShipRoleType::ALL`] order.
    ///
    /// Preference times an advancement factor times a local factor. Lower
    /// advancement favors scavengers and higher favors armored ships;
    /// material and volatiles favor industrial hulls and traffic favors
    /// civilian and armored ones. BEFORE content eligibility: a fighter role
    /// with no eligible weapon is not removed here.
    pub fn role_weights(&self, environment: Environment) -> [f32; 4] {
        let Environment {
            material_density: m,
            volatiles: v,
            human_activity: h,
        } = environment;
        let a = self.advancement;
        let factors = ShipRoleType::ALL.map(|role| match role {
            ShipRoleType::Civilian => 0.5 + h,
            ShipRoleType::Industrial => (0.5 + m) * (0.75 + 0.5 * v),
            ShipRoleType::Scavenger => 1.0 - 0.8 * a,
            ShipRoleType::Armored => (0.4 + 0.6 * a) * (0.75 + 0.5 * h),
        });
        std::array::from_fn(|index| self.role_preference[index] * factors[index])
    }
}

/// One civilization in reach of a place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CivilizationReach {
    /// The civilization.
    pub civilization: Civilization,
    /// The 3D distance from the place to its centroid, below the reach.
    pub distance: Meters,
    /// Its influence at the place, above zero.
    pub influence: f32,
    /// Its influence times the outward factor: the weight a static ship at
    /// the place is drawn from it with.
    pub selection_weight: f32,
}

/// The civilizations of one world seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilizationField {
    world_seed: u32,
    curve: AdvancementCurveType,
}

impl CivilizationField {
    /// The civilizations `world_seed` opens, with advancement read through
    /// `curve`.
    pub fn new(world_seed: u32, curve: AdvancementCurveType) -> Self {
        Self { world_seed, curve }
    }

    /// The civilization at `node`. Every draw is a fixed number off its own
    /// stream, so an outcome never shifts a later draw.
    pub fn civilization(&self, node: [i32; 3]) -> Civilization {
        let id = CivilizationId {
            world_seed: self.world_seed,
            node,
        };

        let mut centroid = id.stream(b"centroid");
        let [x, y, z] = node.map(|index| {
            index as f32 * CIVILIZATION_LATTICE.0 + centroid.signed() * CIVILIZATION_JITTER.0
        });
        let centroid = Meters3::new(x, y, z);

        let status = if id.stream(b"status").unit() < LIVING_CHANCE {
            CivilizationStatusType::Living
        } else {
            CivilizationStatusType::Extinct
        };

        let radius = centroid.length().0;
        let curve = match self.curve {
            AdvancementCurveType::Linear => (radius / ADVANCEMENT_SATURATION.0).clamp(0.0, 1.0),
            AdvancementCurveType::Smoothstep => ramp(radius, 0.0, ADVANCEMENT_SATURATION.0),
        };
        let offset = id.stream(b"advancement").signed() * ADVANCEMENT_VARIATION;
        let advancement = (curve + offset).clamp(0.0, 1.0);

        let mut roles = id.stream(b"roles");
        let mut role_preference: [f32; 4] =
            std::array::from_fn(|_| ROLE_PREFERENCE_FLOOR + roles.unit());
        let specialist = roles.unit() < SPECIALIST_CHANCE;
        let specialty = ((roles.unit() * 4.0) as usize).min(3);
        if specialist {
            role_preference[specialty] *= SPECIALIST_FACTOR;
        }
        let no_scavengers = roles.unit() < FIGHTER_ABSENT_CHANCE;
        let no_armored = roles.unit() < FIGHTER_ABSENT_CHANCE;
        if no_scavengers {
            role_preference[2] = 0.0;
        }
        if no_armored {
            role_preference[3] = 0.0;
        }

        Civilization {
            id,
            centroid,
            status,
            advancement,
            role_preference,
        }
    }

    /// Every civilization whose centroid is closer to `position` than the
    /// reach, in node order.
    ///
    /// A bounded scan: the nodes whose jittered centroid could be in reach,
    /// at most four per axis.
    ///
    /// # Errors
    ///
    /// [`SectorFault::InvalidGeometry`] for a position that is not finite or
    /// whose nodes fall outside `i32`, and [`SectorFault::Generation`] for a
    /// position with no civilization in reach. The lattice constants make the
    /// second unreachable; if it happens, it is a bug in them, and a place
    /// with no civilization must not quietly draw nothing.
    pub fn in_reach(&self, position: Meters3) -> Result<Vec<CivilizationReach>, SectorFault> {
        let refuse = || SectorFault::InvalidGeometry {
            id: "civilization_reach".to_string(),
        };
        if !position.get().is_finite() {
            return Err(refuse());
        }
        let lattice = f64::from(CIVILIZATION_LATTICE.0);
        let margin = f64::from(INFLUENCE_REACH.0 + CIVILIZATION_JITTER.0);
        let mut ranges = [(0, 0); 3];
        for (range, axis) in ranges.iter_mut().zip(position.get().to_array()) {
            let low = ((f64::from(axis) - margin) / lattice).ceil();
            let high = ((f64::from(axis) + margin) / lattice).floor();
            if low < f64::from(i32::MIN) || high > f64::from(i32::MAX) {
                return Err(refuse());
            }
            *range = (low as i32, high as i32);
        }

        let mut reach = Vec::new();
        for x in ranges[0].0..=ranges[0].1 {
            for y in ranges[1].0..=ranges[1].1 {
                for z in ranges[2].0..=ranges[2].1 {
                    let civilization = self.civilization([x, y, z]);
                    let distance = position.distance(civilization.centroid);
                    if distance.0 >= INFLUENCE_REACH.0 {
                        continue;
                    }
                    let influence = influence(distance);
                    reach.push(CivilizationReach {
                        civilization,
                        distance,
                        influence,
                        selection_weight: influence
                            * outward_factor(position, civilization.centroid),
                    });
                }
            }
        }

        if reach.is_empty() {
            let at = position.get();
            return Err(SectorFault::Generation {
                id: "civilization_reach".to_string(),
                field: "coverage",
                value: format!(
                    "no civilization within {} m of {:.0} {:.0} {:.0} m",
                    INFLUENCE_REACH.0, at.x, at.y, at.z
                ),
            });
        }
        Ok(reach)
    }
}

/// A civilization's influence at `distance` from its centroid.
///
/// Two parabolas joined with matching value and slope at the core edge:
/// `1 - (d/C)^2 / 2` inside the core, then `((R - d)/(R - C))^k / 2` out to
/// the reach. One at the centroid, one half at the core edge, positive inside
/// the reach, and zero with zero slope at and past it.
fn influence(distance: Meters) -> f32 {
    let (core, reach) = (INFLUENCE_CORE.0, INFLUENCE_REACH.0);
    let d = distance.0;
    if d >= reach {
        0.0
    } else if d <= core {
        1.0 - 0.5 * (d / core) * (d / core)
    } else {
        0.5 * ((reach - d) / (reach - core)).powf(INFLUENCE_TAPER)
    }
}

/// How much a place farther from the world origin than a centroid is favored
/// over one nearer to it.
///
/// Linear in the radial fraction `x = (|p| - |c|) / |p - c|`, which is `1`
/// straight outward, `-1` straight inward and `0` sideways: the factor is
/// [`OUTWARD_PREFERENCE`] outward, one inward and their mean sideways, so the
/// ratio holds at any distance and inward selection stays possible.
fn outward_factor(position: Meters3, centroid: Meters3) -> f32 {
    let distance = position.distance(centroid).0;
    let x = if distance > 0.0 {
        // The triangle inequality bounds this to [-1, 1]; the clamp only
        // absorbs rounding.
        ((position.length().0 - centroid.length().0) / distance).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    1.0 + (OUTWARD_PREFERENCE - 1.0) * (1.0 + x) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEEDS: [u32; 3] = [20_260_922, 12_345, 4_000_000_000];

    fn field(seed: u32) -> CivilizationField {
        CivilizationField::new(seed, AdvancementCurveType::Linear)
    }

    /// Points on a sphere of `radius`, spread by a golden-angle spiral.
    fn shell(radius: f32, count: usize) -> impl Iterator<Item = Meters3> {
        (0..count).map(move |index| {
            let height = 1.0 - 2.0 * (index as f32 + 0.5) / count as f32;
            let ring = (1.0 - height * height).sqrt();
            let turn = index as f32 * 2.399_963;
            Meters3::new(ring * turn.cos(), height, ring * turn.sin()) * radius
        })
    }

    /// The lattice-gap corners, where the nearest centroid can be farthest,
    /// and four shells out past advancement saturation: every place has a
    /// civilization in reach, and every one listed has positive influence.
    #[test]
    fn every_position_has_a_civilization_in_reach() {
        let lattice = CIVILIZATION_LATTICE.0;
        let corners = (-4..4).flat_map(move |x| {
            (-4..4).flat_map(move |y| {
                (-4..4).map(move |z| {
                    Meters3::new(
                        (x as f32 + 0.5) * lattice,
                        (y as f32 + 0.5) * lattice,
                        (z as f32 + 0.5) * lattice,
                    )
                })
            })
        });
        let shells = [0.0, 500_000.0, 1_000_000.0, 1_250_000.0]
            .into_iter()
            .flat_map(|radius| shell(radius, 200));
        let positions: Vec<Meters3> = corners.chain(shells).collect();

        for seed in SEEDS {
            let field = field(seed);
            for &position in &positions {
                let reach = field
                    .in_reach(position)
                    .unwrap_or_else(|fault| panic!("seed {seed} at {position:?}: {fault}"));
                for entry in reach {
                    assert!(
                        entry.influence > 0.0 && entry.distance.0 < INFLUENCE_REACH.0,
                        "seed {seed} at {position:?}: {} listed at {:?} with influence {}",
                        entry.civilization.id,
                        entry.distance,
                        entry.influence
                    );
                }
            }
        }
    }

    /// Halfway between two face-adjacent centroids, both civilizations count:
    /// influence overlaps, and there is no border where one takes over.
    #[test]
    fn neighbouring_civilizations_overlap_between_their_centroids() {
        for seed in SEEDS {
            let field = field(seed);
            for node in [[0, 0, 0], [3, -2, 1], [-4, 4, -1]] {
                for axis in 0..3 {
                    let mut neighbour = node;
                    neighbour[axis] += 1;
                    let (a, b) = (field.civilization(node), field.civilization(neighbour));
                    let midpoint = (a.centroid + b.centroid) * 0.5;
                    let reach = field
                        .in_reach(midpoint)
                        .unwrap_or_else(|fault| panic!("seed {seed} at {midpoint:?}: {fault}"));
                    for civilization in [a, b] {
                        assert!(
                            reach
                                .iter()
                                .any(|entry| entry.civilization.id == civilization.id
                                    && entry.influence > 0.0),
                            "seed {seed}: {} is missing halfway to its neighbour",
                            civilization.id
                        );
                    }
                }
            }
        }
    }

    /// Influence is one at the centroid, falls steadily, is one half at the
    /// core edge with no kink, stays positive a meter inside the reach and is
    /// zero at and past it.
    #[test]
    fn influence_falls_to_zero_only_at_the_reach() {
        let (core, reach) = (INFLUENCE_CORE.0, INFLUENCE_REACH.0);
        assert_eq!(influence(Meters(0.0)), 1.0);
        assert!((influence(INFLUENCE_CORE) - 0.5).abs() < 1e-6);
        assert!(influence(Meters(reach - 1.0)) > 0.0);
        assert_eq!(influence(INFLUENCE_REACH), 0.0);
        assert_eq!(influence(Meters(reach + 1.0)), 0.0);

        let mut previous = influence(Meters(0.0));
        for step in 1..320 {
            let next = influence(Meters(step as f32 * 1_000.0));
            assert!(next < previous, "influence rose at {step} km");
            previous = next;
        }

        let h = 10.0;
        let (below, at, above) = (
            influence(Meters(core - h)),
            influence(INFLUENCE_CORE),
            influence(Meters(core + h)),
        );
        let (slope_in, slope_out) = ((at - below) / h, (above - at) / h);
        assert!(
            (slope_in - slope_out).abs() < 1e-7,
            "slope {slope_in} inside the core edge and {slope_out} outside it"
        );
    }

    /// The same node is the same civilization from every query, in any
    /// order; the advancement curve changes advancement only.
    #[test]
    fn a_civilization_is_the_same_from_every_query() {
        for seed in SEEDS {
            let node = [1, -1, 2];
            let field = field(seed);
            let direct = field.civilization(node);
            let near = direct.centroid + Meters3::new(30_000.0, 0.0, 0.0);
            let far = direct.centroid + Meters3::new(-150_000.0, 90_000.0, 60_000.0);
            let listed = |position: Meters3| {
                field
                    .in_reach(position)
                    .unwrap_or_else(|fault| panic!("seed {seed} at {position:?}: {fault}"))
                    .into_iter()
                    .find(|entry| entry.civilization.id == direct.id)
                    .unwrap_or_else(|| panic!("seed {seed}: {} not in reach", direct.id))
                    .civilization
            };
            let (near_first, far_second) = (listed(near), listed(far));
            let (far_first, near_second) = (listed(far), listed(near));
            for civilization in [near_first, far_second, far_first, near_second] {
                assert_eq!(civilization, direct);
            }

            let smooth =
                CivilizationField::new(seed, AdvancementCurveType::Smoothstep).civilization(node);
            assert_eq!(
                (
                    smooth.id,
                    smooth.centroid,
                    smooth.status,
                    smooth.role_preference
                ),
                (
                    direct.id,
                    direct.centroid,
                    direct.status,
                    direct.role_preference
                )
            );
        }
    }

    /// A non-finite position refuses rather than reading as some place.
    #[test]
    fn a_non_finite_position_is_refused() {
        let field = field(SEEDS[0]);
        for position in [
            Meters3::new(f32::NAN, 0.0, 0.0),
            Meters3::new(0.0, f32::INFINITY, 0.0),
            Meters3::new(0.0, 0.0, f32::NEG_INFINITY),
        ] {
            assert!(
                matches!(
                    field.in_reach(position),
                    Err(SectorFault::InvalidGeometry { .. })
                ),
                "{position:?} must refuse"
            );
        }
    }
}
