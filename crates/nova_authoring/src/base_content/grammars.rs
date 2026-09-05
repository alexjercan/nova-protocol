//! Built-in SHIP GRAMMAR content: the table a procedurally generated hull is
//! drawn from.
//!
//! A grammar is content like a style is content, so this file is a builder and
//! not a table of constants - `content gen` serializes it into
//! `assets/base/grammars/base.content.ron` and a mod overlays it by id.
//!
//! One grammar ships: `STANDARD_HULL_GRAMMAR_ID`, the collapse the
//! `wfc_ships` row and the `wfc_arena` fight were tuned on. Every number here
//! was set by looking at hulls; none of it is derived from anything, which is
//! exactly why it is authored rather than computed.
//!
//! What is NOT here is the RULE. Which parts may sit beside which is read off
//! the catalog's link points by the generator, and clearance is read off a
//! part's kind - so a mod that ships a section already changes what may be
//! built without touching this file. What a grammar adds is the taste the
//! catalog cannot carry: how often a part is offered, the one part that is
//! only allowed to point one way, and how big and how sparse a hull is.

use nova_ship::prelude::{
    GrammarAim, GrammarGrid, GrammarKeel, GrammarPart, GrammarVacuum, ShipGrammarConfig,
    BASIC_CONTROLLER_SECTION_ID, BASIC_THRUSTER_SECTION_ID, PDC_KINETIC_TURRET_SECTION_ID,
    REINFORCED_HULL_SECTION_ID, STANDARD_HULL_GRAMMAR_ID,
};

/// The pierce PDC: the same housing as the kinetic mount wearing a different
/// round, drawn so a generated hull comes out with a MIXED battery.
const PDC_PIERCE_TURRET_SECTION_ID: &str = "pdc_pierce_turret_section";
/// The two-cell torpedo tube.
const TORPEDO_SECTION_ID: &str = "torpedo_section";

/// Every built-in grammar, in a stable order.
pub(crate) fn grammar_catalog() -> Vec<ShipGrammarConfig> {
    vec![standard_hull()]
}

/// The standard hull: a keeled, mirrored warship five cells tall and eleven
/// long, with a drive deck at the transom and a nose the vacuum taper draws
/// out of it.
fn standard_hull() -> ShipGrammarConfig {
    ShipGrammarConfig {
        id: STANDARD_HULL_GRAMMAR_ID.to_string(),
        name: "Standard Hull".to_string(),
        grid: GrammarGrid {
            // Four cells to a side, so every face has an interior for the skin
            // to run a flat plate across. Below three a hull is all rim and
            // comes out a heap of ramps.
            half_width: 4,
            height: 5,
            // Half again what the hull is wide. Most of why the results read
            // as craft rather than as boxes.
            length: 11,
        },
        vacuum: GrammarVacuum {
            // Low, and lower than it was before the hulls were clad: skin on a
            // lattice is one plate per strut, which is noise rather than a
            // surface. Lower again once the fittings were priced up, because
            // clearance already makes a well-armed hull thin out on its own.
            base: 0.22,
            taper: 1.4,
            // The transom's own row, kept sparse so the drives have a deck to
            // stand on. A drive carries one socket and wants its other five
            // faces clear; on a hull built this solid it otherwise has nowhere
            // to be, and three ships in a row came out with no engines.
            stern: 9.0,
            // The whole silhouette: a nose at one end and a broad tail at the
            // other.
            bow_taper: 24.0,
        },
        keel: GrammarKeel {
            hull: REINFORCED_HULL_SECTION_ID.to_string(),
            bridge: BASIC_CONTROLLER_SECTION_ID.to_string(),
            stern_deck: REINFORCED_HULL_SECTION_ID.to_string(),
            stern_drive: BASIC_THRUSTER_SECTION_ID.to_string(),
            // No spinal gun on the standard hull. A lance is a decision about
            // what kind of ship this is, and the base warship is not one - the
            // editor seats one when a builder ticks it.
            bow_gun: None,
        },
        parts: vec![
            GrammarPart {
                prototype: REINFORCED_HULL_SECTION_ID.to_string(),
                weight: 6.0,
                aim: None,
                zone: None,
            },
            GrammarPart {
                // The keel already lays one down, so this only decides how
                // often a hull grows a second bridge blister. Harmless either
                // way: the section is the ship's heart, not a resource.
                prototype: BASIC_CONTROLLER_SECTION_ID.to_string(),
                weight: 0.15,
                aim: None,
                zone: None,
            },
            GrammarPart {
                // Priced at twice what an unaimed part would be, because an
                // aim is a rule and rules are paid for in weight: an aft-only
                // drive competes for a small set of cells instead of for every
                // exposed face. Measured over 12 seeds, the aim alone took
                // drives 258 -> 74; at 6.4 they come back to 102, which is a
                // stern bank instead of engines on the roof.
                prototype: BASIC_THRUSTER_SECTION_ID.to_string(),
                weight: 6.4,
                aim: Some(GrammarAim::Aft),
                zone: None,
            },
            GrammarPart {
                // The fitting clearance costs most: a torpedo is born two cells
                // out, so its whole lane must be void and nothing beside the
                // lane may want cladding in it. Most of those drawn are eroded.
                // Free to point anywhere - a broadside tube is a real warship.
                prototype: TORPEDO_SECTION_ID.to_string(),
                weight: 1.4,
                aim: None,
                zone: None,
            },
            GrammarPart {
                // The two mounts are ONE housing on one socket wearing two
                // guns, so they split one share between them rather than each
                // carrying the full figure - which would double how much
                // battery a hull grows. Kinetic keeps the larger share as the
                // general-purpose round.
                prototype: PDC_KINETIC_TURRET_SECTION_ID.to_string(),
                weight: 1.0,
                aim: None,
                zone: None,
            },
            GrammarPart {
                prototype: PDC_PIERCE_TURRET_SECTION_ID.to_string(),
                weight: 0.6,
                aim: None,
                zone: None,
            },
        ],
    }
}
