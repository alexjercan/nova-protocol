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
    event::ScriptAdd,
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
        // DOUBLE VERTICAL LINE - two rails, and the shot goes between them.
        Some(SectionKind::Railgun(_)) => ("\u{2016}", "RAILGUN"),

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
        // WHITE SQUARE CONTAINING BLACK SMALL SQUARE - a crate with something
        // in it.
        ScenarioObjectKind::SalvageCrate(_) => ("\u{25a3}", "SALVAGE"),
        // BLACK SUN WITH RAYS.
        ScenarioObjectKind::Light(_) => ("\u{2600}", "LIGHT"),
        // CIRCLE WITH LEFT HALF BLACK - a lit world with a terminator, told
        // apart from the rock's solid disc at a glance.
        ScenarioObjectKind::Planet(_) => ("\u{25d0}", "PLANET"),
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

/// A handler in the script. DOWNWARDS ZIGZAG ARROW - something arrived and
/// this is what caught it.
pub(crate) const HANDLER: &str = "\u{21af}";
/// A filter: the handler's way of declining. WHITE DOWN-POINTING TRIANGLE - a
/// funnel, which is what a filter is.
pub(crate) const FILTER: &str = "\u{25bd}";
/// A boolean over other filters. WHITE DOWN-POINTING SMALL TRIANGLE - the
/// funnel mark, one size down, because this row is funnels all the way in.
pub(crate) const COMBINATOR: &str = "\u{25bf}";
/// An action the handler runs. RIGHTWARDS ARROW - a thing that happens.
pub(crate) const ACTION: &str = "\u{2192}";
/// A sequence. RIGHTWARDS DOUBLE ARROW - the action mark, doubled: not one
/// thing happening but a chain of them.
pub(crate) const SEQUENCE: &str = "\u{21d2}";
/// One beat of a sequence. BULLET.
pub(crate) const STEP: &str = "\u{2022}";
/// The event a step waits for. BOX DRAWINGS DOUBLE VERTICAL - a shut gate.
pub(crate) const GATE: &str = "\u{2551}";

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

/// A container whose children are drawn under it. BLACK DOWN-POINTING SMALL
/// TRIANGLE - pointing at what it is showing.
pub(crate) const OPEN: &str = "\u{25be}";
/// A container holding its children back. BLACK RIGHT-POINTING SMALL TRIANGLE -
/// the same caret, turned: press it and it points down.
pub(crate) const SHUT: &str = "\u{25b8}";

/// The mark an Add row of the script palette wears: the same glyph the node it
/// makes will wear in the tree.
pub(crate) fn script_mark(add: ScriptAdd) -> &'static str {
    match add {
        ScriptAdd::Handler => HANDLER,
        ScriptAdd::Filter => FILTER,
        ScriptAdd::Action => ACTION,
        ScriptAdd::Sequence => SEQUENCE,
        ScriptAdd::Step => STEP,
        ScriptAdd::Gate => GATE,
    }
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
