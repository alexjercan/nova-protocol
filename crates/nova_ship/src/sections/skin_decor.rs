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
//! piece is yawed to the direction the surface runs, both from the plate
//! vocabulary.

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::Collider;
use bevy::prelude::*;
use nova_gameplay::prelude::{destructible_body, AssetRef};

use crate::sections::{
    fixture::prelude::SectionFixture,
    shell_shape::REACH,
    shell_skin::SkinPlate,
    skin_reading::PlateReading,
    skin_style::{ShipStyleConfig, StyleFixtureConfig},
};

/// The scatter, the placement it answers with, and the decoration bundle.
pub mod prelude {
    pub use super::{decor_body, decor_pose, scatter_decor, DecorPlacement, ShipDecorMarker};
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
    /// Quarter turns about the plate's own outward axis, aligning the piece to
    /// the run. Always `0` for a piece that does not align.
    pub turns: u8,
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
    plates
        .iter()
        .zip(readings)
        .enumerate()
        .filter_map(|(index, (plate, reading))| {
            let fixture = style
                .fixtures
                .iter()
                .position(|fixture| claims(fixture, plate, reading))?;
            Some(DecorPlacement {
                plate: index,
                fixture,
                turns: turns_for(plate, reading, &style.fixtures[fixture]),
            })
        })
        .collect()
}

/// Whether one fixture claims one plate: the neighbourhood filter, the lattice,
/// then the share.
///
/// In that order because each is cheaper than the one before it is likely to
/// matter - and because the share must be the LAST word, or a rule's density
/// would change when an unrelated filter did.
fn claims(fixture: &StyleFixtureConfig, plate: &SkinPlate, reading: &PlateReading) -> bool {
    let rule = &fixture.scatter;
    if !rule.accepts(reading) {
        return false;
    }
    if !on_lattice(plate.cell, reading.out, rule.stride) {
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

/// The quarter turns that point a piece's own `+Z` down the run.
///
/// Quarter turns and not an angle: the plate lattice is square, the alignment
/// direction is a cardinal, and a piece snapped to the grid is the whole point.
/// The run has no SIGN - a rib strip along `+Z` and the same strip along `-Z`
/// are the same strip - so the first turn that lies on the line wins.
fn turns_for(plate: &SkinPlate, reading: &PlateReading, fixture: &StyleFixtureConfig) -> u8 {
    if !fixture.scatter.align {
        return 0;
    }
    let along = reading.along.as_vec3();
    (0..4)
        .find(|turns| {
            let facing = plate.rotation * (Quat::from_rotation_y(quarter(*turns)) * Vec3::Z);
            facing.dot(along).abs() > 0.9
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

/// Where a decoration stands on the plate it is bolted to.
///
/// In the PLATE's frame, which is already the frame a greeble is authored in:
/// `+Y` is out of the plate, so the piece only has to be lifted to the surface
/// and yawed. The lift is the plate's own centre height, which is the mean of
/// its eight boundary samples - the same number the collider is cut to.
pub fn decor_pose(plate: &SkinPlate, turns: u8) -> Transform {
    Transform::from_translation(Vec3::Y * (-REACH + plate.shape.volume()))
        .with_rotation(Quat::from_rotation_y(quarter(turns)))
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
/// `damage_tint` - never graded by the damage of the hull under it. Its damage
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
/// difference at this size is centimetres. A COMPOUND of one box because that
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
        skin_style::{ScatterRule, ShipStyleConfig},
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
        let placements = scatter_decor(
            &plates,
            &read_plates(&structure, &plates),
            &style(vec![
                fixture("first", ScatterRule::default()),
                fixture("second", ScatterRule::default()),
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
                    align: true,
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
