//! DECORATION: the greebles a style scatters over a derived skin.
//!
//! A decoration is a [`SectionFixture`] in the full sense - health, mass, a
//! collider, shot off and gone - and it is a child of the PLATE it stands on,
//! so a plate coming off takes its greebles and a destroyed section takes both.
//! Nothing here is behaviour: if shooting a thing off should cost the ship a
//! capability it is a section, not decoration.
//!
//! # The scatter is a pure function of the structure
//!
//! There is no RNG anywhere in this module and there must never be one. A
//! decoration is claimed by hashing the CELL it would stand in together with the
//! fixture's id, so:
//!
//! - the same ship always wears the same greebles, saved or not saved;
//! - the editor can re-derive the whole skin every frame the ghost moves a cell
//!   without anything flickering, which is the only reason showing it live is
//!   safe at all;
//! - two ships built the same way are decorated the same way, and one built
//!   somewhere else on the lattice is not.
//!
//! The claim is on a GRID rather than by blue noise, which the decoration
//! research settled: Poisson sampling deliberately destroys alignment, and
//! alignment is the whole difference between a greeble that reads as bolted on
//! and one that reads as confetti. A rule claims cells on its own stride and a
//! piece is yawed to the direction the surface runs or to the direction it
//! falls away, both from the plate vocabulary.
//!
//! One thing is decided by a BLOCK of hull rather than by a cell, and it is
//! marked where it happens: the per-patch floor
//! ([`fill_patches`]) puts a piece back wherever a rule's share thinned a whole
//! block of ship down to nothing, because every other knob here is per plate and
//! they multiply - a rule tuned on a 150-plate hull otherwise lands one piece on
//! a 20-plate one. A block is a fixed division of the ship's own cells, so a
//! hull that grows keeps every piece outside the block it grew into.

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::Collider;
use bevy::{platform::collections::HashMap, prelude::*};
use nova_gameplay::prelude::{destructible_body, AssetRef};

use crate::sections::{
    fixture::prelude::SectionFixture,
    shell_shape::REACH,
    shell_skin::SkinPlate,
    skin_reading::PlateReading,
    skin_style::{ScatterAlign, ShipStyleConfig, StyleFixtureConfig},
};

/// The scatter, the placement it answers with, the decoration bundle, and the
/// per-rule reach and tally a style author tunes against.
pub mod prelude {
    pub use super::{
        decor_body, decor_pose, decor_reach, decor_tally, scatter_decor, DecorPlacement,
        DecorReason, ShipDecorMarker,
    };
}

/// The smallest a decoration's collider box may be on any axis, in cells.
///
/// A zero-volume collider has no mass and nothing to hit, so a fixture authored
/// without one would be art a round passes through. Clamped rather than refused:
/// a style is data, and bad data must not stop a ship spawning.
const MIN_COLLIDER: f32 = 0.02;

/// One decoration the scatter claimed, as INDICES into the two lists it was
/// handed.
///
/// Indices rather than entities or clones, because the same answer has to serve
/// a live ship (which spawns fixtures) and the editor's preview (which spawns
/// pictures of them), and neither should have to hand the scatter its world.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DecorPlacement {
    /// Which plate takes the piece, indexing the plates passed in.
    pub plate: usize,
    /// Which of the style's fixtures it is, indexing
    /// [`ShipStyleConfig::fixtures`].
    pub fixture: usize,
    /// Quarter turns about the plate's own outward axis, turning the piece to
    /// the axis its rule aligns to. `0` for a piece that does not align, and
    /// for one whose plate has no such axis to turn to.
    pub turns: u8,
    /// WHICH of the two ways of claiming a plate put this piece here.
    pub reason: DecorReason,
}

/// Why a piece stands where it stands.
///
/// Two answers, and the difference is what a dump is read for: a rule that
/// covers a hull through its own share is tuned, and one whose pieces are
/// nearly all [`Patch`](DecorReason::Patch) is a rule whose share was thinner
/// than the hull it landed on and is being carried by the floor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DecorReason {
    /// The rule's own filter, lattice and share all admitted the plate.
    Rule,
    /// The share left this block of hull bare and the per-patch floor put one
    /// piece back. See [`fill_patches`].
    Patch,
}

impl DecorReason {
    /// The stable lowercase name a dump spells this with.
    pub fn name(self) -> &'static str {
        match self {
            Self::Rule => "rule",
            Self::Patch => "patch",
        }
    }
}

