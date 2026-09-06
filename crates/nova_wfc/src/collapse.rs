//! The solve, and the passes that turn a legal tiling into a hull worth
//! looking at.
//!
//! The collapse says what may sit next to what. Everything else here says what
//! a HULL is allowed to look like once it has: the studs come off, whatever
//! cannot fire where it stands comes off, the dents are packed, and what is
//! left hanging off the keel is the ship.

use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;
use nova_ship::prelude::{
    blocked_exits, exit_lanes, BlockedExit, BlockedExitReason, GrammarKeel, GrammarVacuum,
    GrammarZone, ShipExit, SkinStructure,
};
use rand::{rngs::StdRng, RngExt, SeedableRng};

use crate::{
    grid::{Grid, FACES},
    tiles::{compatible, mirror_cell, seam_allows, upright_tile, Family, ShipCell, Tile, VACUUM},
};

/// How many neighbours a section wants before it counts as part of a hull
/// rather than as a stud on one.
const SPIKE_SUPPORT: usize = 3;
/// How many solid faces a hole needs before it stops being a feature.
const FILL_SUPPORT: usize = 3;

/// One collapse in progress: the tiles, the block they are laid in, and the
/// grammar's taste about where the result may be sparse.
pub(crate) struct Collapse<'a> {
    pub(crate) tiles: &'a [Tile],
    pub(crate) families: &'a [Family],
    pub(crate) grid: Grid,
    /// The row the keel is laid along.
    pub(crate) keel_row: usize,
    pub(crate) vacuum: GrammarVacuum,
    pub(crate) keel: &'a GrammarKeel,
}

