//! The way back: each document a builder opened another one over, the newest
//! last, with the view, camera and selection it was left with.
//!
//! An open pushes a stop and Back pops one, so a nested open returns step by
//! step. Neither is an edit: the history and the unsaved mark never see them.
//! A history step that reopens another asset leaves the trail alone, because
//! the stops still name documents the builder came from.

use std::collections::HashMap;

use bevy::prelude::*;

use super::{ModProject, OpenAsset};
use crate::{
    config::{EditorCamera, EditorViewType, SelectedNode},
    frame::CameraFraming,
    node::{AutoLayout, EditContext, NodeId, ShipNode},
};

/// The most stops the trail keeps. A builder who opens forty documents in a row
/// is not walking back through all of them; the oldest go first.
const TRAIL_CAP: usize = 32;

/// One document left for another.
#[derive(Debug, Clone)]
pub(crate) struct TrailStop {
    /// The asset that was open, `None` for the document a mod with no scenario
    /// founds.
    pub(crate) open: Option<OpenAsset>,
    view: EditorViewType,
    camera: Option<(Transform, CameraFraming)>,
    /// Each selected node as the node ids from the root down, primary last.
    selected: Vec<Vec<String>>,
}

/// The documents Back returns to, newest last.
#[derive(Resource, Debug, Default)]
pub(crate) struct DocumentTrail {
    stops: Vec<TrailStop>,
}

impl DocumentTrail {
    /// The stop Back returns to.
    pub(crate) fn last(&self) -> Option<&TrailStop> {
        self.stops.last()
    }

    /// Forget every stop: a new project starts with no way back.
    pub(crate) fn clear(&mut self) {
        self.stops.clear();
    }
}

/// A camera pose Back put back, for [`crate::node::sync_camera_focus`] to land
/// in place of the frame it takes on a document switch.
#[derive(Resource, Debug, Default)]
pub(crate) struct CameraReturn(pub(crate) Option<(Transform, CameraFraming)>);

/// The node ids from the root down to `node`, the root's own left out.
fn node_path(world: &World, root: Entity, node: Entity) -> Option<Vec<String>> {
    let mut path = Vec::new();
    let mut at = node;
    while at != root {
        path.push(world.get::<NodeId>(at)?.0.clone());
        at = world.get::<ChildOf>(at)?.parent();
    }
    path.reverse();
    Some(path)
}

/// The node `path` names under `root`, if it still stands.
fn node_at(world: &World, root: Entity, path: &[String]) -> Option<Entity> {
    let mut at = root;
    for step in path {
        at = world
            .get::<Children>(at)?
            .iter()
            .find(|child| world.get::<NodeId>(*child).is_some_and(|id| id.0 == *step))?;
    }
    Some(at)
}

/// Remember where the builder stands, before a verb opens another document over
/// it.
pub(crate) fn leave_stop(world: &mut World) {
    let Some(project) = world.get_resource::<ModProject>() else {
        return;
    };
    let open = project.data.open.clone();
    let view = world
        .get_resource::<EditorViewType>()
        .copied()
        .unwrap_or_default();
    let camera = world
        .query_filtered::<&Transform, With<EditorCamera>>()
        .iter(world)
        .next()
        .copied()
        .zip(world.get_resource::<CameraFraming>().copied());
    let root = world
        .get_resource::<EditContext>()
        .and_then(EditContext::scenario);
    let selected = root.map_or_else(Vec::new, |root| {
        world
            .get_resource::<SelectedNode>()
            .map(|selected| selected.iter().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|node| node_path(world, root, node))
            .collect()
    });
    let mut trail = world.get_resource_or_init::<DocumentTrail>();
    trail.stops.push(TrailStop {
        open,
        view,
        camera,
        selected,
    });
    if trail.stops.len() > TRAIL_CAP {
        trail.stops.remove(0);
    }
}

/// Take the newest stop off the trail.
pub(crate) fn take_stop(world: &mut World) -> Option<TrailStop> {
    world.get_resource_mut::<DocumentTrail>()?.stops.pop()
}

/// Put `stop`'s view, camera and selection back on the document now open.
pub(crate) fn restore_stop(world: &mut World, stop: &TrailStop) {
    if let Some(mut view) = world.get_resource_mut::<EditorViewType>() {
        *view = stop.view;
    }
    super::conform_view_now(world);
    if let Some(camera) = stop.camera {
        world.get_resource_or_init::<CameraReturn>().0 = Some(camera);
    }
    let Some(root) = world
        .get_resource::<EditContext>()
        .and_then(EditContext::scenario)
    else {
        return;
    };
    let nodes: Vec<Entity> = stop
        .selected
        .iter()
        .filter_map(|path| node_at(world, root, path))
        .collect();
    if let Some(mut selected) = world.get_resource_mut::<SelectedNode>() {
        selected.set_all(nodes);
    }
}

/// The ships of each scenario still in the automatic row, by node id.
///
/// The row is the session's: no file holds it, so a switch to another document
/// and back lifts every ship at a fixed pose. A ship New Ship had just minted
/// then stood on the first one, and stopped moving aside as its neighbours
/// grew.
#[derive(Resource, Debug, Default)]
pub(crate) struct RowShips(HashMap<String, Vec<String>>);

/// The open scenario's id and root, if a scenario is open.
fn open_scenario_root(world: &World) -> Option<(String, Entity)> {
    let open = world.get_resource::<ModProject>()?.data.open.clone()?;
    let root = world.get_resource::<EditContext>()?.scenario()?;
    open.is_scenario().then_some((open.id, root))
}

/// Remember which ships of the open scenario are in the row, before the
/// document is thrown away.
pub(crate) fn keep_row(world: &mut World) {
    let Some((scenario, root)) = open_scenario_root(world) else {
        return;
    };
    // A ship moved off its row point has left the row, even before the reflow
    // takes the marker off it.
    let ids = world
        .query_filtered::<(&NodeId, &ChildOf, &Transform, &AutoLayout), With<ShipNode>>()
        .iter(world)
        .filter(|(_, parent, pose, row)| parent.parent() == root && pose.translation == row.0)
        .map(|(id, ..)| id.0.clone())
        .collect();
    world
        .get_resource_or_init::<RowShips>()
        .0
        .insert(scenario, ids);
}

/// Put the row back on the ships of the scenario just stood up.
pub(crate) fn restore_row(world: &mut World) {
    let Some((scenario, root)) = open_scenario_root(world) else {
        return;
    };
    let Some(ids) = world
        .get_resource::<RowShips>()
        .and_then(|row| row.0.get(&scenario))
        .cloned()
    else {
        return;
    };
    let laid: Vec<(Entity, Vec3)> = world
        .query_filtered::<(Entity, &NodeId, &ChildOf, &Transform), With<ShipNode>>()
        .iter(world)
        .filter(|(_, id, parent, _)| parent.parent() == root && ids.contains(&id.0))
        .map(|(ship, _, _, pose)| (ship, pose.translation))
        .collect();
    for (ship, at) in laid {
        world.entity_mut(ship).insert(AutoLayout(at));
    }
}
