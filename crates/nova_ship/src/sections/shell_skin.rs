//! The SKIN: a ship's cladding, derived from the structure it wraps.
//!
//! A skin is not built and not saved. It is a FUNCTION of the sections a ship
//! already has - give it the filled cells and which of their faces carry
//! sockets, and it answers with a plate per cell of outer surface. Nothing
//! places a plate by hand, nothing stores one, and a ship reloaded after the
//! generator improves comes back wearing the better skin.
//!
//! That is also what makes it cheap to show while a player is still building.
//! Recomputing a pure function of the structure is deterministic: the same
//! structure gives the same skin, always, so a preview cannot flicker and a
//! placement cannot make the rest of the hull reshuffle. There is no solver
//! here and no search - the shape of every plate is decided by the
//! neighbourhood its cell sits in and nothing else.
//!
//! Three steps, in order:
//!
//! 1. [`cladding_cells`] decides WHERE skin may go: the empty cells that touch
//!    the hull, less the ones a blind face forbids and the ones with nowhere to
//!    stand.
//! 2. [`boundary_heights`] reads the [`ShellShape`] a cell's outward face
//!    calls for - its level, what each edge does, and how low the
//!    neighbourhood holds each corner - off that cell's neighbourhood.
//! 3. [`plate_for`] turns that shape into the plate's own frame and finds the
//!    rotation that draws it.
//!
//! Two plates read the same foot and the same slope on the edge they share, so
//! their surfaces meet exactly. That is why nothing has to generate a seam
//! between two plates in the same plane: there is no gap to fill.
//!
//! [`spawn_ship_skin`] is where that meets a live ship: a hull marked
//! [`ShipSkin`] is read back into a [`SkinStructure`] at spawn, and every plate
//! the derivation answers with is bolted to the section it clads.

use std::collections::BTreeMap;

use avian3d::prelude::{Collider, ColliderDensity};
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use nova_gameplay::prelude::{
    destructible_body, Health, IntegritySystems, NovaDamageSystems, NovaRoundSystems,
    SectionMarker, SpaceshipRootMarker,
};

use crate::sections::{
    base_section::prelude::SectionFootprint,
    clearance::prelude::SectionExit,
    design_sections::SectionRemoveRequest,
    docking_section::DockingSectionMarker,
    fixture::{
        prelude::{SectionFixture, ShedFixtureMarker},
        refill_shed_budget, shed_dead_fixtures, ShedBudget,
    },
    integrity::build_ship_integrity_graph,
    link_points::prelude::{LinkPoint, SectionLinkPoints},
    shell_shape::{ShellEdgeType, ShellShape, ShellSurface, FULL, HALF, REACH},
    skin_decor::{
        decor_body, decor_pose, decor_tally, dress_skin_decor, solve_decor, ShipDecorMarker,
    },
    skin_reading::{read_plates, relief_tally},
    skin_report::{skin_report, skin_summary},
    skin_style::{GameStyles, ShipStyle, ShipStyleConfig},
};

/// The prelude: `SkinStructure`, `SkinPlate`, `PlacedPart`, `derive_skin`,
/// `read_structure`, `section_cell`, `plate_body`, `SkinAssets`, `ShipSkin`,
/// `ShipSkinMarker`, `SkinSurfaceMarker` and `ShipSkinPlugin`.
pub mod prelude {
    pub use super::{
        derive_skin, plate_body, read_structure, section_cell, PlacedPart, ShipSkin,
        ShipSkinMarker, ShipSkinPlugin, ShipSkinRestored, SkinAssets, SkinPlate, SkinStructure,
        SkinSurfaceMarker,
    };
}

/// The health a plate covering a WHOLE cell carries, scaled down by how much of
/// its cell the shape actually fills.
///
/// Well under a hull section's 200: cladding is meant to come off first and
/// leave the structure behind it still to be fought for. A full plate is about
/// twenty rounds of the player's PDC, a half-cell one about ten, so a burst
/// strips a patch of skin rather than punching a hole in the ship.
const SKIN_HEALTH_PER_CELL: f32 = 80.0;

/// The density every plate is built at.
///
/// ONE number for the whole skin, and no per-plate figure, because
/// [`destructible_body`] hands this to avian as a DENSITY and avian derives real
/// mass as `density * collider_volume`. The collider already carries the shape,
/// so a quarter-height plate weighs a quarter of a full one for free, and a
/// second per-plate number would only count the same shape twice.
///
/// A quarter of a section's, which all stand at 1.0. A plate is a sheet over a
/// frame rather than a solid block of ship, and a clad ship carries hundreds of
/// them against a handful of sections - at structural density the skin would
/// outweigh the ship it wraps and every thruster tune would move with it.
const SKIN_DENSITY: f32 = 0.25;

/// The six cardinal directions, in the order a face index counts them. `face ^
/// 1` is the opposite face, which is the whole reason for the pairing.
pub(crate) const FACES: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

/// The four corners of a cell's outward face, as signs on the two axes that
/// face lies in, in the order a shape's digits are read. A quarter turn about
/// the out axis is one step along this list.
///
/// The SHAPE's own frame counts its digits the same way, with `+Y` out and the
/// pair read as `(x, z)`, which is what lets the vocabulary turn a shape's
/// corner slot back into a direction on the hull.
pub(super) const FACE_CORNERS: [(i32, i32); 4] = [(1, 1), (-1, 1), (-1, -1), (1, -1)];

/// How far down a plate looks for something to lie on. Past a few cells the
/// answer stops telling it anything.
const SUPPORT_REACH: usize = 4;

/// Reciprocal of the quantum a section's pose is quantised onto before it is
/// read as a cell - a power of two, so the snap itself is exact.
///
/// There is NO GRID on a spawned ship. A section's pose comes out of mating one
/// socket to another, so a whole cell of travel arrives as 0.99999997 and a
/// part that stands on a cell boundary sits a quarter cell off its own centre.
/// Rounding the raw float would put a section half a cell from where it stands
/// often enough to matter, and the skin would then clad the wrong neighbourhood.
pub(crate) const PLACEMENT_SNAP: f32 = 4096.0;

/// What one filled cell turns outward: the faces carrying a link-point socket,
/// and the faces something fires, launches or exhausts through.
///
/// Both are SETS rather than single answers because a cell is a bucket, not a
/// section: two half-size parts standing either side of a boundary land in one.
#[derive(Default, Clone, Copy, Debug)]
struct CellFaces {
    /// Faces offering a socket, indexed as [`FACES`].
    sockets: [bool; 6],
    /// Faces something fires through, indexed as [`FACES`].
    exits: [bool; 6],
}

/// A ship's structure, as the skin needs to see it.
///
/// Three facts per filled cell: that it is filled, which of its six faces carry
/// a link-point socket once the section is rotated, and which of them something
/// FIRES through. That is enough, and deliberately less than a section - the
/// skin has no business knowing what a cell holds, only what it presents.
///
/// The exits are here because the skin and the clearance rule need the same
/// fact and must never disagree about it: [`crate::sections::clearance`] reads
/// this type to decide which lanes have to stay void, and [`exit_pocket`] reads
/// it to decide which cells may not be clad.
#[derive(Default, Clone, Debug)]
pub struct SkinStructure {
    cells: HashMap<IVec3, CellFaces>,
    /// Cells a docking port's collar bares, read by [`exit_pocket`] so the
    /// skin leaves them clear like any other pocket.
    collars: HashSet<IVec3>,
}

impl SkinStructure {
    /// A cell filled by a section presenting `faces`, indexed as [`FACES`], and
    /// firing through none of them.
    pub fn insert(&mut self, cell: IVec3, faces: [bool; 6]) {
        self.cells.entry(cell).or_default().sockets = faces;
    }

    /// Bare the collar beside a docking port that fires through `cell`'s
    /// `out` face: the 8 cells lateral to `cell`, and the 8 lateral to
    /// `cell + FACES[out]` where the mated hull's own port stands.
    ///
    /// A port's face can stand recessed in its cell, so two mated hulls both
    /// clad the cells beside the seam and their plates meet there: about half
    /// of the plate pairs that kept a parked hull off a generated station's
    /// port were in this ring. Both rings are bared, the one the mated hull's
    /// port stands in as well as the port's own.
    pub fn insert_collar(&mut self, cell: IVec3, out: usize) {
        let normal = FACES[out];
        for anchor in [cell, cell + normal] {
            for x in -1..=1 {
                for y in -1..=1 {
                    for z in -1..=1 {
                        let lateral = IVec3::new(x, y, z);
                        if lateral == IVec3::ZERO || lateral.dot(normal) != 0 {
                            continue;
                        }
                        self.collars.insert(anchor + lateral);
                    }
                }
            }
        }
    }

    /// A cell something fires, launches or exhausts through `out`.
    ///
    /// Separate from the sockets because it is a separate fact: a face with
    /// nothing to mate is usually just a flank, and only the ONE face a part's
    /// business points through needs the space in front of it.
    pub fn insert_exit(&mut self, cell: IVec3, out: usize) {
        self.cells.entry(cell).or_default().exits[out] = true;
    }

    /// A cell filled by a section whose sockets point along `normals` and which
    /// fires along `exit`, both already turned into the ship's frame.
    ///
    /// Sockets are ADDED to whatever the cell already presents, and a normal
    /// that points between two faces is dropped. Both cases are the same one: a
    /// cell is a bucket, not a section, and two half-size parts standing either
    /// side of a boundary land in one. What the skin needs to know is what the
    /// cell as a whole turns outward.
    pub fn insert_section(
        &mut self,
        cell: IVec3,
        normals: impl IntoIterator<Item = Vec3>,
        exit: Option<Vec3>,
    ) {
        let faces = self.cells.entry(cell).or_default();
        for normal in normals {
            if let Some(face) = face_index(normal) {
                faces.sockets[face] = true;
            }
        }
        if let Some(out) = exit.and_then(face_index) {
            faces.exits[out] = true;
        }
    }

    /// Whether a cell holds structure.
    pub fn filled(&self, cell: IVec3) -> bool {
        self.cells.contains_key(&cell)
    }

    /// Whether the section in `cell` offers a socket on its `face`, or `None`
    /// if the cell is empty.
    pub(crate) fn offers(&self, cell: IVec3, face: usize) -> Option<bool> {
        self.cells.get(&cell).map(|faces| faces.sockets[face])
    }

    /// Whether the section in `cell` fires, launches or exhausts through its
    /// `face`.
    pub(crate) fn fires(&self, cell: IVec3, face: usize) -> bool {
        self.cells.get(&cell).is_some_and(|faces| faces.exits[face])
    }

    /// Whether the section in `cell` fires through any face at all, which is
    /// what makes it a FITTING rather than a piece of hull.
    pub(crate) fn fires_at_all(&self, cell: IVec3) -> bool {
        self.cells
            .get(&cell)
            .is_some_and(|faces| faces.exits.iter().any(|exit| *exit))
    }

    /// Every cell that holds structure.
    pub fn filled_cells(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.cells.keys().copied()
    }

    /// Whether the structure holds no cells at all, which is a ship with
    /// nothing to clad.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

/// One plate of derived skin: which cell it stands in, the shape it wears, and
/// the turn that puts that shape the right way round.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkinPlate {
    /// The grid cell the plate fills.
    pub cell: IVec3,
    /// The cell of the structure the plate BOLTS DOWN to, across its own floor.
    /// Always filled - [`stands`] would not have let the plate face this way
    /// otherwise - and it is what the spawner parents a plate by, so a section
    /// dying takes its own cladding with it.
    pub anchor: IVec3,
    /// The shape, whose id also names its prototype.
    pub shape: ShellShape,
    /// The turn that takes the shape's own frame to the cell's.
    pub rotation: Quat,
}

/// The whole skin a structure calls for.
///
/// Deterministic: the same structure gives the same plates in the same order,
/// which is what lets a caller diff two skins or show one as a preview without
/// anything jumping about.
pub fn derive_skin(structure: &SkinStructure) -> Vec<SkinPlate> {
    let skin = cladding_cells(structure);
    let mut plates: Vec<SkinPlate> = skin
        .iter()
        .filter_map(|cell| plate_for(structure, &skin, *cell))
        .collect();
    plates.sort_unstable_by_key(|plate| (plate.cell.x, plate.cell.y, plate.cell.z));
    plates
}

/// Where the skin may stand, decided from the structure alone.
///
/// The goal is total coverage: STRUCTURE MUST NOT FACE VACUUM. Only the skin is
/// allowed to be the last thing before space, so every empty cell that touches
/// the hull is claimed unless something makes it impossible.
///
/// Three things do. A neighbouring section may FIRE into the cell - a mount's
/// muzzle, a bay's mouth, a drive's bell - and nothing may be built in the
/// space a part's business points through, which is what leaves those mouths
/// open without any of the three being named here. A docking port's collar
/// bares the same way, so the cells beside its mouth stay clear for the hull
/// that mates to it. And a plate has to bolt down
/// to structure through one face and show an EMPTY cell through the opposite
/// one ([`stands`]), so a cell wedged between sections on every axis has nowhere
/// to stand.
///
/// ONE PASS, because neither test reads the skin being built. What the cells
/// around a candidate are already wearing cannot decide whether it may be clad:
/// a plate is bounded by its own cell, so no two of them can be in each other's
/// way. Refusing on that ground is what left the inside angle of every L bare.
///
/// The EXIT face and not every blind one. A fitting turns five faces with
/// nothing to mate to the world and fires through exactly one of them, so
/// refusing all five opened a cross-shaped well of bare hull round every drive
/// and every gun: five cells of structure facing vacuum to keep one cell clear.
/// A flank is not a muzzle.
pub fn cladding_cells(structure: &SkinStructure) -> HashSet<IVec3> {
    let mut skin: HashSet<IVec3> = HashSet::new();
    for cell in structure.filled_cells() {
        for direction in FACES {
            let candidate = cell + direction;
            if structure.filled(candidate) {
                continue;
            }
            let footing = (0..FACES.len()).any(|out| stands(structure, candidate, out));
            if footing && !exit_pocket(structure, candidate) {
                skin.insert(candidate);
            }
        }
    }
    skin
}