/// Every decoration a style puts on a derived skin.
///
/// Takes the VOCABULARY and not the structure: everything a rule asks about is
/// in [`read_plates`](super::skin_reading::read_plates)'s answer, so the scatter
/// cannot reach past it into the derivation and a caller reads the
/// neighbourhood once for both this and whatever else wants it.
///
/// Deterministic in every part: the plates arrive in the derivation's order, the
/// fixtures are tried in the style's authored order, and the claim is a hash of
/// the cell. Nothing is sampled and nothing is carried between plates, so this
/// can be run on one plate or on a whole ship with the same answer.
///
/// A plate takes AT MOST ONE piece. The alternative - letting every rule that
/// matches drop something - reads as a pile rather than as a ship, and it makes
/// two independently authored fixtures interfere. First rule wins, which is what
/// makes the authored order a priority order.
pub fn scatter_decor(
    plates: &[SkinPlate],
    readings: &[PlateReading],
    style: &ShipStyleConfig,
) -> Vec<DecorPlacement> {
    let mut claimed: Vec<Option<usize>> = plates
        .iter()
        .zip(readings)
        .map(|(plate, reading)| {
            style
                .fixtures
                .iter()
                .position(|fixture| claims(fixture, plate, reading))
        })
        .collect();
    // Kept so the answer can say WHICH pass put each piece down: everything the
    // floor adds is a slot that was `None` here.
    let by_share = claimed.clone();
    fill_patches(plates, readings, style, &mut claimed);

    claimed
        .into_iter()
        .enumerate()
        .filter_map(|(index, fixture)| {
            let fixture = fixture?;
            Some(DecorPlacement {
                plate: index,
                fixture,
                turns: turns_for(&plates[index], &readings[index], &style.fixtures[fixture]),
                reason: match by_share[index] {
                    Some(_) => DecorReason::Rule,
                    None => DecorReason::Patch,
                },
            })
        })
        .collect()
}

/// How many plates of a hull each of a style's rules can stand on AT ALL -
/// everything its filter and its lattice admit, before the share thins it and
/// before priority takes any of it away. Index-aligned with
/// [`ShipStyleConfig::fixtures`].
///
/// The number a priority order hides, and the one a style author is most often
/// wrong about. A rule can read as narrow and admit four fifths of a ship; put
/// first, it then carpets the hull and every rule under it lands nothing, and
/// from a screenshot that is indistinguishable from a rule whose filter was
/// simply too tight. This against the tally of what each rule actually TOOK
/// tells the two apart.
///
/// Deliberately BEFORE the share, so it is an upper bound a rule can be read
/// against: the share is a knob the author already set, and folding it in would
/// mix "this hull has nowhere for my piece" with "I asked for less".
///
/// Diagnostic only: nothing in the placement path reads this, and the spawn log
/// asks for it only when the log level would print it.
pub fn decor_reach(
    plates: &[SkinPlate],
    readings: &[PlateReading],
    style: &ShipStyleConfig,
) -> Vec<usize> {
    style
        .fixtures
        .iter()
        .map(|fixture| {
            plates
                .iter()
                .zip(readings)
                .filter(|(plate, reading)| eligible(fixture, plate, reading))
                .count()
        })
        .collect()
}

/// What each of a style's rules took, against what it would have taken with
/// nothing above it - `name xTAKEN of REACH`, listing the starved rules too.
///
/// A rule that takes nothing is the one worth printing, and the two ways it can
/// happen look identical on screen: `x0 of 78` says a rule above it ate the
/// hull or its own share threw the hull away, and `x0 of 0` says its filter
/// matches nothing this hull offers at all. Those are opposite mistakes with
/// opposite fixes.
pub fn decor_tally(style: &ShipStyleConfig, taken: &[usize], reach: &[usize]) -> String {
    let listed: Vec<String> = style
        .fixtures
        .iter()
        .enumerate()
        .map(|(index, fixture)| {
            format!(
                "{} x{} of {}",
                fixture.id,
                taken.get(index).copied().unwrap_or_default(),
                reach.get(index).copied().unwrap_or_default(),
            )
        })
        .collect();
    match listed.is_empty() {
        true => "none".to_string(),
        false => listed.join(", "),
    }
}

/// Whether one fixture claims one plate: the neighbourhood filter, the lattice,
/// then the share.
///
/// In that order because each is cheaper than the one before it is likely to
/// matter - and because the share must be the LAST word, or a rule's density
/// would change when an unrelated filter did.
fn claims(fixture: &StyleFixtureConfig, plate: &SkinPlate, reading: &PlateReading) -> bool {
    let rule = &fixture.scatter;
    if !eligible(fixture, plate, reading) {
        return false;
    }
    if rule.chance >= 1.0 {
        return true;
    }
    if rule.chance <= 0.0 {
        return false;
    }
    share(cell_hash(plate.cell, reading.out, &fixture.id)) < rule.chance
}

