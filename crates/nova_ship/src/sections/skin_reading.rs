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
//!   [`PlateReading::border`] and [`PlateReading::fitting`] are both distances
//!   the derivation already had the ingredients for.
//!
//! # What a real hull is made of
//!
//! Measured, and it is the number a style author has to start from: the
//! generator's own hulls come out four fifths FALLING PLATE and a seventh
//! [`Step`], with [`Flat`] under a seventh and [`Peak`] absent. A rule written
//! for flat panels alone lands on almost nothing.
//!
//! That is why the falling plate is THREE classes and not one. A hull's
//! straight edge ([`Brink`]), its corner-cut panels ([`Bevel`]) and its tips and
//! saddles ([`Spur`]) are different places to stand something, and the split
//! costs nothing: the derivation already put a corner on the floor for exactly
//! one reason, and counting those corners is the whole of it.
//!
//! [`Bevel`]: PlateRelief::Bevel
//! [`Brink`]: PlateRelief::Brink
//! [`Flat`]: PlateRelief::Flat
//! [`Peak`]: PlateRelief::Peak
//! [`Spur`]: PlateRelief::Spur
//! [`Step`]: PlateRelief::Step

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::sections::{
    shell_shape::{ShellShape, FULL},
    shell_skin::{
        ends_against, face_plane, step, support_depth, SkinPlate, SkinStructure, FACES,
        FACE_CORNERS,
    },
};

/// The prelude: `PlateReading`, `PlateRelief`, `PlateFacing`, `read_plates` and
/// `relief_tally`.
pub mod prelude {
    pub use super::{read_plates, relief_tally, PlateFacing, PlateReading, PlateRelief};
}

/// How far a run walk looks along one direction before it stops counting.
///
/// A rule that wants "a long flat panel" is answered by 8; past that the extra
/// cells tell a scatter rule nothing it would act on differently, and the walk
/// is per plate per direction.
pub const RUN_REACH: u8 = 8;

/// How far a plate looks across its own surface for a fitting.
///
/// [`PlateReading::fitting`] saturates HERE rather than at some sentinel, so
/// `fitting == FITTING_REACH` reads as "nothing near" and a rule never has to
/// spell a magic number.
pub const FITTING_REACH: u8 = 3;

/// What the top of a plate is shaped like, in the terms a scatter rule asks in.
///
/// Read off the eight boundary samples and nothing else, so it is exactly as
/// stable as the skin is: the same structure gives the same reading.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlateRelief {
    /// Every sample at the same height: a flat panel. The one place a big piece
    /// of decoration fits, and MEASURED RARE - see the module note.
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
    /// The skin falls away at ONE CORNER and stands at the running height
    /// everywhere else: a panel with a corner taken off.
    ///
    /// The gentlest of the three falling plates, and the one that reads as
    /// panel rather than as edge - a piece sized for [`Flat`] very nearly fits.
    ///
    /// [`Flat`]: PlateRelief::Flat
    Bevel,
    /// The skin falls away along ONE WHOLE SIDE: the straight edge of a hull,
    /// and the plate a rim strip or a fairing belongs on.
    ///
    /// The only falling plate with a single outward direction, which is
    /// [`PlateReading::fall`] - so this is the one a piece can be turned
    /// OUTBOARD on.
    Brink,
    /// The skin falls away on TWO SIDES OR MORE: an outer corner, the tip of a
    /// spar, a saddle between two runs. The pointy end of a ship.
    Spur,
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
    /// The in-plane direction the surface falls away toward, in the ship's
    /// cells, or [`IVec3::ZERO`] where it does not fall one way.
    ///
    /// OUTBOARD, always: a corner only dies to the floor where open space
    /// stands at it, so this points off the ship rather than across it. It is
    /// the second alignment axis, and the one [`along`] cannot give - a fairing
    /// or a rim strip turned to this leans out over the edge it stands on
    /// instead of lying down the edge.
    ///
    /// A cardinal on a [`Brink`] and nothing else worth turning to: every other
    /// relief either does not fall, falls at a diagonal, or falls two ways and
    /// cancels. See [`fall_of`].
    ///
    /// [`along`]: PlateReading::along
    /// [`Brink`]: PlateRelief::Brink
    pub fall: IVec3,
    /// What the top of the plate is shaped like.
    pub relief: PlateRelief,
    /// How many of the eight cells around this one IN THE PLANE the surface
    /// carries on into, rather than ending against vacuum: 8 is the middle of a
    /// field of plate, 0 is a lone stud.
    ///
    /// The same predicate the CORNERS are read off, which is why a [`Flat`]
    /// plate always reads 8 - a corner can only sit at the running height when
    /// all four of its cells continue. It earns its place on the falling
    /// plates, as the FINE reading of what the relief classes count coarsely:
    /// a [`Brink`] one cell from the end of a spar and one along the flank of a
    /// wide hull are the same relief and read 3 against 5.
    ///
    /// [`Brink`]: PlateRelief::Brink
    /// [`Flat`]: PlateRelief::Flat
    pub enclosure: u8,
    /// How many cells the run of LIKE plate through this one is long, itself
    /// included, measured [`along`] and capped at [`RUN_REACH`].
    ///
    /// Like means same-facing and same [`relief`]: a flat panel's run is its
    /// flat neighbours, and the run of a [`Brink`] is the straight edge of the
    /// ship it lies on. Measuring it per relief rather than only on flat plate
    /// is what makes it useful on a real hull - the derived skin comes out four
    /// fifths falling plate, so a reading that only spoke about flat panels
    /// would be silent about most of every ship.
    ///
    /// [`Brink`]: PlateRelief::Brink
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
    /// How many STEPS ACROSS THE SURFACE the nearest FITTING is - a drive, a
    /// bay or a gun mount, the parts that stand proud of the plating and fire
    /// through it. Saturates at [`FITTING_REACH`], which reads as "no fitting
    /// near".
    ///
    /// This is the research's "weight decoration toward link points", answered
    /// with the only fact that discriminates on a hull of cubes: every hull face
    /// offers a socket, so socket proximity is constant, and it is where a ship
    /// SHOOTS FROM that marks where it gets interesting.
    ///
    /// Counted in FACE steps and not in rings, which was measured rather than
    /// chosen: a ring of 1 is the eight cells round a plate, and on a hull as
    /// full of drives and bays as the generator draws that is nearly every plate
    /// on the ship - a rule reading "right beside a nozzle" carpeted 45% of a
    /// hull and starved every rule under it. Four cells is what "beside"
    /// actually means.
    pub fitting: u8,
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
                fall: fall_of(plate),
                relief,
                enclosure: enclosure(structure, &clad, plate.cell, u, v),
                run: (reach + 1).min(RUN_REACH),
                border: *alike.iter().min().unwrap_or(&0),
                height: (plate.shape.volume() * f32::from(FULL)).round() as u8,
                depth: support_depth(structure, plate.cell, face) as u8,
                fitting: fitting_distance(structure, plate.cell, u, v),
            }
        })
        .collect()
}