/// Whether some section FIRES into this cell, or a docking port's collar
/// bares it, which is what keeps the cladding out of it either way.
///
/// The mouth of a drive bell and the muzzle of a gun are both this and nothing
/// else: the one face a part's business points through needs the space in front
/// of it, and a plate laid there is a ship firing into its own plating. A
/// port's collar is the same refusal for a different reason: the cells beside
/// its mouth are left bare so a mated hull's flank has room there too (see
/// [`SkinStructure::insert_collar`]). Read by [`cladding_cells`] to refuse the
/// cell, and by [`boundary_heights`] to tell a pocket like this from open space
/// - the skin ends against a pocket, and tapers away into open space.
///
/// Read off the EXIT and never off an absent socket. A fitting's flanks carry
/// nothing to mate either and nothing comes out of them, which is the same fact
/// [`crate::sections::clearance::exit_normal`] exists for; reading blind faces
/// instead refused five cells round every fitting to protect one.
pub(super) fn exit_pocket(structure: &SkinStructure, cell: IVec3) -> bool {
    structure.collars.contains(&cell)
        || (0..FACES.len()).any(|face| structure.fires(cell + FACES[face], face ^ 1))
}

/// Whether a plate in `cell` may show `out` to space.
///
/// It BOLTS DOWN to structure through the face opposite its out face, and shows
/// an EMPTY cell through the out face itself. Without the first, cladding hangs
/// off other cladding with nothing solid to carry it - and worse, it reads its
/// boundary in a plane none of its neighbours agreed to, so the surface it draws
/// need not meet theirs. Without the second the plate is buried in the ship.
///
/// Empty, and never "empty and not already clad". A plate bolts to structure and
/// stays inside its own cell, so the plate in the cell ahead can neither face
/// the same way (its own bolt face would be this empty cell) nor reach into this
/// one: it hugs another wall of the same concave feature. Refusing a cell whose
/// every direction was taken left the INSIDE ANGLE of every L bare, and both
/// walls of every two-cell slot, and covered neither.
fn stands(structure: &SkinStructure, cell: IVec3, out: usize) -> bool {
    let bolted = structure.offers(cell + FACES[out ^ 1], out) == Some(true);
    bolted && !structure.filled(cell + FACES[out])
}

/// The two axes a face lies in, lowest first.
pub(super) fn face_plane(out: usize) -> (usize, usize) {
    match out / 2 {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// A step of `sign` along `axis`.
pub(super) fn step(axis: usize, sign: i32) -> IVec3 {
    IVec3::AXES[axis] * sign
}

/// The shape a cell's `out` face calls for, in the cell's own frame: corners
/// and edges indexed as [`FACE_CORNERS`] over the face's two axes.
///
/// Every part is read off the cells around this one, and what two plates
/// share they read the same way, so their surfaces meet:
///
/// - the LEVEL is the whole cell beside structure the skin climbs in this
///   plane ([`walls`]), and half a cell everywhere else. See [`plate_level`].
/// - each EDGE reads the cell across it. Structure the skin climbs is a
///   [`Wall`](ShellEdgeType::Wall). More skin joins at the lower of the two
///   levels, so a raised block steps down to the deck at half a cell. A
///   structure or a POCKET the skin ends against holds the level: a gun well
///   or the mouth of a drive bay is a hole in the skin, not the end of it.
///   Open space is the rim, read by [`open_edge`].
/// - each SOFT CORNER averages what the four cells meeting there say the
///   corner's height should be ([`corner_weight`]), so a raised block is
///   capped with a bevel where the hull around it is lower.
///
/// A DIAGONAL plate - a whole-cell plate whose wall touches its own structure
/// along an edge only, which is a staircase rather than a step - ramps at
/// slope one away from that wall instead of standing a full block, and its
/// open end falls into the same plane, so a diagonal stair is one continuous
/// ramp.
pub fn boundary_heights(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
    out: usize,
) -> ShellShape {
    let (u, v) = face_plane(out);
    let level = plate_level(structure, cell, out);
    let diagonal = match level {
        FULL => diagonal_wall(structure, skin, cell, out),
        _ => None,
    };
    let mut feet = [level; 4];
    let mut edges = [ShellEdgeType::Plain; 4];
    for slot in 0..4 {
        let across = edge_direction(u, v, slot);
        let next = cell + across;
        let ramp = diagonal == Some(-across);
        (feet[slot], edges[slot]) = if walls(structure, next, u, v) {
            (level, ShellEdgeType::Wall)
        } else if skin.contains(&next) {
            let edge = match ramp {
                true => ShellEdgeType::Ramp,
                false => ShellEdgeType::Plain,
            };
            (level.min(plate_level(structure, next, out)), edge)
        } else if ends_against(structure, skin, next) {
            (level, ShellEdgeType::Plain)
        } else {
            open_edge(structure, skin, cell, out, across, ramp, diagonal.is_some())
        };
    }
    let soft_corners = std::array::from_fn(|slot| {
        let (su, sv) = FACE_CORNERS[slot];
        let (du, dv) = (step(u, su), step(v, sv));
        let sum: u8 = [cell, cell + du, cell + dv, cell + du + dv]
            .iter()
            .map(|at| corner_weight(structure, skin, *at, out))
            .sum();
        // A corner on a wall holds the level: the plate closes against the
        // whole face of the structure there, and a cap would bare its corner.
        let on_wall = [slot, (slot + 3) % 4]
            .iter()
            .any(|edge| edges[*edge] == ShellEdgeType::Wall);
        match on_wall {
            true => level,
            // The rounded mean of the four. Only a whole-cell plate is capped
            // by it, and never below half a cell.
            false => ((sum + 2) / 4).clamp(HALF, level),
        }
    });
    ShellShape::new(level, feet, edges, soft_corners).expect("the derivation builds legal shapes")
}

/// The foot and type of an edge a plate meets OPEN SPACE across.
///
/// Three cases, in the order they win:
///
/// - the low side of a diagonal plate, or the edge beside a diagonal plate's
///   ramp into the open: a [`Ramp`](ShellEdgeType::Ramp) to the floor, in the
///   ramp's own plane.
/// - any other open edge of a diagonal plate is its
///   [`RampEnd`](ShellEdgeType::RampEnd).
/// - a CONVEX hull edge, where the plate round the corner faces this way: both
///   plates come down to half a cell and the one with the lower face index owns
///   the skirt that closes the corner. Round a diagonal plate, which ramps and
///   owns no skirt, this plate folds to a ledge at half a cell.
///
/// Anything else is a rim, chamfered to half a cell off a whole-cell plate and
/// to the floor off a half-cell one.
fn open_edge(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
    out: usize,
    across: IVec3,
    ramp: bool,
    diagonal: bool,
) -> (u8, ShellEdgeType) {
    let level = plate_level(structure, cell, out);
    let side = cell - FACES[out] + across;
    let across_face = face_of(across);
    let side_out = match skin.contains(&side) {
        true => facing(structure, side),
        false => None,
    };
    let meets_ramp = side_out.is_some_and(|side_out| {
        open_ramp(structure, skin, side, side_out).is_some_and(|toward| {
            let pair = [FACES[side_out], toward];
            pair.contains(&FACES[out]) && pair.contains(&across)
        })
    });
    let side_diagonal = side_out.is_some_and(|side_out| {
        plate_level(structure, side, side_out) == FULL
            && diagonal_wall(structure, skin, side, side_out).is_some()
    });
    if ramp || meets_ramp {
        (0, ShellEdgeType::Ramp)
    } else if diagonal {
        (0, ShellEdgeType::RampEnd)
    } else if side_out == Some(across_face) {
        // A diagonal plate round the corner ramps, it owns no skirt; this
        // plate folds to a ledge at half a cell against its ramp end.
        let edge = match out < across_face && !side_diagonal {
            true => ShellEdgeType::Skirt,
            false => ShellEdgeType::Fold,
        };
        (HALF, edge)
    } else if level == FULL {
        (HALF, ShellEdgeType::Plain)
    } else {
        (0, ShellEdgeType::Plain)
    }
}

/// The level of a plate facing `out` from `cell`: a whole cell beside
/// structure the skin climbs in the plate's own plane, half a cell otherwise.
fn plate_level(structure: &SkinStructure, cell: IVec3, out: usize) -> u8 {
    let (u, v) = face_plane(out);
    let beside = [step(u, 1), step(u, -1), step(v, 1), step(v, -1)];
    match beside.iter().any(|d| walls(structure, cell + *d, u, v)) {
        true => FULL,
        false => HALF,
    }
}

/// The in-plane direction of a wall a plate meets DIAGONALLY: the wall stands
/// beside the plate, but the plate's own structure has nothing under it - the
/// two touch along an edge, which is a staircase rather than a step on a deck.
///
/// Only where the plate has somewhere to ramp to, away from the wall: more
/// skin or open space. Against structure there it is a block.
fn diagonal_wall(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
    out: usize,
) -> Option<IVec3> {
    let (u, v) = face_plane(out);
    let bolt = cell - FACES[out];
    [step(u, 1), step(u, -1), step(v, 1), step(v, -1)]
        .into_iter()
        .find(|d| walls(structure, cell + *d, u, v) && !structure.filled(bolt + *d))
        .filter(|wall| {
            let low = cell - *wall;
            skin.contains(&low) || !ends_against(structure, skin, low)
        })
}

/// The direction a diagonal plate's ramp falls toward, when that side is open
/// space rather than more skin or something the skin ends against.
fn open_ramp(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
    out: usize,
) -> Option<IVec3> {
    if plate_level(structure, cell, out) != FULL {
        return None;
    }
    let toward = -diagonal_wall(structure, skin, cell, out)?;
    (!ends_against(structure, skin, cell + toward)).then_some(toward)
}

/// What one of the four cells round a corner says that corner's height
/// should be, in quarter cells. [`boundary_heights`] averages the four.
///
/// - a whole cell for structure the skin climbs in this plane, or structure in
///   the cube above this cell: an overhang or a higher layer.
/// - the cell's own level for skin lying in this plane, which is skin with
///   structure under it.
/// - a quarter cell for skin turned onto another face: a convex corner.
/// - half a cell for a pocket, or a part the skin ends against.
/// - the floor for open space.
fn corner_weight(structure: &SkinStructure, skin: &HashSet<IVec3>, at: IVec3, out: usize) -> u8 {
    let (u, v) = face_plane(out);
    let n = FACES[out];
    if walls(structure, at, u, v) || structure.filled(at + n) {
        FULL
    } else if skin.contains(&at) && structure.filled(at - n) {
        plate_level(structure, at, out)
    } else if skin.contains(&at) {
        1
    } else if ends_against(structure, skin, at) {
        HALF
    } else {
        0
    }
}

/// The in-plane step across edge `slot` of a face lying in axes `u` and `v`.
fn edge_direction(u: usize, v: usize, slot: usize) -> IVec3 {
    let (au, av) = FACE_CORNERS[slot];
    let (bu, bv) = FACE_CORNERS[(slot + 1) % 4];
    step(u, (au + bu) / 2) + step(v, (av + bv) / 2)
}

/// The [`FACES`] index of a cardinal step.
fn face_of(direction: IVec3) -> usize {
    FACES
        .iter()
        .position(|face| *face == direction)
        .expect("an edge steps along a cardinal")
}

/// Whether the skin has something to END AGAINST at `cell`, rather than the
/// open space it tapers away into.
///
/// Three ways, and they are one claim: the surface cannot carry on past here.
/// The cell is CLAD, so the surface simply continues into it. It holds
/// STRUCTURE, which the skin either climbs ([`walls`]) or stops at, and either
/// way it is not vacuum. Or it is a POCKET an exit keeps the cladding out of,
/// which is a muzzle mouth or the mouth of a drive bell - a hole in the skin
/// rather than the end of it.
pub(super) fn ends_against(structure: &SkinStructure, skin: &HashSet<IVec3>, cell: IVec3) -> bool {
    skin.contains(&cell) || structure.filled(cell) || exit_pocket(structure, cell)
}

/// Whether the structure in `cell` is a WALL to a skin running in the plane of
/// the axes `u` and `v`: something the plating beside it has to climb to close
/// against.
///
/// Filled is not enough, and reading it that way is what put a fin beside every
/// nozzle. A drive or a gun mount stands a whole cell proud of the hull and
/// offers nothing to mate on any face but the one it bolts down through, so
/// there is nothing there for a plate to close against - the skin runs PAST it
/// at its own height and the part sticks out of the surface, which is what a
/// nozzle is supposed to do. Structure the skin really does have to climb says
/// so by offering a socket in the skin's own plane.
pub(super) fn walls(structure: &SkinStructure, cell: IVec3, u: usize, v: usize) -> bool {
    [u, v]
        .iter()
        .any(|axis| (0..2).any(|side| structure.offers(cell, axis * 2 + side) == Some(true)))
}

/// The plate a clad cell calls for.
///
/// The one genuine choice is WHICH WAY it faces, where more than one direction
/// stands. That goes to the deepest structure the plate can reach ([`facing`]):
/// a plate on the roof finds the whole ship below it and one turned on its side
/// finds a single cell, so the flat answer wins by a wide margin and a wall
/// does not come out as a field of pyramids.
///
/// The rotation is FOUND rather than derived: for each turn that puts the out
/// face the right way, the shape the cell wants is read back into the shape's
/// own frame, and the turn that leaves that reading canonical is the one that
/// places the canonical shape correctly.
pub fn plate_for(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
) -> Option<SkinPlate> {
    let out = facing(structure, cell)?;
    let wanted = boundary_heights(structure, skin, cell, out);

    for rotation in turns() {
        if face_index(rotation * Vec3::Y) != Some(out) {
            continue;
        }
        let mut local = wanted;
        let mut legible = true;
        for (slot, &(x, z)) in FACE_CORNERS.iter().enumerate() {
            let corner = corner_slot(out, rotation * Vec3::new(x as f32, 0.0, z as f32));
            let edge = edge_slot(out, rotation * local_edge(slot));
            match (corner, edge) {
                (Some(corner), Some(edge)) => {
                    local.feet[slot] = wanted.feet[edge];
                    local.edges[slot] = wanted.edges[edge];
                    local.soft_corners[slot] = wanted.soft_corners[corner];
                }
                _ => legible = false,
            }
        }
        if legible && local == local.canonical() {
            return Some(SkinPlate {
                cell,
                anchor: cell + FACES[out ^ 1],
                shape: local,
                rotation,
            });
        }
    }
    None
}

/// The face a clad cell's plate shows to space: of the faces it may stand
/// on, the one with the deepest structure under it, and on a tie the one whose
/// bolt layer - the cells round the one it bolts to - is widest, so a cell
/// beside a raised part faces the deck it stands on rather than turning on its
/// side.
fn facing(structure: &SkinStructure, cell: IVec3) -> Option<usize> {
    (0..FACES.len())
        .filter(|out| stands(structure, cell, *out))
        .max_by_key(|out| {
            (
                support_depth(structure, cell, *out),
                bolt_layer(structure, cell, *out),
            )
        })
}

/// How many of the nine cells round and including the one a plate facing
/// `out` bolts to are filled.
fn bolt_layer(structure: &SkinStructure, cell: IVec3, out: usize) -> usize {
    let (u, v) = face_plane(out);
    let bolt = cell - FACES[out];
    (-1..=1)
        .flat_map(|a| (-1..=1).map(move |b| bolt + step(u, a) + step(v, b)))
        .filter(|at| structure.filled(*at))
        .count()
}

/// How deep the structure under a plate runs, looking straight down from its
/// bolt face.
pub(super) fn support_depth(structure: &SkinStructure, cell: IVec3, out: usize) -> usize {
    let mut depth = 0;
    let mut under = cell + FACES[out ^ 1];
    while structure.filled(under) && depth < SUPPORT_REACH {
        depth += 1;
        under += FACES[out ^ 1];
    }
    depth
}

/// The outward direction of the side edge `edge` stands on, in a shape's OWN
/// frame, where the out face is `+Y`.
fn local_edge(edge: usize) -> Vec3 {
    let (ax, az) = FACE_CORNERS[edge];
    let (bx, bz) = FACE_CORNERS[(edge + 1) % 4];
    Vec3::new((ax + bx) as f32, 0.0, (az + bz) as f32) * 0.5
}

/// The index in [`FACES`] of the cardinal direction `direction` points along,
/// or `None` if it points between two of them.
pub(crate) fn face_index(direction: Vec3) -> Option<usize> {
    FACES.iter().position(|face| {
        direction.abs_diff_eq(Vec3::new(face.x as f32, face.y as f32, face.z as f32), 1e-3)
    })
}

/// Which corner of `out`'s face a direction points at.
fn corner_slot(out: usize, direction: Vec3) -> Option<usize> {
    let (u, v) = face_plane(out);
    let direction = direction.to_array();
    if direction[out / 2].abs() > 1e-3 {
        return None;
    }
    FACE_CORNERS
        .iter()
        .position(|(su, sv)| *su as f32 * direction[u] > 1e-3 && *sv as f32 * direction[v] > 1e-3)
}

/// Which edge of `out`'s face a direction points across.
fn edge_slot(out: usize, direction: Vec3) -> Option<usize> {
    let (u, v) = face_plane(out);
    (0..4).find(|edge| {
        let (au, av) = FACE_CORNERS[*edge];
        let (bu, bv) = FACE_CORNERS[(edge + 1) % 4];
        let wanted = step(u, (au + bu) / 2) + step(v, (av + bv) / 2);
        Vec3::new(wanted.x as f32, wanted.y as f32, wanted.z as f32).dot(direction) > 0.5
    })
}

/// The twenty-four turns that take the grid to itself, swept out of the quarter
/// turns about the three axes and deduplicated by where they send the axes.
fn turns() -> Vec<Quat> {
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut seen = Vec::new();
    let mut turns = Vec::new();
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let rotation = (Quat::from_rotation_x(x as f32 * quarter)
                    * Quat::from_rotation_y(y as f32 * quarter)
                    * Quat::from_rotation_z(z as f32 * quarter))
                .normalize();
                let basis = [
                    face_index(rotation * Vec3::X),
                    face_index(rotation * Vec3::Y),
                    face_index(rotation * Vec3::Z),
                ];
                if basis.iter().all(Option::is_some) && !seen.contains(&basis) {
                    seen.push(basis);
                    turns.push(rotation);
                }
            }
        }
    }
    turns
}

