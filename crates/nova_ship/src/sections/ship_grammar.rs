//! A ship GRAMMAR: the authored table a procedural hull is drawn from.
//!
//! The generator that reads this ([`nova_wfc`](https://docs.rs/nova_wfc), the
//! wave-function-collapse collapse) already takes its RULE from the catalog:
//! which parts may sit next to which is read off their link points, so a mod
//! that ships a section already changes what the collapse may build. What it
//! cannot read off a section is TASTE - how often a part should be offered,
//! whether it is allowed to point anywhere, how big a hull is and how sparse.
//!
//! That taste is this type. It is content for the same reason a style is: a
//! generator with its catalog welded into it is a generator no mod can join,
//! and every number below was tuned by looking at hulls rather than derived
//! from anything.
//!
//! Nothing here is optional-with-a-silent-default. A grammar names every role
//! it seeds and every part it draws, and an id it names that the catalog does
//! not hold is an error at lint and again at load.

use bevy::prelude::*;

/// The grammar content type, its parts, and the loaded catalog.
pub mod prelude {
    pub use super::{
        GameGrammars, GrammarAim, GrammarGrid, GrammarKeel, GrammarPart, GrammarVacuum,
        GrammarZone, ShipGrammarConfig, MAX_GRAMMAR_CELLS, STANDARD_HULL_GRAMMAR_ID,
    };
}

/// The id of the base game's shipped grammar: the keeled, mirrored warship the
/// editor's generator and both `wfc` examples draw from.
///
/// Named here rather than beside its builder for the reason the section ids
/// are: the crates that ASK for it - the editor, the examples - cannot reach
/// the authoring crate that authors it.
///
/// Every consumer in the tree names THIS id, and nothing yet chooses another:
/// see [`GameGrammars::get_grammar`] for what that means for a mod.
pub const STANDARD_HULL_GRAMMAR_ID: &str = "standard_hull";

/// The most cells one grammar's grid may hold.
///
/// The one number here that is not taste. The collapse lays down a domain per
/// cell before it can refuse anything, and the count is a product of three
/// authored numbers, so a slipped digit is an allocation nobody asked for and,
/// unbounded, a `u32` product that wraps to a grid of nothing. The shipped hull
/// is 220 cells and the editor grows it to hold a capital drive; this leaves
/// room for one some three hundred times larger.
pub const MAX_GRAMMAR_CELLS: u64 = 65_536;

/// The face a part is allowed to point down, in SHIP space.
///
/// Authored as a direction rather than as an index, because an index into a
/// face table is a fact about the generator and this is a fact about a ship.
/// The collapse reads one cell and its neighbour, so it cannot tell a main
/// drive from a manoeuvring thruster; this is the one claim a grammar makes
/// about where a part belongs ON A SHIP.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GrammarAim {
    /// Toward the stern: `+Z`. Where a main drive exhausts.
    Aft,
    /// Toward the nose: `-Z`.
    Bow,
    /// To starboard: `+X`. The half the collapse runs on before it mirrors.
    Starboard,
    /// To port: `-X`.
    Port,
    /// Up: `+Y`.
    Dorsal,
    /// Down: `-Y`.
    Ventral,
}

impl GrammarAim {
    /// The unit direction this face points along, in ship space.
    pub fn normal(self) -> Vec3 {
        match self {
            Self::Starboard => Vec3::X,
            Self::Port => Vec3::NEG_X,
            Self::Dorsal => Vec3::Y,
            Self::Ventral => Vec3::NEG_Y,
            Self::Aft => Vec3::Z,
            Self::Bow => Vec3::NEG_Z,
        }
    }
}

