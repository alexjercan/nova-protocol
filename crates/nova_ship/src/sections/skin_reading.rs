//! The VOCABULARY: what kind of place a derived plate stands in.
//!
//! [`derive_skin`](super::shell_skin::derive_skin) already works out everything
//! below on its way to a shape - which cells are clad, which way each plate
//! faces, how deep the structure under it runs, whether its corners die to the
//! floor or climb a wall - and then throws all of it away except the eight
//! boundary samples. This module reads it back out, so a scatter rule can ask
//! "is this a long flat panel, or the rim of a spine, or the plate beside a
//! nozzle" without recomputing anything and without knowing how a shape is
//! derived.
//!
//! It is a SECOND PASS over the finished skin, never a second derivation. The
//! plates are the input: their cells are the clad set, `cell - anchor` is the
//! face each shows to space, and their shapes are the relief. So a reading costs
//! a handful of hash lookups per plate and cannot disagree with the skin it
//! describes.
//!
//! Two things the vocabulary is FOR, both from the decoration research:
//!
//! - decoration claims cells on the plate GRID rather than being scattered by
//!   blue noise, because alignment is what makes a greeble read as bolted on
//!   rather than as confetti. [`PlateReading::along`] is the axis it aligns to.
//! - decoration is weighted toward BORDERS and FITTINGS, which is free here:
//!   [`PlateReading::border`] and [`PlateReading::pocket`] are both distances
//!   the derivation already had the ingredients for.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::sections::{
    shell_shape::{ShellShape, FULL},
    shell_skin::{
        blind_pocket, ends_against, face_plane, step, support_depth, SkinPlate, SkinStructure,
        FACES,
    },
};

/// The prelude: `PlateReading`, `PlateRelief`, `PlateFacing` and `read_plates`.
pub mod prelude {
    pub use super::{read_plates, PlateFacing, PlateReading, PlateRelief};
}

/// How far a run walk looks along one direction before it stops counting.
///
/// A rule that wants "a long flat panel" is answered by 8; past that the extra
/// cells tell a scatter rule nothing it would act on differently, and the walk
/// is per plate per direction.
pub const RUN_REACH: u8 = 8;

/// How far a plate looks across its own surface for the mouth of a fitting.
///
/// [`PlateReading::pocket`] saturates HERE rather than at some sentinel, so
/// `pocket == POCKET_REACH` reads as "nothing near" and a rule never has to
/// spell a magic number.
pub const POCKET_REACH: u8 = 3;

/// What the top of a plate is shaped like, in the terms a scatter rule asks in.
///
/// Read off the eight boundary samples and nothing else, so it is exactly as
/// stable as the skin is: the same structure gives the same reading.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlateRelief {
    /// Every sample at the same height: a flat panel, and the plate most of a
    /// clad hull is made of. The one place a big piece of decoration fits.
    Flat,
    /// Some sample at the WHOLE cell: the plate climbs structure standing proud
    /// beside it. A hard edge, and the inside corner of a step.
    Step,
    /// Corners on the floor with a crest across the middle: the tent a run of
    /// skin one cell wide comes out as. Its long axis is
    /// [`PlateReading::along`].
    Ridge,
    /// Every sample on the floor, so the middle of the plate rides at half a
    /// cell: the stud a LONE clad cell comes out as.
    Peak,
    /// The edge of the skin: some samples on the floor and some above, so the
    /// surface tapers away across this plate.
    Rim,
}

/// Which way a plate faces, in the SHIP's own frame.
///
/// Coarse on purpose. A rule wants "masts on the roof, not under the belly",
/// and the six faces collapse to three answers a style author can hold in their
/// head.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlateFacing {
    /// Any face at all - the authored default, so a rule that does not care
    /// says nothing.
    #[default]
    Any,
    /// The roof: the plate shows `+Y` of the ship to space.
    Up,
    /// The belly: `-Y`.
    Down,
    /// A flank, a nose or a transom - anything not up or down.
    Side,
}

impl PlateFacing {
    /// Whether this facing filters nothing - the `skip_serializing_if` a style
    /// keeps the field out of its RON with.
    pub fn is_any(&self) -> bool {
        *self == Self::Any
    }