/// Whether a plate is the kind of place this fixture belongs AND stands on its
/// lattice - everything the share is then applied to.
///
/// Split out because the per-patch floor drops the SHARE and nothing else: a
/// floor piece is one the share happened to thin away, not one the rule never
/// wanted, and it stands on the same grid the rest of the rule does.
fn eligible(fixture: &StyleFixtureConfig, plate: &SkinPlate, reading: &PlateReading) -> bool {
    fixture.scatter.accepts(reading) && on_lattice(plate.cell, reading.out, fixture.scatter.stride)
}

/// The best unclaimed plate in one block of hull, and whether the block already
/// carries a piece of the rule looking at it.
#[derive(Default)]
struct Patch {
    /// The lowest hash seen in this block, and the plate that carried it.
    best: Option<(u64, usize)>,
    /// Whether the share already put a piece of this rule here.
    served: bool,
}

/// Put one piece back wherever a rule's own share left a whole block of hull
/// bare - the density normalisation, applied after the share and only to the
/// rules that asked for one.
///
/// See [`ScatterRule::patch`](super::skin_style::ScatterRule::patch) for why
/// this exists and exactly how much a growing hull can move. The two properties
/// it rests on are here: a block is a fixed division of the SHIP's own cells, so
/// nothing shifts when a hull grows, and the floor only ever lands on a plate
/// nothing else took, so priority still means what it says.
fn fill_patches(
    plates: &[SkinPlate],
    readings: &[PlateReading],
    style: &ShipStyleConfig,
    claimed: &mut [Option<usize>],
) {
    for (index, fixture) in style.fixtures.iter().enumerate() {
        let size = fixture.scatter.patch;
        if size == 0 {
            continue;
        }
        // The face is in the key as well as the block: a corner of a ship is
        // clad from two directions, and a piece on the roof does not stand in
        // for one on the flank.
        let mut blocks: HashMap<(IVec3, IVec3), Patch> = HashMap::new();
        for (slot, (plate, reading)) in plates.iter().zip(readings).enumerate() {
            let key = (block_of(plate.cell, size), reading.out);
            match claimed[slot] {
                Some(taken) if taken == index => blocks.entry(key).or_default().served = true,
                Some(_) => continue,
                None => {
                    if !eligible(fixture, plate, reading) {
                        continue;
                    }
                    let hash = cell_hash(plate.cell, reading.out, &fixture.id);
                    let patch = blocks.entry(key).or_default();
                    if patch.best.is_none_or(|(best, _)| hash < best) {
                        patch.best = Some((hash, slot));
                    }
                }
            }
        }
        for patch in blocks.values() {
            match (patch.served, patch.best) {
                (false, Some((_, slot))) => claimed[slot] = Some(index),
                _ => continue,
            }
        }
    }
}

/// Which block of `size` cells a cell falls in, as a fixed division of the
/// SHIP's own lattice.
///
/// `div_euclid` and not `/`, so the blocks either side of the origin are the
/// same size as every other - integer division rounds toward zero and would
/// make the middle block twice as wide, which is a seam down the middle of
/// every ship that happens to straddle its own origin.
fn block_of(cell: IVec3, size: u8) -> IVec3 {
    let size = i32::from(size);
    IVec3::new(
        cell.x.div_euclid(size),
        cell.y.div_euclid(size),
        cell.z.div_euclid(size),
    )
}

/// Whether a cell stands on the rule's own lattice.
///
/// Read on the two axes the plate's face LIES IN, never on all three: a stride
/// over the out axis would decorate a hull in stripes that jump wherever the
/// surface steps, and what a rule means by "every other cell" is every other
/// cell ACROSS THE SURFACE.
fn on_lattice(cell: IVec3, out: IVec3, stride: u8) -> bool {
    if stride <= 1 {
        return true;
    }
    let (u, v) = plane_axes(out);
    let stride = i32::from(stride);
    cell[u].rem_euclid(stride) == 0 && cell[v].rem_euclid(stride) == 0
}

/// The two axes a face whose normal is `out` lies in, lowest first.
fn plane_axes(out: IVec3) -> (usize, usize) {
    match (out.x != 0, out.y != 0) {
        (true, _) => (1, 2),
        (_, true) => (0, 2),
        _ => (0, 1),
    }
}

