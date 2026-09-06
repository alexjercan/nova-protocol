//! The wave-function-collapse ship generator: a catalog and a grammar in, a
//! hull nobody drew out.
//!
//! # What the rule IS
//!
//! The adjacency rule is the CATALOG's own link points. A prototype earns a
//! place on the grid by its sockets alone, and two cells may sit face to face
//! exactly when the sockets they turn to each other agree - so a mod that ships
//! a section already changes what may be built, with nothing here knowing about
//! it. Clearance is read off a part's kind the same way: a nozzle, a muzzle and
//! a bay's mouth all need their lane, and the rule that says so is
//! `nova_ship`'s, the one the editor refuses placements with. A hull this
//! generator cannot draw is a hull a player cannot build.
//!
//! What the catalog cannot carry is TASTE - how often a part should be offered,
//! which one part is only allowed to point one way, how big and how sparse a
//! hull is. That is the [`ShipGrammarConfig`], authored as content beside the
//! sections it draws from.
//!
//! # What comes out
//!
//! A [`ShipHull`] of catalog prototypes: exactly what an authored ship carries,
//! so a generated hull is content and not a special case. It can be spawned,
//! saved into a mod bundle, or lifted into the editor's document and edited by
//! hand.
//!
//! Nothing here decides who flies a hull or which side it fights for. The
//! generator makes ships; a scenario makes a fight.
//!
//! # Failing rather than photographing
//!
//! Every entry point returns `Result`. A grammar naming a part the catalog does
//! not hold, a keel role that cannot meet its own reflection, a seed whose
//! collapse blocks its own exits - each of those is a line the caller shows,
//! never a hull handed out anyway. The examples turn them into a failed run;
//! the editor writes them to its status line and keeps the document as it was.
#![warn(missing_docs)]

mod check;
mod collapse;
mod grid;
mod tiles;

#[cfg(test)]
mod tests;

use bevy::prelude::*;
use nova_scenario::prelude::{SectionSource, ShipHull, SpaceshipSectionConfig};
use nova_ship::prelude::{
    GameGrammars, GameSections, GameStyles, GrammarGrid, SectionFootprint, ShipGrammarConfig,
    MAX_GRAMMAR_CELLS,
};

use crate::{
    collapse::Collapse,
    grid::{mirrored, Grid},
    tiles::{Family, Tile},
};

/// Everything a caller needs to collapse a hull and check what came out.
pub mod prelude {
    pub use super::{style_at, StyleId, TileSet};
    pub use crate::{
        check::{hull_errors, lint_errors, place, unmated_contacts, Placed},
        grid::GRID_EPSILON,
        tiles::rotated_half_extents,
    };
}

/// Refuse a grammar the collapse cannot be run in, BEFORE anything indexes a
/// cell or draws a weight.
///
/// How many cells the grammar's seeded stern drive spans, upright.
///
/// `UVec3::ONE` for a drive the catalog does not hold: naming what is missing
/// is [`tiles::build`]'s line to say, and a wrong grid bound said first would
/// bury it.
pub(crate) fn stern_drive_span(sections: &GameSections, grammar: &ShipGrammarConfig) -> UVec3 {
    sections
        .get_section(&grammar.keel.stern_drive)
        .map_or(UVec3::ONE, |config| {
            SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0
        })
}

/// How many cells the grammar's seeded bow gun spans, upright, or zero for a
/// hull plan that seats none.
///
/// `UVec3::ONE` for a gun the catalog does not hold, for the reason
/// [`stern_drive_span`] gives.
pub(crate) fn bow_gun_span(sections: &GameSections, grammar: &ShipGrammarConfig) -> UVec3 {
    let Some(prototype) = grammar.keel.bow_gun.as_deref() else {
        return UVec3::ZERO;
    };
    sections
        .get_section(prototype)
        .map_or(UVec3::ONE, |config| {
            SectionFootprint::from_collider(config.base.collider.unwrap_or_default()).0
        })
}