    /// Whether a plate whose out face points along `normal` (in ship cells)
    /// answers this facing.
    pub fn accepts(self, normal: IVec3) -> bool {
        match self {
            Self::Any => true,
            Self::Up => normal == IVec3::Y,
            Self::Down => normal == IVec3::NEG_Y,
            Self::Side => normal.y == 0,
        }
    }
}

/// The neighbourhood one plate stands in.
///
/// Everything here is a function of the structure, so two ships built the same
/// way read the same and a plate re-derived in the editor reads what the flown
/// ship's plate will.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub struct PlateReading {
    /// The face the plate shows to space, as a direction in the ship's cells.
    pub out: IVec3,
    /// The in-plane direction the surface runs FURTHEST along through this
    /// plate, as a positive cardinal in the ship's cells.
    ///
    /// The alignment axis, and the reason the vocabulary exists at all: a rib
    /// run or a trim strip turned to this reads as bolted to the hull, and the
    /// same piece at a free yaw reads as litter. Ties go to the lower axis, so
    /// a square patch of plate still answers the same way every time.
    pub along: IVec3,
    /// What the top of the plate is shaped like.
    pub relief: PlateRelief,
    /// How many of the eight cells around this one IN THE PLANE the surface
    /// carries on into, rather than ending against vacuum: 8 is the middle of a
    /// field of plate, 0 is a lone stud.
    ///
    /// The same predicate the CORNERS are read off, which is why a [`Flat`]
    /// plate always reads 8 - a corner can only sit at the running height when
    /// all four of its cells continue. It earns its place on the lopsided
    /// plates, where it separates "the rim at the edge of the ship" from "the
    /// plate beside a one-cell notch".
    ///
    /// [`Flat`]: PlateRelief::Flat
    pub enclosure: u8,
    /// How many cells the run of LIKE plate through this one is long, itself
    /// included, measured [`along`] and capped at [`RUN_REACH`].
    ///
    /// Like means same-facing and same [`relief`]: a flat panel's run is its
    /// flat neighbours, and the run of a RIM is the edge of the ship it lies on.
    /// Measuring it per relief rather than only on flat plate is what makes it
    /// useful on a real hull - the derived skin comes out four fifths rim, so a
    /// reading that only spoke about flat panels would be silent about most of
    /// every ship.
    ///
    /// [`along`]: PlateReading::along
    /// [`relief`]: PlateReading::relief
    pub run: u8,
    /// How many cells of like plate lie between this one and the nearest end of
    /// its patch: 0 is a plate ON the edge of it, and the decoration research
    /// says that is where a greeble wants to be.
    pub border: u8,
    /// How much of its cell the plate fills, in QUARTER cells - the same five
    /// heights its boundary samples are drawn from.
    ///
    /// Free: it is the plate's own volume, which its collider and its health are
    /// already cut from. It is the cheapest way to tell a rim that is nearly a
    /// full panel from a rim that is a sliver of floor, and the two cannot carry
    /// the same piece.
    pub height: u8,
    /// How many cells of structure stand under the plate, capped at the
    /// derivation's own reach.
    ///
    /// Free: [`plate_for`](super::shell_skin::plate_for) already measures it to
    /// decide which way a plate faces, and then drops it. It separates the skin
    /// over the BODY of a ship from the skin over a one-cell spar, which is the
    /// difference between a radiator bank looking bolted on and looking
    /// impossible.
    pub depth: u8,
    /// How many cells away, across this surface, the nearest POCKET is - the
    /// mouth of a drive bay or a gun well, which is a hole the skin was refused
    /// rather than the end of it. Saturates at [`POCKET_REACH`], which reads as
    /// "no fitting near".
    ///
    /// This is the research's "weight decoration toward link points", answered
    /// with the only socket fact that discriminates on a hull of cubes: every
    /// hull face offers a socket, so socket proximity is constant, and it is the
    /// BLIND faces - the fittings - that mark where a ship gets interesting.
    pub pocket: u8,
}