/// The quarter turns that point a piece's own `+Z` down the axis its rule
/// aligns to.
///
/// Quarter turns and not an angle: the plate lattice is square, both axes are
/// cardinals, and a piece snapped to the grid is the whole point.
///
/// The two axes differ in one way that matters. The RUN has no sign - a rib
/// strip along `+Z` and the same strip along `-Z` are the same strip - so the
/// first turn lying anywhere on that line wins. The FALL does: it points off
/// the ship, and a fairing turned the other way leans inboard over the hull it
/// is supposed to shroud. A plate with no fall is left unturned, which is why
/// [`Outward`] is a rule for the falling plate.
///
/// [`Outward`]: ScatterAlign::Outward
fn turns_for(plate: &SkinPlate, reading: &PlateReading, fixture: &StyleFixtureConfig) -> u8 {
    let (axis, signed) = match fixture.scatter.align {
        ScatterAlign::Free => return 0,
        ScatterAlign::Run => (reading.along, false),
        ScatterAlign::Outward => (reading.fall, true),
    };
    if axis == IVec3::ZERO {
        return 0;
    }
    let axis = axis.as_vec3().normalize();
    (0..4)
        .find(|turns| {
            let facing = plate.rotation * (Quat::from_rotation_y(quarter(*turns)) * Vec3::Z);
            let lean = facing.dot(axis);
            match signed {
                // A DIAGONAL fall - an outer corner - leans two turns equally,
                // and the first of them wins. Half a right angle off is what a
                // square grid can do about a corner.
                true => lean > 0.5,
                false => lean.abs() > 0.9,
            }
        })
        .unwrap_or(0)
}

/// `turns` quarter turns as an angle.
fn quarter(turns: u8) -> f32 {
    f32::from(turns) * FRAC_PI_2
}

/// The hash a plate's claim is decided by: FNV-1a over the cell, the face it
/// shows and the fixture's id.
///
/// Written out rather than taken from `DefaultHasher` because this decides what
/// a SHIP LOOKS LIKE. The standard hasher's output is not promised to be stable
/// across releases of the standard library, and a ship that comes back wearing
/// different antennae after a toolchain bump is exactly the failure the whole
/// derivation exists to avoid.
///
/// The face is in the hash as well as the cell: a corner cell can be clad from
/// two directions on two different ships, and the two plates are different
/// places.
fn cell_hash(cell: IVec3, out: IVec3, salt: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for value in [cell.x, cell.y, cell.z, out.x, out.y, out.z] {
        for byte in value.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
        }
    }
    for byte in salt.as_bytes() {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(PRIME);
    }
    hash
}

/// A hash as a share in `[0, 1)`, off the high bits - the low bits of FNV-1a
/// are the least mixed.
fn share(hash: u64) -> f32 {
    (hash >> 40) as f32 / (1u64 << 24) as f32
}

/// Where a decoration stands on the plate it is bolted to, and which way it is
/// stood up.
///
/// In the PLATE's frame, which is already the frame a greeble is authored in:
/// `+Y` is out of the plate and `y = 0` is the mounting face. The lift is the
/// plate's own centre height, which is the mean of its eight boundary samples -
/// the same number the collider is cut to, and the exact height the fan's own
/// middle vertex rides at, so the piece stands ON the surface rather than near
/// it.
///
/// # The lean was the bigger half of the placement defect
///
/// This used to lift and yaw and nothing else, which stood every piece upright
/// in the PLATE's cell whatever the plate's top was doing. Measured over 526
/// plates of the `wfc_ships` row: 82% of plates lean more than 15 degrees off
/// their own out face, the mean tilt under a placed piece was 26 degrees, and
/// 63% of placed pieces stood on a plate whose top is ONE FLAT SURFACE - a
/// ramp, with nothing wrong with it, wearing a piece balanced on one edge.
///
/// So the piece is turned onto [`ShellShape::seat_normal`] first and yawed
/// second. Composed in that order because the yaw is a turn ACROSS THE
/// SURFACE - the quarter turn [`turns_for`] chose against the plate's in-plane
/// axes - and turning first then bedding keeps that choice meaning what it
/// said.
///
/// A CONE is left standing up its own cell, and that is the seat normal's
/// business rather than this one's: the pieces that stand on one are the ones
/// whose rule asked for the high ground, and a mast on a spar tip wants to
/// stand up the tip.
///
/// [`ShellShape::seat_normal`]: super::shell_shape::ShellShape::seat_normal
pub fn decor_pose(plate: &SkinPlate, turns: u8) -> Transform {
    Transform::from_translation(Vec3::Y * (-REACH + plate.shape.volume())).with_rotation(
        Quat::from_rotation_arc(Vec3::Y, plate.shape.seat_normal())
            * Quat::from_rotation_y(quarter(turns)),
    )
}

/// Marks a decoration and carries the model that draws it.
///
/// The model rides on the marker so the render half can be an observer with
/// nothing but this component to read, exactly as a plate's shape rides on
/// `ShipSkinMarker`. A headless server spawns the marker and stops there.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct ShipDecorMarker(#[reflect(ignore)] pub AssetRef<WorldAsset>);