/// The gate exists because a grammar is content: a mod ships one, and the
/// editor's Generate block builds one out of what the builder ticked. Neither
/// is a compile-time constant, and every bound below is one the solve would
/// otherwise reach by subtracting past zero, indexing off the end of the grid,
/// or asking `rand` for a number out of an empty range. A generator that
/// panics on a bad table is a generator that takes the game down with it, so
/// each of these is a line the caller can put on a status bar instead.
fn runnable(sections: &GameSections, grammar: &ShipGrammarConfig) -> Result<(), String> {
    let grid = grammar.grid;
    // Bounded from ABOVE before anything else, because every check under this
    // one is about a grid small enough to be worth measuring. A domain per
    // cell is laid down before the first contradiction can be found, so an
    // authored size is an allocation this crate is asked to make on trust.
    if grid.cells() > MAX_GRAMMAR_CELLS {
        return Err(format!(
            "grammar '{}' asks for a {}x{}x{} grid, which is {} cells; the collapse holds one \
             domain per cell and stops at {MAX_GRAMMAR_CELLS}",
            grammar.id,
            grid.half_width,
            grid.height,
            grid.length,
            grid.cells(),
        ));
    }
    // The drive is seeded whole, standing one cell off the centreline with its
    // deck plate in front of it, so the grid has to hold the block AND leave
    // the seam column beside it free. A one-cell drive asks for the two
    // columns the collapse has always needed; a 5x5x3 capital asks for six.
    let drive = stern_drive_span(sections, grammar);
    let wanted = UVec3::new(drive.x + 1, drive.y, drive.z + 1);
    let short = |axis: &str, have: u32, want: u32| {
        format!(
            "grammar '{}' is {have} cell(s) {axis} and its seeded stern drive '{}' needs {want}",
            grammar.id, grammar.keel.stern_drive
        )
    };
    if grid.half_width < wanted.x {
        return Err(short("across its half-width", grid.half_width, wanted.x));
    }
    if grid.height < wanted.y {
        return Err(short("tall", grid.height, wanted.y));
    }
    if grid.length < wanted.z {
        return Err(short("long", grid.length, wanted.z));
    }

    // The bow gun stands IN the keel column, so unlike the drive it cannot be
    // centred on a block of cells: one column is all it has. A wider gun is
    // refused by name rather than seeded crooked.
    let bow = bow_gun_span(sections, grammar);
    if let Some(prototype) = grammar.keel.bow_gun.as_deref() {
        if bow.x != 1 || bow.y != 1 {
            return Err(format!(
                "grammar '{}' seats '{prototype}' as its bow gun, but a spinal gun stands on \
                 the keel line and that one is {} cell(s) across and {} tall",
                grammar.id, bow.x, bow.y
            ));
        }
        // Both seeds eat into the same column, and what is left has to hold a
        // keel: a hull cell and the flight computer at least.
        if grid.length < bow.z + drive.z + 2 {
            return Err(format!(
                "grammar '{}' is {} cell(s) long, and its bow gun '{prototype}' and stern \
                 drive '{}' leave no keel between them",
                grammar.id, grid.length, grammar.keel.stern_drive
            ));
        }
    }

    let vacuum = grammar.vacuum;
    for (label, value) in [
        ("base", vacuum.base),
        ("taper", vacuum.taper),
        ("stern", vacuum.stern),
        ("bow_taper", vacuum.bow_taper),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!(
                "grammar '{}' prices vacuum {label} at {value}, which is not a weight",
                grammar.id
            ));
        }
    }

    for part in &grammar.parts {
        if !part.weight.is_finite() || part.weight < 0.0 {
            return Err(format!(
                "grammar '{}' draws '{}' at {}, which is not a weight",
                grammar.id, part.prototype, part.weight
            ));
        }
    }
    // Something has to be drawable. With every weight at zero the draw has
    // nothing to spend and the cell it is asked about has no answer.
    if !grammar.parts.iter().any(|part| part.weight > 0.0) {
        return Err(format!(
            "grammar '{}' draws nothing: give at least one part a weight above zero",
            grammar.id
        ));
    }
    Ok(())
}

/// The id of the style a clad hull wears, read off the merged content.
pub type StyleId<'a> = Option<&'a str>;

/// The style at this index of the merged catalog, wrapped.
///
/// The wrap is what lets an index be a plain increment, and the catalog is
/// whatever the content shipped - a mod that adds a fifth look joins the
/// rotation with nothing here changing.
pub fn style_at(styles: &GameStyles, index: usize) -> StyleId<'_> {
    if styles.is_empty() {
        return None;
    }
    styles
        .get(index % styles.len())
        .map(|style| style.id.as_str())
}

/// One grammar read against one catalog: every orientation of every drawable
/// part, and the block they are laid in.
///
/// Built ONCE and collapsed many times. Reading the catalog is the expensive
/// half (24 rotations per part, each checked against the part's link points)
/// and it does not depend on the seed, so a roster of eight hulls builds this
/// once and runs eight collapses over it.
pub struct TileSet {
    tiles: Vec<Tile>,
    families: Vec<Family>,
    grammar: ShipGrammarConfig,
    grid: Grid,
}

