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
//! Advancement is a smoothstep of the centroid's distance from the world
//! origin, saturating at [`ADVANCEMENT_SATURATION`], with a seeded offset
//! scaled by the curve value: slow near the origin and fast far out, so the
//! space a new ship starts in fields many small hulls and size climbs with
//! distance.
//!
//! Two civilizations are neighbours when their centroids are closer than
//! [`STANCE_NEIGHBOUR_RADIUS`]; the seed draws one symmetric [`StanceType`]
//! per pair of living neighbours ([`CivilizationField::edge`]) and a
//! designated rival keeps every living civilization with a living neighbour
//! on at least one Enemy edge. The seed also draws each civilization's
//! stance toward the player ([`Civilization::stance`]), which the player's
//! events move (`nova_gameplay::civilization::CivilizationStances`).
//!
//! PURE. The open world's ship planner reads it to pick each generated ship's
//! civilization and role, the stance systems read the neighbourhood, and
//! debug diagnostics read it too. The advancement curve and the stance
//! constants are decided; every other constant here is provisional until the
//! diagnostic maps are reviewed.

use nova_events::prelude::{Meters, Meters3, SectorCoord};
use nova_gameplay::prelude::{CivilizationId, Fnv32, SeedStream, ShipRoleType, StanceType};
use nova_world::prelude::*;

use crate::{
    clusters::ramp, environment::Environment, ARMOURED_STYLE_ID, CIVILIAN_STYLE_ID,
    INDUSTRIAL_STYLE_ID, SALVAGE_STYLE_ID,
};

/// The spacing of the civilization lattice: one civilization per node. Each
/// multiple of it from the origin is one ring of civilizations.
pub(crate) const CIVILIZATION_LATTICE: Meters = Meters(240_000.0);

/// The civilization ring of the cell `coord` on a grid of `edge`: the whole
/// lattice steps its centre lies from the world origin. Cell (0, 0, 0) is
/// ring 0; at a 32 km edge cell (8, 0, 0), 256 km out, is ring 1 and cell
/// (15, 0, 0), 480 km out, ring 2. The role goals count a job by the ring of
/// its posting station's cell.
pub(crate) fn ring_of(coord: SectorCoord, edge: Meters) -> u32 {
    (coord.centre(edge).length().get() / CIVILIZATION_LATTICE.get()).floor() as u32
}

/// How far a centroid may move off its node on each axis, in either direction.
pub(crate) const CIVILIZATION_JITTER: Meters = Meters(40_000.0);

/// The distance at which influence has fallen to one half.
const INFLUENCE_CORE: Meters = Meters(160_000.0);

/// The distance at and past which influence is zero.
pub(crate) const INFLUENCE_REACH: Meters = Meters(320_000.0);

/// The power the outer taper is raised to: `2 (R - C) / C`, the one exponent
/// that matches the inner parabola's slope at the core edge.
const INFLUENCE_TAPER: f32 = 2.0 * (INFLUENCE_REACH.0 - INFLUENCE_CORE.0) / INFLUENCE_CORE.0;

/// The centroid radius at which the advancement curve reaches one. Far
/// enough that the first four rings read at most 0.15, so they field small
/// hulls only; medium hulls appear near 2,000 km and the largest past
/// 3,000 km.
const ADVANCEMENT_SATURATION: Meters = Meters(4_000_000.0);

/// The seeded advancement offset as a share of the curve value: a
/// civilization reads `curve * (1 + [-0.1, 0.1))`. Scaled by the curve so an
/// inner civilization varies by a few hundredths and cannot lift its ring
/// into larger hulls, while one at saturation still varies by a tenth.
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

/// How close two centroids must be for their civilizations to be
/// neighbours: the smallest round radius above the farthest two face-adjacent
/// centroids can lie (`sqrt(L^2 + 2 (2J)^2)`, 339.4 km) and 60 km under the
/// nearest two centroids two nodes apart can lie (`L * 2 - 4 J`, 400 km), so
/// every face neighbour is a neighbour and no neighbour lies outside the
/// 3x3x3 block of nodes round a node, for every seed.
pub const STANCE_NEIGHBOUR_RADIUS: Meters = Meters(340_000.0);

