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
//! 2. [`boundary_heights`] reads the eight samples a cell's outward face
//!    carries, off that cell's neighbourhood.
//! 3. [`plate_for`] turns those samples into a [`ShellShape`] and the rotation
//!    that draws them.
//!
//! Every sample is SHARED with the cells around it, so neighbours read the same
//! numbers and their surfaces meet exactly. That is why nothing has to generate
//! a seam between two plates in the same plane: there is no gap to fill.
//!
//! [`spawn_ship_skin`] is where that meets a live ship: a hull marked
//! [`ShipSkin`] is read back into a [`SkinStructure`] at spawn, and every plate
//! the derivation answers with is bolted to the section it clads.

use std::collections::BTreeSet;

use avian3d::prelude::Collider;
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use nova_gameplay::prelude::{
    destructible_body, HealthZeroMarker, IntegritySystems, SectionMarker, SpaceshipRootMarker,
};

use crate::sections::{
    fixture::prelude::SectionFixture,
    integrity::build_ship_integrity_graph,
    link_points::prelude::{LinkPoint, SectionLinkPoints},
    shell_shape::{ShellShape, ShellSurface, FULL, HALF, REACH},
    skin_decor::{
        decor_body, decor_pose, decor_reach, decor_tally, dress_skin_decor, scatter_decor,
        ShipDecorMarker,
    },
    skin_reading::{read_plates, relief_tally},
    skin_style::{GameStyles, ShipStyle, ShipStyleConfig},
};