impl TileSet {
    /// Read `grammar` against `sections`.
    ///
    /// `Err` carries the line to show: a grammar the collapse cannot be run in
    /// at all, a part the catalog does not hold, one whose sockets do not
    /// survive the centreline mirror, or one that fires through a face it also
    /// offers a socket on.
    pub fn build(sections: &GameSections, grammar: &ShipGrammarConfig) -> Result<Self, String> {
        runnable(sections, grammar)?;
        let (tiles, families) = tiles::build(sections, grammar)?;
        let grid = Grid::starboard_half(
            grammar.grid.half_width,
            grammar.grid.height,
            grammar.grid.length,
        );
        Ok(Self {
            tiles,
            families,
            grammar: grammar.clone(),
            grid,
        })
    }

    /// Read the grammar named by `id` out of the merged catalog, then build it.
    ///
    /// The lookup is separate from the read so that "no such grammar" and "that
    /// grammar cannot be built" are two different lines.
    pub fn from_catalog(
        sections: &GameSections,
        grammars: &GameGrammars,
        id: &str,
    ) -> Result<Self, String> {
        let grammar = grammars
            .get_grammar(id)
            .ok_or_else(|| format!("no ship grammar '{id}' in the merged content"))?;
        Self::build(sections, grammar)
    }

    /// The grammar this set was read from.
    pub fn grammar(&self) -> &ShipGrammarConfig {
        &self.grammar
    }

    /// The block of cells a hull is collapsed in, as authored.
    pub fn grid(&self) -> GrammarGrid {
        self.grammar.grid
    }

    /// The ship-space z of the grid's own BOW FACE: the front of the `z = 0`
    /// cell row, which is also the face the bow keel cell presents.
    ///
    /// The one piece of grid geometry a caller outside this crate needs, and it
    /// is needed by exactly the callers that bolt something onto a finished
    /// hull: a stamp has to know where the nose is.
    pub fn bow_face(&self) -> f32 {
        -(self.grammar.grid.length as f32) * 0.5
    }

    /// One collapsed ship: structure mirrored into a whole hull, and a flag
    /// asking the game to clad it.
    ///
    /// The HULL only - who flies it and which side it fights for are the
    /// caller's decisions, not the generator's.
    pub fn hull(&self, seed: u64, clad: bool, style: StyleId) -> Result<ShipHull, String> {
        let collapse = Collapse {
            tiles: &self.tiles,
            families: &self.families,
            grid: self.grid,
            keel_row: self.grammar.grid.height as usize / 2,
            vacuum: self.grammar.vacuum,
            keel: &self.grammar.keel,
        };
        let (chosen, kept) = collapse.run(seed)?;

        let mut sections = Vec::new();
        for cell in 0..self.grid.cells() {
            // A non-emitting segment's cell is claimed, but the section it is
            // part of is emitted once, by the segment that carries the offset.
            if !kept[cell] || !self.tiles[chosen[cell]].emits {
                continue;
            }
            let Some(part) = &self.tiles[chosen[cell]].part else {
                continue;
            };
            let (x, y, z) = self.grid.coords(cell);
            let position = self.grid.centre(cell) + part.offset;
            let source = SectionSource::Prototype(part.prototype.clone());
            sections.push(SpaceshipSectionConfig {
                id: format!("starboard_{x}_{y}_{z}"),
                position,
                rotation: part.rotation,
                source: source.clone(),
                modifications: vec![],
            });
            sections.push(SpaceshipSectionConfig {
                id: format!("port_{x}_{y}_{z}"),
                position: Vec3::new(-position.x, position.y, position.z),
                rotation: mirrored(part.rotation),
                source,
                modifications: vec![],
            });
        }

        // A one-off hull: the collapse builds a new one every seed, so there is
        // nothing for a catalog id to name - the caller inlines it.
        Ok(ShipHull {
            sections,
            // The whole of the skin, from this side. The plates, their shapes
            // and where they stand are the game's business, derived from the
            // sections above at spawn - which is what makes a clad hull
            // evidence rather than a picture of what this crate already
            // decided.
            skin: clad,
            // The look, by id, out of the MERGED content - never a literal. A
            // clad ship wears whatever style the content shipped, so nothing
            // here names a greeble, a rule or a cell, and a mod that overlays
            // the style changes the ship without changing the generator.
            style: clad.then_some(style).flatten().map(str::to_string),
            ..default()
        })
    }
}