/// The part of a hull a prototype is allowed to sit in.
///
/// The second claim a grammar makes about where a part belongs ON A SHIP, and
/// it answers a different question from [`GrammarAim`]: an aim says which way a
/// part POINTS, a zone says where it STANDS. A broadside tube is aimed and
/// unzoned; a dorsal turret is zoned and unaimed.
///
/// Authored as a region of the ship rather than as a cell range, because a cell
/// range is a fact about one grid and this has to survive a grammar being made
/// wider or longer. The generator resolves each of these against whatever grid
/// it was handed.
///
/// A zone holds a part only if EVERY cell of it is inside: a three-cell lance
/// zoned `Bow` must lie wholly in the forward third, not merely start there.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GrammarZone {
    /// The forward third of the hull.
    Bow,
    /// The middle third.
    Amidships,
    /// The aft third.
    Stern,
    /// Above the keel row.
    Dorsal,
    /// Below the keel row.
    Ventral,
    /// The outboard half of the hull, clear of the centreline.
    Flank,
}

/// One prototype the collapse may draw, and how often.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrammarPart {
    /// The catalog section id.
    pub prototype: String,
    /// Draw weight for the PART, spent across whatever orientations of it are
    /// legal in a given cell. Taste, not a rule - it decides how OFTEN a part
    /// is offered, never WHERE it may go.
    ///
    /// A fitting's weight is far above what a headcount would suggest, and has
    /// to be: a fitting is not competing with hull for a cell, it is competing
    /// for a cell WITH ROOM AROUND IT, and most of the ones drawn are taken
    /// back off when their exit lane turns out to be blocked.
    pub weight: f32,
    /// The only face this part may fire, launch or exhaust through, or `None`
    /// for a part free to point any way the mating rule allows.
    ///
    /// An override in the sense the authoring rule reserves `Option` for:
    /// absent means "the mating rule alone decides", which is the answer for
    /// a turret that traverses and for a broadside tube.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub aim: Option<GrammarAim>,
    /// The only region of the hull this part may stand in, or `None` for a part
    /// free to stand anywhere the mating rule allows.
    ///
    /// Taste with teeth, like [`aim`](GrammarPart::aim): it cannot make an
    /// illegal placement legal, only forbid a legal one.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub zone: Option<GrammarZone>,
}

/// The block of cells a hull is collapsed in, counted across the STARBOARD
/// half - the ship is this wide either side of the centreline.
///
/// Cells, not metres: one build-grid cell is one world unit, and these are
/// counts of cells rather than a length. Every one of the three is set by the
/// SKIN rather than by the hull: a plate is a flat run only where it has
/// neighbours on all four sides, so a surface has to be at least three cells
/// across before it has any interior at all, and a hull that is all rim comes
/// out a heap of ramps instead of a ship.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrammarGrid {
    /// Cells across the starboard half. Each rise is one more row of deck on
    /// every face.
    pub half_width: u32,
    /// Cells tall. A flank wants a middle for the reason a deck does.
    pub height: u32,
    /// Cells nose to tail. Half again what the hull is wide is most of why the
    /// results read as craft rather than as boxes.
    pub length: u32,
}

impl GrammarGrid {
    /// How many cells the collapse runs in.
    ///
    /// In `u64` because the answer is a product of three authored `u32`s: the
    /// widening is what lets a grid too big to run be REFUSED rather than
    /// wrap into a grid that looks empty. See [`MAX_GRAMMAR_CELLS`].
    pub fn cells(self) -> u64 {
        u64::from(self.half_width) * u64::from(self.height) * u64::from(self.length)
    }
}

/// How emptiness is priced against the solid weights, per cell.
///
/// Pure taste: it decides WHERE the collapse may be sparse, never WHAT may sit
/// next to what.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrammarVacuum {
    /// Base draw weight of vacuum. Low: a porous hull is a lattice, and skin
    /// on a lattice is one plate per strut - noise, not a surface.
    pub base: f32,
    /// How much likelier vacuum gets per cell of distance off the keel.
    pub taper: f32,
    /// How much likelier vacuum gets in the LAST row, so the drives have
    /// somewhere to stand. A drive carries one socket and needs its other five
    /// faces clear, so on a hull built solid it has nowhere to be at all.
    pub stern: f32,
    /// How much likelier vacuum gets at the bow than at the stern. This is the
    /// whole silhouette: a ship with a nose and a broad tail, not a brick.
    pub bow_taper: f32,
}

