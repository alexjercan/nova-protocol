//! The ported pair the docking examples fly: a tender with a bow port and a
//! moored spar with a port on its near end.
//!
//! Neither hull is shipped content. Both are built here from catalog
//! PROTOTYPE sections, so mounting a port on a playable ship costs the base
//! fleet nothing. Each port stands at cell z = -2, so its face stands 25 m off
//! its hull's origin.

use bevy::prelude::*;
use nova_protocol::prelude::*;

/// One prototype section at a build cell, square with the hull.
fn part(id: &str, prototype: &str, cell: Vec3) -> SpaceshipSectionConfig {
    SpaceshipSectionConfig {
        id: id.to_string(),
        position: cell,
        rotation: Quat::IDENTITY,
        source: SectionSource::prototype(prototype),
    }
}

/// The tender: a bow port, a spine, two shoulder plates and a drive.
///
/// The port stands at the very bow with nothing in front of it, which is the
/// port's own clearance rule - its face is what the capture is measured from,
/// and a plate bolted over it would be a hatch that opens into a wall.
pub fn tender() -> ShipDesign {
    clad(
        vec![
            part(
                "dock_bow",
                DOCKING_PORT_SECTION_ID,
                Vec3::new(0.0, 0.0, -2.0),
            ),
            part("bow", REINFORCED_HULL_SECTION_ID, Vec3::new(0.0, 0.0, -1.0)),
            part("bridge", BASIC_CONTROLLER_SECTION_ID, Vec3::ZERO),
            part("shoulder_port", LIGHT_HULL_SECTION_ID, Vec3::NEG_X),
            part("shoulder_starboard", LIGHT_HULL_SECTION_ID, Vec3::X),
            part("drive", BASIC_THRUSTER_SECTION_ID, Vec3::Z),
        ],
        "industrial",
    )
}

/// The spar: a plain moored stack with one port on its near end and nothing
/// that flies. It is the thing you dock WITH, not a second ship.
pub fn spar() -> ShipDesign {
    clad(
        vec![
            part(
                "dock_fore",
                DOCKING_PORT_SECTION_ID,
                Vec3::new(0.0, 0.0, -2.0),
            ),
            part(
                "fore",
                REINFORCED_HULL_SECTION_ID,
                Vec3::new(0.0, 0.0, -1.0),
            ),
            part("midships", REINFORCED_HULL_SECTION_ID, Vec3::ZERO),
            part("aft", REINFORCED_HULL_SECTION_ID, Vec3::Z),
            part("mast_high", LIGHT_HULL_SECTION_ID, Vec3::Y),
            part("mast_low", LIGHT_HULL_SECTION_ID, Vec3::NEG_Y),
        ],
        "industrial",
    )
}

/// A hand-built cell list wearing the derived skin, the way every block hull
/// in the fleet is dressed. A design that leaves this off renders as bare
/// cells, which is a look the game ships nowhere.
fn clad(sections: Vec<SpaceshipSectionConfig>, style: &str) -> ShipDesign {
    ShipDesign {
        sections,
        presentation: ShipPresentationConfig {
            skin: true,
            style: Some(style.to_string()),
            ..ShipPresentationConfig::base_voice()
        },
        ..default()
    }
}