/// Read the neighbourhood of every plate in a derived skin.
///
/// Index-aligned with `plates`, and a pure function of the two arguments: the
/// clad set, each plate's out face and each plate's relief all come off the
/// plates themselves, so this cannot drift from the skin it describes.
pub fn read_plates(structure: &SkinStructure, plates: &[SkinPlate]) -> Vec<PlateReading> {
    // The finished skin, indexed the two ways the walks below ask for it: the
    // clad cells (what a corner's own predicate reads), and what each clad cell
    // WEARS (which way it faces, and whether it is flat).
    let clad: HashSet<IVec3> = plates.iter().map(|plate| plate.cell).collect();
    let surface: HashMap<IVec3, (IVec3, PlateRelief)> = plates
        .iter()
        .map(|plate| (plate.cell, (plate_out(plate), relief_of(&plate.shape))))
        .collect();

    plates
        .iter()
        .map(|plate| {
            let out = plate_out(plate);
            let face = face_of(out);
            let relief = relief_of(&plate.shape);
            let (u, v) = face_plane(face);

            // The four in-plane walks, each stopping twice: once when the
            // surface stops carrying on in the same plane, and once when it
            // stops being the same KIND of place.
            let mut coplanar = [0u8; 4];
            let mut alike = [0u8; 4];
            for (slot, (axis, sign)) in [(u, 1), (u, -1), (v, 1), (v, -1)].into_iter().enumerate() {
                let (reach, same) = walk(&surface, plate.cell, step(axis, sign), out, relief);
                coplanar[slot] = reach;
                alike[slot] = same;
            }

            // The alignment axis: whichever way the like run goes further, with
            // the coplanar run as the tiebreak so a lone plate still points the
            // way its own surface does rather than always answering `+X`.
            let span = |spans: [u8; 4]| (spans[0] + spans[1], spans[2] + spans[3]);
            let (first, second) = span(alike);
            let (tie_first, tie_second) = span(coplanar);
            let along = match (second > first) || (second == first && tie_second > tie_first) {
                true => IVec3::AXES[v],
                false => IVec3::AXES[u],
            };
            let reach = first.max(second);

            PlateReading {
                out,
                along,
                relief,
                enclosure: enclosure(structure, &clad, plate.cell, u, v),
                run: (reach + 1).min(RUN_REACH),
                border: *alike.iter().min().unwrap_or(&0),
                height: (plate.shape.volume() * f32::from(FULL)).round() as u8,
                depth: support_depth(structure, plate.cell, face) as u8,
                pocket: pocket_distance(structure, plate.cell, u, v),
            }
        })
        .collect()
}

/// How far the surface carries on from `cell` in one direction: the number of
/// cells that face the same way, and the number of those that are also the same
/// kind of place.
///
/// The like count can never run past the coplanar one - a plate that faces
/// another way is not part of this surface whatever its shape is - so the two
/// are counted in one walk rather than two.
fn walk(
    surface: &HashMap<IVec3, (IVec3, PlateRelief)>,
    cell: IVec3,
    direction: IVec3,
    out: IVec3,
    kind: PlateRelief,
) -> (u8, u8) {
    let mut coplanar = 0;
    let mut alike = 0;
    let mut still_alike = true;
    for reach in 1..=RUN_REACH {
        match surface.get(&(cell + direction * reach as i32)) {
            Some((facing, relief)) if *facing == out => {
                coplanar = reach;
                still_alike &= *relief == kind;
                if still_alike {
                    alike = reach;
                }
            }
            _ => break,
        }
    }
    (coplanar, alike)
}

/// How many of the eight in-plane cells around `cell` the surface ends against
/// rather than tapering into.
fn enclosure(
    structure: &SkinStructure,
    clad: &HashSet<IVec3>,
    cell: IVec3,
    u: usize,
    v: usize,
) -> u8 {
    let mut count = 0;
    for du in -1..=1 {
        for dv in -1..=1 {
            if du == 0 && dv == 0 {
                continue;
            }
            let next = cell + step(u, du) + step(v, dv);
            count += u8::from(ends_against(structure, clad, next));
        }
    }
    count
}