/// Whether a ship wears the derived skin. One flag and nothing else: a skin has
/// nothing to author, being a function of the structure it wraps.
///
/// OPT IN, not the default. The derivation reads a ship as a lattice of unit
/// cells, which is what a ship built out of the catalog's cube sections is; the
/// modelled craft a mod brings are parts of their own sizes standing at half
/// cells, and bucketing those into a unit grid would wrap them in a skin that
/// fits nothing. Until a hull can say which of the two it is, the ships that
/// want cladding ask for it.
#[derive(Component, Clone, Copy, Default, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ShipSkin(pub bool);

/// Marks a ship root whose cladding a thaw is restoring from its frozen
/// record, so [`spawn_ship_skin`] must not derive one and clad it again.
///
/// The thaw spawns a plate per frozen fixture itself, in the same command
/// batch as the section it clads - by the time `spawn_ship_skin` would see the
/// `Added<SectionLinkPoints>` edge, this marker is already on the root.
/// `spawn_ship_skin` takes it off in that pass: it covers the one batch the
/// thaw spawned, and a section a station adds later is re-clad round itself.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct ShipSkinRestored;

/// Marks a derived-skin plate and remembers the shape it wears.
///
/// Two jobs, and the second is what makes the render half possible: a rebuild
/// finds every plate the skin laid last time without touching anything else on
/// the ship, and the dressing observer reads the shape back off the plate
/// instead of the skin having to hand it over.
#[derive(Component, Clone, Copy, Debug, Reflect)]
#[reflect(Component)]
pub struct ShipSkinMarker(pub ShellShape);

/// The whole of a plate that is not a look: what takes damage, weighs
/// something and stops a round.
///
/// A plate is a [`SectionFixture`]: it carries no link points and is not a
/// [`SectionMarker`], so it never enters the link-point graph, never counts
/// toward the ship's health, and is never saved. The structure it wraps is the
/// only thing that is real.
///
/// Health and collider both come off [`ShellShape::volume`], so a plate that
/// fills a quarter of its cell is a quarter as tough and, through avian's
/// density rule, a quarter as heavy.
///
/// GAMEPLAY, all of it. A headless server builds this and stops there; the
/// meshes are hung off it separately by [`dress_skin_plate`].
pub fn plate_body(plate: &SkinPlate, pose: Transform) -> impl Bundle {
    let volume = plate.shape.volume();
    (
        Name::new(format!("Skin Plate {}", plate.shape.id())),
        // Two markers answering two questions: `SectionFixture` says this is
        // not structure, `ShipSkinMarker` says which system owns it.
        SectionFixture,
        ShipSkinMarker(plate.shape),
        pose,
        // Health, density and `Visibility::Inherited` in one bundle - which is
        // why nothing sets visibility here as well.
        destructible_body(SKIN_HEALTH_PER_CELL * volume, SKIN_DENSITY),
        plate_collider(volume),
    )
}

/// A plate rebuilt from a frozen record rather than derived fresh: the exact
/// `shape`, `pose` and `health` [`frozen::freeze_section`](super::frozen::freeze_section)
/// captured, in place of the full-health shape [`plate_body`] would hand a
/// freshly derived one. The collider is rebuilt from `shape`, the same way a
/// fresh plate's is, rather than carried in the frozen record.
///
/// `ShipSkinMarker` still lands on it, so [`dress_skin_plate`] dresses it
/// exactly as it dresses a freshly derived plate.
pub(crate) fn frozen_plate_body(shape: ShellShape, pose: Transform, health: Health) -> impl Bundle {
    (
        Name::new(format!("Skin Plate {}", shape.id())),
        SectionFixture,
        ShipSkinMarker(shape),
        pose,
        health,
        ColliderDensity(SKIN_DENSITY),
        Visibility::Inherited,
        plate_collider(shape.volume()),
    )
}

/// The live sections a root clads: each one's entity beside its
/// [`PlacedPart`], in child order.
type SkinSectionQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static SectionLinkPoints,
        Option<&'static SectionExit>,
        Option<&'static SectionFootprint>,
        Has<DockingSectionMarker>,
    ),
    With<SectionMarker>,
>;

/// Every live section of `root`, as the skin reads it.
fn skin_parts<'a>(
    root: Entity,
    q_children: &Query<&Children>,
    q_sections: &'a SkinSectionQuery,
) -> (Vec<Entity>, Vec<PlacedPart<'a>>) {
    let mut entities: Vec<Entity> = Vec::new();
    let mut placed: Vec<PlacedPart> = Vec::new();
    for section in q_children.get(root).into_iter().flatten() {
        let Ok((transform, link_points, exit, footprint, port)) = q_sections.get(*section) else {
            continue;
        };
        entities.push(*section);
        placed.push(PlacedPart {
            position: transform.translation,
            rotation: transform.rotation,
            link_points: link_points.as_slice(),
            footprint: footprint.map_or(UVec3::ONE, |footprint| **footprint),
            exit: exit.map(|exit| exit.0),
            port,
        });
    }
    (entities, placed)
}

/// Derive a ship's skin and bolt it on: the whole skin for a freshly spawned
/// ship, and the plates round the new sections when a live ship grows.
///
/// Runs on the `Added<SectionLinkPoints>` edge the integrity graph is built
/// off. Every section of a fresh ship lands in one run of commands, so the
/// batch is the whole ship the first time this sees it, and the whole skin is
/// laid once. A station add, swap or rebuild spawns sections into a ship that
/// already wears its skin; then only the cells whose plate the new structure
/// changes are re-clad ([`changed_cells`]): the plate there comes off and the
/// derived one goes on. Everywhere else the hull keeps what it wears, so a
/// plate combat took off stays off - the skin is a pure function of the
/// structure, and re-deriving it all would grow back whatever was shot away.
///
/// Each plate is parented to the SECTION IT CLADS rather than to the ship root.
/// That is what buys a destroyed section taking its own cladding down with it,
/// for free, and it is why the pose has to be turned into the section's frame
/// here: the derivation works in cells of the ship.
///
/// Decoration is laid in the same pass, see [`lay_skin`].
///
/// A whole batch on a root carrying [`ShipSkinRestored`] is skipped: a thaw
/// already spawned its exact plates (and their decor) from a frozen record in
/// the same command batch, and deriving a fresh, full-health skin on top would
/// double the cladding. The marker is taken off then, so a later station add
/// on the thawed ship re-clads round itself like any other.
#[expect(
    clippy::type_complexity,
    reason = "the root, its sections and their plates, each one filtered query"
)]
fn spawn_ship_skin(
    mut commands: Commands,
    q_added: Query<(Entity, &ChildOf), (With<SectionMarker>, Added<SectionLinkPoints>)>,
    q_clad: Query<
        (&ShipSkin, Option<&ShipStyle>, Has<ShipSkinRestored>),
        With<SpaceshipRootMarker>,
    >,
    q_children: Query<&Children>,
    q_sections: SkinSectionQuery,
    q_plates: Query<&Transform, With<ShipSkinMarker>>,
    styles: Option<Res<GameStyles>>,
) {
    let mut grown: BTreeMap<Entity, HashSet<Entity>> = BTreeMap::new();
    for (section, ChildOf(root)) in &q_added {
        if q_clad.get(*root).is_ok_and(|(skin, ..)| skin.0) {
            grown.entry(*root).or_default().insert(section);
        }
    }

    for (root, new) in grown {
        let Ok((_, worn, restored)) = q_clad.get(root) else {
            continue;
        };
        let (entities, placed) = skin_parts(root, &q_children, &q_sections);
        if entities.is_empty() {
            continue;
        }
        let whole = entities.iter().all(|section| new.contains(section));
        if whole && restored {
            commands.entity(root).remove::<ShipSkinRestored>();
            continue;
        }
        let phase = lattice_phase(placed.iter().map(|part| part.position));
        let before: Option<Vec<PlacedPart>> = (!whole).then(|| {
            placed
                .iter()
                .zip(&entities)
                .filter(|(_, section)| !new.contains(*section))
                .map(|(part, _)| part.clone())
                .collect()
        });
        let style = worn
            .zip(styles.as_deref())
            .and_then(|(worn, styles)| worn.resolve(styles));
        lay_skin(
            &mut commands,
            SkinLayout {
                root,
                entities: &entities,
                placed: &placed,
                before: before.as_deref(),
                phase,
                style,
            },
            &q_children,
            &q_sections,
            &q_plates,
        );
    }
}

/// Re-clad a clad ship round a section a [`SectionRemoveRequest`] takes
/// off: the cells whose plate the smaller hull changes lose their plates
/// and take the new ones, the rest stay as they are. The section's own
/// plates leave with it.
///
/// Runs while the section is still live, so the hull before the removal is
/// read off the ship as it stands; the lattice phase is that hull's, so the
/// plates that stay keep their cells.
#[expect(
    clippy::type_complexity,
    reason = "the root, its sections and their plates, each one filtered query"
)]
pub(crate) fn reclad_round_removal(
    request: On<SectionRemoveRequest>,
    mut commands: Commands,
    q_clad: Query<(&ShipSkin, Option<&ShipStyle>), With<SpaceshipRootMarker>>,
    q_children: Query<&Children>,
    q_sections: SkinSectionQuery,
    q_plates: Query<&Transform, With<ShipSkinMarker>>,
    styles: Option<Res<GameStyles>>,
) {
    let root = request.entity;
    let Ok((skin, worn)) = q_clad.get(root) else {
        return;
    };
    if !skin.0 {
        return;
    }
    let (entities, placed) = skin_parts(root, &q_children, &q_sections);
    let phase = lattice_phase(placed.iter().map(|part| part.position));
    let (kept, kept_placed): (Vec<Entity>, Vec<PlacedPart>) = entities
        .iter()
        .zip(&placed)
        .filter(|(section, _)| **section != request.section)
        .map(|(section, part)| (*section, part.clone()))
        .unzip();
    if kept.is_empty() {
        return;
    }
    let style = worn
        .zip(styles.as_deref())
        .and_then(|(worn, styles)| worn.resolve(styles));
    lay_skin(
        &mut commands,
        SkinLayout {
            root,
            entities: &kept,
            placed: &kept_placed,
            before: Some(&placed),
            phase,
            style,
        },
        &q_children,
        &q_sections,
        &q_plates,
    );
}