/// The share of neighbour pairs whose drawn edge is Ally.
pub const STANCE_ALLY_SHARE: f32 = 0.25;

/// The share of neighbour pairs whose drawn edge is Enemy, before the
/// designated rival turns more.
pub const STANCE_ENEMY_SHARE: f32 = 0.25;

// Every face pair is within the radius (one lattice step plus the two
// jitters' worst case) and no pair two nodes apart is (two steps less two
// jitters toward each other).
const _: () = assert!(
    CIVILIZATION_LATTICE.0 * CIVILIZATION_LATTICE.0
        + 8.0 * CIVILIZATION_JITTER.0 * CIVILIZATION_JITTER.0
        < STANCE_NEIGHBOUR_RADIUS.0 * STANCE_NEIGHBOUR_RADIUS.0
        && STANCE_NEIGHBOUR_RADIUS.0 <= 2.0 * CIVILIZATION_LATTICE.0 - 2.0 * CIVILIZATION_JITTER.0
);

/// How much a specialty multiplies that role's preference. A scavenger
/// specialist is the pirate-like case.
const SPECIALIST_FACTOR: f32 = 4.0;

/// The chance a civilization fields no scavengers, and separately the chance
/// it fields no armored ships. Civilian and industrial preference never reach
/// zero, so a civilization with no eligible weapon still has a buildable role.
const FIGHTER_ABSENT_CHANCE: f32 = 0.15;

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

/// Whether a civilization still fields intact ships or only left derelicts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CivilizationStatusType {
    /// Its ships are intact.
    Living,
    /// It left derelicts.
    Extinct,
}

/// The id of the base style every ship of `role` wears, intact or wrecked.
/// The open world refuses to arm over content that authors none of these.
pub const fn role_style_id(role: ShipRoleType) -> &'static str {
    match role {
        ShipRoleType::Civilian => CIVILIAN_STYLE_ID,
        ShipRoleType::Industrial => INDUSTRIAL_STYLE_ID,
        ShipRoleType::Scavenger => SALVAGE_STYLE_ID,
        ShipRoleType::Armored => ARMOURED_STYLE_ID,
    }
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
    /// from the origin, plus a seeded offset scaled by that curve value.
    pub advancement: f32,
    /// Its seeded preference for each role, in [`ShipRoleType::ALL`] order.
    /// Each is at least zero; only fighter roles can be zero.
    pub role_preference: [f32; 4],
    /// Its seeded stance toward the player, drawn independently of every
    /// other trait: Enemy, Ally or Neutral with one chance in three each. The
    /// player's events move the table stance off it; its crews fly the
    /// effective stance's side.
    pub stance: StanceType,
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
}

impl CivilizationField {
    /// The civilizations `world_seed` opens.
    pub fn new(world_seed: u32) -> Self {
        Self { world_seed }
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

        let curve = advancement_curve(centroid.length());
        let offset = id.stream(b"advancement").signed() * ADVANCEMENT_VARIATION;
        let advancement = (curve * (1.0 + offset)).clamp(0.0, 1.0);

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

        let stances = [StanceType::Enemy, StanceType::Ally, StanceType::Neutral];
        let stance = stances[((id.stream(b"allegiance").unit() * 3.0) as usize).min(2)];

        Civilization {
            id,
            centroid,
            status,
            advancement,
            role_preference,
            stance,
        }
    }

