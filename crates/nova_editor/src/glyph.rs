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
use nova_ship::prelude::{GameSections, SectionKind};

use crate::{
    gallery::GalleryCategory,
    node::{ObjectChoice, ObjectNode, SectionNode, ShipDriver},
};

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

/// The mark a ship row wears, and what it means.
pub(crate) fn ship_mark(driver: ShipDriver) -> (&'static str, &'static str) {
    match driver {
        ShipDriver::Player => (SHIP_PLAYER, "SHIP - PLAYER"),
        ShipDriver::Ai => (SHIP_AI, "SHIP - AI"),
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
    match section.resolve(catalog).map(|config| &config.kind) {
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
        None => ("?", "PART"),
    }
}

/// The mark a world object wears, and its kind.
///
/// Distinct from every section mark, and from every other object's - a mark
/// that means two things is a mark a builder cannot trust anywhere. The one
/// deliberate overlap is the SHIP marks, which a seeded hull shares with a
/// built one because it is the same kind of thing.
pub(crate) fn object_mark(object: &ObjectNode) -> (&'static str, &'static str) {
    match &object.kind {
        // BULLSEYE - a well with a centre and no body.
        ScenarioObjectKind::Anchor(_) => ("\u{25ce}", "ANCHOR"),
        // BLACK CIRCLE - a rock.
        ScenarioObjectKind::Asteroid(_) => ("\u{25cf}", "ASTEROID"),
        // A HULL, and the tree says so. A picket filed under a generic object
        // mark read as scenery next to the ship being built, when it is the
        // same kind of thing seeded rather than minted - so it wears the ship
        // marks, told apart by who is at the controls exactly as a built ship
        // is.
        ScenarioObjectKind::Spaceship(config) => spaceship_mark(&config.controller),
        // BLACK FLAG - a waypoint.
        ScenarioObjectKind::Beacon(_) => ("\u{2691}", "BEACON"),
        // WHITE SQUARE CONTAINING BLACK SMALL SQUARE - a crate with something
        // in it.
        ScenarioObjectKind::SalvageCrate(_) => ("\u{25a3}", "SALVAGE"),
        // BLACK SUN WITH RAYS.
        ScenarioObjectKind::Light(_) => ("\u{2600}", "LIGHT"),
    }
}

/// The mark a seeded hull wears, and what it means.
///
/// The third state is the one a built ship cannot be in: a hull with no
/// controller station-keeps, which is what every derelict in a scenario is.
fn spaceship_mark(controller: &SpaceshipController) -> (&'static str, &'static str) {
    match controller {
        SpaceshipController::Player(_) => (SHIP_PLAYER, "SHIP - PLAYER"),
        SpaceshipController::AI(_) => (SHIP_AI, "SHIP - AI"),
        SpaceshipController::None => (SHIP_ADRIFT, "SHIP - ADRIFT"),
    }
}

/// The mark the Add row that CREATES an object wears.
///
/// The same glyph the created node will wear in the tree, which is how the menu
/// teaches the tree's alphabet without a legend.
pub(crate) fn choice_mark(choice: ObjectChoice) -> &'static str {
    object_mark(&choice.stock()).0
}

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
    }
}

#[cfg(test)]
mod tests;