/// The whole of a decoration that is not a look: what takes damage, weighs
/// something and stops a round.
///
/// A [`SectionFixture`], so it is never structure, never in the integrity
/// graph, never in the ship's health and - through the ancestor walk in
/// `damage_cracks` - never graded by the damage of the hull under it. Its damage
/// read is that it comes off.
///
/// GAMEPLAY, all of it: a headless server builds this and never loads a mesh.
pub fn decor_body(fixture: &StyleFixtureConfig, pose: Transform) -> impl Bundle {
    (
        Name::new(format!("Skin Decor {}", fixture.id)),
        SectionFixture,
        ShipDecorMarker(fixture.model.clone()),
        pose,
        // Health, density and `Visibility::Inherited` in one bundle.
        destructible_body(fixture.health, fixture.density),
        decor_collider(fixture.collider),
    )
}

/// The box a decoration stops rounds with: the authored size, standing ON the
/// mount face rather than centred on the entity.
///
/// A CUBOID and not the model. A greeble is a handful of small solids with gaps
/// between them, a convex hull of one would swallow the gaps anyway, and the
/// difference at this size is centimeters. A COMPOUND of one box because that
/// is the only way to offset a collider from the entity carrying it, and a
/// piece is authored with its foot at `y = 0`.
fn decor_collider(size: Vec3) -> Collider {
    let size = size.max(Vec3::splat(MIN_COLLIDER));
    Collider::compound(vec![(
        Vec3::Y * size.y * 0.5,
        Quat::IDENTITY,
        Collider::cuboid(size.x, size.y, size.z),
    )])
}

