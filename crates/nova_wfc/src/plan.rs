//! The input the collapse is run from: the grid it fills, how it prices
//! emptiness, the spine it is handed, and the parts it may draw.
//!
//! This is not content. The RULE the generator obeys is the catalog's own link
//! points - which parts may sit next to which is read off their sockets, so a
//! mod that ships a section already changes what may be built. What the catalog
//! cannot carry is TASTE: how often a part should be offered, whether it is
//! allowed to point anywhere, how big a hull is and how sparse. That taste is
//! generator policy, and it lives here in code beside the collapse that reads
//! it rather than in a file a mod overlays by id.
//!
//! [`WfcPlan::standard_hull`] is the one plan the base game ships. Every number
//! in it was set by looking at hulls; none of it is derived from anything. The
//! editor starts from it and bends it to what the builder ticked, and each
//! `wfc` example builds its own - which is the point of the split: a bench may
//! diverge from the editor without a content schema moving.

use bevy::prelude::*;
use nova_ship::prelude::{
    BASIC_CONTROLLER_SECTION_ID, BASIC_THRUSTER_SECTION_ID, PDC_KINETIC_TURRET_SECTION_ID,
    PDC_PIERCE_TURRET_SECTION_ID, REINFORCED_HULL_SECTION_ID, TORPEDO_SECTION_ID,
};

/// The most cells one plan's grid may hold.
///
/// The one number here that is not taste. The collapse lays down a domain per
/// cell before it can refuse anything, and the count is a product of three
/// numbers the caller chose, so a slipped digit is an allocation nobody asked
/// for and, unbounded, a `u32` product that wraps to a grid of nothing. The
/// shipped hull is 220 cells and the editor grows it to hold a capital drive;
/// this leaves room for one some three hundred times larger.
pub const MAX_WFC_CELLS: u64 = 65_536;