impl Collapse<'_> {
    /// Draw weight of vacuum in one cell of the STRUCTURAL grid: the taper that
    /// shapes the silhouette. Pure taste - it decides WHERE the generator may
    /// be sparse, never WHAT may sit next to what.
    fn vacuum_weight(&self, cell: usize) -> f32 {
        let (x, y, z) = self.grid.coords(cell);
        let length = self.grid.size.z as usize;
        let off_keel = x as f32 + (y as f32 - self.keel_row as f32).abs();
        let toward_bow = (length - 1 - z) as f32 / (length - 1) as f32;
        let stern = if z == length - 1 {
            self.vacuum.stern
        } else {
            0.0
        };
        self.vacuum.base
            * (1.0 + self.vacuum.taper * off_keel + self.vacuum.bow_taper * toward_bow + stern)
    }

    /// Whether a tile points the way its part is allowed to point
    /// ([`Family::aim`]).
    ///
    /// Which way a part points is the PART's business, not one cell's, so this
    /// reads [`Tile::aims`] rather than the cell's own clearance: a big drive
    /// carries an exit on its exhaust face alone, and the rest of it points
    /// the same way regardless.
    ///
    /// A UNARY constraint, like [`seam_allows`] beside it, and the only thing
    /// in this crate that knows a ship has a BACK. [`compatible`] is binary: it
    /// sees a cell and its neighbour, and a drive bolted to the roof is as
    /// legal to it as one bolted to the transom. Which way a drive fires is not
    /// a free choice either - it carries one socket, on its forward end, so the
    /// face it is bolted to IS the direction it exhausts.
    ///
    /// Free, and safe by construction. Striking options out of an OPENING
    /// domain is the standard way to add a unary constraint to a constraint
    /// solve, and it cannot empty a domain here because [`VACUUM`] is
    /// compatible with everything and is never struck. There is no backtracking
    /// in this collapse and none is needed.
    fn aim_allowed(&self, tile: &Tile) -> bool {
        tile.family
            .and_then(|family| self.families[family].aim)
            .is_none_or(|aim| tile.aims == Some(aim))
    }

    /// Whether a tile stands in the region its part is allowed to stand in
    /// ([`Family::zone`]).
    ///
    /// A UNARY constraint like [`Self::aim_allowed`] above it, and free for the
    /// same reason. It rules on one CELL, which is what gives a multi-cell part
    /// the "every cell inside" reading the content type promises: each segment
    /// of a block is its own tile in its own cell and each is filtered here, so
    /// a three-cell lance zoned `Bow` can only stand where all three of its
    /// cells are forward.
    ///
    /// Thirds along the hull, halves either side of the keel row, and the
    /// outboard half of the starboard grid. The keel ROW itself is neither
    /// dorsal nor ventral: it is the spine, not a flank.
    fn zone_allowed(&self, cell: usize, tile: &Tile) -> bool {
        let Some(zone) = tile.family.and_then(|family| self.families[family].zone) else {
            return true;
        };
        let (x, y, z) = self.grid.coords(cell);
        let (width, length) = (self.grid.size.x as usize, self.grid.size.z as usize);
        match zone {
            GrammarZone::Bow => z * 3 < length,
            GrammarZone::Amidships => z * 3 >= length && z * 3 < length * 2,
            GrammarZone::Stern => z * 3 >= length * 2,
            GrammarZone::Dorsal => y > self.keel_row,
            GrammarZone::Ventral => y < self.keel_row,
            GrammarZone::Flank => x * 2 >= width,
        }
    }

    /// The structural pass's opening domains: structure only, the seam, the aim
    /// and the zone ruled on as unary constraints, every other grid edge
    /// treated as the vacuum it is.
    fn domains(&self) -> Vec<Vec<bool>> {
        (0..self.grid.cells())
            .map(|cell| {
                let on_seam = self.grid.coords(cell).0 == 0;
                (0..self.tiles.len())
                    .map(|index| {
                        if !self.aim_allowed(&self.tiles[index]) {
                            return false;
                        }
                        if !self.zone_allowed(cell, &self.tiles[index]) {
                            return false;
                        }
                        if on_seam && !seam_allows(&self.tiles[index]) {
                            return false;
                        }
                        (0..FACES.len()).all(|face| {
                            (face == 1 && on_seam)
                                || self.grid.neighbour(cell, face).is_some()
                                || compatible(self.tiles, index, face, VACUUM)
                        })
                    })
                    .collect()
            })
            .collect()
    }

    /// Lay the keel: collapse the spine by hand, before the generator gets a
    /// say.
    ///
    /// Hull the whole way, with the flight computer forward of centre where a
    /// bridge reads. Taste, and one guarantee - a seeded keel is one connected
    /// structure to grow on, so a ship is never a cloud of islands.
    ///
    /// It stops short at BOTH ends, and those rows belong to the seeds either
    /// side of it. Aft, [`Self::seed_stern`]: the drive standing beside the
    /// keel turns a blind flank at the seam, and a keel cube there would press
    /// a socket into it. Forward, [`Self::seed_bow`], which stands its gun IN
    /// this column rather than beside it.
    ///
    /// The keel is mirrored like everything else, so a ship carries a symmetric
    /// PAIR of the computer. One would do (it is the ship's heart, not a
    /// resource) and two are no worse.
    fn seed_keel(&self, domains: &mut [Vec<bool>]) -> Result<(), String> {
        let length = self.grid.size.z as usize;
        let bow = self.bow_span()?.z as usize;
        let stern = self.stern_span()?.z as usize;
        // Forward of centre, but never inside the bow gun, and never past the
        // last cell the keel actually reaches.
        let bridge = (length / 3).clamp(bow, length - stern - 1);
        for z in bow..length - stern {
            let cell = self.grid.index(0, self.keel_row, z);
            let prototype = if z == bridge {
                &self.keel.bridge
            } else {
                &self.keel.hull
            };
            let keel = upright_tile(self.tiles, prototype)
                .ok_or_else(|| format!("'{prototype}' cannot stand on the grid at all"))?;
            if !domains[cell][keel] {
                return Err(format!(
                    "a keel section sits ON the centreline, so it has to be able to meet its \
                     own reflection: '{prototype}' cannot"
                ));
            }
            assign(domains, cell, keel);
        }
        Ok(())
    }

    /// Stand the SPINAL GUN on the bow end of the keel, which the mirror makes
    /// a tight pair either side of the centreline.
    ///
    /// Seeded rather than rolled for, for the reason the stern drive is: a gun
    /// the whole ship aims fires down its own axis, and `erode_blocked_exits`
    /// takes off anything whose lane is not clear. A lane that long is only
    /// ever clear at an END of the hull - and the bow is the sparsest part of
    /// the grid, where a drawn gun has almost nothing to mate to.
    ///
    /// Standing it in the keel COLUMN is what makes the lane free: the gun's
    /// muzzle cell is the bow-most row, so the lane in front of it leaves the
    /// grid and there is nothing there to block. Nothing is carved, unlike the
    /// bench stamp this replaces.
    ///
    /// A hull with no spinal gun seeds nothing here, and the keel starts at
    /// `z = 0` exactly as it did before.
    fn seed_bow(&self, domains: &mut [Vec<bool>]) -> Result<(), String> {
        let Some(prototype) = self.keel.bow_gun.as_deref() else {
            return Ok(());
        };
        let gun = upright_tile(self.tiles, prototype)
            .ok_or_else(|| format!("'{prototype}' cannot stand on the grid at all"))?;
        let cell = self.grid.index(0, self.keel_row, 0);
        if !domains[cell][gun] {
            return Err(format!(
                "the bow gun is seeded before anything is drawn, so '{prototype}' has to be \
                 able to stand on the bow of the keel, meeting its own reflection: it cannot"
            ));
        }
        assign(domains, cell, gun);
        Ok(())
    }

    /// How many cells the seeded bow gun spans, upright, or zero for a hull
    /// plan that seats none.
    fn bow_span(&self) -> Result<UVec3, String> {
        let Some(prototype) = self.keel.bow_gun.as_deref() else {
            return Ok(UVec3::ZERO);
        };
        upright_tile(self.tiles, prototype)
            .map(|tile| self.tiles[tile].span)
            .ok_or_else(|| format!("'{prototype}' cannot stand on the grid at all"))
    }

    /// Lay the DRIVE DECK: a plate the size of the drive's mount face, and the
    /// drive bolted to its aft face, which the mirror makes a pair either side
    /// of the centreline.
    ///
    /// The drive is seeded rather than rolled for because it has nowhere else
    /// to be. It wants its exhaust face clear to the stern and its four flanks
    /// blind, which the collapse can only stumble on: a one-cell drive found
    /// the transom about a tenth of the time, and a 3x3x2 one - nine lanes to
    /// keep clear at once - was drawn into the middle of the hull and eroded
    /// again every seed. What the roll then fills is the transom AROUND the
    /// drives, which is the part worth rolling for.
    ///
    /// A drive may not stand ON the centreline: its sockets are on its forward
    /// face, so it has no `-x` socket to meet its own reflection with
    /// ([`seam_allows`]). So the block stands one cell off, and the keel has
    /// already stopped short to leave the seam beside it free.
    ///
    /// The block is seeded by its own MINIMUM CORNER - the one segment that
    /// emits - and propagation lays the rest of it out. A domain that empties
    /// while it does is a grammar whose drive does not fit its grid, which
    /// [`crate::runnable`] has already refused.
    fn seed_stern(&self, domains: &mut [Vec<bool>]) -> Result<(), String> {
        let (length, height) = (self.grid.size.z as usize, self.grid.size.y as usize);
        let span = self.stern_span()?;
        let (across, tall, deep) = (span.x as usize, span.y as usize, span.z as usize);
        // Centred on the keel row, and slid back inside a grid too short to
        // centre it in.
        let bottom = (self.keel_row + 1).saturating_sub(tall.div_ceil(2));
        let bottom = bottom.min(height - tall);

        let deck = upright_tile(self.tiles, &self.keel.stern_deck)
            .ok_or_else(|| format!("'{}' cannot stand on the grid at all", self.keel.stern_deck))?;
        for x in 1..=across {
            for y in bottom..bottom + tall {
                let cell = self.grid.index(x, y, length - deep - 1);
                if !domains[cell][deck] {
                    return Err(format!(
                        "the drive deck is seeded before anything is drawn, so '{}' has to \
                         stand there",
                        self.keel.stern_deck
                    ));
                }
                assign(domains, cell, deck);
            }
        }

        let drive = upright_tile(self.tiles, &self.keel.stern_drive).ok_or_else(|| {
            format!(
                "'{}' cannot stand on the grid at all",
                self.keel.stern_drive
            )
        })?;
        let corner = self.grid.index(1, bottom, length - deep);
        if !domains[corner][drive] {
            return Err(format!(
                "the drive deck is seeded before anything is drawn, so '{}' has to stand there",
                self.keel.stern_drive
            ));
        }
        assign(domains, corner, drive);
        Ok(())
    }

    /// How many cells the seeded stern drive spans, upright.
    fn stern_span(&self) -> Result<UVec3, String> {
        upright_tile(self.tiles, &self.keel.stern_drive)
            .map(|tile| self.tiles[tile].span)
            .ok_or_else(|| {
                format!(
                    "'{}' cannot stand on the grid at all",
                    self.keel.stern_drive
                )
            })
    }

    /// Collapse the grid: one tile index per cell.
    ///
    /// `vacuum_weight` prices emptiness per cell; it is taste, and it cannot
    /// make an illegal placement legal, because the domain it draws from was
    /// already filtered by the rule.
    fn solve(&self, mut domains: Vec<Vec<bool>>, seed: u64) -> Result<Vec<usize>, String> {
        let mut rng = StdRng::seed_from_u64(seed);
        self.propagate(&mut domains, (0..self.grid.cells()).collect());

        while let Some(cell) = lowest_entropy(&domains, &mut rng) {
            let pick = self.draw(&domains[cell], cell, &mut rng);
            assign(&mut domains, cell, pick);
            self.propagate(&mut domains, VecDeque::from([cell]));
        }

        domains
            .iter()
            .enumerate()
            .map(|(cell, domain)| {
                domain
                    .iter()
                    .position(|allowed| *allowed)
                    .ok_or_else(|| format!("cell {cell} collapsed to nothing"))
            })
            .collect()
    }

    /// Arc-consistency: strike from every neighbour the options nothing left in
    /// this cell can sit beside, and follow the change outward.
    fn propagate(&self, domains: &mut [Vec<bool>], mut pending: VecDeque<usize>) {
        while let Some(cell) = pending.pop_front() {
            for face in 0..FACES.len() {
                let Some(next) = self.grid.neighbour(cell, face) else {
                    continue;
                };
                let mut changed = false;
                for candidate in 0..self.tiles.len() {
                    if !domains[next][candidate] {
                        continue;
                    }
                    let supported = (0..self.tiles.len()).any(|here| {
                        domains[cell][here] && compatible(self.tiles, here, face, candidate)
                    });
                    if !supported {
                        domains[next][candidate] = false;
                        changed = true;
                    }
                }
                if changed {
                    pending.push_back(next);
                }
            }
        }
    }

    /// Draw one of a cell's remaining options, by weight.
    ///
    /// A weight is authored per PART and spent per part: whatever orientations
    /// of it are still standing in this cell SHARE it. That keeps the authored
    /// number meaning "how often do I want one of these, where one is allowed"
    /// - immune both to how symmetric a part happens to be (the hull cube
    /// presents one tile, the bay six) and to how many of its orientations this
    /// particular cell has already ruled out. Without it a mount, which by its
    /// single socket has at most one legal orientation anywhere, would be
    /// outvoted six to one by parts that merely have more ways to sit.
    fn draw(&self, domain: &[bool], cell: usize, rng: &mut StdRng) -> usize {
        let mut standing = vec![0.0f32; self.families.len()];
        for tile in (0..self.tiles.len()).filter(|index| domain[*index]) {
            if let Some(family) = self.tiles[tile].family {
                standing[family] += 1.0;
            }
        }
        let weight = |index: usize| match self.tiles[index].family {
            Some(family) => self.families[family].weight / standing[family],
            None => self.vacuum_weight(cell),
        };
        let standing_here = || (0..self.tiles.len()).filter(|index| domain[*index]);
        let total: f32 = standing_here().map(weight).sum();
        // A weighted draw needs a positive finite total to roll inside. The
        // grammar gate rejects the tables that would make one zero, but the
        // number reaching here has been divided by a per-cell orientation
        // count, so this is the last place to notice - and `rand` PANICS on an
        // empty range. Falling back to a flat draw keeps the collapse running
        // on a cell whose taste cancelled out, which is a duller ship rather
        // than a dead game.
        if !total.is_finite() || total <= 0.0 {
            let options = standing_here().count();
            return standing_here()
                .nth(rng.random_range(0..options.max(1)))
                .unwrap_or(VACUUM);
        }
        let mut roll = rng.random_range(0.0..total);
        let mut last = VACUUM;
        for index in standing_here() {
            roll -= weight(index);
            if roll <= 0.0 {
                return index;
            }
            last = index;
        }
        last
    }

    /// Take a cell off the ship, and with it every other cell of the same
    /// multi-cell part: half a bay is not a part, and the cell a dead half
    /// leaves behind would be packed with hull INSIDE the surviving half's
    /// body.
    fn drop_part(&self, chosen: &[usize], kept: &mut [bool], cell: usize) {
        if !kept[cell] {
            return;
        }
        kept[cell] = false;
        for face in 0..FACES.len() {
            let Some(next) = self.grid.neighbour(cell, face) else {
                continue;
            };
            if self.tiles[chosen[cell]].joints[face] == Some(chosen[next]) {
                self.drop_part(chosen, kept, next);
            }
        }
    }

    /// Erode the studs off the structure.
    ///
    /// This is here because of the SKIN. A hull cell hanging off the ship by
    /// one face is a stud, and the coverage rule then wraps it in five plates -
    /// which is a pyramid standing on a pin. Cladding does not hide a lumpy
    /// hull, it draws a hard outline around one, so the lumps are taken off
    /// before the skin goes on rather than dressed afterwards.
    ///
    /// What counts as enough is read off the section, not authored: a part is
    /// asked for two neighbours or for as many as it has sockets, whichever is
    /// fewer. A PDC mount carries one socket and so is never eroded for having
    /// one neighbour; a six-socket hull cube standing on one is.
    ///
    /// Erosion only removes, and a removal can only lower a neighbour's count,
    /// so it settles. It can strand a limb, which is why
    /// [`Self::keel_component`] runs after.
    fn erode_studs(&self, chosen: &[usize], kept: &mut [bool]) {
        loop {
            let mut changed = false;
            for cell in 0..self.grid.cells() {
                let (x, y, _) = self.grid.coords(cell);
                if !kept[cell] || (x == 0 && y == self.keel_row) {
                    continue;
                }
                let tile = &self.tiles[chosen[cell]];
                if !tile.is_solid() {
                    continue;
                }
                // A cell on the centreline meets its own reflection there, and
                // the seam rule already guarantees that mate.
                let mut neighbours = usize::from(x == 0);
                for face in 0..FACES.len() {
                    if let Some(next) = self.grid.neighbour(cell, face) {
                        neighbours +=
                            usize::from(kept[next] && self.tiles[chosen[next]].is_solid());
                    }
                }
                let wanted = tile.faces.iter().filter(|open| **open).count();
                if neighbours < wanted.min(SPIKE_SUPPORT) {
                    self.drop_part(chosen, kept, cell);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Fill the pits.
    ///
    /// The other half of [`Self::erode_studs`], and there for the same reason.
    /// A one-cell dent in the hull is a hole the skin then has to line with
    /// five plates, and lining dents is how a hull ends up looking like a
    /// sponge. Any empty cell with [`FILL_SUPPORT`] solid faces around it is
    /// packed with hull instead - cheaper in sections, and far cheaper to look
    /// at.
    ///
    /// A cell that any section turns a BLIND face to is never filled. Those are
    /// the gun wells and the muzzle mouths, and they are dents on purpose: the
    /// same sentence that keeps the skin out of them keeps the filler out too.
    /// Filling is otherwise always legal - a hull cube offers a socket on all
    /// six faces, and every neighbour it meets was checked to offer one back.
    fn fill_pits(
        &self,
        chosen: &mut [usize],
        kept: &mut [bool],
        lanes: &HashSet<IVec3>,
    ) -> Result<(), String> {
        let hull = upright_tile(self.tiles, &self.keel.hull)
            .ok_or_else(|| format!("'{}' cannot stand on the grid at all", self.keel.hull))?;
        loop {
            let mut changed = false;
            for cell in 0..self.grid.cells() {
                if kept[cell] && self.tiles[chosen[cell]].is_solid() {
                    continue;
                }
                // Filling is the one pass that ADDS structure, so it is the one
                // that can block a lane after the lanes have been cleared. A
                // hull cube offers a socket on all six faces, so packing a dent
                // BESIDE a lane demands cladding inside it - which is the same
                // conflict as packing the lane itself.
                let (x, y, z) = self.grid.coords(cell);
                let here = IVec3::new(x as i32, y as i32, z as i32);
                if lanes.contains(&here)
                    || FACES
                        .iter()
                        .any(|face| lanes.contains(&(here + face.as_ivec3())))
                {
                    continue;
                }
                // A cell on the centreline is backed by its own reflection.
                let mut solid = usize::from(x == 0);
                let mut blind = false;
                for face in 0..FACES.len() {
                    let Some(next) = self.grid.neighbour(cell, face) else {
                        continue;
                    };
                    if !kept[next] || !self.tiles[chosen[next]].is_solid() {
                        continue;
                    }
                    solid += 1;
                    blind |= !self.tiles[chosen[next]].faces[face ^ 1];
                }
                if !blind && solid >= FILL_SUPPORT {
                    chosen[cell] = hull;
                    kept[cell] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(())
    }

    /// Keep only the structure hanging off the keel.
    ///
    /// The collapse can leave an island: a mount whose one socket faced a cell
    /// that ended up empty, or a pocket of hull cut off by vacuum. Those are
    /// legal tiles and an ILLEGAL ship - `derive_link_point_graph` rejects a
    /// disconnected section graph outright - so they are dropped here rather
    /// than linted about later. Dropping solids only ever turns a mate into an
    /// exposed socket, which no rule forbids, so what survives is still valid.
    fn keel_component(&self, chosen: &[usize], standing: &[bool]) -> Vec<bool> {
        let mut kept = vec![false; self.grid.cells()];
        let start = self.grid.index(0, self.keel_row, 0);
        let mut pending = VecDeque::from([start]);
        kept[start] = true;
        while let Some(cell) = pending.pop_front() {
            for face in 0..FACES.len() {
                let Some(next) = self.grid.neighbour(cell, face) else {
                    continue;
                };
                // MATING, not adjacency. Two solids may now sit face to face
                // without either carrying a socket there - a drive's flank
                // against a mount's housing - and a pair like that is touching,
                // not joined. Walking plain adjacency would call such a piece
                // attached and leave it in the hull for the graph check to find
                // floating. A JOINT is joined by definition: the two cells are
                // one body.
                let joined = (self.tiles[chosen[cell]].faces[face]
                    && self.tiles[chosen[next]].faces[face ^ 1])
                    || self.tiles[chosen[cell]].joints[face] == Some(chosen[next]);
                if !kept[next] && standing[next] && self.tiles[chosen[next]].is_solid() && joined {
                    kept[next] = true;
                    pending.push_back(next);
                }
            }
        }
        kept
    }

    /// The collapsed half mirrored into the whole ship, in cells.
    ///
    /// The cells are the ones the SKIN will bucket the spawned sections into:
    /// the starboard half stands at `x + 0.5` and the port half at `-x - 0.5`,
    /// so on a lattice with a half-cell phase they come out as `x` and `-1 - x`.
    ///
    /// Clearance has to be judged on the whole ship rather than on the half
    /// that was collapsed, because a lane pointing at the centreline crosses it
    /// - and what is waiting on the far side is the part's own reflection,
    /// firing back.
    pub(crate) fn ship_cells(&self, chosen: &[usize], kept: &[bool]) -> Vec<ShipCell<'_>> {
        let mut cells = Vec::new();
        for cell in 0..self.grid.cells() {
            if !kept[cell] || !self.tiles[chosen[cell]].is_solid() {
                continue;
            }
            let tile = &self.tiles[chosen[cell]];
            let Some(family) = tile.family else {
                continue;
            };
            let (x, y, z) = self.grid.coords(cell);
            let starboard = ShipCell {
                cell: IVec3::new(x as i32, y as i32, z as i32),
                prototype: self.families[family].prototype.as_str(),
                faces: tile.faces,
                exit: tile.exit,
            };
            let port = mirror_cell(&starboard);
            cells.push(starboard);
            cells.push(port);
        }
        cells
    }

    /// The half-grid cell a mirrored one is a copy of.
    fn starboard_of(&self, cell: IVec3) -> usize {
        let x = if cell.x < 0 { -1 - cell.x } else { cell.x };
        self.grid
            .index(x as usize, cell.y as usize, cell.z as usize)
    }

    /// Take off the parts that cannot fire, launch or exhaust where they stand.
    ///
    /// The non-local half of clearance, and it has to be a PASS rather than a
    /// rule because no rule between two neighbouring cells can see a lane. It
    /// belongs beside [`Self::erode_studs`] for the same reason that one does:
    /// the collapse says what may sit next to what, and these two say what a
    /// hull is allowed to look like once it has.
    ///
    /// The PART goes, never the hull in front of it. Dropping a solid only ever
    /// turns a mate into an exposed socket, which no rule forbids, and dropping
    /// the hull instead would open a lane by gutting the ship it is bolted to.
    /// One part per pass, so a bay blocked only by another bay is not taken off
    /// with it.
    ///
    /// Removal cannot create work: taking a part away deletes its lane and
    /// frees everyone else's, so this settles.
    fn erode_blocked_exits(&self, chosen: &[usize], kept: &mut [bool]) {
        loop {
            let (structure, exits) = ship_lattice(&self.ship_cells(chosen, kept));
            let Some(blocked) = blocked_exits(&structure, &exits)
                .into_iter()
                .min_by_key(|blocked| self.starboard_of(blocked.part))
            else {
                return;
            };
            self.drop_part(chosen, kept, self.starboard_of(blocked.part));
        }
    }

    /// Run one seed to a pruned, smoothed structure: the chosen tile per cell
    /// and which cells survived.
    ///
    /// Prune, then smooth: take the studs off, take off whatever cannot fire
    /// where it stands, pack the dents, and prune again because erosion can
    /// strand whatever a stud was holding on.
    ///
    /// Clearing the lanes BEFORE the dents are packed is what keeps the hull
    /// solid: the cell a blocked bay is taken out of is a dent like any other,
    /// and packing it afterwards leaves hull rather than a hole for the skin to
    /// line. The filler is handed the surviving lanes so it cannot undo them.
    pub(crate) fn run(&self, seed: u64) -> Result<(Vec<usize>, Vec<bool>), String> {
        let mut domains = self.domains();
        self.seed_keel(&mut domains)?;
        self.seed_stern(&mut domains)?;
        self.seed_bow(&mut domains)?;
        let mut chosen = self.solve(domains, seed)?;

        let mut kept = self.keel_component(&chosen, &vec![true; self.grid.cells()]);
        self.erode_studs(&chosen, &mut kept);
        self.erode_blocked_exits(&chosen, &mut kept);
        let (structure, exits) = ship_lattice(&self.ship_cells(&chosen, &kept));
        let lanes: HashSet<IVec3> = exit_lanes(&structure, &exits).into_iter().collect();
        self.fill_pits(&mut chosen, &mut kept, &lanes)?;
        let kept = self.keel_component(&chosen, &kept);

        // A check, not the fix: `compatible` and `erode_blocked_exits` are what
        // make it pass. A generator that can draw a bay firing into its own
        // hull has to REFUSE the seed rather than hand one out.
        //
        // No test drives this arm, and the record says so rather than pretending
        // otherwise: a sweep of 7,200 seeds over 120 bent grammars - vacuum
        // priced from 0.001 to 4, every part from featherweight to twenty, four
        // grid shapes, with and without a seeded bow gun - reached it never.
        // Erosion drops a blocked part before this reads, and the filler is
        // handed the surviving lanes so it cannot put one back. Reachable only
        // if one of those two stops holding, which is exactly when it earns its
        // keep.
        let cells = self.ship_cells(&chosen, &kept);
        let blocked = blocked_findings(&cells);
        if !blocked.is_empty() {
            return Err(format!(
                "seed {seed}: the collapse blocked its own exits:\n  {}",
                blocked.join("\n  ")
            ));
        }
        Ok((chosen, kept))
    }
}

fn assign(domains: &mut [Vec<bool>], cell: usize, tile: usize) {
    for (index, allowed) in domains[cell].iter_mut().enumerate() {
        *allowed = index == tile;
    }
}

/// The next cell to observe: fewest options left, ties broken by the roll.
fn lowest_entropy(domains: &[Vec<bool>], rng: &mut StdRng) -> Option<usize> {
    let mut fewest = usize::MAX;
    let mut candidates = Vec::new();
    for (cell, domain) in domains.iter().enumerate() {
        let options = domain.iter().filter(|allowed| **allowed).count();
        if options < 2 {
            continue;
        }
        if options < fewest {
            fewest = options;
            candidates.clear();
        }
        if options == fewest {
            candidates.push(cell);
        }
    }
    (!candidates.is_empty()).then(|| candidates[rng.random_range(0..candidates.len())])
}

/// The mirrored ship as `nova_ship`'s clearance rule reads it: what each cell
/// presents, and the exits those cells carry.
///
/// The RULE is not here. It is `nova_ship::sections::clearance`, which the
/// editor refuses placements with, so a hull this generator cannot draw is also
/// a hull a player cannot build. What is here is only the reading of a ship
/// that has been collapsed onto a grid and mirrored.
fn ship_lattice(cells: &[ShipCell]) -> (SkinStructure, Vec<ShipExit>) {
    let mut structure = SkinStructure::default();
    for placed in cells {
        structure.insert(placed.cell, placed.faces);
        if let Some(out) = placed.exit {
            structure.insert_exit(placed.cell, out);
        }
    }
    let exits = cells
        .iter()
        .filter_map(|placed| {
            Some(ShipExit {
                cell: placed.cell,
                out: placed.exit?,
            })
        })
        .collect();
    (structure, exits)
}

/// Every blocked exit on a mirrored ship, spelled with the prototypes standing
/// in it.
fn blocked_findings(cells: &[ShipCell]) -> Vec<String> {
    let (structure, exits) = ship_lattice(cells);
    blocked_exits(&structure, &exits)
        .iter()
        .map(|blocked| finding(cells, blocked))
        .collect()
}

/// One finding, spelled with the prototypes standing in it.
fn finding(cells: &[ShipCell], blocked: &BlockedExit) -> String {
    let named = |cell: IVec3| -> &str {
        cells
            .iter()
            .find(|placed| placed.cell == cell)
            .map(|placed| placed.prototype)
            .unwrap_or("vacuum")
    };
    match blocked.reason {
        BlockedExitReason::Structure => format!(
            "'{}' at {:?} fires into '{}' at {:?}",
            named(blocked.part),
            blocked.part,
            named(blocked.blocker),
            blocked.lane,
        ),
        BlockedExitReason::Cladding => format!(
            "'{}' at {:?} fires, and '{}' at {:?} wants cladding in the lane at {:?}",
            named(blocked.part),
            blocked.part,
            named(blocked.blocker),
            blocked.blocker,
            blocked.lane,
        ),
    }
}