    /// The living neighbours of `node`: every living civilization other than
    /// `node`'s whose centroid is closer than [`STANCE_NEIGHBOUR_RADIUS`], in
    /// node order; none for an extinct node, which holds no edge. Only the
    /// 3x3x3 block round the node is read: the lattice and jitter bound
    /// every farther pair past the radius.
    pub fn neighbours(&self, node: [i32; 3]) -> Vec<Civilization> {
        let centre = self.civilization(node);
        let mut found = Vec::new();
        if centre.status != CivilizationStatusType::Living {
            return found;
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if (dx, dy, dz) == (0, 0, 0) {
                        continue;
                    }
                    let other = self.civilization([node[0] + dx, node[1] + dy, node[2] + dz]);
                    if other.status == CivilizationStatusType::Living
                        && centre.centroid.distance(other.centroid).0 < STANCE_NEIGHBOUR_RADIUS.0
                    {
                        found.push(other);
                    }
                }
            }
        }
        found
    }

    /// The stance two living neighbours `a` and `b` hold toward each other:
    /// the drawn edge, turned Enemy when either designates the other as its
    /// rival. `None` when they are not living neighbours. Symmetric: the
    /// pair stream is keyed by the two nodes in order, and each designation
    /// reads only its own civilization's drawn edges.
    pub fn edge(&self, a: [i32; 3], b: [i32; 3]) -> Option<StanceType> {
        if a == b || !self.neighbours(a).iter().any(|other| other.id.node == b) {
            return None;
        }
        let drawn = self.drawn_edge(a, b);
        if drawn == StanceType::Enemy || self.rival(a) == Some(b) || self.rival(b) == Some(a) {
            return Some(StanceType::Enemy);
        }
        Some(drawn)
    }

    /// The pair stream of `a` and `b`: the world seed, `stance` and the two
    /// nodes in lexicographic order, so either end reads the same draws.
    fn pair_stream(&self, a: [i32; 3], b: [i32; 3]) -> SeedStream {
        let (first, second) = if a <= b { (a, b) } else { (b, a) };
        let mut hash = Fnv32::new()
            .write(&self.world_seed.to_le_bytes())
            .write(b"stance");
        for index in first.into_iter().chain(second) {
            hash = hash.write(&index.to_le_bytes());
        }
        SeedStream::new(hash.finish())
    }

    /// The edge the pair stream draws before any rival: the first draw
    /// against the shares.
    fn drawn_edge(&self, a: [i32; 3], b: [i32; 3]) -> StanceType {
        let unit = self.pair_stream(a, b).unit();
        if unit < STANCE_ALLY_SHARE {
            StanceType::Ally
        } else if unit >= 1.0 - STANCE_ENEMY_SHARE {
            StanceType::Enemy
        } else {
            StanceType::Neutral
        }
    }

    /// The living neighbour `node` designates as its rival: `None` when it
    /// has no living neighbour or one of its drawn edges is Enemy already;
    /// else the neighbour with the lowest second draw of their pair stream.
    fn rival(&self, node: [i32; 3]) -> Option<[i32; 3]> {
        let neighbours = self.neighbours(node);
        if neighbours
            .iter()
            .any(|other| self.drawn_edge(node, other.id.node) == StanceType::Enemy)
        {
            return None;
        }
        neighbours
            .iter()
            .map(|other| {
                let mut stream = self.pair_stream(node, other.id.node);
                let _first = stream.unit();
                (stream.unit(), other.id.node)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)))
            .map(|(_, other)| other)
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

