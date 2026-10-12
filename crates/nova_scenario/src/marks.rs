//! Objective marks on generated bodies, kept across a sector's retire and
//! thaw.
//!
//! A streamed body is despawned when its cell retires and spawned again from
//! the frozen record when the cell comes back, and `ObjectiveMarkerTarget` is
//! not part of that record. So a mark on a generated body is remembered here
//! by its [`GeneratedBodyId`], and put back on whichever entity next wears
//! that id.

use std::collections::BTreeMap;

use bevy::prelude::*;
use nova_events::prelude::GeneratedBodyId;
use nova_gameplay::prelude::ObjectiveMarkerTarget;

/// Glob-import surface: `use nova_scenario::marks::prelude::*` brings the
/// marks map and its re-apply system into scope.
pub mod prelude {
    pub use super::{reapply_generated_marks, GeneratedMarks};
}

/// The objective marks the live scenario holds on generated bodies: each
/// body's typed id and the label its chip shows.
///
/// Written by `ObjectiveMarkerAttach` and `ObjectiveMarkerDetach` when their
/// target resolves to a generated body, and read by
/// [`reapply_generated_marks`] whenever a body wearing a marked id spawns -
/// on a cell's first stream, on its thaw, or on a held ship's late spawn.
/// Scoped to the scenario: the unload sweep clears it with the event world.
/// A world save lists the map and a resumed load puts it back before the
/// first cell streams in.
#[derive(Resource, Clone, Debug, Default, Deref, DerefMut)]
pub struct GeneratedMarks(pub BTreeMap<GeneratedBodyId, String>);

/// Put the remembered mark back on every body that just appeared wearing a
/// marked [`GeneratedBodyId`].
///
/// `Added`, not an observer on the component: a thaw inserts the id in the
/// same command batch as the rest of the body, and the HUD's marker observer
/// wants the body whole when the chip grows.
pub fn reapply_generated_marks(
    mut commands: Commands,
    marks: Res<GeneratedMarks>,
    appeared: Query<(Entity, &GeneratedBodyId), Added<GeneratedBodyId>>,
) {
    if marks.is_empty() {
        return;
    }
    for (entity, id) in &appeared {
        let Some(label) = marks.get(id) else {
            continue;
        };
        debug!("reapply_generated_marks: '{id}' <- '{label}'");
        commands
            .entity(entity)
            .insert(ObjectiveMarkerTarget::new(label));
    }
}

#[cfg(test)]
mod tests {
    use nova_events::prelude::SectorCoord;

    use super::*;

    /// A body that spawns wearing a marked id is marked on the frame it
    /// appears, with the remembered label; one wearing an unmarked id is not.
    /// The detector runs as a system, which is the only place `Added` is
    /// read, and the rig takes a warm-up update before the spawn so the
    /// spawn's tick is not the app's first.
    #[test]
    fn a_body_that_appears_with_a_marked_id_is_marked_again() {
        let mut app = App::new();
        app.init_resource::<GeneratedMarks>();
        app.add_systems(Update, reapply_generated_marks);
        app.update();

        let marked_id = GeneratedBodyId::new(SectorCoord::new(-2, 0, 0), "background_rock", 3);
        let other_id = GeneratedBodyId::new(SectorCoord::new(-2, 0, 0), "background_rock", 4);
        app.world_mut()
            .resource_mut::<GeneratedMarks>()
            .insert(marked_id.clone(), "WRECK".to_string());

        let marked = app.world_mut().spawn(marked_id.clone()).id();
        let other = app.world_mut().spawn(other_id).id();
        app.update();

        assert_eq!(
            app.world()
                .get::<ObjectiveMarkerTarget>(marked)
                .map(|marker| marker.label.as_str()),
            Some("WRECK"),
            "the remembered mark is back on the body wearing its id"
        );
        assert!(
            app.world().get::<ObjectiveMarkerTarget>(other).is_none(),
            "an unmarked id gets no chip"
        );

        // A second frame re-applies nothing: the mark is on, and `Added` is
        // an edge.
        app.world_mut()
            .entity_mut(marked)
            .remove::<ObjectiveMarkerTarget>();
        app.update();
        assert!(
            app.world().get::<ObjectiveMarkerTarget>(marked).is_none(),
            "a body already seen is not re-marked every frame"
        );

        // A thaw is a NEW entity wearing the same id: it is marked again.
        app.world_mut().entity_mut(marked).despawn();
        let thawed = app.world_mut().spawn(marked_id).id();
        app.update();
        assert_eq!(
            app.world()
                .get::<ObjectiveMarkerTarget>(thawed)
                .map(|marker| marker.label.as_str()),
            Some("WRECK"),
            "the thawed body carries the mark"
        );
    }
}
