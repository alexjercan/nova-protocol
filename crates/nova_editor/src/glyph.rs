//! The editor's icon alphabet: one glyph per kind, wherever a kind is drawn.
//!
//! ONE match per family, so the mark in the tree, the mark on the Add row that
//! creates the thing, and the word a hover reveals can never drift apart.
//!
//! Text glyphs rather than image assets, because every surface that draws them
//! is a terminal and a one-character column costs nothing to lay out. LINE ART
//! rather than punctuation, because punctuation ran out: `o` was a controller
//! AND an asteroid, `!` a torpedo AND a beacon, and a mark two kinds share
//! marks neither. Every codepoint below is in the shipped Iosevka Term face,
//! written as an escape so this file stays ASCII.

use nova_scenario::prelude::{ScenarioObjectKind, SpaceshipController};
use nova_ship::prelude::{GalleryCategory, GameSections, SectionKind};

use crate::node::{ObjectChoice, ObjectNode, SectionNode, ShipDriver, ShipNode};

/// The document root. TRIGRAM FOR HEAVEN - a stack of rules, which is what a
/// scenario is.
pub(crate) const SCENARIO: &str = "\u{2630}";
/// The ship Play hands to the player. BLACK RIGHT-POINTING TRIANGLE.
pub(crate) const SHIP_PLAYER: &str = "\u{25b6}";
/// A design standing beside it. WHITE RIGHT-POINTING TRIANGLE - the same craft,
/// not flown by you.
pub(crate) const SHIP_AI: &str = "\u{25b7}";
/// A hull nobody is at the controls of. WHITE RIGHT-POINTING SMALL TRIANGLE -
/// the AI's mark, gone quiet: a craft that is there and is going nowhere.
pub(crate) const SHIP_ADRIFT: &str = "\u{25b9}";

/// The mark on the ship the editor is INSIDE.
///
/// The row's TRAILING column, not its lead: which ship you are in and who flies
/// it are two facts, and one column cannot hold both - entering the player's
/// ship used to hide the fact that it was the player's.
pub(crate) const INSIDE: &str = "@";

/// A ship marked as a station: the station object's own mark, so a station
/// reads the same built here or placed from a catalog.
pub(crate) const SHIP_STATION: &str = "\u{25a2}";

/// The mark a ship row wears, and what it means. A station is marked as one
/// whoever drives it: what the hull IS outranks who is at its controls.
pub(crate) fn ship_mark(ship: &ShipNode) -> (&'static str, &'static str) {
    if ship.station {
        return (SHIP_STATION, "STATION");
    }
    match ship.driver {
        ShipDriver::Player => (SHIP_PLAYER, "SHIP - PLAYER"),
        ShipDriver::Ai => (SHIP_AI, "SHIP - AI"),
        ShipDriver::Adrift => (SHIP_ADRIFT, "SHIP - ADRIFT"),
    }
}

/// The mark a section row wears, and what that mark MEANS in one word.
///
/// A row is 150px wide and its id clips, so the icon is what a builder actually
/// reads down the list: it has to carry the kind on its own.
pub(crate) fn section_mark(
    section: &SectionNode,
    catalog: Option<&GameSections>,
) -> (&'static str, &'static str) {
    kind_mark(section.resolve(catalog).map(|config| &config.kind))
}

/// The mark and word of a section kind. `None` is a part the catalog does not
/// resolve.
pub(crate) fn kind_mark(kind: Option<&SectionKind>) -> (&'static str, &'static str) {
    match kind {
        // WHITE RECTANGLE - a plate.
        Some(SectionKind::Hull(_)) => ("\u{25ad}", "HULL"),
        // SQUARE WITH ORTHOGONAL CROSSHATCH FILL - a board.
        Some(SectionKind::Controller(_)) => ("\u{25a6}", "CONTROLLER"),
        // BLACK UP-POINTING TRIANGLE - a nozzle, pointing the way it pushes.
        Some(SectionKind::Thruster(_)) => ("\u{25b2}", "THRUSTER"),
        // POSITION INDICATOR - a crosshair.
        Some(SectionKind::Turret(_)) => ("\u{2316}", "TURRET"),
        // BLACK DIAMOND - a warhead.
        Some(SectionKind::Torpedo(_)) => ("\u{25c6}", "TORPEDO"),
        // DOUBLE VERTICAL LINE - two rails, and the shot goes between them.
        Some(SectionKind::Railgun(_)) => ("\u{2016}", "RAILGUN"),
        // BULLSEYE - a bore inside a collar, seen down the docking axis.
        Some(SectionKind::Docking(_)) => ("\u{25ce}", "DOCKING"),
        // WHITE SQUARE CONTAINING BLACK SMALL SQUARE - a canister in a hold.
        Some(SectionKind::CargoIntake(_)) => ("\u{25a3}", "INTAKE"),
        // FISHEYE - the emitter lens, seen down the beam.
        Some(SectionKind::Mining(_)) => ("\u{25c9}", "MINING"),

        None => ("?", "PART"),
    }
}