/// Give a decoration the model that draws it, the moment one appears.
///
/// The RENDER half, and the whole of it. The model goes on the decoration's OWN
/// entity rather than on a child: a greeble is authored in exactly the frame the
/// fixture stands in, so a child would carry an identity transform and cost an
/// entity per piece over a hull that has hundreds.
pub(crate) fn dress_skin_decor(
    add: On<Add, ShipDecorMarker>,
    mut commands: Commands,
    q_decor: Query<&ShipDecorMarker>,
    asset_server: Res<AssetServer>,
) {
    let decor = add.entity;
    let Ok(ShipDecorMarker(model)) = q_decor.get(decor) else {
        return;
    };
    commands
        .entity(decor)
        .insert(WorldAssetRoot(model.resolve(&asset_server)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::{
        shell_skin::{derive_skin, SkinStructure},
        skin_reading::{read_plates, PlateRelief},
        skin_style::{ScatterRule, ScatterSeat, ShipStyleConfig},
    };

    /// A section that mates on every face, like a hull cube.
    const OPEN: [bool; 6] = [true; 6];

    /// A slab of hull `size` cells on a side, one cell thick.
    fn slab(size: i32) -> SkinStructure {
        let mut structure = SkinStructure::default();
        for x in 0..size {
            for z in 0..size {
                structure.insert(IVec3::new(x, 0, z), OPEN);
            }
        }
        structure
    }

    /// Every placement one style makes on one structure.
    fn scatter(structure: &SkinStructure, style: &ShipStyleConfig) -> Vec<DecorPlacement> {
        let plates = derive_skin(structure);
        let readings = read_plates(structure, &plates);
        scatter_decor(&plates, &readings, style)
    }

    fn fixture(id: &str, scatter: ScatterRule) -> StyleFixtureConfig {
        StyleFixtureConfig {
            id: id.to_string(),
            model: AssetRef::from("self://gltf/greebles/placeholder_block.glb#Scene0".to_string()),
            health: 10.0,
            density: 0.1,
            collider: Vec3::new(0.2, 0.1, 0.2),
            scatter,
        }
    }

    fn style(fixtures: Vec<StyleFixtureConfig>) -> ShipStyleConfig {
        ShipStyleConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            surfaces: Vec::new(),
            fixtures,
        }
    }

    /// The claim on a plate is a pure function of the structure: the same hull
    /// scattered twice gives the same pieces on the same cells.
    ///
    /// This is the property the editor preview rests on. A scatter that read an
    /// RNG would pass every other test in this file and still flicker on screen
    /// while a hull was dragged.
    #[test]
    fn the_same_hull_always_wears_the_same_decoration() {
        let structure = slab(6);
        let plates = derive_skin(&structure);
        let style = style(vec![fixture(
            "vent",
            ScatterRule {
                chance: 0.5,
                ..default()
            },
        )]);

        let first = scatter_decor(&plates, &read_plates(&structure, &plates), &style);
        let second = scatter_decor(&plates, &read_plates(&structure, &plates), &style);
        assert!(!first.is_empty(), "a 6x6 deck should take some decoration");
        assert_eq!(first, second, "the scatter is not reproducible");

        // And it is the STRUCTURE that decides, not the order it was built in.
        let mut backwards = SkinStructure::default();
        for x in (0..6).rev() {
            for z in (0..6).rev() {
                backwards.insert(IVec3::new(x, 0, z), OPEN);
            }
        }
        let rebuilt = derive_skin(&backwards);
        let reversed = scatter_decor(&rebuilt, &read_plates(&backwards, &rebuilt), &style);
        assert_eq!(first, reversed, "the scatter depends on insertion order");
    }

    /// A share of one half takes roughly half the eligible plates, and the same
    /// half every time. A rule at 1.0 takes all of them.
    #[test]
    fn the_share_thins_a_rule_out_without_moving_it() {
        let structure = slab(8);
        let plates = derive_skin(&structure);
        let flat = ScatterRule {
            relief: vec![PlateRelief::Flat],
            ..default()
        };

        let readings = read_plates(&structure, &plates);
        let all = scatter_decor(
            &plates,
            &readings,
            &style(vec![fixture("panel", flat.clone())]),
        );
        let half = scatter_decor(
            &plates,
            &readings,
            &style(vec![fixture(
                "panel",
                ScatterRule {
                    chance: 0.5,
                    ..flat
                },
            )]),
        );

        assert!(all.len() > 20, "an 8x8 deck has plenty of flat plate");
        let taken = half.len() as f32 / all.len() as f32;
        assert!(
            (0.3..0.7).contains(&taken),
            "half a share took {taken} of the eligible plates",
        );
        // Thinning REMOVES claims, it does not move them: every survivor stood
        // on a plate the full rule had already claimed.
        for placement in &half {
            assert!(
                all.iter().any(|other| other.plate == placement.plate),
                "the share moved a piece instead of dropping one",
            );
        }
    }

    /// A stride claims cells on a lattice, which is what makes a run of vents
    /// read as a row rather than as a sprinkle.
    #[test]
    fn a_stride_claims_a_lattice_and_not_a_sprinkle() {
        let structure = slab(8);
        let plates = derive_skin(&structure);
        let placements = scatter_decor(
            &plates,
            &read_plates(&structure, &plates),
            &style(vec![fixture(
                "row",
                ScatterRule {
                    relief: vec![PlateRelief::Flat],
                    stride: 2,
                    ..default()
                },
            )]),
        );

        assert!(!placements.is_empty(), "a stride of 2 fits an 8-cell deck");
        for placement in &placements {
            let cell = plates[placement.plate].cell;
            // Every claim is on the even lattice of the two axes its face lies
            // in. The roof of this slab faces +Y, so that is x and z.
            assert_eq!(
                (cell.x.rem_euclid(2), cell.z.rem_euclid(2)),
                (0, 0),
                "a piece landed off the lattice at {cell:?}",
            );
        }
    }

    /// One plate takes at most one piece, and the FIRST rule that matches gets
    /// it - so a style's authored order is its priority order.
    #[test]
    fn a_plate_takes_one_piece_and_the_first_rule_wins() {
        let structure = slab(5);
        let plates = derive_skin(&structure);
        // Seated anywhere, so "every plate" means every plate: the deck's
        // outer corners are cones and a default rule would leave them bare.
        let anywhere = ScatterRule {
            seat: ScatterSeat::Any,
            ..default()
        };
        let placements = scatter_decor(
            &plates,
            &read_plates(&structure, &plates),
            &style(vec![
                fixture("first", anywhere.clone()),
                fixture("second", anywhere),
            ]),
        );

        assert_eq!(
            placements.len(),
            plates.len(),
            "an unfiltered rule covers every plate",
        );
        assert!(
            placements.iter().all(|placement| placement.fixture == 0),
            "the second rule claimed a plate the first had already taken",
        );
    }

    /// An aligned piece is yawed so its own `+Z` lies along the run, and the
    /// yaw is a quarter turn - the pieces line up with each other and with the
    /// hull.
    #[test]
    fn an_aligned_piece_turns_to_the_run() {
        // A spine four cells long: its roof is a one-cell run down +Z.
        let mut structure = SkinStructure::default();
        for z in 0..4 {
            structure.insert(IVec3::new(0, 0, z), OPEN);
        }
        let plates = derive_skin(&structure);
        let placements = scatter_decor(
            &plates,
            &read_plates(&structure, &plates),
            &style(vec![fixture(
                "rib",
                ScatterRule {
                    relief: vec![PlateRelief::Ridge],
                    // A crest is a cone every time, so a rib on one is a rule
                    // that asked for the high ground.
                    seat: ScatterSeat::Any,
                    align: ScatterAlign::Run,
                    ..default()
                },
            )]),
        );

        assert!(!placements.is_empty(), "the spine's roof is a ridge");
        for placement in &placements {
            let plate = &plates[placement.plate];
            let pose = decor_pose(plate, placement.turns);
            // The piece's own +Z, taken all the way out into the ship's frame.
            let facing = plate.rotation * pose.rotation * Vec3::Z;
            assert!(
                facing.dot(Vec3::Z).abs() > 0.9,
                "a rib on the spine points {facing} instead of down it",
            );
        }
    }

    /// A piece is BEDDED on the plate it stands on: its own `+Y` comes out
    /// along the plate's top normal, so it lies on the surface rather than
    /// standing upright in the cell.
    ///
    /// The measured defect this closes: 82% of plates lean more than 15 degrees
    /// off their own out face, and a lifted-and-yawed piece stood upright on
    /// every one of them.
    #[test]
    fn a_piece_is_bedded_on_the_surface_it_stands_on() {
        // A deck with a block on it: a level middle, and a ramp climbing the
        // block square-on. Both are one flat surface; only one is level.
        let mut structure = slab(6);
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        let plates = derive_skin(&structure);
        let at = |cell: IVec3| {
            plates
                .iter()
                .find(|plate| plate.cell == cell)
                .expect("a plate stands here")
        };

        let level = at(IVec3::new(4, 1, 4));
        assert!(level.shape.tilt() < 1e-4, "the far deck is level");
        let up = level.rotation * decor_pose(level, 0).rotation * Vec3::Y;
        assert!(
            up.abs_diff_eq(Vec3::Y, 1e-4),
            "a piece on a level panel was turned to {up}",
        );

        let ramp = at(IVec3::new(1, 1, 2));
        assert!(ramp.shape.is_coplanar() && ramp.shape.tilt() > 0.4);
        let pose = decor_pose(ramp, 0);
        assert!(
            (pose.rotation * Vec3::Y).abs_diff_eq(ramp.shape.top_normal(), 1e-4),
            "a piece on a ramp stands up {} against a surface facing {}",
            pose.rotation * Vec3::Y,
            ramp.shape.top_normal(),
        );
        // ...and the whole of the turn is that bedding: the piece leans exactly
        // as far as the plate does, and no further.
        let leaned = (pose.rotation * Vec3::Y)
            .dot(Vec3::Y)
            .clamp(-1.0, 1.0)
            .acos();
        assert!(
            (leaned - ramp.shape.tilt()).abs() < 1e-4,
            "the piece turned {leaned} rad on a plate leaning {} rad",
            ramp.shape.tilt(),
        );

        // A CONE is the exception, and it is left alone: the diagonal of the
        // same raise falls three ways from its own middle, so a piece there
        // stands up the cell rather than lying down one facet of it.
        let cone = at(IVec3::new(1, 1, 1));
        assert!(!cone.shape.is_coplanar() && cone.shape.tilt() > 0.4);
        assert!(
            (decor_pose(cone, 0).rotation * Vec3::Y).abs_diff_eq(Vec3::Y, 1e-4),
            "a piece on a crease was laid down one of its facets",
        );
    }

    /// An OUTWARD piece is turned off the ship, square to the run and with a
    /// sign - which is the whole difference between a fairing shrouding an edge
    /// and one leaning back over the hull behind it.
    ///
    /// Measured ACROSS THE SURFACE, because a piece is bedded onto the plate:
    /// the edge of a hull drops half a cell over a cell, so a fairing on it
    /// noses 27 degrees down the slope and its raw `+Z` is not the cardinal it
    /// was turned to. Two claims survive that - which way it points along the
    /// plate it lies on, and that it rakes DOWN with the edge rather than
    /// standing square out of it.
    #[test]
    fn an_outward_piece_turns_off_the_ship() {
        let structure = slab(6);
        let plates = derive_skin(&structure);
        let readings = read_plates(&structure, &plates);
        let placements = scatter_decor(
            &plates,
            &readings,
            &style(vec![fixture(
                "fairing",
                ScatterRule {
                    relief: vec![PlateRelief::Brink],
                    align: ScatterAlign::Outward,
                    ..default()
                },
            )]),
        );

        assert!(!placements.is_empty(), "a 6x6 deck has a straight edge");
        for placement in &placements {
            let plate = &plates[placement.plate];
            let fall = readings[placement.plate].fall.as_vec3();
            let out = readings[placement.plate].out.as_vec3();
            let pose = decor_pose(plate, placement.turns);
            let facing = plate.rotation * pose.rotation * Vec3::Z;
            let heading = (facing - out * facing.dot(out)).normalize();
            assert!(
                heading.dot(fall) > 0.9,
                "a fairing on an edge falling {fall} heads {heading}",
            );
            assert!(
                facing.dot(out) < -0.1,
                "a fairing on a hull edge stands square out of it ({facing})",
            );
        }
    }

    /// The DENSITY FLOOR: a rule thinned to nothing over a block of hull gets
    /// one piece back there, and a rule the share already served does not.
    ///
    /// The measured failure this exists for: every knob is per plate and they
    /// multiply, so a rule tuned on a generated hull put one visible piece on a
    /// hand-built one.
    #[test]
    fn a_patch_puts_one_piece_back_where_the_share_left_none() {
        let structure = slab(8);
        // A share far too thin to cover the deck: what a rule tuned for a big
        // hull does to a small one.
        let thin = ScatterRule {
            relief: vec![PlateRelief::Flat],
            chance: 0.02,
            ..default()
        };
        let bare = scatter(&structure, &style(vec![fixture("panel", thin.clone())]));
        let floored = scatter(
            &structure,
            &style(vec![fixture("panel", ScatterRule { patch: 3, ..thin })]),
        );

        assert!(
            floored.len() > bare.len(),
            "the floor put nothing back: {} against {}",
            floored.len(),
            bare.len(),
        );
        // A FLOOR and not a move: everything the share claimed is still there.
        for placement in &bare {
            assert!(
                floored.contains(placement),
                "the floor moved a piece the share had already placed",
            );
        }
    }

    /// A hull that GROWS keeps the decoration it had everywhere the new cell is
    /// not - which is the promise the whole derivation rests on, and the one a
    /// density normalisation is most likely to break.
    #[test]
    fn growing_a_hull_leaves_the_decoration_outside_the_new_block_alone() {
        let mut structure = slab(8);
        let style = style(vec![fixture(
            "panel",
            ScatterRule {
                relief: vec![PlateRelief::Flat],
                chance: 0.25,
                patch: 3,
                ..default()
            },
        )]);

        // Placements as CELLS, since the plate indices themselves shift when
        // the derivation gains a plate.
        let cells = |structure: &SkinStructure| -> Vec<(IVec3, u8)> {
            let plates = derive_skin(structure);
            let readings = read_plates(structure, &plates);
            scatter_decor(&plates, &readings, &style)
                .into_iter()
                .map(|placement| (plates[placement.plate].cell, placement.turns))
                .collect()
        };
        let before = cells(&structure);
        assert!(before.len() > 4, "an 8x8 deck takes several pieces");

        // One cell, in the far corner, three blocks away from most of the deck.
        structure.insert(IVec3::new(8, 0, 8), OPEN);
        let after = cells(&structure);

        // The new cell reshapes the plates around it, so the honest claim is
        // about the hull it did NOT touch: everything two blocks clear of it
        // wears exactly what it wore.
        let untouched = |cell: &IVec3| cell.x < 5 || cell.z < 5;
        let kept: Vec<(IVec3, u8)> = before
            .iter()
            .copied()
            .filter(|(cell, _)| untouched(cell))
            .collect();
        assert!(kept.len() > 3, "the far side of the deck holds pieces");
        for piece in kept {
            assert!(
                after.contains(&piece),
                "{piece:?} moved when a cell was added across the ship",
            );
        }
    }

    /// The reach a priority order hides: what each rule WOULD take on its own,
    /// which is what says a rule landed nothing because it was starved rather
    /// than because its filter was too tight.
    #[test]
    fn the_reach_of_a_rule_is_what_it_would_take_alone() {
        let structure = slab(6);
        let plates = derive_skin(&structure);
        let readings = read_plates(&structure, &plates);
        let style = style(vec![
            fixture(
                "greedy",
                ScatterRule {
                    seat: ScatterSeat::Any,
                    ..default()
                },
            ),
            fixture(
                "starved",
                ScatterRule {
                    relief: vec![PlateRelief::Flat],
                    ..default()
                },
            ),
        ]);

        let reach = decor_reach(&plates, &readings, &style);
        let taken = scatter_decor(&plates, &readings, &style);
        assert_eq!(reach[0], plates.len(), "an empty rule reaches every plate");
        assert!(reach[1] > 0, "a 6x6 deck has flat plate on its roof");
        assert!(
            !taken.iter().any(|placement| placement.fixture == 1),
            "the greedy rule was supposed to starve the one under it",
        );

        // The reach is an UPPER BOUND, which is what makes it readable: a rule
        // can never take more plates than its own filter admits.
        let mut took = vec![0usize; style.fixtures.len()];
        for placement in &taken {
            took[placement.fixture] += 1;
        }
        for (index, took) in took.into_iter().enumerate() {
            assert!(took <= reach[index], "rule {index} took more than it can");
        }
    }

    /// A decoration stands ON the plate's surface, not inside it and not
    /// floating above it.
    #[test]
    fn a_piece_sits_on_the_surface_of_its_plate() {
        let structure = slab(5);
        let plates = derive_skin(&structure);
        let flat = plates
            .iter()
            .find(|plate| plate.shape.volume() > 0.4)
            .expect("a 5x5 deck has flat plate on its roof");
        let pose = decor_pose(flat, 0);
        assert!(
            (pose.translation.y - (-REACH + flat.shape.volume())).abs() < 1e-5,
            "a piece at {} is not on the plate's own top",
            pose.translation,
        );
    }
}