/// The spine the collapse is handed, laid down before it gets a say.
///
/// Taste and one guarantee: a seeded keel is one connected structure to grow
/// on, so a ship is never a cloud of islands, and a seeded stern is what makes
/// the aft-facing surface a drive needs to stand on.
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GrammarKeel {
    /// What the keel is made of, and what a one-cell dent is packed with.
    /// Must carry a socket on every face it can be met on.
    pub hull: String,
    /// The flight computer, laid a third of the way back where a bridge reads.
    pub bridge: String,
    /// The block beside the last keel cell that the seeded drive bolts to.
    pub stern_deck: String,
    /// The drive bolted to that block's aft face. Mirrored like everything
    /// else, so a hull comes out with a PAIR either side of the centreline.
    pub stern_drive: String,
    /// The spinal gun standing on the bow end of the keel, or `None` for a hull
    /// plan that seats no spinal gun.
    ///
    /// Seeded rather than drawn, for the reason the stern drive is: a gun the
    /// whole SHIP aims fires down its own axis, and a lane that long is only
    /// ever clear at an END of the hull. It stands ON the keel line, so the
    /// mirror gives a tight PAIR either side of the centreline and the recoil
    /// stays on the ship's axis.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub bow_gun: Option<String>,
}

/// One authored ship grammar.
///
/// Resolved by [`id`](ShipGrammarConfig::id) out of [`GameGrammars`], which the
/// mod merge fills exactly as it fills the section catalog - so a mod's grammar
/// with the id of a base one REPLACES it. A grammar under a NEW id merges and
/// is then reachable by nothing: see [`GameGrammars::get_grammar`].
#[derive(Clone, Debug, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShipGrammarConfig {
    /// The id a generator names this grammar by.
    pub id: String,
    /// The name a picker would show. Nothing shows it yet - there is no
    /// grammar picker - so this is the label waiting for one, not a string any
    /// player has read.
    pub name: String,
    /// The block of cells the collapse runs in.
    pub grid: GrammarGrid,
    /// How emptiness is priced against the parts.
    pub vacuum: GrammarVacuum,
    /// The spine laid down before the collapse starts.
    pub keel: GrammarKeel,
    /// The parts the collapse may draw, and how often. The DRAW, not the rule:
    /// a part left out of this list is never offered, and one on it may still
    /// go nowhere the catalog's link points refuse.
    pub parts: Vec<GrammarPart>,
}

impl ShipGrammarConfig {
    /// Every section id this grammar names, in one pass - the set a lint
    /// resolves against the catalog.
    pub fn named_sections(&self) -> impl Iterator<Item = &str> {
        [
            self.keel.hull.as_str(),
            self.keel.bridge.as_str(),
            self.keel.stern_deck.as_str(),
            self.keel.stern_drive.as_str(),
        ]
        .into_iter()
        .chain(self.keel.bow_gun.as_deref())
        .chain(self.parts.iter().map(|part| part.prototype.as_str()))
    }
}

/// The loaded catalog of authored grammars, filled by the mod merge exactly as
/// the section catalog is.
#[derive(Resource, Clone, Debug, Deref, DerefMut, Default)]
pub struct GameGrammars(pub Vec<ShipGrammarConfig>);

impl GameGrammars {
    /// The grammar with this id, or `None` if nothing authored it.
    ///
    /// The parameter is a promise the game does not yet keep. Every caller in
    /// the tree passes [`STANDARD_HULL_GRAMMAR_ID`]: there is no picker, no
    /// scenario field and no flag that names a grammar, so the ONE way a mod
    /// changes procedural generation is to author `id: "standard_hull"` and
    /// retune the shipped line - which retunes the editor and both `wfc`
    /// examples at once, and cannot sit beside the base hull. Selecting
    /// between grammars is the feature this signature is shaped for and is not
    /// built.
    pub fn get_grammar(&self, id: &str) -> Option<&ShipGrammarConfig> {
        self.0.iter().find(|grammar| grammar.id == id)
    }
}
