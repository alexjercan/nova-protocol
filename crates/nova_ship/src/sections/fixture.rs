//! What is bolted TO a ship as opposed to what a ship is MADE of.
//!
//! A [`SectionFixture`] hangs off a section, takes damage and comes off, and
//! never speaks for the structure it is attached to. Skin plates are the first
//! kind; change this module when the line between structure and dressing moves.

use bevy::prelude::*;
use nova_gameplay::prelude::HealthIsolated;

/// The fixture marker.
pub mod prelude {
    pub use super::SectionFixture;
}

/// Something attached to a section that is NOT part of the ship's structure.
///
/// A fixture has health, mass, a collider and a look, and that is the whole of
/// it. It is never a node in the integrity graph, never counted in a ship's
/// aggregate health, never in the editor palette, and never a
/// [`SectionMarker`](nova_gameplay::prelude::SectionMarker). The test for which
/// side of the line a thing falls on is CAPABILITY: if shooting it off should
/// cost the ship something it can do, it is a section, not a fixture. Cladding
/// costs the ship nothing but the cladding, so it is a fixture.
///
/// The marker earns its place because three systems read `SectionMarker` and
/// every one of them takes a fixture for structure without it:
///
/// - `grade_section_tints` reads `(&Health, Has<SectionInactiveMarker>)` off
///   anything marked a section, so a plate would redden and burn as its own
///   health fell. A fixture's damage feedback is that it COMES OFF, and a
///   reddening plate says the ship is hurt when only its paint is.
/// - `build_ship_integrity_graph` derives connectivity from the link points of
///   everything marked a section, so a run of cladding would bridge two halves
///   of a ship that combat had already cut apart.
/// - a ship's aggregate health is the sum of its sections, and a clad ship
///   carries hundreds of plates against a handful of parts worth counting. The
///   hull bar would read the skin, not the ship.
///
/// A fixture requires [`HealthIsolated`] because it stands IN FRONT of the
/// section it hangs off rather than being part of it. Without that, a plate's
/// damage bubbles into the hull it is bolted to and cladding makes a ship take
/// more damage than going bare - and a piercing round that spends budget on the
/// plate and then hits the hull charges it twice over.
#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
#[require(HealthIsolated)]
pub struct SectionFixture;