/// The face a part is allowed to point down, in SHIP space.
///
/// A direction rather than an index, because an index into a face table is a
/// fact about the generator and this is a fact about a ship. The collapse reads
/// one cell and its neighbour, so it cannot tell a main drive from a
/// manoeuvring thruster; this is the one claim a plan makes about where a part
/// belongs ON A SHIP.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
pub enum WfcAim {
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

impl WfcAim {
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
/// The second claim a plan makes about where a part belongs ON A SHIP, and it
/// answers a different question from [`WfcAim`]: an aim says which way a part
/// POINTS, a zone says where it STANDS. A broadside tube is aimed and unzoned;
/// a dorsal turret is zoned and unaimed.
///
/// A region of the ship rather than a cell range, because a cell range is a
/// fact about one grid and this has to survive a plan being made wider or
/// longer. The generator resolves each of these against whatever grid it was
/// handed.
///
/// A zone holds a part only if EVERY cell of it is inside: a three-cell lance
/// zoned `Bow` must lie wholly in the forward third, not merely start there.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect)]
pub enum WfcZone {
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
pub struct WfcPart {
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
    /// Absent means "the mating rule alone decides", which is the answer for a
    /// turret that traverses and for a broadside tube.
    pub aim: Option<WfcAim>,
    /// The only region of the hull this part may stand in, or `None` for a part
    /// free to stand anywhere the mating rule allows.
    ///
    /// Taste with teeth, like [`aim`](WfcPart::aim): it cannot make an illegal
    /// placement legal, only forbid a legal one.
    pub zone: Option<WfcZone>,
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
pub struct WfcGrid {
    /// Cells across the starboard half. Each rise is one more row of deck on
    /// every face.
    pub half_width: u32,
    /// Cells tall. A flank wants a middle for the reason a deck does.
    pub height: u32,
    /// Cells nose to tail. Half again what the hull is wide is most of why the
    /// results read as craft rather than as boxes.
    pub length: u32,
}

impl WfcGrid {
    /// How many cells the collapse runs in.
    ///
    /// In `u64` because the answer is a product of three `u32`s: the widening
    /// is what lets a grid too big to run be REFUSED rather than wrap into a
    /// grid that looks empty. See [`MAX_WFC_CELLS`].
    pub fn cells(self) -> u64 {
        u64::from(self.half_width) * u64::from(self.height) * u64::from(self.length)
    }
}

/// How emptiness is priced against the solid weights, per cell.
///
/// Pure taste: it decides WHERE the collapse may be sparse, never WHAT may sit
/// next to what.
#[derive(Clone, Copy, Debug, Default, Reflect)]
pub struct WfcVacuum {
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
pub struct WfcKeel {
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
    pub bow_gun: Option<String>,
}

/// One hull plan: everything the collapse needs that the catalog cannot say.
///
/// Built in code by whoever runs the generator. The editor starts from
/// [`WfcPlan::standard_hull`] and replaces its draw with the rows the builder
/// ticked; `wfc_arena` starts from the same default and bends it for the fight
/// it stages. Neither has an id, because nothing looks one up.
#[derive(Clone, Debug, Default, Reflect)]
pub struct WfcPlan {
    /// The block of cells the collapse runs in.
    pub grid: WfcGrid,
    /// How emptiness is priced against the parts.
    pub vacuum: WfcVacuum,
    /// The spine laid down before the collapse starts.
    pub keel: WfcKeel,
    /// The parts the collapse may draw, and how often. The DRAW, not the rule:
    /// a part left out of this list is never offered, and one on it may still
    /// go nowhere the catalog's link points refuse.
    pub parts: Vec<WfcPart>,
}

impl WfcPlan {
    /// The standard hull: a keeled, mirrored warship five cells tall and eleven
    /// long, with a drive deck at the transom and a nose the vacuum taper draws
    /// out of it.
    ///
    /// The collapse the `wfc_ships` row and the `wfc_arena` fight were tuned
    /// on, and what the editor's Generate block starts from.
    pub fn standard_hull() -> Self {
        Self {
            grid: WfcGrid {
                // Four cells to a side, so every face has an interior for the
                // skin to run a flat plate across. Below three a hull is all
                // rim and comes out a heap of ramps.
                half_width: 4,
                height: 5,
                // Half again what the hull is wide. Most of why the results
                // read as craft rather than as boxes.
                length: 11,
            },
            vacuum: WfcVacuum {
                // Low, and lower than it was before the hulls were clad: skin
                // on a lattice is one plate per strut, which is noise rather
                // than a surface. Lower again once the fittings were priced up,
                // because clearance already makes a well-armed hull thin out on
                // its own.
                base: 0.22,
                taper: 1.4,
                // The transom's own row, kept sparse so the drives have a deck
                // to stand on. A drive carries one socket and wants its other
                // five faces clear; on a hull built this solid it otherwise has
                // nowhere to be, and three ships in a row came out with no
                // engines.
                stern: 9.0,
                // The whole silhouette: a nose at one end and a broad tail at
                // the other.
                bow_taper: 24.0,
            },
            keel: WfcKeel {
                hull: REINFORCED_HULL_SECTION_ID.to_string(),
                bridge: BASIC_CONTROLLER_SECTION_ID.to_string(),
                stern_deck: REINFORCED_HULL_SECTION_ID.to_string(),
                stern_drive: BASIC_THRUSTER_SECTION_ID.to_string(),
                // No spinal gun on the standard hull. A lance is a decision
                // about what kind of ship this is, and the base warship is not
                // one - the editor seats one when a builder ticks it.
                bow_gun: None,
            },
            parts: vec![
                WfcPart {
                    prototype: REINFORCED_HULL_SECTION_ID.to_string(),
                    weight: 6.0,
                    aim: None,
                    zone: None,
                },
                WfcPart {
                    // The keel already lays one down, so this only decides how
                    // often a hull grows a second bridge blister. Harmless
                    // either way: the section is the ship's heart, not a
                    // resource.
                    prototype: BASIC_CONTROLLER_SECTION_ID.to_string(),
                    weight: 0.15,
                    aim: None,
                    zone: None,
                },
                WfcPart {
                    // Priced at twice what an unaimed part would be, because an
                    // aim is a rule and rules are paid for in weight: an
                    // aft-only drive competes for a small set of cells instead
                    // of for every exposed face. Measured over 12 seeds, the
                    // aim alone took drives 258 -> 74; at 6.4 they come back to
                    // 102, which is a stern bank instead of engines on the
                    // roof.
                    prototype: BASIC_THRUSTER_SECTION_ID.to_string(),
                    weight: 6.4,
                    aim: Some(WfcAim::Aft),
                    zone: None,
                },
                WfcPart {
                    // The fitting clearance costs most: a torpedo is born two
                    // cells out, so its whole lane must be void and nothing
                    // beside the lane may want cladding in it. Most of those
                    // drawn are eroded. Free to point anywhere - a broadside
                    // tube is a real warship.
                    prototype: TORPEDO_SECTION_ID.to_string(),
                    weight: 1.4,
                    aim: None,
                    zone: None,
                },
                WfcPart {
                    // The two mounts are ONE housing on one socket wearing two
                    // guns, so they split one share between them rather than
                    // each carrying the full figure - which would double how
                    // much battery a hull grows. Kinetic keeps the larger share
                    // as the general-purpose round.
                    prototype: PDC_KINETIC_TURRET_SECTION_ID.to_string(),
                    weight: 1.0,
                    aim: None,
                    zone: None,
                },
                WfcPart {
                    prototype: PDC_PIERCE_TURRET_SECTION_ID.to_string(),
                    weight: 0.6,
                    aim: None,
                    zone: None,
                },
            ],
        }
    }

    /// Every section id this plan names, in one pass - the set a caller
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