/// What [`lay_skin`] clads: the root, its live sections (each entity beside
/// its part), the hull it wore before when only the change is re-clad, the
/// lattice phase and the decoration style.
struct SkinLayout<'a> {
    root: Entity,
    entities: &'a [Entity],
    placed: &'a [PlacedPart<'a>],
    /// The hull before a change, or `None` to clad the whole ship.
    before: Option<&'a [PlacedPart<'a>]>,
    phase: Vec3,
    style: Option<&'a ShipStyleConfig>,
}

/// Lay the skin of `layout.placed`: every plate, or with a `before` hull
/// only the plates of the cells the change reshaped, despawning the plates
/// that stood there.
///
/// Decoration is solved in the same pass, from the ship's [`ShipStyle`], and
/// BEFORE any plate is spawned: a plate that takes a piece is reshaped to seat
/// it ([`solve_decor`]). It hangs off the PLATE rather than the section, one
/// level further out for the same reason: a plate shot off takes its greebles
/// with it, which is what makes stripping a patch of skin read as stripping a
/// patch of skin. The solve runs over the whole skin, and a re-clad spawns its
/// answers on the plates it lays only. The hull before a re-clad is seated the
/// same way before it is compared, so a plate whose piece, and with it its
/// shape, changed is re-laid with its new piece.
fn lay_skin(
    commands: &mut Commands,
    layout: SkinLayout,
    q_children: &Query<&Children>,
    q_sections: &SkinSectionQuery,
    q_plates: &Query<&Transform, With<ShipSkinMarker>>,
) {
    let SkinLayout {
        root,
        entities,
        placed,
        before,
        phase,
        style,
    } = layout;
    let (structure, cells) = read_cells(placed, phase);
    if structure.is_empty() {
        return;
    }
    let mut plates = derive_skin(&structure);
    let decor = style.map_or_else(Vec::new, |style| {
        solve_decor(&structure, &mut plates, style)
    });
    // Index-aligned with `plates`: whether this pass lays that plate.
    let relaid: Vec<bool> = match before {
        None => vec![true; plates.len()],
        Some(before) => {
            let (before, _) = read_cells(before, phase);
            let mut worn = derive_skin(&before);
            if let Some(style) = style {
                solve_decor(&before, &mut worn, style);
            }
            let changed = changed_cells(&worn, &plates);
            for section in entities {
                let Ok((pose, ..)) = q_sections.get(*section) else {
                    continue;
                };
                for child in q_children.get(*section).into_iter().flatten() {
                    let Ok(local) = q_plates.get(*child) else {
                        continue;
                    };
                    let centre = pose.translation + pose.rotation * local.translation;
                    if changed.contains(&section_cell(centre, phase)) {
                        commands.entity(*child).despawn();
                    }
                }
            }
            plates
                .iter()
                .map(|plate| changed.contains(&plate.cell))
                .collect()
        }
    };

    let sections: HashMap<IVec3, (Entity, Transform)> = cells
        .into_iter()
        .zip(entities.iter().copied())
        .zip(placed)
        .flat_map(|((cells, entity), part)| {
            let pose = Transform::from_translation(part.position).with_rotation(part.rotation);
            cells.into_iter().map(move |cell| (cell, (entity, pose)))
        })
        .collect();

    let mut shapes: HashSet<ShellShape> = HashSet::new();
    // Index-aligned with `plates`, so the solve's answers can be looked
    // up by the index it gives them back. `None` where a plate was skipped.
    let mut laid: Vec<Option<Entity>> = Vec::with_capacity(plates.len());
    for (plate, relay) in plates.iter().zip(&relaid) {
        if !relay {
            laid.push(None);
            continue;
        }
        // The anchor is filled by construction, so a miss means two
        // sections shared a cell and only one of them is on record.
        let Some((section, pose)) = sections.get(&plate.anchor) else {
            laid.push(None);
            continue;
        };
        let turn = pose.rotation.inverse();
        let centre = plate.cell.as_vec3() + phase;
        let local = Transform::from_translation(turn * (centre - pose.translation))
            .with_rotation(turn * plate.rotation);
        let entity = commands
            .spawn((plate_body(plate, local), ChildOf(*section)))
            .id();
        shapes.insert(plate.shape);
        laid.push(Some(entity));
    }

    // The root already remembers where the ship was hit - `DamageMarks` is
    // a requirement of `SpaceshipRootMarker`. Not inserted here: hull
    // sections carve too, so an unclad ship has a shape to carve after all.
    let readings = read_plates(&structure, &plates);
    let mut taken: Vec<usize> = vec![0; style.map_or(0, |style| style.fixtures.len())];
    if let Some(style) = style {
        for placement in &decor {
            let Some(plate) = laid[placement.plate] else {
                continue;
            };
            let fixture = &style.fixtures[placement.fixture];
            let pose = decor_pose(&plates[placement.plate], placement.turns);
            commands.spawn((decor_body(fixture, pose), ChildOf(plate)));
            taken[placement.fixture] += 1;
        }
    }

    // The shape count is the interesting half of the skin: it is how many
    // meshes the ship costs, and the shape space it is drawn from is
    // unbounded. The RELIEF histogram and the per-fixture tally are the
    // interesting half of the look - a style author tuning greeble weights
    // needs to know what a real hull actually took.
    debug!(
        "spawn_ship_skin: ship {root} {} {} plate(s) over {} shape(s); \
         relief {}; decoration {}; {}",
        if before.is_none() {
            "clad in"
        } else {
            "re-clad"
        },
        laid.iter().flatten().count(),
        shapes.len(),
        relief_tally(&readings),
        style.map_or_else(
            || "none (no style)".to_string(),
            |style| decor_tally(style, &taken),
        ),
        // The SHAPE half of the same picture: how much of this hull is
        // creased, how much of it is the saddle, how many hull faces are
        // bare, and how much of the decoration landed somewhere a
        // flat-bottomed piece actually fits. None of it is legible in a
        // screenshot and all of it decides a style.
        skin_summary(&skin_report(&structure, &plates, &readings, style, &decor)),
    );
}

/// The cells whose plate differs between the skin `before` a ship grew and
/// the skin `after`: a plate gained, lost, reshaped, turned or bolted to
/// another cell. Only these are re-clad, so the change stays local to the
/// sections that caused it.
fn changed_cells(before: &[SkinPlate], after: &[SkinPlate]) -> HashSet<IVec3> {
    let before: HashMap<IVec3, &SkinPlate> =
        before.iter().map(|plate| (plate.cell, plate)).collect();
    let after: HashMap<IVec3, &SkinPlate> = after.iter().map(|plate| (plate.cell, plate)).collect();
    before
        .keys()
        .chain(after.keys())
        .filter(|cell| before.get(*cell) != after.get(*cell))
        .copied()
        .collect()
}

/// One placed part, as everything that reads a ship into cells sees it: where it
/// stands, which way it is turned, the sockets it presents and the face it fires
/// through.
///
/// Deliberately not a `SectionConfig`. The skin spawner holds live entities, the
/// editor holds a build state, and the generator holds tiles; none of the three
/// has a catalog entry to hand at the point it asks, and the skin has no
/// business knowing what a cell holds anyway - only what it presents.
#[derive(Clone, Debug)]
pub struct PlacedPart<'a> {
    /// Where the part stands, in the ship's frame.
    pub position: Vec3,
    /// How it is turned, in the ship's frame.
    pub rotation: Quat,
    /// The sockets it carries, in its OWN frame.
    pub link_points: &'a [LinkPoint],
    /// Number of occupied cells along each local axis.
    pub footprint: UVec3,
    /// The direction it fires, launches or exhausts, in its OWN frame, or
    /// `None` for a part whose whole surface is structure. See
    /// [`crate::sections::clearance::exit_normal`], which is where a kind is
    /// turned into this.
    pub exit: Option<Vec3>,
    /// Whether the part is a docking port. The skin leaves its collar bare
    /// (see [`SkinStructure::insert_collar`]), so a mated hull's flank has
    /// room beside the mouth.
    pub port: bool,
}

/// Read a set of placed parts as the structure the skin sees: the lattice they
/// stand on, and the cell each one fills, in the order they were given.
///
/// ONE reading, shared by everything that clads: the spawner reads a live ship's
/// sections, the editor reads the build state plus the part under the pointer.
/// Two readings would put the lattice in two places and clad the same ship two
/// ways - and the editor's whole claim is that what it shows is what flies.
pub fn read_structure(placed: &[PlacedPart]) -> (SkinStructure, Vec3, Vec<Vec<IVec3>>) {
    let phase = lattice_phase(placed.iter().map(|part| part.position));
    let (structure, cells) = read_cells(placed, phase);
    (structure, phase, cells)
}

/// The same reading, on a lattice somebody else already took.
///
/// The clearance rule asks TWICE about one ship - with a part and without it -
/// and both readings have to land in the same cells, or the difference it
/// measures is about the bucketing rather than about the placement.
pub(crate) fn read_cells(placed: &[PlacedPart], phase: Vec3) -> (SkinStructure, Vec<Vec<IVec3>>) {
    let mut structure = SkinStructure::default();
    let mut occupied = Vec::with_capacity(placed.len());
    for part in placed {
        let half = (part.footprint.as_vec3() - Vec3::ONE) * 0.5;
        let mut cells = Vec::with_capacity(part.footprint.element_product() as usize);
        for x in 0..part.footprint.x {
            for y in 0..part.footprint.y {
                for z in 0..part.footprint.z {
                    let local = Vec3::new(x as f32, y as f32, z as f32) - half;
                    cells.push(section_cell(part.position + part.rotation * local, phase));
                }
            }
        }
        cells.sort_unstable_by_key(|cell| (cell.x, cell.y, cell.z));
        cells.dedup();

        let exit = part.exit.map(|exit| part.rotation * exit);
        let out = exit.and_then(face_index);
        let edge = out.map(|face| {
            cells
                .iter()
                .map(|cell| cell.dot(FACES[face]))
                .max()
                .unwrap_or_default()
        });
        for &cell in &cells {
            let fires = out
                .zip(edge)
                .is_some_and(|(face, edge)| cell.dot(FACES[face]) == edge)
                .then_some(exit)
                .flatten();
            if let Some(face) = out.filter(|_| part.port && fires.is_some()) {
                structure.insert_collar(cell, face);
            }
            // A `Bare` socket is a dead end: it still mates, but the skin
            // reads its face as offering nothing, so no plate stands on it
            // and no plate climbs it.
            structure.insert_section(
                cell,
                part.link_points
                    .iter()
                    .filter(|point| point.skin.is_clad())
                    .map(|point| part.rotation * point.normal),
                fires,
            );
        }
        occupied.push(cells);
    }
    (structure, occupied)
}

/// Where the cell lattice a ship's sections stand on has its origin, per axis.
///
/// A ship has NO GRID, and what it does stand on need not be the integers. A
/// hull mirrored about its centreline puts every section half a cell off it, so
/// reading those cells off the integers would leave a phantom empty column down
/// the spine of the ship: the two halves would never touch, and both skins would
/// die to the floor along a seam that is not there. The lattice is read out of
/// the sections instead - per axis, the offset most of them share - so the cells
/// come from the ship rather than from an assumption about it.
///
/// The MOST COMMON offset, not the first: a part that stands on a cell boundary
/// rather than in the middle (a mount on a hull face) carries an offset of its
/// own, and it is the hull it is bolted to that says where the cells are.
pub(crate) fn lattice_phase(positions: impl Iterator<Item = Vec3> + Clone) -> Vec3 {
    let cell = PLACEMENT_SNAP as i32;
    Vec3::from(std::array::from_fn(|axis| {
        let mut tally: HashMap<i32, usize> = HashMap::new();
        for position in positions.clone() {
            let quantum = (position[axis] * PLACEMENT_SNAP).round() as i32;
            *tally.entry(quantum.rem_euclid(cell)).or_default() += 1;
        }
        tally
            .into_iter()
            // Ties go to the offset nearest the origin, so the answer does not
            // depend on the order a hash map happened to hand them over.
            .max_by_key(|(offset, count)| (*count, -offset))
            .map(|(offset, _)| offset as f32 / PLACEMENT_SNAP)
            .unwrap_or_default()
    }))
}

/// The cell a section stands in, on the lattice `phase` places. See
/// [`PLACEMENT_SNAP`] for why the quantum is taken out before the rounding.
///
/// Public because a READER needs it too: a dump holding a live plate has to put
/// that plate back on the same lattice the derivation used, and rounding the
/// raw float is exactly the mistake this function exists to prevent.
pub fn section_cell(position: Vec3, phase: Vec3) -> IVec3 {
    (((position - phase) * PLACEMENT_SNAP).round() / PLACEMENT_SNAP)
        .round()
        .as_ivec3()
}

/// How rough a plate is where no style says otherwise.
const SKIN_ROUGHNESS: f32 = 0.65;

/// How metallic a plate is where no style says otherwise.
const SKIN_METALLIC: f32 = 0.15;

/// The meshes and materials every derived plate shares.
///
/// The MESHES are keyed by SHAPE, so a hundred plates wearing one shape hold one
/// mesh between them and a shape nothing on screen wears is never built. Lazily
/// filled, which is what makes the size of the shape space irrelevant: a whole
/// ship touches a couple of dozen of them.
///
/// The MATERIALS are keyed by style and then by role. A style can only change
/// what a plate is made OF - the mesh is a function of the hull and cannot be
/// authored - so two ships wearing the same style share their materials whatever
/// shapes they came out as, and the undressed derivation is just the style keyed
/// by the empty id.
#[derive(Resource, Default)]
pub struct SkinAssets {
    meshes: HashMap<ShellShape, Vec<(ShellSurface, Handle<Mesh>)>>,
    materials: HashMap<String, HashMap<ShellSurface, Handle<StandardMaterial>>>,
}

impl SkinAssets {
    /// The mesh and material of every surface role `shape` is cut into, dressed
    /// by `style`, building whatever this pair is the first to need.
    pub fn surfaces(
        &mut self,
        shape: ShellShape,
        style: Option<&ShipStyleConfig>,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
        let surfaces = self
            .meshes
            .entry(shape)
            .or_insert_with(|| {
                shape
                    .surfaces()
                    .into_iter()
                    .map(|(role, mesh)| (role, meshes.add(mesh)))
                    .collect()
            })
            .clone();
        let key = style.map(|style| style.id.as_str()).unwrap_or_default();
        let dressed = match self.materials.get_mut(key) {
            Some(dressed) => dressed,
            None => self.materials.entry(key.to_string()).or_default(),
        };
        surfaces
            .into_iter()
            .map(|(role, mesh)| {
                let material = dressed
                    .entry(role)
                    .or_insert_with(|| {
                        let dress = style.and_then(|style| style.finish(role));
                        materials.add(StandardMaterial {
                            base_color: dress.map_or_else(|| role.colour(), |dress| dress.color),
                            perceptual_roughness: dress
                                .map_or(SKIN_ROUGHNESS, |dress| dress.roughness),
                            metallic: dress.map_or(SKIN_METALLIC, |dress| dress.metallic),
                            ..default()
                        })
                    })
                    .clone();
                (mesh, material)
            })
            .collect()
    }
}

