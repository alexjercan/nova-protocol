//! Development fixtures shared by the playable, systems and screenshot
//! examples.
//!
//! These are hulls, weapons and a skin style that the GAME does not ship. They
//! live here because more than one example needs each of them and because
//! nothing in the base catalog should exist only to serve a bench: the capital
//! warship and its siege ordnance are deliberately unbalanced, the size-sweep
//! hulls are reference geometry rather than content, and the placeholder style
//! is scaffolding.
//!
//! A fixture is self-contained. Ships are spawned INLINE
//! (`ShipDesignSource::Inline`), the two siege weapons are authored inline on
//! the hull that mounts them, and every asset reference is spelled as the
//! merge resolves a base bundle path - so no example needs content installed
//! for it and no fixture can reach the section drawer.
//!
//! Included with `#[path = "../shared/dev_fixtures/mod.rs"] mod dev_fixtures;`.

// One source, many example targets: what one example leaves unused another
// needs, so no single build can use all of it.
#![expect(
    dead_code,
    reason = "one source, many example targets: what one example leaves unused another needs"
)]

pub mod block;
pub mod sections;
pub mod styles;

use nova_protocol::prelude::*;

/// The salvage raider: the armed scavenger hull.
pub fn raider() -> ShipDesign {
    block::salvage_raider().design()
}

/// The industrial carrier: the largest fixture hull.
pub fn carrier() -> ShipDesign {
    block::industrial_carrier().design()
}

/// The capital warship: two spinal siege lances, six siege bays, ten mounts.
pub fn warship() -> ShipDesign {
    block::stolen_warship().design()
}

/// The unarmed salvage skiff: the smallest fixture hull.
pub fn skiff() -> ShipDesign {
    block::salvage_skiff().design()
}

/// The unarmed salvage tug.
pub fn tug() -> ShipDesign {
    block::salvage_tug().design()
}

/// The armed salvage claw.
pub fn claw() -> ShipDesign {
    block::salvage_claw().design()
}

/// The cleanup leader: one mount and one Serpent bay.
pub fn cleanup_leader() -> ShipDesign {
    block::salvage_leader().design()
}

/// A spawn of one fixture design, whole.
pub fn design(design: ShipDesign) -> ShipDesignSource {
    ShipDesignSource::Inline(design)
}