/// The prelude: `SkinStructure`, `SkinPlate`, `derive_skin`, `read_structure`,
/// `plate_body`, `SkinAssets`, `ShipSkin`, `ShipSkinMarker` and
/// `ShipSkinPlugin`.
pub mod prelude {
    pub use super::{
        derive_skin, plate_body, read_structure, ShipSkin, ShipSkinMarker, ShipSkinPlugin,
        SkinAssets, SkinPlate, SkinStructure,
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
const PLACEMENT_SNAP: f32 = 4096.0;

/// A ship's structure, as the skin needs to see it.
///
/// Only two facts per filled cell: that it is filled, and which of its six
/// faces carry a link-point socket once the section is rotated. That is enough,
/// and deliberately less than a section - the skin has no business knowing what
/// a cell holds, only what it presents.
#[derive(Default, Clone, Debug)]
pub struct SkinStructure {
    cells: HashMap<IVec3, [bool; 6]>,
}

impl SkinStructure {
    /// A cell filled by a section presenting `faces`, indexed as [`FACES`].
    pub fn insert(&mut self, cell: IVec3, faces: [bool; 6]) {
        self.cells.insert(cell, faces);
    }

    /// A cell filled by a section whose sockets point along `normals`, already
    /// turned into the ship's frame.
    ///
    /// Sockets are ADDED to whatever the cell already presents, and a normal
    /// that points between two faces is dropped. Both cases are the same one: a
    /// cell is a bucket, not a section, and two half-size parts standing either
    /// side of a boundary land in one. What the skin needs to know is what the
    /// cell as a whole turns outward.
    pub fn insert_section(&mut self, cell: IVec3, normals: impl IntoIterator<Item = Vec3>) {
        let faces = self.cells.entry(cell).or_insert([false; 6]);
        for normal in normals {
            if let Some(face) = face_index(normal) {
                faces[face] = true;
            }
        }
    }

    /// Whether a cell holds structure.
    pub fn filled(&self, cell: IVec3) -> bool {
        self.cells.contains_key(&cell)
    }

    /// Whether the section in `cell` offers a socket on its `face`, or `None`
    /// if the cell is empty.
    pub(crate) fn offers(&self, cell: IVec3, face: usize) -> Option<bool> {
        self.cells.get(&cell).map(|faces| faces[face])
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
/// Two things do. A neighbouring section may turn a BLIND face to the cell - a
/// mount's barrel, a bay's muzzle - and nothing may be built against one of
/// those, which is what leaves the gun wells and the muzzle mouths open without
/// either being named here. And a plate has to bolt down to structure on one
/// face while showing vacuum on the opposite one, so a cell wedged between
/// sections on every axis has nowhere to stand; those are dropped repeatedly
/// until none are left, since dropping a cell only ever frees its neighbours to
/// be the vacuum they needed.
pub fn cladding_cells(structure: &SkinStructure) -> HashSet<IVec3> {
    let mut skin: HashSet<IVec3> = HashSet::new();
    for cell in structure.filled_cells() {
        for face in 0..FACES.len() {
            let candidate = cell + FACES[face];
            if structure.filled(candidate) {
                continue;
            }
            let touches_socket = (0..FACES.len())
                .any(|f| structure.offers(candidate + FACES[f], f ^ 1) == Some(true));
            if touches_socket && !blind_pocket(structure, candidate) {
                skin.insert(candidate);
            }
        }
    }

    loop {
        let doomed: Vec<IVec3> = skin
            .iter()
            .copied()
            .filter(|cell| !(0..FACES.len()).any(|out| stands(structure, &skin, *cell, out)))
            .collect();
        if doomed.is_empty() {
            break;
        }
        for cell in doomed {
            skin.remove(&cell);
        }
    }
    skin
}

/// Whether some section turns a BLIND face into this cell, which is what keeps
/// the cladding out of it.
///
/// A gun well and the mouth of a drive bay are both this and nothing else: a
/// mount presents five faces with nothing to mate and a drive presents five,
/// and no plate may be laid against one of them. Read by [`cladding_cells`] to
/// refuse the cell, and by [`boundary_heights`] to tell a pocket like this from
/// open space - the skin ends against a pocket, and tapers away into open
/// space.
pub(super) fn blind_pocket(structure: &SkinStructure, cell: IVec3) -> bool {
    (0..FACES.len()).any(|face| structure.offers(cell + FACES[face], face ^ 1) == Some(false))
}

/// Whether a plate in `cell` may show `out` to space.
///
/// It BOLTS DOWN to structure through the face opposite its out face, and shows
/// vacuum through the out face itself. Both halves matter. Without the first,
/// cladding hangs off other cladding with nothing solid to carry it - and
/// worse, it samples its boundary in a plane none of its neighbours agreed to,
/// so the surface it draws need not meet theirs. Without the second it is not
/// the last thing before space, which is the one job a skin has.
fn stands(structure: &SkinStructure, skin: &HashSet<IVec3>, cell: IVec3, out: usize) -> bool {
    let bolted = structure.offers(cell + FACES[out ^ 1], out) == Some(true);
    let ahead = cell + FACES[out];
    bolted && !structure.filled(ahead) && !skin.contains(&ahead)
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

/// The eight boundary samples a cell's `out` face carries.
///
/// Every one is SHARED, and every one is a function of the cells that share it:
///
/// - a CORNER belongs to the four cells that meet at it, and reads the coarse
///   heights only. A WALL in any of those four means the plating has to reach
///   the top of the cell to close against it - see [`walls`], which is choosier
///   than "there is something there". Otherwise the corner stands at the half
///   cell a skin runs at if every one of the four is something the surface ENDS
///   AGAINST, and dies to the floor if any of them is open space.
///
///   That last distinction is the whole of the rim. A skin that ends because it
///   ran out of hull TAPERS away, which is what leaves a ridge along a spine and
///   a stud on a lone cell. A skin that ends against a POCKET a blind face keeps
///   it out of - a gun well, the mouth of a drive bay - stops dead at the cell
///   boundary and its own wall drops vertically there, which is the ridge round
///   the well. Tapering into those was what filled a bank of drives with
///   pyramids: the cells between the nozzles are surrounded by pockets, so every
///   sample they carried was on the floor and each came out a cone.
///
///   Holding a rim corner up IN PROPORTION to how many of its cells are clad
///   was tried and reverted: it makes every edge LIVE, so the dead-edge vote
///   below never fires, and a one-cell run flattens from a ridge into a low
///   strip. See the task notes.
/// - a MIDPOINT belongs to the two cells across the edge, and on a LIVE edge is
///   simply the mean of its two corners - the height the straight line between
///   them passes the middle at. Quarter cells are in the alphabet so that mean
///   always lands on a real sample. Only a DEAD edge, with both corners on the
///   floor, reads the neighbourhood itself.
///
/// Because each sample is a function of its own set and nothing else, every
/// cell around it arrives at the same number, and two plates meet exactly.
pub fn boundary_heights(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
    out: usize,
) -> ShellShape {
    let (u, v) = face_plane(out);
    let height = |set: &[IVec3]| -> u8 {
        if set.iter().any(|next| walls(structure, *next, u, v)) {
            FULL
        } else if set.iter().all(|next| ends_against(structure, skin, *next)) {
            HALF
        } else {
            0
        }
    };

    let corners = std::array::from_fn(|slot| {
        let (su, sv) = FACE_CORNERS[slot];
        let (step_u, step_v) = (step(u, su), step(v, sv));
        height(&[cell, cell + step_u, cell + step_v, cell + step_u + step_v])
    });
    let midpoints = std::array::from_fn(|slot| {
        let (near, far) = (corners[slot], corners[(slot + 1) % 4]);
        if near > 0 || far > 0 {
            return (near + far) / 2;
        }
        let (au, av) = FACE_CORNERS[slot];
        let (bu, bv) = FACE_CORNERS[(slot + 1) % 4];
        let across = step(u, (au + bu) / 2) + step(v, (av + bv) / 2);
        height(&[cell, cell + across])
    });

    ShellShape { corners, midpoints }
}

/// Whether the skin has something to END AGAINST at `cell`, rather than the
/// open space it tapers away into.
///
/// Three ways, and they are one claim: the surface cannot carry on past here.
/// The cell is CLAD, so the surface simply continues into it. It holds
/// STRUCTURE, which the skin either climbs ([`walls`]) or stops at, and either
/// way it is not vacuum. Or it is a POCKET a blind face keeps the cladding out
/// of, which is a gun well or the mouth of a drive bay - a hole in the skin
/// rather than the end of it.
pub(super) fn ends_against(structure: &SkinStructure, skin: &HashSet<IVec3>, cell: IVec3) -> bool {
    skin.contains(&cell) || structure.filled(cell) || blind_pocket(structure, cell)
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
fn walls(structure: &SkinStructure, cell: IVec3, u: usize, v: usize) -> bool {
    [u, v]
        .iter()
        .any(|axis| (0..2).any(|side| structure.offers(cell, axis * 2 + side) == Some(true)))
}

/// The plate a clad cell calls for.
///
/// The one genuine choice is WHICH WAY it faces, where more than one direction
/// stands. That goes to the deepest structure the plate can reach: a plate on
/// the roof finds the whole ship below it and one turned on its side finds a
/// single cell, so the flat answer wins by a wide margin and a wall does not
/// come out as a field of pyramids.
///
/// The rotation is FOUND rather than derived: for each turn that puts the out
/// face the right way, the samples the cell wants are read back into the
/// shape's own frame, and the turn that leaves that reading canonical is the
/// one that places the canonical shape correctly.
pub fn plate_for(
    structure: &SkinStructure,
    skin: &HashSet<IVec3>,
    cell: IVec3,
) -> Option<SkinPlate> {
    let out = (0..FACES.len())
        .filter(|out| stands(structure, skin, cell, *out))
        .max_by_key(|out| support_depth(structure, cell, *out))?;
    let wanted = boundary_heights(structure, skin, cell, out);

    for rotation in turns() {
        if face_index(rotation * Vec3::Y) != Some(out) {
            continue;
        }
        let mut local = ShellShape {
            corners: [0; 4],
            midpoints: [0; 4],
        };
        let mut legible = true;
        for slot in 0..4 {
            let (x, z) = FACE_CORNERS[slot];
            let corner = corner_slot(out, rotation * Vec3::new(x as f32, 0.0, z as f32));
            let edge = edge_slot(out, rotation * local_edge(slot));
            match (corner, edge) {
                (Some(corner), Some(edge)) => {
                    local.corners[slot] = wanted.corners[corner];
                    local.midpoints[slot] = wanted.midpoints[edge];
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
/// semantic craft (the racer, the haulers) are modelled parts of their own sizes
/// standing at half cells, and bucketing those into a unit grid would wrap them
/// in a skin that fits nothing. Until a hull can say which of the two it is, the
/// ships that want cladding ask for it.
#[derive(Component, Clone, Copy, Default, Debug, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct ShipSkin(pub bool);

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

/// Derive a freshly spawned ship's skin and bolt it on.
///
/// Runs on the SPAWN BATCH, the same `Added<SectionLinkPoints>` edge the
/// integrity graph is built off: every section of a ship lands in one run of
/// commands, so the batch is complete the first time this sees it and a ship is
/// clad exactly once. Nothing re-derives afterwards, which is what lets combat
/// take plates off and keeps them off - the skin is a pure function of the
/// structure, and re-running it would grow back whatever was shot away.
///
/// Each plate is parented to the SECTION IT CLADS rather than to the ship root.
/// That is what buys a destroyed section taking its own cladding down with it,
/// for free, and it is why the pose has to be turned into the section's frame
/// here: the derivation works in cells of the ship.
///
/// Decoration is scattered in the same pass, from the ship's [`ShipStyle`]. It
/// hangs off the PLATE rather than the section, one level further out for the
/// same reason: a plate shot off takes its greebles with it, which is what makes
/// stripping a patch of skin read as stripping a patch of skin.
fn spawn_ship_skin(
    mut commands: Commands,
    q_added: Query<&ChildOf, (With<SectionMarker>, Added<SectionLinkPoints>)>,
    q_clad: Query<(&ShipSkin, Option<&ShipStyle>), With<SpaceshipRootMarker>>,
    q_children: Query<&Children>,
    q_sections: Query<(&Transform, &SectionLinkPoints), With<SectionMarker>>,
    styles: Option<Res<GameStyles>>,
) {
    let roots: BTreeSet<Entity> = q_added
        .iter()
        .map(|ChildOf(root)| *root)
        .filter(|root| q_clad.get(*root).is_ok_and(|(skin, _)| skin.0))
        .collect();

    for root in roots {
        let Ok(children) = q_children.get(root) else {
            continue;
        };
        let mut entities: Vec<Entity> = Vec::new();
        let mut placed: Vec<(Transform, &[LinkPoint])> = Vec::new();
        for section in children.iter() {
            let Ok((transform, link_points)) = q_sections.get(section) else {
                continue;
            };
            entities.push(section);
            placed.push((*transform, link_points.as_slice()));
        }
        let (structure, phase, cells) = read_structure(&placed);
        if structure.is_empty() {
            continue;
        }
        let sections: HashMap<IVec3, (Entity, Transform)> = cells
            .into_iter()
            .zip(entities)
            .zip(placed)
            .map(|((cell, entity), (transform, _))| (cell, (entity, transform)))
            .collect();

        let mut shapes: HashSet<ShellShape> = HashSet::new();
        let plates = derive_skin(&structure);
        // Index-aligned with `plates`, so the scatter's answers can be looked
        // up by the index it gives them back. `None` where a plate was skipped.
        let mut laid: Vec<Option<Entity>> = Vec::with_capacity(plates.len());
        for plate in &plates {
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

        let style = q_clad
            .get(root)
            .ok()
            .and_then(|(_, style)| style)
            .zip(styles.as_ref())
            .and_then(|(worn, styles)| worn.resolve(styles));
        let readings = read_plates(&structure, &plates);
        let mut taken: Vec<usize> = vec![0; style.map_or(0, |style| style.fixtures.len())];
        if let Some(style) = style {
            for placement in scatter_decor(&plates, &readings, style) {
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
        // interesting half of the look - a style author tuning a rule needs to
        // know what a real hull actually offers, and guessing at it from a
        // screenshot is how a rule ends up firing everywhere or nowhere.
        //
        // Each fixture is logged as TAKEN of REACH: what it actually got, and
        // what it would have got with nothing above it in the priority list. The
        // gap between the two is the starvation a screenshot cannot show, and
        // the reach is only computed when this line would be printed.
        debug!(
            "spawn_ship_skin: ship {root} clad in {} plate(s) over {} shape(s); \
             relief {}; decoration {}",
            laid.iter().flatten().count(),
            shapes.len(),
            relief_tally(&readings),
            style.map_or_else(
                || "none (no style)".to_string(),
                |style| decor_tally(style, &taken, &decor_reach(&plates, &readings, style)),
            ),
        );
    }
}

/// Read a set of placed sections as the structure the skin sees: the lattice
/// they stand on, and the cell each one fills, in the order they were given.
///
/// ONE reading, shared by everything that clads: the spawner reads a live ship's
/// sections, the editor reads the build state plus the part under the pointer.
/// Two readings would put the lattice in two places and clad the same ship two
/// ways - and the editor's whole claim is that what it shows is what flies.
///
/// `placed` pairs a section's pose in the ship's frame with the sockets it
/// carries in its OWN frame; the turn into the ship's frame happens here.
pub fn read_structure(placed: &[(Transform, &[LinkPoint])]) -> (SkinStructure, Vec3, Vec<IVec3>) {
    let phase = lattice_phase(placed.iter().map(|(pose, _)| pose.translation));
    let mut structure = SkinStructure::default();
    let mut cells = Vec::with_capacity(placed.len());
    for (pose, link_points) in placed {
        let cell = section_cell(pose.translation, phase);
        structure.insert_section(
            cell,
            link_points.iter().map(|point| pose.rotation * point.normal),
        );
        cells.push(cell);
    }
    (structure, phase, cells)
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
pub(crate) fn section_cell(position: Vec3, phase: Vec3) -> IVec3 {
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
                        let dress = style.and_then(|style| style.surface(role));
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
/// The style is found by walking UP from the plate, the same way `damage_tint`
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
    for (mesh, material) in assets.surfaces(*shape, style, &mut meshes, &mut materials) {
        commands.spawn((
            Name::new("Skin Surface"),
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
/// is a few centimetres of a cell. The box is also exactly `volume` cells of
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

/// Take a fixture off the ship the moment its health runs out.
///
/// Structure is destroyed through the integrity graph, and a fixture is
/// deliberately not in it, so without this a dead plate would sit there at zero
/// health still stopping rounds. There is no wreck and no husk: a plate coming
/// off IS the damage read, and what it uncovers is the hull behind it.
fn despawn_dead_fixtures(
    mut commands: Commands,
    q_dead: Query<Entity, (With<SectionFixture>, With<HealthZeroMarker>)>,
) {
    for fixture in &q_dead {
        // `try_despawn`: the section a fixture hangs on can die the same frame
        // and take its children with it before this command lands.
        commands.entity(fixture).try_despawn();
    }
}

/// Clads a ship at spawn, dresses the plates it lays, and sweeps the dead ones.
///
/// SPLIT down the middle, and the split is not where a look/no-look reading
/// would put it. A plate carries a collider, health and mass: a round has to
/// stop on one and a shot-off plate has to leave a hole whether or not anybody
/// is watching, so DERIVING the skin and SWEEPING dead plates are gameplay and
/// run on a headless server too. `render` gates the meshes and their materials
/// and nothing else.
#[derive(Default, Clone, Debug)]
pub struct ShipSkinPlugin {
    /// Whether plates are given the meshes that draw them.
    pub render: bool,
}

impl Plugin for ShipSkinPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ShipSkin>();
        app.register_type::<ShipSkinMarker>();
        app.register_type::<ShipStyle>();
        app.register_type::<ShipDecorMarker>();
        app.register_type::<SectionFixture>();
        // Empty until the mod merge fills it, so a headless test or a scenario
        // with no styles authored still resolves (to nothing) rather than
        // panicking on a missing resource.
        app.init_resource::<GameStyles>();
        app.add_systems(
            Update,
            (
                // After the graph so structure is settled before it is dressed,
                // and before the integrity set so a plate is never spawned onto
                // a section the same frame that set takes it away.
                spawn_ship_skin
                    .after(build_ship_integrity_graph)
                    .before(IntegritySystems),
                despawn_dead_fixtures,
            ),
        );

        if self.render {
            app.init_resource::<SkinAssets>();
            app.add_observer(dress_skin_plate);
            app.add_observer(dress_skin_decor);
        }
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::{ColliderDensity, ComputeMassProperties3d};
    use nova_gameplay::prelude::{
        Health, HealthApplyDamage, HealthIsolated, NovaHealthPlugin, SectionMarker,
    };

    use super::*;
    use crate::sections::link_points::unit_cube_link_points;

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
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), NovaHealthPlugin));
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.add_systems(Update, (spawn_ship_skin, despawn_dead_fixtures));
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

    fn shape(corners: [u8; 4], midpoints: [u8; 4]) -> ShellShape {
        ShellShape::new(corners, midpoints).expect("a legal shape")
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
        let clad = shape([HALF; 4], [HALF; 4]);
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
    fn a_full_plate_outweighs_and_outlasts_a_quarter_one() {
        let mut app = skin_app();
        let section = app.world_mut().spawn(SectionMarker).id();
        let block = shape([FULL; 4], [FULL; 4]);
        let quarter = shape([1; 4], [1; 4]);
        assert_eq!(block.volume(), 1.0, "a full block fills its cell");
        assert_eq!(
            quarter.volume(),
            0.25,
            "a quarter-cell plate fills a quarter"
        );

        let solid = spawn_plate_under(&mut app, section, block);
        let thin = spawn_plate_under(&mut app, section, quarter);

        let health_of = |plate: Entity| app.world().get::<Health>(plate).unwrap().max;
        assert_eq!(health_of(solid), SKIN_HEALTH_PER_CELL);
        assert!(
            (health_of(solid) / health_of(thin) - 4.0).abs() < EPSILON,
            "a full plate should outlast a quarter one four to one",
        );

        assert!(
            (plate_mass(&app, solid) - SKIN_DENSITY).abs() < EPSILON,
            "a full cell of plate weighs one cell of skin",
        );
        assert!(
            (plate_mass(&app, solid) / plate_mass(&app, thin) - 4.0).abs() < 1e-3,
            "a full plate should outweigh a quarter one four to one",
        );
    }

    /// A plate whose health runs out comes off the ship.
    ///
    /// Nothing else would take it: destruction reaches structure through the
    /// integrity graph, and a fixture is deliberately not in it. The section
    /// behind survives, which is what makes losing skin a different event from
    /// losing a part.
    #[test]
    fn a_plate_at_zero_health_comes_off_and_leaves_its_section() {
        let mut app = skin_app();
        let section = app
            .world_mut()
            .spawn((SectionMarker, Health::new(500.0)))
            .id();
        let plate = spawn_plate_under(&mut app, section, shape([HALF; 4], [HALF; 4]));

        let health = app.world().get::<Health>(plate).unwrap().max;
        app.world_mut().trigger(HealthApplyDamage {
            entity: plate,
            source: None,
            amount: health,
        });
        app.update();

        assert!(
            app.world().get_entity(plate).is_err(),
            "a dead plate is still bolted to the hull",
        );
        assert!(
            app.world().get_entity(section).is_ok(),
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
        let plate = spawn_plate_under(&mut app, section, shape([FULL; 4], [FULL; 4]));
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

    /// The skin never claims a cell a blind face points into.
    ///
    /// This is what leaves a gun well open without anything naming guns: a
    /// mount presents five faces with no socket, and nothing may be built
    /// against one of those.
    #[test]
    fn a_blind_face_keeps_the_skin_off_it() {
        let mut structure = hull(&[IVec3::ZERO]);
        // A mount above the cube, bolted down by its base (-Y) and blind on
        // everything else.
        let mut mount = [false; 6];
        mount[3] = true;
        structure.insert(IVec3::Y, mount);

        let plates = derive_skin(&structure);
        let clad: Vec<IVec3> = plates.iter().map(|plate| plate.cell).collect();
        for face in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z, IVec3::Y * 2] {
            assert!(
                !clad.contains(&(IVec3::Y + face)),
                "the skin closed over the mount's blind {face:?} face",
            );
        }
    }

    /// Two plates across a seam read the same samples.
    ///
    /// This is the property the whole scheme rests on, so it is checked over
    /// real neighbours rather than argued: every pair of clad cells that share
    /// a face and face the same way must agree about the two corners and the
    /// midpoint they hold in common.
    #[test]
    fn neighbouring_plates_agree_about_the_samples_they_share() {
        let structure = hull(&[
            IVec3::ZERO,
            IVec3::X,
            IVec3::X * 2,
            IVec3::Z,
            IVec3::X + IVec3::Z,
        ]);
        let skin = cladding_cells(&structure);
        assert!(!skin.is_empty(), "this hull has an outside");

        for cell in skin.iter().copied() {
            for out in 0..FACES.len() {
                if !stands(&structure, &skin, cell, out) {
                    continue;
                }
                let mine = boundary_heights(&structure, &skin, cell, out);
                let (u, v) = face_plane(out);
                for (axis, sign) in [(u, 1), (u, -1), (v, 1), (v, -1)] {
                    let next = cell + step(axis, sign);
                    if !skin.contains(&next) || !stands(&structure, &skin, next, out) {
                        continue;
                    }
                    let theirs = boundary_heights(&structure, &skin, next, out);
                    // The shared edge: my slot facing them, their slot facing
                    // me. The midpoint is one sample, so the two must match.
                    let mid_of = |shape: &ShellShape, towards: IVec3| -> u8 {
                        let slot = (0..4)
                            .find(|slot| {
                                let (au, av) = FACE_CORNERS[*slot];
                                let (bu, bv) = FACE_CORNERS[(slot + 1) % 4];
                                step(u, (au + bu) / 2) + step(v, (av + bv) / 2) == towards
                            })
                            .expect("an edge faces every neighbour in the plane");
                        shape.midpoints[slot]
                    };
                    assert_eq!(
                        mid_of(&mine, step(axis, sign)),
                        mid_of(&theirs, step(axis, -sign)),
                        "{cell:?} and {next:?} disagree about the midpoint between them",
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

    /// A one-cell notch fills in; a two-cell notch reads as a valley.
    ///
    /// The resolution limit of a corner-sampled height field, pinned so it is a
    /// known property rather than a surprise. All four corners of a one-cell
    /// gap touch structure, so the only shape that fits is the full block and
    /// the notch closes over. Widen it and the middle samples drop to half.
    #[test]
    fn a_notch_needs_two_cells_before_it_reads_as_concave() {
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
        let plate = plate_for(&narrow, &skin, gap).expect("a plate fits the notch");
        assert_eq!(
            plate.shape.corners, [FULL; 4],
            "a one-cell notch has structure at every corner, so it fills in",
        );
    }

    /// A one-cell run of skin is a RIDGE, not a low strip.
    ///
    /// A rim that faces OPEN SPACE tapers away, and this is what that buys: a
    /// run one cell wide has two of the four cells clad at every corner, so
    /// every corner dies to the floor, every edge is DEAD, and the midpoint vote
    /// is free to run the crest along the run. A tent, and the owner wants the
    /// tents - it is only where the skin ends against something that it has to
    /// stop dead instead.
    #[test]
    fn a_one_cell_run_of_skin_reads_as_a_ridge() {
        // A spine three cells long: the skin over its roof is one cell wide.
        let structure = hull(&[IVec3::ZERO, IVec3::Z, IVec3::Z * 2]);
        let skin = cladding_cells(&structure);
        let crest = IVec3::Y + IVec3::Z;
        assert!(skin.contains(&crest), "the roof of the spine is clad");

        // Face 2 is +Y, the only way this plate can stand.
        let shape = boundary_heights(&structure, &skin, crest, 2);
        assert_eq!(
            shape.corners, [0; 4],
            "a corner of a one-cell run faces open space on two sides, so it \
             tapers away rather than standing up",
        );
        let along = [shape.midpoints[0], shape.midpoints[2]];
        let across = [shape.midpoints[1], shape.midpoints[3]];
        let (crest_line, eaves) = match along {
            [HALF, HALF] => (along, across),
            _ => (across, along),
        };
        assert_eq!(
            crest_line, [HALF; 2],
            "the run should carry a crest at half a cell down its length",
        );
        assert_eq!(
            eaves, [0; 2],
            "the sides of the run should fall to the floor, or it is a slab",
        );
    }

    /// A drive standing proud of the hull is not a wall for the skin to climb.
    ///
    /// A corner reads the whole cell only where structure OFFERS a mating face
    /// in the skin's own plane. A hull cube does; a drive, which carries one
    /// socket on the end it bolts down through, does not - and reading "filled"
    /// instead put a full-cell fin on the plate diagonally off every nozzle.
    #[test]
    fn a_blind_part_standing_proud_is_not_a_wall_the_skin_climbs() {
        // A deck of hull with something standing on the middle of it.
        let cells: Vec<IVec3> = (-1..=1)
            .flat_map(|x| (-1..=1).map(move |z| IVec3::new(x, 0, z)))
            .collect();
        let deck = hull(&cells);
        let corner = IVec3::new(1, 1, 1);

        // With a hull cube on the deck, the plate diagonally off it climbs.
        let mut walled = deck.clone();
        walled.insert(IVec3::Y, OPEN);
        let skin = cladding_cells(&walled);
        let shape = boundary_heights(&walled, &skin, corner, 2);
        assert!(
            skin.contains(&corner) && shape.corners.contains(&FULL),
            "the plate beside a hull block has to close against it: `{}`",
            shape.id(),
        );

        // With a drive there instead, nothing on that corner is climbable.
        let mut proud = deck;
        let mut drive = [false; 6];
        drive[3] = true;
        proud.insert(IVec3::Y, drive);
        let skin = cladding_cells(&proud);
        let shape = boundary_heights(&proud, &skin, corner, 2);
        assert!(
            skin.contains(&corner) && !shape.corners.contains(&FULL),
            "`{}` grew a fin beside a nozzle there is nothing to close against",
            shape.id(),
        );
    }

    /// A bank of drives leaves the skin between them WHOLE.
    ///
    /// The configuration the owner photographed, in miniature: a stern block
    /// with drives bolted across it. Each drive is blind on five faces, so the
    /// cells around it are refused cladding and the skin that survives between
    /// them is in narrow strips and lone cells, walled in by those pockets.
    ///
    /// Two shapes must not appear anywhere on this hull, and both did:
    ///
    /// - a corner at the WHOLE CELL, which is a plate climbing a drive there is
    ///   nothing to close against. A fin, standing beside a nozzle.
    /// - a boundary entirely on the floor, whose middle rides at half a cell.
    ///   A CONE, one per gap in the bank.
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
        // Bolted by -Z, blind everywhere else, in the staggered bank the
        // generator draws on a transom.
        let mut drive = [false; 6];
        drive[5] = true;
        for cell in [
            IVec3::new(1, 4, 3),
            IVec3::new(0, 3, 3),
            IVec3::new(1, 3, 3),
            IVec3::new(1, 2, 3),
            IVec3::new(0, 1, 3),
        ] {
            structure.insert(cell, drive);
        }

        let plates = derive_skin(&structure);
        let cone = ShellShape::new([0; 4], [0; 4]).expect("a legal shape");
        let bad: Vec<String> = plates
            .iter()
            .filter(|plate| plate.shape.corners.contains(&FULL) || plate.shape == cone)
            .map(|plate| format!("{:?} wears `{}`", plate.cell, plate.shape.id()))
            .collect();
        assert!(
            bad.is_empty(),
            "a drive is not a wall to climb and a pocket is not open space:\n  {}",
            bad.join("\n  "),
        );

        // The cell the bank walls in on every side: pockets beside it, drives
        // across its corners. It carries the running height right out to the
        // boundary and drops vertically there, which is the ridge.
        let walled_in = plates
            .iter()
            .find(|plate| plate.cell == IVec3::new(2, 1, 3))
            .expect("the gap in the middle of the bank is clad");
        assert_eq!(
            walled_in.shape,
            ShellShape::new([HALF; 4], [HALF; 4]).expect("a legal shape"),
            "`{}` stands in a hole in the skin, not at the edge of one",
            walled_in.shape.id(),
        );
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
        let root = app
            .world_mut()
            .spawn((
                SpaceshipRootMarker,
                ShipSkin(clad),
                ShipStyle(style.map(str::to_string)),
            ))
            .id();
        for (position, rotation) in sections {
            app.world_mut().spawn((
                SectionMarker,
                SectionLinkPoints(unit_cube_link_points()),
                Transform::from_translation(*position).with_rotation(*rotation),
                ChildOf(root),
            ));
        }
        root
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

    /// A style whose fixtures cover every plate, for the spawn-path tests.
    fn test_style() -> ShipStyleConfig {
        ShipStyleConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            surfaces: Vec::new(),
            fixtures: vec![crate::sections::skin_style::StyleFixtureConfig {
                id: "block".to_string(),
                model: nova_gameplay::prelude::AssetRef::from(
                    "self://gltf/greebles/placeholder_block.glb#Scene0".to_string(),
                ),
                health: 10.0,
                density: 0.1,
                collider: Vec3::new(0.2, 0.1, 0.2),
                scatter: default(),
            }],
        }
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
        let clad = shape([HALF; 4], [HALF; 4]);
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