/// Hang the meshes on a plate the moment one appears.
///
/// The RENDER half, and the whole of it: everything a plate does to the
/// simulation is already on it before this runs. One child per surface role,
/// because one mesh carries one material.
///
/// The style is found by walking UP from the plate, the same way `damage_cracks`
/// finds a mesh's owning section. A plate hangs two levels under a live ship's
/// root and one level under the editor's preview root, and neither is the
/// plate's business to know about.
fn dress_skin_plate(
    add: On<Add, ShipSkinMarker>,
    mut commands: Commands,
    q_plate: Query<&ShipSkinMarker>,
    q_child_of: Query<&ChildOf>,
    q_style: Query<&ShipStyle>,
    styles: Option<Res<GameStyles>>,
    mut assets: ResMut<SkinAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let plate = add.entity;
    let Ok(ShipSkinMarker(shape)) = q_plate.get(plate) else {
        return;
    };
    let style = styles
        .as_ref()
        .zip(worn_style(plate, &q_child_of, &q_style))
        .and_then(|(styles, worn)| worn.resolve(styles));
    hang_surfaces(
        &mut commands,
        plate,
        *shape,
        style,
        &mut assets,
        &mut meshes,
        &mut materials,
    );
}

/// Tags a mesh a plate is drawn with, so re-dressing takes the old set off
/// without touching anything else hanging on the plate - a greeble is a child
/// too, and it belongs to the decoration system rather than this one.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct SkinSurfaceMarker;

/// Hang one shape's meshes on a plate.
///
/// The first dressing and every re-dressing both come through here, so a worn
/// plate is built exactly the way a fresh one is: same cache, same style
/// lookup, same surface roles.
pub(crate) fn hang_surfaces(
    commands: &mut Commands,
    plate: Entity,
    shape: ShellShape,
    style: Option<&ShipStyleConfig>,
    assets: &mut SkinAssets,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    for (mesh, material) in assets.surfaces(shape, style, meshes, materials) {
        commands.spawn((
            Name::new("Skin Surface"),
            SkinSurfaceMarker,
            ChildOf(plate),
            Mesh3d(mesh),
            MeshMaterial3d(material),
        ));
    }
}

/// The style the ship a plate hangs on wears, found by walking its ancestors.
fn worn_style<'a>(
    plate: Entity,
    q_child_of: &Query<&ChildOf>,
    q_style: &'a Query<&ShipStyle>,
) -> Option<&'a ShipStyle> {
    let mut current = plate;
    loop {
        if let Ok(style) = q_style.get(current) {
            return Some(style);
        }
        current = q_child_of.get(current).ok()?.0;
    }
}

/// The collider a plate filling `volume` of its cell stands in: a box across
/// the whole footprint, as tall as the plate fills, sitting on the cell floor.
///
/// A CUBOID rather than the surface itself. The generated meshes are not
/// convex (a wall notched at its midpoint is the counterexample), so a hull
/// would paper over the dips, and cladding is thin enough that the difference
/// is a few centimeters of a cell. The box is also exactly `volume` cells of
/// solid, which is what lets one density cover every shape.
///
/// A COMPOUND of one box, because that is the only way to offset a collider
/// from the entity carrying it. A plate hangs off the floor of its cell, not
/// its middle, so a box centred on the entity would float above the surface a
/// shot has to hit.
fn plate_collider(volume: f32) -> Collider {
    Collider::compound(vec![(
        Vec3::Y * (-REACH + volume * 0.5),
        Quat::IDENTITY,
        Collider::cuboid(REACH * 2.0, volume, REACH * 2.0),
    )])
}

/// Clads a ship at spawn, dresses the plates it lays, and sheds the dead ones.
///
/// SPLIT down the middle, and the split is not where a look/no-look reading
/// would put it. A plate carries a collider, health and mass: a round has to
/// stop on one and a shot-off plate has to leave a hole whether or not anybody
/// is watching, so DERIVING the skin and SHEDDING dead plates are gameplay and
/// run on a headless server too. `render` gates the meshes and their materials
/// and nothing else. What a shed plate then does - see
/// [`shed_dead_fixtures`](super::fixture::shed_dead_fixtures) - is the fixture
/// module's, because greebles are shed by the same rule.
#[derive(Default, Clone, Debug)]
pub struct ShipSkinPlugin {
    /// Whether plates are given the meshes that draw them.
    pub render: bool,
}

