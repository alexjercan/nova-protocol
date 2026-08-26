//! What an editor walk waits on to reach a ship with a part in it.

use nova_protocol::{nova_debug::harness::Predicate, prelude::*};

/// In-step seconds a gesture beat gets before the run gives up on it.
///
/// Every wait in these walks is a CONDITION - the button laid out, the picking
/// pointer registering the press, the preview ship spawned - so this is a
/// backstop rather than a settle. A frame count could only say that some
/// frames had gone by, which buys nothing when the point of the beat is that
/// the editor has REACTED.
pub const BEAT_DEADLINE_SECS: f32 = 20.0;

/// Advance once the editor is inside a ship - what Add Ship does.
///
/// False while there is no [`EditorProbe`] at all. The probe arrives with the
/// editor and every one of these walks starts in the menu, so a beat that
/// READS the resource takes the whole run down on the way in rather than
/// waiting for it.
pub fn inside_a_ship() -> std::sync::Arc<Predicate> {
    std::sync::Arc::new(|world: &bevy::prelude::World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| probe.inside.is_some())
    })
}

/// Advance once the ship being EDITED has a section on it.
///
/// The probe's list, scoped to the edit context. A sweep of every
/// `SectionMarker` in the world is already true before the founding click: a
/// new document opens seeded with the stock range (`node.rs`), whose hulks and
/// pickets are hulls with sections.
pub fn the_ship_is_up() -> std::sync::Arc<Predicate> {
    std::sync::Arc::new(|world: &bevy::prelude::World| {
        world
            .get_resource::<EditorProbe>()
            .is_some_and(|probe| !probe.ship.is_empty())
    })
}