/// The advancement a centroid at `radius` from the world origin reads before
/// its seeded offset: a smoothstep from zero at the origin to one at
/// [`ADVANCEMENT_SATURATION`]. Slow early and fast late, so many small ships
/// come first and larger ones only far out; a straight ramp to the same
/// radius would read the first ring at 0.06, not 0.01.
fn advancement_curve(radius: Meters) -> f32 {
    ramp(radius.0, 0.0, ADVANCEMENT_SATURATION.0)
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
pub(crate) mod sweep {
    //! The seeds and nodes the stance sweep tests of this crate run over.

    /// The seeds the stance sweep runs: the three above and the thirteen the
    /// research census used.
    pub(crate) const SWEEP_SEEDS: [u32; 16] = [
        20_260_922,
        12_345,
        4_000_000_000,
        1,
        2,
        3,
        7,
        10,
        42,
        99,
        1234,
        2026,
        31337,
        65536,
        123_456_789,
        987_654_321,
    ];

    /// Every node of the sweep's centre cube.
    pub(crate) fn sweep_nodes() -> impl Iterator<Item = [i32; 3]> {
        (-4..=4).flat_map(|x| (-4..=4).flat_map(move |y| (-4..=4).map(move |z| [x, y, z])))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        sweep::{sweep_nodes, SWEEP_SEEDS},
        *,
    };

    const SEEDS: [u32; 3] = [20_260_922, 12_345, 4_000_000_000];

    fn field(seed: u32) -> CivilizationField {
        CivilizationField::new(seed)
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
    /// and four shells out to 1,250 km: every place has a
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
    /// order.
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
        }
    }

    /// A cell's ring is whole lattice steps of its centre: on a 32 km grid
    /// cells 0 and 7 along an axis (0 and 224 km) are ring 0, cell 8 (256 km)
    /// ring 1 and cell 15 (480 km) ring 2; a diagonal cell counts its full
    /// distance, so (6, 6, 0) at 271 km is ring 1.
    #[test]
    fn ring_of_reads_whole_lattice_steps_from_the_cell_centre() {
        let edge = Meters(32_000.0);
        for (cell, ring) in [
            (SectorCoord::new(0, 0, 0), 0),
            (SectorCoord::new(7, 0, 0), 0),
            (SectorCoord::new(-8, 0, 0), 1),
            (SectorCoord::new(0, 8, 0), 1),
            (SectorCoord::new(15, 0, 0), 2),
            (SectorCoord::new(6, 6, 0), 1),
        ] {
            assert_eq!(ring_of(cell, edge), ring, "{cell}");
        }
    }

    /// Advancement before the offset reads the decided ring table, rounded to
    /// hundredths: 0.01, 0.04, 0.09 and 0.15 at rings one to four, 0.50 at
    /// 2,000 km, 0.84 at 3,000 km and 1.00 at 4,000 km. Each civilization's
    /// offset stays within a tenth of its own curve value, so an inner
    /// civilization varies by a few hundredths at most.
    #[test]
    fn advancement_reads_the_ring_table_and_varies_by_a_tenth_of_its_curve() {
        let expected = [0.01, 0.04, 0.09, 0.15, 0.50, 0.84, 1.00];
        let radii = (1..=4)
            .map(|ring| ring as f32 * CIVILIZATION_LATTICE.0)
            .chain([2_000_000.0, 3_000_000.0, 4_000_000.0]);
        for (radius, expected) in radii.zip(expected) {
            let curve = advancement_curve(Meters(radius));
            assert_eq!(
                (curve * 100.0).round() / 100.0,
                expected,
                "{} km reads {curve}",
                radius / 1_000.0
            );
        }

        for seed in SEEDS {
            let field = field(seed);
            for x in -6..=6 {
                for y in -6..=6 {
                    for z in -2..=2 {
                        let civilization = field.civilization([x, y, z]);
                        let curve = advancement_curve(civilization.centroid.length());
                        assert!(
                            (civilization.advancement - curve).abs()
                                <= ADVANCEMENT_VARIATION * curve + 1e-6,
                            "seed {seed}: {} reads {} on a curve of {curve}",
                            civilization.id,
                            civilization.advancement
                        );
                    }
                }
            }
        }
    }

    /// Each stance holds about a third of the civilizations, whatever their
    /// status: the stance is its own draw, not a reading of another trait.
    #[test]
    fn a_civilization_stance_is_an_even_draw_of_its_own() {
        for seed in SEEDS {
            let field = field(seed);
            let mut counts = [[0_usize; 3]; 2];
            for x in -6..6 {
                for y in -6..6 {
                    for z in -2..2 {
                        let civilization = field.civilization([x, y, z]);
                        let side = match civilization.stance {
                            StanceType::Enemy => 0,
                            StanceType::Ally => 1,
                            StanceType::Neutral => 2,
                        };
                        let status =
                            usize::from(civilization.status == CivilizationStatusType::Extinct);
                        counts[status][side] += 1;
                    }
                }
            }
            for (status, sides) in counts.iter().enumerate() {
                let total: usize = sides.iter().sum();
                for (side, count) in sides.iter().enumerate() {
                    let share = *count as f32 / total as f32;
                    assert!(
                        (0.25..0.42).contains(&share),
                        "seed {seed} status {status} side {side}: {count} of {total}"
                    );
                }
            }
        }
    }

    /// After the designated-rival rule, every living civilization with a
    /// living neighbour holds at least one Enemy edge, in every seed.
    #[test]
    fn every_living_civilization_with_a_living_neighbour_has_a_rival() {
        for seed in SWEEP_SEEDS {
            let field = field(seed);
            for node in sweep_nodes() {
                if field.civilization(node).status != CivilizationStatusType::Living {
                    continue;
                }
                let neighbours = field.neighbours(node);
                if neighbours.is_empty() {
                    continue;
                }
                assert!(
                    neighbours
                        .iter()
                        .any(|other| field.edge(node, other.id.node) == Some(StanceType::Enemy)),
                    "seed {seed}: {} has {} living neighbours and no rival",
                    field.civilization(node).id,
                    neighbours.len()
                );
            }
        }
    }

    /// An edge reads the same from either end and from a fresh field; a
    /// pair that is not two living neighbours has none.
    #[test]
    fn an_edge_reads_the_same_from_both_ends_and_in_any_order() {
        for seed in SWEEP_SEEDS {
            let field = field(seed);
            for node in sweep_nodes() {
                for other in field.neighbours(node) {
                    let forward = field.edge(node, other.id.node);
                    let backward = field.edge(other.id.node, node);
                    let fresh = CivilizationField::new(seed).edge(other.id.node, node);
                    assert!(
                        forward.is_some() && forward == backward && forward == fresh,
                        "seed {seed}: {node:?} -> {:?} reads {forward:?}, back {backward:?}, fresh {fresh:?}",
                        other.id.node
                    );
                }
            }
            let (a, b) = ([0, 0, 0], [2, 0, 0]);
            assert_eq!(field.edge(a, b), None, "seed {seed}: two nodes apart");
            assert_eq!(field.edge(a, a), None, "seed {seed}: a node to itself");
        }
    }

    /// Every neighbour lies in the 3x3x3 block round the node, within the
    /// radius, and is living; every living face neighbour of a living node
    /// is a neighbour, and an extinct node has none.
    #[test]
    fn a_neighbourhood_never_leaves_the_block_round_the_node() {
        for seed in SWEEP_SEEDS {
            let field = field(seed);
            for node in sweep_nodes() {
                let centre = field.civilization(node);
                for other in field.neighbours(node) {
                    let offset: Vec<i32> = (0..3)
                        .map(|axis| other.id.node[axis] - node[axis])
                        .collect();
                    assert!(
                        offset.iter().all(|step| (-1..=1).contains(step)) && offset != [0, 0, 0],
                        "seed {seed}: {node:?} lists {:?}",
                        other.id.node
                    );
                    assert!(centre.centroid.distance(other.centroid).0 < STANCE_NEIGHBOUR_RADIUS.0);
                    assert_eq!(other.status, CivilizationStatusType::Living);
                }
                if centre.status != CivilizationStatusType::Living {
                    assert!(
                        field.neighbours(node).is_empty(),
                        "seed {seed}: {node:?} is extinct"
                    );
                    continue;
                }
                for axis in 0..3 {
                    for step in [-1, 1] {
                        let mut face = node;
                        face[axis] += step;
                        let other = field.civilization(face);
                        if other.status == CivilizationStatusType::Living {
                            assert!(
                                field.neighbours(node).iter().any(|found| found.id == other.id),
                                "seed {seed}: the living face neighbour {face:?} of {node:?} is missing"
                            );
                        }
                    }
                }
            }
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