/// How many cells away, in the plate's own plane, the nearest blind pocket is.
///
/// Rings outward, so the first hit is the answer and a plate with nothing near
/// costs the whole square. Saturates at [`POCKET_REACH`].
fn pocket_distance(structure: &SkinStructure, cell: IVec3, u: usize, v: usize) -> u8 {
    for reach in 1..POCKET_REACH {
        let ring = i32::from(reach);
        for du in -ring..=ring {
            for dv in -ring..=ring {
                // The ring, not the square: the inside was answered by an
                // earlier pass.
                if du.abs() != ring && dv.abs() != ring {
                    continue;
                }
                if blind_pocket(structure, cell + step(u, du) + step(v, dv)) {
                    return reach;
                }
            }
        }
    }
    POCKET_REACH
}

/// The direction a plate shows to space, in the ship's cells.
///
/// Off the INTEGERS the derivation already stored - a plate's anchor is the cell
/// across its own floor - rather than off its quaternion, so nothing here has an
/// epsilon in it.
fn plate_out(plate: &SkinPlate) -> IVec3 {
    plate.cell - plate.anchor
}

/// The [`FACES`] index of a cardinal direction. Every caller here is handed one
/// the derivation built, so a miss is impossible and falls back to `+X`.
fn face_of(out: IVec3) -> usize {
    FACES.iter().position(|face| *face == out).unwrap_or(0)
}

