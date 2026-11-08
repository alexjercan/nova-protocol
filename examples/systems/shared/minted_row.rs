//! Clicks on a Scene tree row whose id the walk reads from the world.
//!
//! The editor mints a node's id from its parent's counter, and a reload resumes
//! that counter from the highest ordinal the document holds - the range's own
//! `hulk_4` included. So a walk cannot know a minted id when it builds the
//! script: it stamps the id once the node exists, and aims at
//! `Scene Row {id}` when the beat runs.

use std::sync::Arc;

use bevy::prelude::*;
use nova_protocol::{
    nova_debug::harness::{AutopilotPlugin, Predicate},
    prelude::*,
};

/// Reads the stamped id of the row a beat aims at.
pub type RowId = fn(&World) -> String;

/// The tree row of the node `id` names.
fn row_name(world: &World, id: RowId) -> String {
    format!("Scene Row {}", id(world))
}

/// Whether the row `id` names is laid out.
fn row_present(id: RowId) -> Arc<Predicate> {
    Arc::new(move |world: &World| ui_node_present(row_name(world, id))(world))
}

/// The row gestures, on the script.
pub trait RowGestures {
    /// Click the row `id` names once, then wait for `landed`: the beats of
    /// `click_named`, with the name read when each beat runs.
    fn click_row(self, label: &str, id: RowId, landed: Arc<Predicate>, deadline: f32) -> Self;

    /// Double click the row `id` names: press, release, press, release on four
    /// frames in one step, as `double_click_named` does, so the two clicks land
    /// inside the tree's wall-clock window under a software renderer.
    fn double_click_row(self, label: &str, id: RowId, deadline: f32) -> Self;
}

impl RowGestures for AutopilotPlugin<GameStates> {
    fn click_row(self, label: &str, id: RowId, landed: Arc<Predicate>, deadline: f32) -> Self {
        self.step(format!("{label}: the row is up"))
            .until(row_present(id))
            .deadline(deadline)
            .add()
            .step(format!("{label}: aim"))
            .on_enter(move |world: &mut World| {
                let name = row_name(world, id);
                hover_named(name)(world);
            })
            .each(move |world: &mut World, _, _| {
                let name = row_name(world, id);
                keep_hovering_named(name)(world);
            })
            .until(Arc::new(move |world: &World| {
                pointer_over_node(row_name(world, id))(world)
            }))
            .deadline(deadline)
            .add()
            .step(format!("{label}: press"))
            .on_enter(press_mouse(MouseButton::Left))
            .until(pointer_pressed())
            .deadline(deadline)
            .add()
            .step(format!("{label}: release"))
            .on_enter(release_mouse(MouseButton::Left))
            .until(landed)
            .deadline(deadline)
            .add()
    }

    fn double_click_row(self, label: &str, id: RowId, deadline: f32) -> Self {
        self.step(format!("{label}: the row is up"))
            .until(row_present(id))
            .deadline(deadline)
            .add()
            .step(format!("{label}: double click"))
            .on_enter(move |world: &mut World| {
                let name = row_name(world, id);
                click_named(name)(world);
            })
            .each(move |world: &mut World, _, frame| match frame {
                1 | 3 => release_mouse(MouseButton::Left)(world),
                2 => {
                    let name = row_name(world, id);
                    click_named(name)(world);
                }
                _ => {}
            })
            .until(and(frames(4), pointer_released()))
            .deadline(deadline)
            .add()
    }
}