impl Plugin for ShipSkinPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ShipSkin>();
        app.register_type::<ShipSkinRestored>();
        app.register_type::<ShipSkinMarker>();
        app.register_type::<SectionExit>();
        app.register_type::<ShipStyle>();
        app.register_type::<ShipDecorMarker>();
        app.register_type::<SectionFixture>();
        app.register_type::<ShedFixtureMarker>();
        // Empty until the mod merge fills it, so a headless test or a scenario
        // with no styles authored still resolves (to nothing) rather than
        // panicking on a missing resource.
        app.init_resource::<GameStyles>();
        app.add_systems(
            Update,
            // After the graph so structure is settled before it is dressed,
            // and before the integrity set so a plate is never spawned onto a
            // section the same frame that set takes it away.
            spawn_ship_skin
                .after(build_ship_integrity_graph)
                .before(IntegritySystems),
        );
        app.add_observer(reclad_round_removal);
        // On the fixed step, AFTER the physics phase the damage that empties a
        // plate's health resolves in - see `shed_dead_fixtures`, which owns the
        // reasoning for both the clock and the phase. `IntegritySystems` is an
        // `Update` set and is not among the things to order against. The frame
        // ceiling it spends is refilled in `First`, so a frame that banked
        // several fixed steps still pays for only one frame's worth.
        app.init_resource::<ShedBudget>();
        app.add_systems(First, refill_shed_budget);
        app.add_systems(
            FixedPostUpdate,
            shed_dead_fixtures
                .after(NovaRoundSystems)
                .after(NovaDamageSystems),
        );

        if self.render {
            app.register_type::<SkinSurfaceMarker>();
            app.init_resource::<SkinAssets>();
            app.add_observer(dress_skin_plate);
            app.add_observer(dress_skin_decor);
        }
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::{ColliderDensity, ComputeMassProperties3d};
    use bevy_rand::prelude::{EntropyPlugin, WyRand};
    use nova_gameplay::prelude::{
        Health, HealthApplyDamage, HealthIsolated, NovaHealthPlugin, SectionMarker,
    };

    use super::*;
    use crate::sections::link_points::{unit_cube_link_points, LinkPointSkinType};

    /// How close two floats have to be to count as the same number of cells.
    const EPSILON: f32 = 1e-5;

    /// A section that mates on every face, like a hull cube.
    const OPEN: [bool; 6] = [true; 6];

    fn hull(cells: &[IVec3]) -> SkinStructure {
        let mut structure = SkinStructure::default();
        for cell in cells {
            structure.insert(*cell, OPEN);
        }
        structure
    }

    /// A headless app that can clad a ship, spawn plates and sweep dead ones.
    ///
    /// Deliberately without the render half, since that is what the split
    /// claims: everything asserted below has to hold with no meshes anywhere.
    fn skin_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            NovaHealthPlugin,
        ));
        // The shed draws its kick and its spin off the seeded stream, so a rig
        // without one leaves dead cladding bolted on.
        app.add_plugins(EntropyPlugin::<WyRand>::with_seed(7u64.to_ne_bytes()));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_systems(Update, spawn_ship_skin);
        // The drain is on the fixed step, and real deltas between test frames
        // are microseconds - a frame is stated to be one timestep so that it is
        // also exactly one drain. Same schedule and phase the plugin registers,
        // budget included, so the rig drains the way the game does.
        app.init_resource::<ShedBudget>();
        app.add_systems(First, refill_shed_budget);
        app.add_systems(FixedPostUpdate, shed_dead_fixtures);
        let timestep = app.world().resource::<Time<Fixed>>().timestep();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
        // The first tick of a manual clock is dt 0, so anything spawned before
        // one has passed lives in a frame that never advanced.
        app.update();
        app
    }

    /// Spawn one plate of `shape` under `parent`, exactly as the skin does.
    fn spawn_plate_under(app: &mut App, parent: Entity, shape: ShellShape) -> Entity {
        let plate = SkinPlate {
            cell: IVec3::ZERO,
            anchor: IVec3::NEG_Y,
            shape,
            rotation: Quat::IDENTITY,
        };
        app.world_mut()
            .spawn((plate_body(&plate, Transform::IDENTITY), ChildOf(parent)))
            .id()
    }

    /// The mass avian derives for a plate: its density times the volume of the
    /// collider it carries.
    fn plate_mass(app: &App, plate: Entity) -> f32 {
        app.world()
            .get::<Collider>(plate)
            .expect("a plate carries a collider")
            .mass(SKIN_DENSITY)
    }

    /// A level plate standing `level` across its whole cell.
    fn flat(level: u8) -> ShellShape {
        ShellShape::new(level, [level; 4], [ShellEdgeType::Plain; 4], [level; 4])
            .expect("a legal shape")
    }

    /// The edge slot of a plate facing `out` that looks across `towards`.
    fn edge_towards(out: usize, towards: IVec3) -> usize {
        let (u, v) = face_plane(out);
        (0..4)
            .find(|slot| edge_direction(u, v, *slot) == towards)
            .expect("an edge faces every neighbour in the plane")
    }

    /// A plate is a destructible FIXTURE and never a section.
    ///
    /// The absent `SectionMarker` is the load-bearing half: with it a plate
    /// would join the integrity graph, count toward the ship's health, and be
    /// graded by damage like a part of the hull.
    #[test]
    fn a_plate_spawns_as_a_destructible_fixture_and_not_a_section() {
        let mut app = skin_app();
        let section = app.world_mut().spawn(SectionMarker).id();
        let clad = flat(HALF);
        let plate = spawn_plate_under(&mut app, section, clad);

        let world = app.world();
        assert!(
            world.get::<SectionFixture>(plate).is_some(),
            "an unmarked plate is read as structure by everything downstream",
        );
        assert!(
            world.get::<ShipSkinMarker>(plate).is_some(),
            "the skin cannot find its own plates to lay a new one",
        );
        assert!(
            world.get::<SectionMarker>(plate).is_none(),
            "cladding claiming to be structure would hold a severed ship together",
        );
        assert!(
            world.get::<HealthIsolated>(plate).is_some(),
            "a plate that relays its damage inward makes armour worse than none",
        );

        let health = world.get::<Health>(plate).expect("a plate takes damage");
        assert_eq!(health.max, SKIN_HEALTH_PER_CELL * clad.volume());
        assert_eq!(health.current, health.max, "a fresh plate is undamaged");
        assert_eq!(
            world
                .get::<ColliderDensity>(plate)
                .expect("a plate weighs")
                .0,
            SKIN_DENSITY,
        );

        // The box stands ON the cell floor, where the surface is. Centred on
        // the entity it would float half a plate above the thing a shot has to
        // hit.
        let centre = world
            .get::<Collider>(plate)
            .expect("a plate stops a round")
            .center_of_mass();
        assert!(
            (centre.y - (-REACH + clad.volume() * 0.5)).abs() < EPSILON,
            "the collider sits at {centre} rather than on the cell floor",
        );
    }

    /// Health and mass both come off the share of the cell a plate fills.
    ///
    /// Mass is not ours to set: `destructible_body` hands avian a DENSITY and
    /// avian multiplies it by the volume of the collider, so the one density
    /// constant plus a collider cut to the shape gives the whole family its
    /// weights.
    #[test]
    fn a_full_plate_outweighs_and_outlasts_a_half_one() {
        let mut app = skin_app();
        let section = app.world_mut().spawn(SectionMarker).id();
        let block = flat(FULL);
        let half = flat(HALF);
        assert_eq!(block.volume(), 1.0, "a full block fills its cell");
        assert_eq!(half.volume(), 0.5, "a half-cell plate fills half");

        let solid = spawn_plate_under(&mut app, section, block);
        let thin = spawn_plate_under(&mut app, section, half);

        let health_of = |plate: Entity| app.world().get::<Health>(plate).unwrap().max;
        assert_eq!(health_of(solid), SKIN_HEALTH_PER_CELL);
        assert!(
            (health_of(solid) / health_of(thin) - 2.0).abs() < EPSILON,
            "a full plate should outlast a half one two to one",
        );

        assert!(
            (plate_mass(&app, solid) - SKIN_DENSITY).abs() < EPSILON,
            "a full cell of plate weighs one cell of skin",
        );
        assert!(
            (plate_mass(&app, solid) / plate_mass(&app, thin) - 2.0).abs() < 1e-3,
            "a full plate should outweigh a half one two to one",
        );
    }

    /// A plate whose health runs out comes off the ship and drifts away.
    ///
    /// Nothing else would take it: destruction reaches structure through the
    /// integrity graph, and a fixture is deliberately not in it. The section
    /// behind survives, which is what makes losing skin a different event from
    /// losing a part. What the plate does once it is loose is
    /// [`fixture`](super::super::fixture)'s; this is the wiring that gets it
    /// there.
    #[test]
    fn a_plate_at_zero_health_comes_off_and_leaves_its_section() {
        let mut app = skin_app();
        let section = app
            .world_mut()
            .spawn((SectionMarker, Health::new(500.0)))
            .id();
        let plate = spawn_plate_under(&mut app, section, flat(HALF));

        let health = app.world().get::<Health>(plate).unwrap().max;
        app.world_mut().trigger(HealthApplyDamage {
            entity: plate,
            source: None,
            amount: health,
        });
        app.update();

        let world = app.world();
        assert!(
            world.get::<ChildOf>(plate).is_none(),
            "a dead plate is still bolted to the hull",
        );
        assert!(
            world.get::<ShedFixtureMarker>(plate).is_some(),
            "a dead plate came off without becoming debris",
        );
        assert!(
            world.get_entity(section).is_ok(),
            "the section behind it died with its cladding",
        );
    }

    /// A destroyed section takes its plates with it, for free.
    ///
    /// This is what the parenting decision buys: a section is despawned
    /// recursively when the integrity core destroys it, so its cladding goes
    /// too and nothing has to hunt down the plates of a part that no longer
    /// exists.
    #[test]
    fn destroying_a_section_takes_the_plates_clad_to_it() {
        let mut app = skin_app();
        let section = app.world_mut().spawn(SectionMarker).id();
        let plate = spawn_plate_under(&mut app, section, flat(FULL));
        assert!(app.world().get_entity(plate).is_ok(), "the plate is on");

        app.world_mut().entity_mut(section).despawn();
        app.update();

        assert!(
            app.world().get_entity(plate).is_err(),
            "the plate outlived the section it was bolted to",
        );
    }

    /// A lone cube is clad on all six faces and nowhere else.
    #[test]
    fn a_single_cube_is_wrapped_and_the_wrap_is_one_cell_thick() {
        let plates = derive_skin(&hull(&[IVec3::ZERO]));
        assert_eq!(plates.len(), 6, "one plate per face of the cube");
        for plate in &plates {
            // Every plate is a face neighbour of the cube, never a diagonal.
            assert_eq!(
                plate.cell.abs().element_sum(),
                1,
                "{:?} is not against a face",
                plate.cell,
            );
        }
    }

    /// The skin never claims the cell a part FIRES into, and claims every other
    /// cell around it.
    ///
    /// The owner's case, in the words it was reported in: a 3x3 of hull with a
    /// mount standing on the middle of it. The muzzle cell has to stay open, or
    /// the gun shoots its own plating - and the eight cells around the mount
    /// have to be clad, or the deck it stands on shows bare hull art in a cross
    /// through the middle of the plating.
    #[test]
    fn only_the_cell_a_part_fires_into_is_kept_bare() {
        let deck: Vec<IVec3> = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        let mut structure = hull(&deck);
        // A mount on the middle of the deck, bolted down by its base (-Y),
        // blind on everything else and firing straight up.
        let mut mount = [false; 6];
        mount[3] = true;
        structure.insert(IVec3::Y, mount);
        structure.insert_exit(IVec3::Y, 2);

        let plates = derive_skin(&structure);
        let clad: Vec<IVec3> = plates.iter().map(|plate| plate.cell).collect();
        assert!(
            !clad.contains(&(IVec3::Y * 2)),
            "the skin closed over the muzzle",
        );
        for x in -1..=1 {
            for z in -1..=1 {
                let cell = IVec3::new(x, 1, z);
                if cell == IVec3::Y {
                    continue;
                }
                assert!(
                    clad.contains(&cell),
                    "{cell:?} is deck beside a mount and came back bare hull",
                );
            }
        }
    }

    /// A docking port's collar stays bare - the ring of cells beside its
    /// mouth, on its own cell and on the one its sleeve reaches into - and no
    /// other cell gains or loses a plate.
    #[test]
    fn a_docking_port_leaves_its_collar_bare() {
        let hull_points = unit_cube_link_points();
        let port_points = vec![LinkPoint {
            id: "base".to_string(),
            position: Vec3::Z * 0.5,
            normal: Vec3::Z,
            skin: LinkPointSkinType::Clad,
        }];
        let hull_cube = PlacedPart {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            link_points: &hull_points,
            footprint: UVec3::ONE,
            exit: None,
            port: false,
        };
        let port = PlacedPart {
            position: Vec3::NEG_Z,
            rotation: Quat::IDENTITY,
            link_points: &port_points,
            footprint: UVec3::ONE,
            exit: Some(Vec3::NEG_Z),
            port: true,
        };
        let bare_port = PlacedPart {
            port: false,
            ..port.clone()
        };

        let (clad, _, _) = read_structure(&[hull_cube.clone(), port]);
        let (unclad, _, _) = read_structure(&[hull_cube, bare_port]);
        let clad_cells: HashSet<IVec3> = derive_skin(&clad)
            .into_iter()
            .map(|plate| plate.cell)
            .collect();
        let unclad_cells: HashSet<IVec3> = derive_skin(&unclad)
            .into_iter()
            .map(|plate| plate.cell)
            .collect();

        let collar: HashSet<IVec3> = [IVec3::NEG_Z, IVec3::NEG_Z * 2]
            .into_iter()
            .flat_map(|anchor| {
                (-1..=1).flat_map(move |x| {
                    (-1..=1)
                        .filter(move |y| x != 0 || *y != 0)
                        .map(move |y| anchor + IVec3::new(x, y, 0))
                })
            })
            .collect();
        assert_eq!(collar.len(), 16, "the collar is two 8-cell rings");

        assert!(
            clad_cells.is_disjoint(&collar),
            "a plate stood in the port's collar: {:?}",
            clad_cells.intersection(&collar).collect::<Vec<_>>(),
        );
        let outside_collar: HashSet<IVec3> = unclad_cells.difference(&collar).copied().collect();
        assert_eq!(
            clad_cells, outside_collar,
            "the port changed cladding outside its own collar",
        );
    }

    /// Two plates across a seam stand the same height there.
    ///
    /// This is the property the whole scheme rests on, so it is checked over
    /// real neighbours rather than argued: every pair of clad cells that share
    /// a face and face the same way must put the same foot on the edge they
    /// hold in common, or the seam between them is a step.
    #[test]
    fn neighbouring_plates_agree_about_the_edge_they_share() {
        let structure = hull(&[
            IVec3::ZERO,
            IVec3::X,
            IVec3::X * 2,
            IVec3::Z,
            IVec3::X + IVec3::Z,
            IVec3::X * 2 + IVec3::Y,
        ]);
        let skin = cladding_cells(&structure);
        assert!(!skin.is_empty(), "this hull has an outside");

        for cell in skin.iter().copied() {
            for out in 0..FACES.len() {
                if !stands(&structure, cell, out) {
                    continue;
                }
                let mine = boundary_heights(&structure, &skin, cell, out);
                let (u, v) = face_plane(out);
                for towards in [step(u, 1), step(u, -1), step(v, 1), step(v, -1)] {
                    let next = cell + towards;
                    if !skin.contains(&next) || !stands(&structure, next, out) {
                        continue;
                    }
                    let theirs = boundary_heights(&structure, &skin, next, out);
                    assert_eq!(
                        mine.feet[edge_towards(out, towards)],
                        theirs.feet[edge_towards(out, -towards)],
                        "{cell:?} and {next:?} disagree about the seam between them",
                    );
                }
            }
        }
    }

    /// Every clad cell gets a plate.
    ///
    /// [`cladding_cells`] promises a cell has somewhere to stand, and
    /// [`plate_for`] has to deliver on it - a cell cleared for cladding with no
    /// shape that fits is a hole in a hull that is supposed to be continuous.
    /// Checked over a lumpy hull rather than a cube, because the shapes that
    /// fail are the lopsided ones.
    #[test]
    fn every_cell_cleared_for_cladding_gets_a_plate() {
        let structure = hull(&[
            IVec3::ZERO,
            IVec3::X,
            IVec3::X * 2,
            IVec3::Y,
            IVec3::Z,
            IVec3::X + IVec3::Y,
            IVec3::X * 2 + IVec3::Z,
            IVec3::new(1, 1, 1),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 0, -1),
        ]);
        let skin = cladding_cells(&structure);
        assert!(skin.len() > 20, "a lumpy hull has plenty of outside");
        for cell in skin.iter().copied() {
            assert!(
                plate_for(&structure, &skin, cell).is_some(),
                "{cell:?} was cleared for cladding and no plate fits it",
            );
        }
    }

    /// The skin is a pure function of the structure.
    ///
    /// The whole point of deriving it: the same ship gives the same skin, so a
    /// preview cannot flicker and nothing reshuffles when a player moves the
    /// cursor. Insertion ORDER must not matter either, which a hash map makes
    /// worth checking rather than assuming.
    #[test]
    fn the_same_structure_always_gives_the_same_skin() {
        let cells = [
            IVec3::ZERO,
            IVec3::X,
            IVec3::Y,
            IVec3::Z,
            IVec3::X + IVec3::Y,
        ];
        let forwards = derive_skin(&hull(&cells));
        let mut backwards_cells = cells;
        backwards_cells.reverse();
        let backwards = derive_skin(&hull(&backwards_cells));
        assert_eq!(forwards, backwards, "the skin depends on insertion order");
        assert_eq!(forwards, derive_skin(&hull(&cells)), "not reproducible");
    }

    /// A one-cell notch fills in to the whole cell.
    ///
    /// Both sides of the gap are structure the skin climbs, so the plate in it
    /// stands the whole cell and closes against both walls: the notch closes
    /// over rather than showing a slot one cell wide.
    #[test]
    fn a_one_cell_notch_fills_in() {
        // A U: floor of three, walls at each end, gap of one in the middle.
        let narrow = hull(&[
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(2, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(2, 1, 0),
        ]);
        let skin = cladding_cells(&narrow);
        let gap = IVec3::new(1, 1, 0);
        assert!(skin.contains(&gap), "the notch is clad");
        let shape = boundary_heights(&narrow, &skin, gap, 2);
        assert_eq!(shape.level, FULL, "`{}` leaves the notch open", shape.id());
        for towards in [IVec3::X, IVec3::NEG_X] {
            assert_eq!(
                shape.edges[edge_towards(2, towards)],
                ShellEdgeType::Wall,
                "`{}` does not close against the wall at {towards}",
                shape.id(),
            );
        }
    }

    /// A one-cell run of skin is a flat slab, folded down both sides.
    ///
    /// The roof of a spine one cell wide meets the plates on its flanks at
    /// half a cell on both long edges. It used to come out as a tent with a
    /// crest down the middle; now its whole top is one level surface, and the
    /// fall happens at the edges where the hull turns.
    #[test]
    fn a_one_cell_run_of_skin_is_a_flat_slab() {
        let structure = hull(&[IVec3::ZERO, IVec3::Z, IVec3::Z * 2]);
        let skin = cladding_cells(&structure);
        let crest = IVec3::Y + IVec3::Z;
        assert!(skin.contains(&crest), "the roof of the spine is clad");

        // Face 2 is +Y, the only way this plate can stand.
        let shape = boundary_heights(&structure, &skin, crest, 2);
        assert_eq!(shape.level, HALF);
        assert_eq!(
            shape.feet,
            [HALF; 4],
            "`{}` falls inside its cell",
            shape.id()
        );
        for towards in [IVec3::X, IVec3::NEG_X] {
            let edge = shape.edges[edge_towards(2, towards)];
            assert!(
                matches!(edge, ShellEdgeType::Fold | ShellEdgeType::Skirt),
                "the flank at {towards} should fold round the hull edge, not {edge:?}",
            );
        }
        assert!(shape.is_coplanar(), "`{}` is not one flat top", shape.id());
    }

    /// A drive standing proud of the hull is not a wall for the skin to climb.
    ///
    /// A cell reads as a wall only where structure OFFERS a mating face in the
    /// skin's own plane. A hull cube does; a drive, which carries one socket
    /// on the end it bolts down through, does not - and reading "filled"
    /// instead put a full-cell fin beside every nozzle.
    #[test]
    fn a_blind_part_standing_proud_is_not_a_wall_the_skin_climbs() {
        // A deck of hull with something standing on the middle of it.
        let cells: Vec<IVec3> = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        let deck = hull(&cells);
        let beside = IVec3::new(0, 1, 1);

        // With a hull cube on the deck, the plate beside it climbs.
        let mut walled = deck.clone();
        walled.insert(IVec3::Y, OPEN);
        let skin = cladding_cells(&walled);
        let shape = boundary_heights(&walled, &skin, beside, 2);
        assert!(
            skin.contains(&beside)
                && shape.level == FULL
                && shape.edges[edge_towards(2, IVec3::NEG_Z)] == ShellEdgeType::Wall,
            "the plate beside a hull block has to close against it: `{}`",
            shape.id(),
        );

        // With a drive there instead, nothing beside it is climbable.
        let mut proud = deck;
        let mut drive = [false; 6];
        drive[3] = true;
        proud.insert(IVec3::Y, drive);
        proud.insert_exit(IVec3::Y, 2);
        let skin = cladding_cells(&proud);
        let shape = boundary_heights(&proud, &skin, beside, 2);
        assert!(
            skin.contains(&beside)
                && shape.level == HALF
                && !shape.edges.contains(&ShellEdgeType::Wall),
            "`{}` grew a fin beside a nozzle there is nothing to close against",
            shape.id(),
        );
    }

    /// A bank of drives leaves the skin between them WHOLE.
    ///
    /// The configuration the owner photographed, in miniature: a stern block
    /// with drives bolted across it. Each drive is blind on five faces and fires
    /// through one of them, so the transom around them is plated and only the
    /// five cells the nozzles fire into stay bare.
    ///
    /// Two shapes must not appear anywhere on this hull, and both did:
    ///
    /// - a WHOLE-CELL plate, which is a plate climbing a drive there is
    ///   nothing to close against. A fin, standing beside a nozzle.
    /// - a plate whose four feet all stand on the floor. A pyramid, one per
    ///   gap in the bank.
    #[test]
    fn the_skin_between_a_bank_of_drives_is_flat_plate() {
        let mut structure = SkinStructure::default();
        for x in 0..4 {
            for y in 0..5 {
                for z in 0..3 {
                    structure.insert(IVec3::new(x, y, z), OPEN);
                }
            }
        }
        // Bolted by -Z and exhausting out of +Z, blind on the other four, in
        // the staggered bank the generator draws on a transom.
        let mut drive = [false; 6];
        drive[5] = true;
        let bank = [
            IVec3::new(1, 4, 3),
            IVec3::new(0, 3, 3),
            IVec3::new(1, 3, 3),
            IVec3::new(1, 2, 3),
            IVec3::new(0, 1, 3),
        ];
        for cell in bank {
            structure.insert(cell, drive);
            structure.insert_exit(cell, 4);
        }

        let plates = derive_skin(&structure);
        let bad: Vec<String> = plates
            .iter()
            .filter(|plate| plate.shape.level == FULL || plate.shape.feet == [0; 4])
            .map(|plate| format!("{:?} wears `{}`", plate.cell, plate.shape.id()))
            .collect();
        assert!(
            bad.is_empty(),
            "a drive is not a wall to climb and a pocket is not open space:\n  {}",
            bad.join("\n  "),
        );

        // The cell in the gap of the bank: plate beside it and drives across
        // its corners. It is plain deck at half a cell.
        let walled_in = plates
            .iter()
            .find(|plate| plate.cell == IVec3::new(2, 1, 3))
            .expect("the gap in the middle of the bank is clad");
        assert_eq!(
            walled_in.shape,
            flat(HALF),
            "`{}` stands in a hole in the skin, not at the edge of one",
            walled_in.shape.id(),
        );
    }

    /// The corners of every triangle of a plate's `role` surfaces, in the
    /// ship's cells.
    fn world_points(plate: &SkinPlate, role: ShellSurface) -> Vec<Vec3> {
        plate
            .shape
            .surfaces()
            .into_iter()
            .filter(|(surface, _)| *surface == role)
            .flat_map(|(_, mesh)| {
                let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
                else {
                    panic!("a generated shell carries float positions");
                };
                positions
            })
            .map(|p| plate.cell.as_vec3() + plate.rotation * Vec3::from_array(p))
            .collect()
    }

    /// The plate a derived skin lays in `cell`.
    fn plate_in(plates: &[SkinPlate], cell: IVec3) -> &SkinPlate {
        plates
            .iter()
            .find(|plate| plate.cell == cell)
            .unwrap_or_else(|| panic!("{cell:?} is clad"))
    }

    /// A staircase of cubes that touch along an edge only is clad as one
    /// continuous 45 degree ramp on each side.
    ///
    /// Each plate in the stair is a whole-cell plate against the step above
    /// it, and it ramps away from that step rather than standing as a block,
    /// so the treads read as one slope and not as a row of teeth.
    #[test]
    fn a_diagonal_stair_gives_a_continuous_ramp() {
        let cells: Vec<IVec3> = (0..4)
            .flat_map(|z| [(0, 0), (1, 1), (2, 2)].map(|(x, y)| IVec3::new(x, y, z)))
            .collect();
        let plates = derive_skin(&hull(&cells));
        // The cells above the stair and the cells under it, and the plane
        // each side's ramp lies in. Away from the ends of the run, so neither
        // plate meets open space along the stair.
        for (treads, offset) in [([(0, 1), (1, 2)], 1.0), ([(1, 0), (2, 1)], -1.0)] {
            for z in 1..3 {
                for (x, y) in treads {
                    let plate = plate_in(&plates, IVec3::new(x, y, z));
                    for point in world_points(plate, ShellSurface::Top) {
                        assert!(
                            (point.y - (point.x + offset)).abs() < EPSILON,
                            "`{}` at {:?} leaves the ramp y = x + {offset} at {point}",
                            plate.shape.id(),
                            plate.cell,
                        );
                    }
                }
            }
        }
    }

    /// Where the hull turns a convex edge, the plates on the two faces meet.
    ///
    /// The roof plate folds down to half a cell at the edge, and the plate on
    /// the side carries a skirt out past its cell that lands on that same line,
    /// so the corner is closed with no slot along it.
    #[test]
    fn the_two_faces_at_a_convex_corner_meet() {
        let cells: Vec<IVec3> = (0..3)
            .flat_map(|x| (0..3).flat_map(move |y| (0..3).map(move |z| IVec3::new(x, y, z))))
            .collect();
        let plates = derive_skin(&hull(&cells));
        let roof = plate_in(&plates, IVec3::new(2, 3, 1));
        let side = plate_in(&plates, IVec3::new(3, 2, 1));
        assert_eq!(roof.rotation * Vec3::Y, Vec3::Y, "the roof plate faces up");

        // The line the two must share: the roof plate's edge over the side of
        // the hull, half a cell above the hull's top.
        let edge_x = 2.5;
        let edge_y = 3.0;
        let on_line = |points: Vec<Vec3>| -> Vec<Vec3> {
            points
                .into_iter()
                .filter(|p| (p.x - edge_x).abs() < EPSILON && p.y > 2.5 + EPSILON)
                .collect()
        };
        for (name, plate) in [("roof", roof), ("side", side)] {
            let landing = on_line(world_points(plate, ShellSurface::Top));
            assert!(
                !landing.is_empty(),
                "the {name} plate never reaches the corner"
            );
            for point in &landing {
                assert!(
                    (point.y - edge_y).abs() < EPSILON,
                    "the {name} plate `{}` reaches the corner at {point}, not on the shared line",
                    plate.shape.id(),
                );
            }
            let (low, high) = landing.iter().fold((f32::MAX, f32::MIN), |(low, high), p| {
                (low.min(p.z), high.max(p.z))
            });
            assert!(
                (low - 0.5).abs() < EPSILON && (high - 1.5).abs() < EPSILON,
                "the {name} plate meets the corner over z {low}..{high}, not the whole cell",
            );
        }
    }

    /// THE OWNER'S L: three cubes in a run, two more up off one end.
    ///
    /// The inside angle can bolt down two ways and both of the cells it would
    /// show are themselves clad. That used to drop it, and left the two hull
    /// faces looking into the angle bare. It stands: the plate on the roof and
    /// the plate on the upright hug different walls of the same concave corner,
    /// and this one fills the corner between them - a whole cell that closes
    /// against the upright and steps down to the deck away from it.
    #[test]
    fn the_inside_angle_of_an_l_is_clad_and_closes_against_the_upright() {
        let structure = hull(&[
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(2, 0, 0),
            IVec3::new(2, 1, 0),
            IVec3::new(2, 2, 0),
        ]);
        let skin = cladding_cells(&structure);
        let angle = IVec3::new(1, 1, 0);
        assert!(skin.contains(&angle), "the inside angle of the L is bare");

        let plate = plate_for(&structure, &skin, angle).expect("a plate fits the angle");
        assert_eq!(
            plate.anchor,
            IVec3::new(1, 0, 0),
            "the corner plate should bolt to the deepest structure under it",
        );
        let shape = boundary_heights(&structure, &skin, angle, 2);
        // The out face is +Y; the upright stands at +x of the angle.
        let against = edge_towards(2, IVec3::X);
        let away = edge_towards(2, IVec3::NEG_X);
        assert_eq!(
            (shape.level, shape.edges[against]),
            (FULL, ShellEdgeType::Wall),
            "`{}` leaves a gap where it meets the upright",
            shape.id(),
        );
        assert_eq!(
            (shape.feet[away], shape.edges[away]),
            (HALF, ShellEdgeType::Plain),
            "`{}` should step down to the deck away from the upright",
            shape.id(),
        );

        // Every hull face of this ship is covered. It is the whole claim: a
        // five-cell L is small enough to check exhaustively.
        for cell in structure.filled_cells() {
            for (face, direction) in FACES.iter().copied().enumerate() {
                let outward = cell + direction;
                assert!(
                    structure.filled(outward) || skin.contains(&outward),
                    "face {face} of {cell:?} is structure looking at vacuum",
                );
            }
        }
    }

    /// Both walls of a two-cell slot are plated.
    ///
    /// The same refusal in its other shape: each side could only ever show the
    /// other, so the pair used to knock each other out and the slot came back
    /// bare on both faces. They are back to back with a gap between them, which
    /// is what a slot looks like.
    #[test]
    fn a_slot_two_cells_wide_is_plated_on_both_of_its_walls() {
        let structure = hull(&[
            IVec3::new(0, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(3, 0, 0),
            IVec3::new(3, 1, 0),
        ]);
        let skin = cladding_cells(&structure);
        for (cell, anchor) in [
            (IVec3::new(1, 0, 0), IVec3::new(0, 0, 0)),
            (IVec3::new(2, 0, 0), IVec3::new(3, 0, 0)),
            (IVec3::new(1, 1, 0), IVec3::new(0, 1, 0)),
            (IVec3::new(2, 1, 0), IVec3::new(3, 1, 0)),
        ] {
            let plate = plate_for(&structure, &skin, cell)
                .unwrap_or_else(|| panic!("{cell:?} is a wall of the slot and came back bare"));
            assert_eq!(plate.anchor, anchor, "{cell:?} plated the far wall");
        }
    }

    /// No plate can be in another plate's way, which is why nothing has to ask.
    ///
    /// The property the one-pass rule rests on, checked over a lumpy hull
    /// rather than argued: a plate is bounded by its own cell and bolts to
    /// structure, so a plate and the plate in the cell it faces always show
    /// different directions and always bolt to different structure. There is no
    /// pair to arbitrate between.
    #[test]
    fn a_plate_and_the_plate_it_faces_never_stand_on_the_same_wall() {
        let structure = hull(&[
            IVec3::ZERO,
            IVec3::X,
            IVec3::X * 2,
            IVec3::Y,
            IVec3::Z,
            IVec3::X + IVec3::Y,
            IVec3::X * 2 + IVec3::Z,
            IVec3::new(1, 1, 1),
            IVec3::new(-1, 0, 0),
            IVec3::new(0, 0, -1),
        ]);
        let plates = derive_skin(&structure);
        let by_cell: HashMap<IVec3, SkinPlate> =
            plates.iter().map(|plate| (plate.cell, *plate)).collect();
        for plate in &plates {
            let out = plate.cell - plate.anchor;
            assert!(
                !structure.filled(plate.cell),
                "{:?} stands inside structure",
                plate.cell,
            );
            let Some(ahead) = by_cell.get(&(plate.cell + out)) else {
                continue;
            };
            assert_ne!(
                ahead.cell - ahead.anchor,
                out,
                "{:?} is buried behind the plate in front of it",
                plate.cell,
            );
            assert_ne!(
                ahead.anchor, plate.anchor,
                "{:?} and {:?} clad the same face of the same section",
                plate.cell, ahead.cell,
            );
        }
    }

    /// A ship root of cube sections, as the scenario spawner builds one.
    fn spawn_ship(app: &mut App, clad: bool, sections: &[(Vec3, Quat)]) -> Entity {
        spawn_styled_ship(app, clad, None, sections)
    }

    /// The same, wearing a style by id.
    fn spawn_styled_ship(
        app: &mut App,
        clad: bool,
        style: Option<&str>,
        sections: &[(Vec3, Quat)],
    ) -> Entity {
        let parts: Vec<(Vec3, Quat, Vec<LinkPoint>)> = sections
            .iter()
            .map(|(position, rotation)| (*position, *rotation, unit_cube_link_points()))
            .collect();
        spawn_styled_parts(app, clad, style, &parts).0
    }

    /// A ship root of sections carrying their own sockets, and the sections in
    /// the order given.
    fn spawn_styled_parts(
        app: &mut App,
        clad: bool,
        style: Option<&str>,
        parts: &[(Vec3, Quat, Vec<LinkPoint>)],
    ) -> (Entity, Vec<Entity>) {
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                ShipSkin(clad),
                ShipStyle(style.map(str::to_string)),
            ))
            .id();
        let sections = parts
            .iter()
            .map(|(position, rotation, link_points)| {
                app.world_mut()
                    .spawn((
                        SectionMarker,
                        SectionLinkPoints(link_points.clone()),
                        Transform::from_translation(*position).with_rotation(*rotation),
                        ChildOf(root),
                    ))
                    .id()
            })
            .collect();
        (root, sections)
    }

    /// Every plate on `root`, paired with the section it hangs off.
    fn plates_of(app: &App, root: Entity) -> Vec<(Entity, Entity)> {
        let world = app.world();
        let sections: Vec<Entity> = world
            .get::<Children>(root)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        sections
            .into_iter()
            .flat_map(|section| {
                world
                    .get::<Children>(section)
                    .map(|children| children.iter().collect::<Vec<_>>())
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|child| world.get::<ShipSkinMarker>(*child).is_some())
                    .map(move |plate| (plate, section))
            })
            .collect()
    }

    /// A ship is clad at spawn by the game, and only if it asked to be.
    ///
    /// The whole point of deriving the skin: nothing authored a plate, the
    /// scenario named a hull and six plates appeared round it. The bare half
    /// matters as much - a hull whose parts are not unit cells would be wrapped
    /// in a skin that fits nothing, so cladding is asked for.
    #[test]
    fn a_clad_ship_grows_its_skin_at_spawn_and_a_bare_one_does_not() {
        let mut app = skin_app();
        let clad = spawn_ship(&mut app, true, &[(Vec3::ZERO, Quat::IDENTITY)]);
        let bare = spawn_ship(&mut app, false, &[(Vec3::ZERO, Quat::IDENTITY)]);
        app.update();

        assert_eq!(
            plates_of(&app, clad).len(),
            6,
            "a lone cube wants a plate on each of its six faces",
        );
        assert!(
            plates_of(&app, bare).is_empty(),
            "a ship that never asked for a skin was clad anyway",
        );
    }

    /// The skin is laid once, not once per frame.
    ///
    /// It is a pure function of the structure, so a second pass would answer
    /// the same plates again and stack a duplicate on every cell - and after
    /// combat it would grow back whatever had been shot off.
    #[test]
    fn the_skin_is_derived_once_and_never_regrows() {
        let mut app = skin_app();
        let ship = spawn_ship(&mut app, true, &[(Vec3::ZERO, Quat::IDENTITY)]);
        app.update();
        let laid = plates_of(&app, ship);
        assert_eq!(laid.len(), 6);

        let (plate, _) = laid[0];
        app.world_mut().entity_mut(plate).despawn();
        app.update();
        app.update();

        assert_eq!(
            plates_of(&app, ship).len(),
            5,
            "a re-derive grew back a plate that combat had taken off",
        );
    }

    /// The cell each plate on `root` stands in, counted, read off the plate's
    /// pose through the section it hangs off. A ship of unit cubes at whole
    /// positions stands on the lattice with no phase.
    fn plate_cells(app: &App, root: Entity) -> std::collections::BTreeMap<[i32; 3], usize> {
        let world = app.world();
        let mut cells = std::collections::BTreeMap::new();
        for (plate, section) in plates_of(app, root) {
            let pose = world
                .get::<Transform>(section)
                .expect("a section has a pose");
            let local = world.get::<Transform>(plate).expect("a plate has a pose");
            let centre = pose.translation + pose.rotation * local.translation;
            *cells
                .entry(centre.round().as_ivec3().to_array())
                .or_insert(0) += 1;
        }
        cells
    }

    /// A section added to a clad ship re-clads the hull round it, and only
    /// there.
    ///
    /// A station add spawns one section into a live ship. The new cell must
    /// lose the plate that stood in it, the plates beside it must take the
    /// shape the grown hull calls for, no plate may be laid twice, and a
    /// plate combat took off away from the add must stay off.
    #[test]
    fn a_section_added_to_a_clad_ship_reclads_round_it_and_nowhere_else() {
        let mut app = skin_app();
        let line = |count: i32| -> Vec<(Vec3, Quat)> {
            (0..count)
                .map(|x| (Vec3::new(x as f32, 0.0, 0.0), Quat::IDENTITY))
                .collect()
        };
        let ship = spawn_ship(&mut app, true, &line(3));
        app.update();
        let shot = plates_of(&app, ship)
            .into_iter()
            .find(|(plate, section)| {
                let world = app.world();
                let pose = world.get::<Transform>(*section).unwrap();
                let local = world.get::<Transform>(*plate).unwrap();
                (pose.translation + pose.rotation * local.translation)
                    .round()
                    .as_ivec3()
                    == IVec3::new(-1, 0, 0)
            })
            .map(|(plate, _)| plate)
            .expect("the aft end of the line is clad");
        app.world_mut().entity_mut(shot).despawn();

        app.world_mut().spawn((
            SectionMarker,
            SectionLinkPoints(unit_cube_link_points()),
            Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
            ChildOf(ship),
        ));
        app.update();

        let mut expected: std::collections::BTreeMap<[i32; 3], usize> =
            derive_skin(&hull(&[0, 1, 2, 3].map(|x| IVec3::new(x, 0, 0))))
                .iter()
                .map(|plate| (plate.cell.to_array(), 1))
                .collect();
        expected.remove(&[-1, 0, 0]);
        assert_eq!(
            plate_cells(&app, ship),
            expected,
            "the grown ship must wear the grown hull's skin once, less the plate shot off",
        );
    }

    /// A section taken off a clad ship re-clads the hull round the hole, and
    /// only there: the faces it bared take plates, and a plate combat took
    /// off away from it stays off.
    #[test]
    fn a_section_removed_from_a_clad_ship_reclads_round_it_and_nowhere_else() {
        let mut app = skin_app();
        app.add_observer(reclad_round_removal);
        let line: Vec<(Vec3, Quat)> = (0..4)
            .map(|x| (Vec3::new(x as f32, 0.0, 0.0), Quat::IDENTITY))
            .collect();
        let ship = spawn_ship(&mut app, true, &line);
        app.update();
        let at = |app: &App, cell: IVec3| {
            plates_of(app, ship).into_iter().find(|(plate, section)| {
                let world = app.world();
                let pose = world.get::<Transform>(*section).unwrap();
                let local = world.get::<Transform>(*plate).unwrap();
                (pose.translation + pose.rotation * local.translation)
                    .round()
                    .as_ivec3()
                    == cell
            })
        };
        let (shot, _) = at(&app, IVec3::new(-1, 0, 0)).expect("the aft end of the line is clad");
        app.world_mut().entity_mut(shot).despawn();
        let (_, end) = at(&app, IVec3::new(4, 0, 0)).expect("the bow end of the line is clad");

        app.world_mut().trigger(SectionRemoveRequest {
            entity: ship,
            section: end,
        });
        app.world_mut().entity_mut(end).despawn();
        app.update();

        let mut expected: std::collections::BTreeMap<[i32; 3], usize> =
            derive_skin(&hull(&[0, 1, 2].map(|x| IVec3::new(x, 0, 0))))
                .iter()
                .map(|plate| (plate.cell.to_array(), 1))
                .collect();
        expected.remove(&[-1, 0, 0]);
        assert_eq!(
            plate_cells(&app, ship),
            expected,
            "the shorter ship must wear the shorter hull's skin once, less the plate shot off",
        );
    }

    /// A plate hangs off the section it clads, at the cell it was derived for.
    ///
    /// Two claims in one, and the second is what the first costs: parenting to
    /// the section buys recursive teardown, and it means the plate's pose has to
    /// be turned into that section's frame. A yawed section is the cheapest way
    /// to catch that turn being applied the wrong way round - with an upright
    /// one, both frames agree and a broken conversion still passes.
    #[test]
    fn a_plate_is_bolted_to_the_section_it_clads() {
        let mut app = skin_app();
        let yaw = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let ship = spawn_ship(&mut app, true, &[(Vec3::ZERO, yaw), (Vec3::X, yaw)]);
        app.update();

        let plates = plates_of(&app, ship);
        assert!(plates.len() > 6, "a two-cell hull has more than one face");
        for (plate, section) in plates {
            let world = app.world();
            let mount = world.get::<Transform>(section).expect("a placed section");
            let local = world.get::<Transform>(plate).expect("a placed plate");
            // The plate's pose in the SHIP's frame, which is the frame the
            // derivation works in and so the one its cell is counted in.
            let cell = mount.translation + mount.rotation * local.translation;
            assert!(
                (cell - cell.round()).length() < EPSILON,
                "the plate sits at {cell}, off the cell grid it was derived on",
            );
            // It clads the section it hangs on: the cell under its own floor.
            let out = mount.rotation * local.rotation * Vec3::Y;
            assert!(
                (cell - out - mount.translation).length() < EPSILON,
                "the plate at {cell} faces {out} and is bolted to the wrong section",
            );
        }
    }

    /// A hull mirrored about its centreline is one ship, not two.
    ///
    /// The cells are READ off the sections, so a ship standing half a cell off
    /// the integers is clad exactly as the same ship standing on them. Get this
    /// wrong and the two halves land in cells with a phantom empty column
    /// between them: they never touch, no plate bridges the seam, and both
    /// skins die to the floor down the spine of every ship.
    #[test]
    fn a_ship_standing_half_a_cell_off_the_integers_is_clad_the_same() {
        let upright = |cells: [Vec3; 2]| cells.map(|cell| (cell, Quat::IDENTITY));
        let mut app = skin_app();
        // Two cubes flank to flank: on the integers, and mirrored about x = 0.
        let integers = spawn_ship(&mut app, true, &upright([Vec3::ZERO, Vec3::X]));
        let mirrored = spawn_ship(&mut app, true, &upright([Vec3::X * 0.5, Vec3::X * -0.5]));
        app.update();

        assert_eq!(
            plates_of(&app, mirrored).len(),
            plates_of(&app, integers).len(),
            "the mirrored hull was clad as two ships with a seam between them",
        );
        for (plate, section) in plates_of(&app, mirrored) {
            let world = app.world();
            let mount = world.get::<Transform>(section).unwrap().translation;
            let cell = mount + world.get::<Transform>(plate).unwrap().translation;
            assert!(
                cell.x.abs() > 0.4,
                "a plate stands at {cell}, inside the seam the two halves share",
            );
        }
    }

    /// A ship wearing a style grows its decoration with its skin, and every
    /// piece hangs off the PLATE it stands on.
    ///
    /// The parenting is the claim worth pinning: it is what makes stripping a
    /// patch of cladding take the greebles on it, and it is one level further
    /// out than the plates' own.
    #[test]
    fn a_styled_ship_grows_decoration_bolted_to_its_plates() {
        let mut app = skin_app();
        app.insert_resource(GameStyles(vec![test_style()]));
        let ship = spawn_styled_ship(
            &mut app,
            true,
            Some("test"),
            &[(Vec3::ZERO, Quat::IDENTITY)],
        );
        app.update();

        let plates = plates_of(&app, ship);
        assert_eq!(plates.len(), 6);
        let world = app.world();
        let decor: Vec<Entity> = plates
            .iter()
            .flat_map(|(plate, _)| {
                world
                    .get::<Children>(*plate)
                    .map(|children| children.iter().collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .filter(|child| world.get::<ShipDecorMarker>(*child).is_some())
            .collect();
        assert_eq!(
            decor.len(),
            6,
            "a rule with nothing filtered takes every plate",
        );
        for piece in &decor {
            assert!(
                world.get::<SectionFixture>(*piece).is_some(),
                "a greeble read as structure by everything downstream",
            );
            assert!(
                world.get::<SectionMarker>(*piece).is_none(),
                "a greeble claiming to be a section joins the integrity graph",
            );
            assert!(
                world.get::<Collider>(*piece).is_some(),
                "a greeble a round passes through is art, not a fixture",
            );
        }

        // The parenting claim: taking a plate off takes its greeble with it.
        let (plate, _) = plates[0];
        app.world_mut().entity_mut(plate).despawn();
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<ShipDecorMarker>>()
                .iter(app.world())
                .count(),
            5,
            "a greeble outlived the plate it was bolted to",
        );
    }

    /// A DEAD-END socket mates like any other, but the skin leaves it bare
    /// while it is open.
    ///
    /// A hull cube with a solar-panel-like part off its +X face, whose two
    /// sockets face +-X. With `Clad` sockets the panel's open end wears a plate
    /// and the plate a greeble; with `Bare` sockets it wears neither, and the
    /// cube's own cladding is the same either way. A second panel mated through
    /// that bare end still joins one connected graph.
    #[test]
    fn an_open_dead_end_socket_gets_no_plate_and_no_greeble_but_still_links() {
        use crate::sections::link_points::{
            prelude::{derive_link_point_graph, LinkPointSkinType},
            PlacedSectionLinkPoints,
        };

        let panel = |skin: LinkPointSkinType| -> Vec<LinkPoint> {
            unit_cube_link_points()
                .into_iter()
                .filter(|point| point.normal.x != 0.0)
                .map(|point| LinkPoint { skin, ..point })
                .collect()
        };
        let cladding = |skin: LinkPointSkinType| {
            let mut app = skin_app();
            app.insert_resource(GameStyles(vec![test_style()]));
            let (ship, sections) = spawn_styled_parts(
                &mut app,
                true,
                Some("test"),
                &[
                    (Vec3::ZERO, Quat::IDENTITY, unit_cube_link_points()),
                    (Vec3::X, Quat::IDENTITY, panel(skin)),
                ],
            );
            app.update();
            let world = app.world();
            let plates = plates_of(&app, ship);
            let on = |section: Entity| -> (usize, usize) {
                let mine: Vec<Entity> = plates
                    .iter()
                    .filter(|(_, owner)| *owner == section)
                    .map(|(plate, _)| *plate)
                    .collect();
                let decor = mine
                    .iter()
                    .flat_map(|plate| {
                        world
                            .get::<Children>(*plate)
                            .map(|children| children.iter().collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .filter(|child| world.get::<ShipDecorMarker>(*child).is_some())
                    .count();
                (mine.len(), decor)
            };
            (on(sections[0]), on(sections[1]))
        };

        let (cube, end) = cladding(LinkPointSkinType::Clad);
        assert_eq!(
            cube.0, 5,
            "the cube is clad on every face but the mated one"
        );
        assert_eq!(
            end,
            (1, 1),
            "a clad open end wears one plate and its greeble"
        );

        let (bare_cube, bare_end) = cladding(LinkPointSkinType::Bare);
        assert_eq!(
            bare_cube, cube,
            "a dead end changed the cladding of its neighbour"
        );
        assert_eq!(bare_end, (0, 0), "a dead-end socket was clad or decorated");

        let first = panel(LinkPointSkinType::Bare);
        let second = panel(LinkPointSkinType::Bare);
        let cube_points = unit_cube_link_points();
        let placed: Vec<PlacedSectionLinkPoints> = [
            (Vec3::ZERO, cube_points.as_slice()),
            (Vec3::X, first.as_slice()),
            (Vec3::X * 2.0, second.as_slice()),
        ]
        .into_iter()
        .map(|(position, link_points)| PlacedSectionLinkPoints {
            position,
            rotation: Quat::IDENTITY,
            link_points,
        })
        .collect();
        let mates = derive_link_point_graph(&placed)
            .expect("a part attached through a dead end joins the graph");
        assert!(
            mates.iter().any(|mate| {
                (mate.a.section_index, mate.b.section_index) == (1, 2)
                    && first[mate.a.link_point_index].skin == LinkPointSkinType::Bare
            }),
            "the second panel did not mate through the bare end: {mates:?}",
        );
    }

    /// A ship naming a style nothing authored is clad and BARE, rather than
    /// wearing whatever style happened to load first.
    #[test]
    fn an_unknown_style_leaves_a_ship_undecorated() {
        let mut app = skin_app();
        app.insert_resource(GameStyles(vec![test_style()]));
        let ship = spawn_styled_ship(
            &mut app,
            true,
            Some("nothing_authored_this"),
            &[(Vec3::ZERO, Quat::IDENTITY)],
        );
        app.update();

        assert_eq!(plates_of(&app, ship).len(), 6, "the skin is still derived");
        assert_eq!(
            app.world_mut()
                .query_filtered::<(), With<ShipDecorMarker>>()
                .iter(app.world())
                .count(),
            0,
            "an unknown style must not fall back to another look",
        );
    }

    /// A style whose one fixture takes nearly every plate, for the spawn-path
    /// tests.
    fn test_style() -> ShipStyleConfig {
        crate::sections::skin_style::block_style(Vec3::new(0.2, 0.05, 0.2))
    }

    /// The render half dresses a plate the moment it appears.
    ///
    /// Kept apart from the body on purpose: a plate is spawned by gameplay with
    /// no mesh anywhere, and this is the observer a rendering host adds on top.
    #[test]
    fn the_render_half_gives_a_plate_one_mesh_per_surface() {
        let mut app = skin_app();
        app.init_resource::<SkinAssets>();
        app.add_observer(dress_skin_plate);
        let section = app.world_mut().spawn(SectionMarker).id();
        let clad = flat(HALF);
        let plate = spawn_plate_under(&mut app, section, clad);
        app.update();

        let world = app.world();
        let meshes: Vec<Entity> = world
            .get::<Children>(plate)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        assert_eq!(
            meshes.len(),
            clad.surfaces().len(),
            "a plate wants one child per surface role, each with its own material",
        );
        for mesh in meshes {
            assert!(
                world.get::<Mesh3d>(mesh).is_some(),
                "a surface with no mesh"
            );
            assert!(
                world
                    .get::<MeshMaterial3d<StandardMaterial>>(mesh)
                    .is_some(),
                "a surface with no material",
            );
        }
    }
}