/// The mark a world object wears, and its kind.
///
/// Distinct from every section mark, and from every other object's - a mark
/// that means two things is a mark a builder cannot trust anywhere. The one
/// deliberate overlap is the SHIP marks: a hull the document cannot open - one
/// naming a prototype from a mod this file does not carry - is still a ship,
/// and says so with the same marks a ship node wears.
pub(crate) fn object_mark(object: &ObjectNode) -> (&'static str, &'static str) {
    match &object.kind {
        // BULLSEYE - a well with a centre and no body.
        ScenarioObjectKind::Anchor(_) => ("\u{25ce}", "ANCHOR"),
        // BLACK CIRCLE - a rock.
        ScenarioObjectKind::Asteroid(_) => ("\u{25cf}", "ASTEROID"),
        // A HULL, and the tree says so. Told apart by who is at the controls,
        // exactly as a ship node is.
        ScenarioObjectKind::Spaceship(config) => spaceship_mark(&config.controller),
        // BLACK FLAG - a waypoint.
        ScenarioObjectKind::Beacon(_) => ("\u{2691}", "BEACON"),
        // BLACK SUN WITH RAYS.
        ScenarioObjectKind::Light(_) => ("\u{2600}", "LIGHT"),
        // CIRCLE WITH LEFT HALF BLACK - a lit world with a terminator, told
        // apart from the rock's solid disc at a glance.
        ScenarioObjectKind::Planet(_) => ("\u{25d0}", "PLANET"),
        // WHITE SQUARE WITH ROUNDED CORNERS - a parked block nothing flies.
        ScenarioObjectKind::Station(_) => (SHIP_STATION, "STATION"),
    }
}

/// The mark an unopenable hull wears, and what it means.
///
/// The same three states [`ship_mark`] answers with, off the authored
/// controller rather than off the node's driver.
fn spaceship_mark(controller: &SpaceshipController) -> (&'static str, &'static str) {
    match controller {
        SpaceshipController::Player(_) => (SHIP_PLAYER, "SHIP - PLAYER"),
        SpaceshipController::AI(_) => (SHIP_AI, "SHIP - AI"),
        SpaceshipController::None => (SHIP_ADRIFT, "SHIP - ADRIFT"),
    }
}

/// The mark an asset tile wears for its kind, where it has no picture.
pub(crate) fn asset_mark(kind: &str) -> &'static str {
    match kind {
        "scenario" => SCENARIO,
        // BLACK RIGHT-POINTING TRIANGLE - a ship seen from above.
        "ship" => "\u{25b6}",
        // WHITE HEXAGON - a station's ring.
        "station" => "\u{2b21}",
        "section" => "\u{25ad}",
        // DOUBLE VERTICAL LINE - chapters bound together.
        "campaign" => "\u{2016}",
        // BLACK DIAMOND - a swatch.
        "style" => "\u{25c6}",
        // SQUARE WITH ORTHOGONAL CROSSHATCH FILL - a page of lines.
        "lesson" => "\u{25a6}",
        // FISHEYE - a lens over the screen.
        "ui theme" => "\u{25c9}",
        // WHITE SQUARE CONTAINING BLACK SMALL SQUARE - a crate.
        "item" => "\u{25a3}",
        // BULLSEYE - where a career starts.
        "start role" => "\u{25ce}",
        // BLACK UP-POINTING TRIANGLE - a figure on a ledger.
        "price table" => "\u{25b2}",
        // POSITION INDICATOR - a trade route's mark.
        "trade profile" => "\u{2316}",
        // SQUARE WITH UPPER RIGHT TO LOWER LEFT FILL - a shaded picture.
        "image" => "\u{25a8}",
        // WHITE DIAMOND CONTAINING BLACK SMALL DIAMOND - a solid seen whole.
        "mesh" => "\u{25c8}",
        // EIGHTH NOTE - a sound.
        "audio" => "\u{266a}",
        // IDENTICAL TO - lines of text.
        "file" => "\u{2261}",
        _ => "\u{25a1}",
    }
}

/// The mark the Add row that CREATES an object wears.
///
/// The same glyph the created node will wear in the tree, which is how the menu
/// teaches the tree's alphabet without a legend.
pub(crate) fn choice_mark(choice: ObjectChoice) -> &'static str {
    object_mark(&choice.stock()).0
}

/// The chip that opens a picker on a row naming something the document holds.
/// BLACK DOWN-POINTING SMALL TRIANGLE - the one shape every list a control
/// drops is drawn under.
pub(crate) const PICK: &str = "\u{25be}";

/// A row whose value this placement tuned away from the part it names.
/// ANTICLOCKWISE OPEN CIRCLE ARROW - press it and the field goes back to what
/// the part says.
pub(crate) const INHERIT: &str = "\u{21ba}";

/// A container whose children are drawn under it. BLACK DOWN-POINTING SMALL
/// TRIANGLE - pointing at what it is showing.
pub(crate) const OPEN: &str = "\u{25be}";
/// A container holding its children back. BLACK RIGHT-POINTING SMALL TRIANGLE -
/// the same caret, turned: press it and it points down.
pub(crate) const SHUT: &str = "\u{25b8}";

/// The mark an Add row that opens the parts gallery wears: the section kind
/// that category holds.
pub(crate) fn category_mark(category: GalleryCategory) -> &'static str {
    match category {
        GalleryCategory::All => "",
        GalleryCategory::Structure => "\u{25ad}",
        GalleryCategory::Propulsion => "\u{25b2}",
        GalleryCategory::Control => "\u{25a6}",
        GalleryCategory::Weapons => "\u{2316}",
        GalleryCategory::Ordnance => "\u{25c6}",
        GalleryCategory::Docking => "\u{25ce}",
        GalleryCategory::Mod => "\u{2605}",
    }
}

#[cfg(test)]
mod tests;