/// What the top of a plate wearing `shape` is shaped like.
///
/// The order is load-bearing. A plate with every sample on the floor is the
/// STUD, whose middle falls back to half a cell - it would read as flat, and it
/// is the pointiest thing on a hull.
fn relief_of(shape: &ShellShape) -> PlateRelief {
    let samples = [shape.corners, shape.midpoints].concat();
    if samples.iter().all(|sample| *sample == 0) {
        return PlateRelief::Peak;
    }
    if samples.iter().all(|sample| *sample == samples[0]) {
        return PlateRelief::Flat;
    }
    if samples.contains(&FULL) {
        return PlateRelief::Step;
    }
    let crest = |a: usize, b: usize, c: usize, d: usize| {
        shape.midpoints[a] > 0
            && shape.midpoints[b] > 0
            && shape.midpoints[c] == 0
            && shape.midpoints[d] == 0
    };
    if shape.corners.iter().all(|corner| *corner == 0) && (crest(0, 2, 1, 3) || crest(1, 3, 0, 2)) {
        return PlateRelief::Ridge;
    }
    PlateRelief::Rim
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sections::{
        shell_shape::HALF,
        shell_skin::{derive_skin, SkinStructure},
    };

    /// A section that mates on every face, like a hull cube.
    const OPEN: [bool; 6] = [true; 6];

    fn hull(cells: &[IVec3]) -> SkinStructure {
        let mut structure = SkinStructure::default();
        for cell in cells {
            structure.insert(*cell, OPEN);
        }
        structure
    }

    /// A slab of hull `size` cells on a side in the x/z plane, one cell thick.
    fn slab(size: i32) -> SkinStructure {
        let cells: Vec<IVec3> = (0..size)
            .flat_map(|x| (0..size).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        hull(&cells)
    }

    /// Every plate of a structure, paired with its reading.
    fn read(structure: &SkinStructure) -> Vec<(SkinPlate, PlateReading)> {
        let plates = derive_skin(structure);
        let readings = read_plates(structure, &plates);
        plates.into_iter().zip(readings).collect()
    }

    /// The reading of the plate standing in `cell`.
    fn at(read: &[(SkinPlate, PlateReading)], cell: IVec3) -> PlateReading {
        read.iter()
            .find(|(plate, _)| plate.cell == cell)
            .map(|(_, reading)| *reading)
            .unwrap_or_else(|| panic!("no plate stands at {cell:?}"))
    }

    /// The roof of a wide slab reads as a flat panel in the middle and as its
    /// own border at the edge.
    ///
    /// The two facts every look is built on: where the big pieces may go, and
    /// where the trim goes. Both come off a single walk, so they are checked on
    /// one hull.
    #[test]
    fn the_middle_of_a_wide_deck_is_flat_and_its_edge_is_the_border() {
        let structure = slab(5);
        let read = read(&structure);

        let middle = at(&read, IVec3::new(2, 1, 2));
        assert_eq!(
            middle.relief,
            PlateRelief::Flat,
            "a wide roof is flat plate"
        );
        assert_eq!(middle.out, IVec3::Y, "the roof faces up");
        assert_eq!(
            middle.enclosure, 8,
            "every cell around the middle of a deck carries the surface on",
        );
        assert!(
            middle.border > 0,
            "the middle of a 5x5 roof is not on its own edge",
        );
        assert!(
            middle.run >= 3,
            "a 5-cell deck should read as a run, not as a cell: {}",
            middle.run,
        );

        // One in from the corner: still flat, but its patch ends one cell away.
        let edge = at(&read, IVec3::new(1, 1, 1));
        assert_eq!(edge.relief, PlateRelief::Flat);
        assert_eq!(
            edge.border, 0,
            "a plate whose neighbour is the rim is ON the border",
        );
    }

    /// A one-cell spine reads as a RIDGE, aligned down its own length.
    ///
    /// The alignment half is the point: a rib run turned to `along` follows the
    /// spine, and the same piece at a free yaw sits across it.
    #[test]
    fn a_one_cell_spine_reads_as_a_ridge_along_its_length() {
        let structure = hull(&[IVec3::ZERO, IVec3::Z, IVec3::Z * 2, IVec3::Z * 3]);
        let read = read(&structure);
        let crest = at(&read, IVec3::Y + IVec3::Z);

        assert_eq!(crest.relief, PlateRelief::Ridge);
        assert_eq!(crest.along, IVec3::Z, "the ridge runs down the spine");
        // The run is measured over LIKE plate, so a ridge's run is the ridge -
        // which is what a rib strip laid down a spine needs to know.
        assert!(
            crest.run >= 2,
            "the crest of a four-cell spine reads a run of {}",
            crest.run,
        );
        assert_eq!(crest.depth, 1, "one cell of hull under a one-cell spine");
    }

    /// The RIM is what most of a real hull comes out as, so the run and the
    /// border have to speak about it - not only about flat panels.
    ///
    /// Measured, on the wfc row: 100-120 of every 132-162 plates are rim, and
    /// 6-22 are flat. A vocabulary whose run and border were silent on four
    /// fifths of every ship could not carry a look, whatever else it said.
    #[test]
    fn a_rim_carries_a_run_and_a_border_of_its_own() {
        let structure = slab(6);
        let read = read(&structure);
        let rims: Vec<PlateReading> = read
            .iter()
            .map(|(_, reading)| *reading)
            .filter(|reading| reading.relief == PlateRelief::Rim && reading.out == IVec3::Y)
            .collect();

        assert!(rims.len() > 4, "the roof of a 6x6 slab has a rim round it");
        assert!(
            rims.iter().any(|reading| reading.run >= 3),
            "a rim running down one side of a slab is a RUN, not a lone cell",
        );
        assert!(
            rims.iter().any(|reading| reading.border == 0),
            "some rim plate is at the end of its own run",
        );
    }

    /// A lone cube's plates are STUDS, enclosed by nothing.
    #[test]
    fn a_lone_cube_wears_studs() {
        let structure = hull(&[IVec3::ZERO]);
        let read = read(&structure);
        for (plate, reading) in &read {
            assert_eq!(
                reading.relief,
                PlateRelief::Peak,
                "{:?} wears `{}`",
                plate.cell,
                plate.shape.id(),
            );
            assert_eq!(reading.enclosure, 0, "a lone plate has nothing beside it");
            assert_eq!(reading.depth, 1);
        }
    }

    /// A plate beside a block standing proud reads as a STEP.
    #[test]
    fn the_plate_against_a_proud_block_reads_as_a_step() {
        let mut structure = slab(5);
        structure.insert(IVec3::new(2, 1, 2), OPEN);
        let read = read(&structure);

        let beside = at(&read, IVec3::new(1, 1, 2));
        assert_eq!(
            beside.relief,
            PlateRelief::Step,
            "the plate against a proud block has to climb it",
        );
    }

    /// Depth separates the skin over the BODY of a ship from the skin over a
    /// spar, and it costs nothing - the derivation measures it anyway.
    #[test]
    fn depth_counts_the_structure_under_a_plate() {
        let deep: Vec<IVec3> = (0..4).map(|y| IVec3::new(0, -y, 0)).collect();
        let structure = hull(&deep);
        let read = read(&structure);

        assert_eq!(
            at(&read, IVec3::Y).depth,
            4,
            "a plate on a four-cell tower stands on four cells",
        );
        assert_eq!(
            at(&read, IVec3::X).depth,
            1,
            "a plate on the FLANK of that tower stands on one",
        );
    }

    /// A blind face's pocket is measured across the surface, so the plates
    /// beside a nozzle can be decorated differently from the open hull.
    #[test]
    fn a_plate_beside_a_fitting_reads_the_pocket_it_leaves() {
        let mut structure = slab(7);
        // A drive standing on the deck: bolted down through its base, blind on
        // every other face, so it keeps the skin out of the cells beside it.
        let mut drive = [false; 6];
        drive[3] = true;
        structure.insert(IVec3::new(3, 1, 3), drive);
        let read = read(&structure);

        let near = at(&read, IVec3::new(3, 1, 1));
        assert!(
            near.pocket < POCKET_REACH,
            "the plate two cells off a nozzle should see its pocket, read {}",
            near.pocket,
        );
        let far = at(&read, IVec3::new(0, 1, 0));
        assert_eq!(
            far.pocket, POCKET_REACH,
            "the far corner of the deck has no fitting near it",
        );
    }

    /// The reading is a pure function of the structure, exactly as the skin is.
    ///
    /// The whole scatter rests on this: a decoration hashed off a plate that
    /// reads differently between two runs would flicker in the editor.
    #[test]
    fn the_same_structure_always_reads_the_same() {
        let cells: Vec<IVec3> = (0..3)
            .flat_map(|x| (0..3).flat_map(move |z| (0..2).map(move |y| IVec3::new(x, y, z))))
            .collect();
        let forwards = hull(&cells);
        let mut reversed = cells.clone();
        reversed.reverse();
        let backwards = hull(&reversed);

        let a = read_plates(&forwards, &derive_skin(&forwards));
        let b = read_plates(&backwards, &derive_skin(&backwards));
        assert_eq!(a, b, "the reading depends on insertion order");
    }

    /// A flat plate always reads fully enclosed, which is what says the
    /// vocabulary and the boundary samples are reading the SAME predicate.
    #[test]
    fn a_flat_plate_is_always_fully_enclosed() {
        let structure = slab(6);
        for (plate, reading) in read(&structure) {
            if reading.relief == PlateRelief::Flat {
                assert_eq!(
                    reading.enclosure,
                    8,
                    "`{}` is flat but reads {} of 8 enclosed",
                    plate.shape.id(),
                    reading.enclosure,
                );
            }
        }
    }

    /// The relief classes are read off the samples alone, so they can be pinned
    /// on the shapes themselves rather than on a hull that happens to make one.
    #[test]
    fn the_relief_classes_are_read_off_the_eight_samples() {
        let shape = |corners: [u8; 4], midpoints: [u8; 4]| {
            ShellShape::new(corners, midpoints).expect("a legal shape")
        };
        assert_eq!(
            relief_of(&shape([HALF; 4], [HALF; 4])),
            PlateRelief::Flat,
            "the plate a run of skin travels at",
        );
        assert_eq!(
            relief_of(&shape([0; 4], [0; 4])),
            PlateRelief::Peak,
            "every sample on the floor is the STUD, not a flat plate",
        );
        assert_eq!(
            relief_of(&shape([0; 4], [HALF, 0, HALF, 0])),
            PlateRelief::Ridge,
        );
        assert_eq!(
            relief_of(&shape([FULL, 0, 0, 0], [HALF, 0, 0, HALF])),
            PlateRelief::Step,
        );
        assert_eq!(
            relief_of(&shape([HALF, HALF, 0, 0], [HALF, 1, 0, 1])),
            PlateRelief::Rim,
        );
    }
}