/// How many plates of each relief a skin came out as, as `name xN` pairs.
///
/// What a hull OFFERS is what a style can use, and it is not obvious from
/// looking at one: a hand-built ship is nearly all studs, ridges and spurs at a
/// quarter of a cell, and a rule written for flat panels lands nowhere on it.
/// Logged by both the spawner and the build view, so a rule is tuned against a
/// measurement rather than against a screenshot.
pub fn relief_tally(readings: &[PlateReading]) -> String {
    let count = |wanted: PlateRelief| {
        readings
            .iter()
            .filter(|reading| reading.relief == wanted)
            .count()
    };
    let listed: Vec<String> = [
        ("flat", PlateRelief::Flat),
        ("step", PlateRelief::Step),
        ("ridge", PlateRelief::Ridge),
        ("peak", PlateRelief::Peak),
        ("bevel", PlateRelief::Bevel),
        ("brink", PlateRelief::Brink),
        ("spur", PlateRelief::Spur),
    ]
    .into_iter()
    .filter_map(|(name, relief)| match count(relief) {
        0 => None,
        found => Some(format!("{name} x{found}")),
    })
    .collect();
    match listed.is_empty() {
        true => "none".to_string(),
        false => listed.join(", "),
    }
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

/// How many face steps away, in the plate's own plane, the nearest FITTING is.
///
/// A fitting is a cell that fires through some face, which is a drive, a bay or
/// a gun mount and nothing else. Read off the exit and not off an absent socket:
/// a fitting's flanks carry nothing to mate either, and so does an authored hull
/// with a face left bare.
///
/// Walks outward a step at a time, so the first hit is the answer. Saturates at
/// [`FITTING_REACH`]. See [`PlateReading::fitting`] for why the distance is
/// counted in face steps rather than in rings.
fn fitting_distance(structure: &SkinStructure, cell: IVec3, u: usize, v: usize) -> u8 {
    for reach in 1..FITTING_REACH {
        let far = i32::from(reach);
        for du in -far..=far {
            // The rim of the diamond, not its inside: the nearer cells were
            // answered by an earlier pass. The two sides meet where `dv` is
            // zero, which costs one repeated lookup and no branch.
            let dv = far - du.abs();
            for dv in [dv, -dv] {
                if structure.fires_at_all(cell + step(u, du) + step(v, dv)) {
                    return reach;
                }
            }
        }
    }
    FITTING_REACH
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
    // Everything left FALLS, and the three classes are how many ways. Read off
    // the corners on the FLOOR, which the derivation puts there for exactly one
    // reason: open space stands at that corner. Structure holds a corner up,
    // and so does the mouth of a fitting, so the count is the number of
    // directions this plate has vacuum in.
    let fallen = fallen_corners(shape);
    match (fallen.count_ones(), fallen) {
        (1, _) => PlateRelief::Bevel,
        // The four masks whose two corners sit on one side of the face. The
        // other two, the diagonals, are saddles and fall two ways.
        (2, 0b0011 | 0b0110 | 0b1100 | 0b1001) => PlateRelief::Brink,
        _ => PlateRelief::Spur,
    }
}

/// Which of a shape's four corner slots die to the cell floor, as a bit per
/// slot in the order the digits are read.
fn fallen_corners(shape: &ShellShape) -> u8 {
    (0..4).fold(0u8, |mask, slot| match shape.corners[slot] {
        0 => mask | 1 << slot,
        _ => mask,
    })
}

/// The in-plane direction a plate's surface falls away toward, in the SHIP's
/// cells, or [`IVec3::ZERO`] where it does not fall one way.
///
/// The sum of the directions of the corners on the floor, taken in the shape's
/// own frame and then turned by the plate's own rotation - which is the same
/// path [`plate_for`](super::shell_skin::plate_for) read the samples along, so
/// nothing here re-derives a face or an axis.
///
/// The sum is the whole trick, and it is why the answer is worth having:
/// [`Brink`]'s two adjacent corners add to the CARDINAL between them, a saddle's
/// two opposite corners cancel to zero, and a [`Peak`] or a [`Ridge`], which
/// fall every way and two ways, cancel as well.
///
/// [`Brink`]: PlateRelief::Brink
/// [`Peak`]: PlateRelief::Peak
/// [`Ridge`]: PlateRelief::Ridge
fn fall_of(plate: &SkinPlate) -> IVec3 {
    let mut local = Vec3::ZERO;
    for slot in 0..4 {
        if plate.shape.corners[slot] == 0 {
            let (x, z) = FACE_CORNERS[slot];
            local += Vec3::new(x as f32, 0.0, z as f32);
        }
    }
    let fall = plate.rotation * local;
    // Every component is a sum of unit steps turned by a grid rotation, so it
    // lands on a whole number and the half is only ever a float epsilon.
    IVec3::from(std::array::from_fn(|axis| match fall[axis] {
        component if component > 0.5 => 1,
        component if component < -0.5 => -1,
        _ => 0,
    }))
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

    /// FALLING PLATE is what most of a real hull comes out as, so the run and
    /// the border have to speak about it - not only about flat panels.
    ///
    /// Measured, on the wfc row: 100-120 of every 132-162 plates fall, and 6-22
    /// are flat. A vocabulary whose run and border were silent on four fifths
    /// of every ship could not carry a look, whatever else it said.
    #[test]
    fn a_brink_carries_a_run_and_a_border_of_its_own() {
        let structure = slab(6);
        let read = read(&structure);
        let brinks: Vec<PlateReading> = read
            .iter()
            .map(|(_, reading)| *reading)
            .filter(|reading| reading.relief == PlateRelief::Brink && reading.out == IVec3::Y)
            .collect();

        assert!(
            brinks.len() > 4,
            "the roof of a 6x6 slab has a straight edge down each side",
        );
        assert!(
            brinks.iter().any(|reading| reading.run >= 3),
            "an edge running down one side of a slab is a RUN, not a lone cell",
        );
        assert!(
            brinks.iter().any(|reading| reading.border == 0),
            "some edge plate is at the end of its own run",
        );
    }

    /// The falling plate splits three ways on the ONE hull that shows all
    /// three, and each class carries the fall the scatter turns a piece to.
    ///
    /// This is the whole of the widening: before it, every plate below was one
    /// bucket covering four fifths of a ship, and a rule could not tell the
    /// straight edge of a hull from the tip of a spar.
    #[test]
    fn the_falling_plate_splits_into_edge_corner_and_bevel() {
        // A 4x4 deck with one corner cell missing: its roof carries a straight
        // edge down each side, an outer corner at three of the four corners,
        // and - at the notch - the corner-cut panel a bevel is.
        let mut cells: Vec<IVec3> = (0..4)
            .flat_map(|x| (0..4).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        cells.retain(|cell| *cell != IVec3::new(3, 0, 3));
        let read = read(&hull(&cells));
        let roof = |cell: IVec3| at(&read, cell);

        // The middle of a side: the deck falls away on one whole side, and the
        // fall is the CARDINAL pointing off the ship.
        let edge = roof(IVec3::new(1, 1, 0));
        assert_eq!(edge.relief, PlateRelief::Brink);
        assert_eq!(
            edge.fall,
            IVec3::NEG_Z,
            "a brink on the -Z flank falls outboard, off the ship",
        );

        // An outer corner: two whole sides gone, so it falls off the ship on
        // the diagonal between them and no quarter turn lies on it.
        let corner = roof(IVec3::new(0, 1, 0));
        assert_eq!(corner.relief, PlateRelief::Spur);
        assert_eq!(corner.fall, IVec3::new(-1, 0, -1));

        // The plate across the notch: the deck carries on down both of its own
        // sides and only the one corner dies.
        let bevel = roof(IVec3::new(2, 1, 2));
        assert_eq!(
            bevel.relief,
            PlateRelief::Bevel,
            "a panel with one corner taken off is not the edge of a ship",
        );
        assert_eq!(bevel.fall, IVec3::new(1, 0, 1), "it falls into the notch");
    }

    /// A plate that falls EVERY way, or two ways at once, has no outward
    /// direction - and says so, rather than picking one.
    #[test]
    fn a_plate_that_falls_two_ways_reads_no_fall_at_all() {
        // A one-cell spine: its roof is a ridge, falling off both flanks.
        let spine = hull(&[IVec3::ZERO, IVec3::Z, IVec3::Z * 2, IVec3::Z * 3]);
        let crest = at(&read(&spine), IVec3::Y + IVec3::Z);
        assert_eq!(crest.relief, PlateRelief::Ridge);
        assert_eq!(crest.fall, IVec3::ZERO);

        // A lone cube: every plate is a stud, falling on all four sides.
        for (_, reading) in read(&hull(&[IVec3::ZERO])) {
            assert_eq!(reading.relief, PlateRelief::Peak);
            assert_eq!(reading.fall, IVec3::ZERO);
        }
    }

    /// The fall is the SECOND alignment axis, and it has to be square to the
    /// first - a rim strip lies down the edge, a fairing leans out over it.
    #[test]
    fn the_fall_of_an_edge_is_square_to_the_run_along_it() {
        let structure = slab(6);
        for (_, reading) in read(&structure) {
            if reading.relief != PlateRelief::Brink {
                continue;
            }
            assert_ne!(reading.fall, IVec3::ZERO, "a brink falls one way");
            assert_eq!(
                reading.fall.dot(reading.along),
                0,
                "the fall {} lies along the run {}",
                reading.fall,
                reading.along,
            );
            assert_eq!(
                reading.fall.dot(reading.out),
                0,
                "the fall must lie in the plate's own plane",
            );
        }
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

    /// A fitting is found across the surface, so the plates beside a nozzle can
    /// be decorated differently from the open hull.
    #[test]
    fn a_plate_beside_a_fitting_reads_how_far_off_it_stands() {
        let mut structure = slab(7);
        // A drive standing on the deck: bolted down through its base, firing
        // straight up out of the top of its own cell.
        let mut drive = [false; 6];
        drive[3] = true;
        structure.insert(IVec3::new(3, 1, 3), drive);
        structure.insert_exit(IVec3::new(3, 1, 3), 2);
        let read = read(&structure);

        let near = at(&read, IVec3::new(3, 1, 1));
        assert!(
            near.fitting < FITTING_REACH,
            "the plate two cells off a nozzle should see it, read {}",
            near.fitting,
        );
        let far = at(&read, IVec3::new(0, 1, 0));
        assert_eq!(
            far.fitting, FITTING_REACH,
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
        // The three falling classes, told apart by how many corners die.
        assert_eq!(
            relief_of(&shape([HALF, HALF, 0, 0], [HALF, 1, 0, 1])),
            PlateRelief::Brink,
            "two corners on one side is the straight edge of a hull",
        );
        assert_eq!(
            relief_of(&shape([HALF, HALF, HALF, 0], [HALF, HALF, 1, 1])),
            PlateRelief::Bevel,
            "one corner down is a panel with a corner taken off",
        );
        assert_eq!(
            relief_of(&shape([HALF, 0, 0, 0], [1, 0, 0, 1])),
            PlateRelief::Spur,
            "three corners down is the tip of a spar",
        );
        assert_eq!(
            relief_of(&shape([HALF, 0, HALF, 0], [1, 1, 1, 1])),
            PlateRelief::Spur,
            "two corners across the diagonal is a saddle, and falls two ways",
        );
    }
}
