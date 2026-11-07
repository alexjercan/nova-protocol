//! Which tool the viewport is holding: select, move, turn or scale.
//!
//! Q, W, E and R, as in every other editor. The same letters fly the camera
//! with RMB held and roll a part in hand, so each has one owner at a time: the
//! rig while RMB is down, the placement keys while a part is armed, and this
//! otherwise. Events has no stage, so there the letters do nothing.
//!
//! SCALE IS NOT A TRANSFORM. Nova sections mate, they do not stretch, and a
//! scaled hull would put every socket somewhere the solver did not leave it, so
//! `inspect.rs` keeps scale off the pose and this keeps it off the document. The
//! tool drives a node's own SIZE FIELD, in meters, and a node without one
//! refuses the tool.

use bevy::prelude::*;
use nova_events::units::prelude::*;
use nova_scenario::prelude::{LightConfig, ScenarioObjectKind};
use nova_ship::prelude::HeldPart;

use crate::{
    config::{EditorSays, RailTab, SelectedNode},
    node::{ObjectNode, ShipNode},
    placement::alt_held,
};

/// One cell, the grid the stage draws: what Ctrl snaps a move to.
pub(crate) const MOVE_SNAP: Meters = Meters(10.0);
/// What Ctrl snaps a turn to, in degrees.
pub(crate) const TURN_SNAP_DEGREES: f32 = 15.0;
/// What Ctrl snaps a size to.
pub(crate) const SIZE_SNAP: Meters = Meters(5.0);
/// The smallest size the tool writes. Lint refuses a zero radius, and a body
/// that small could not be grabbed again.
pub(crate) const MIN_SIZE: Meters = Meters(1.0);

/// The tool the viewport is holding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StageToolType {
    /// No handles. An empty-space drag draws a selection box.
    Select,
    /// Arrows and plane squares. The default, because a body drag moves too.
    #[default]
    Move,
    /// Rings.
    Turn,
    /// Size handles, on a node that has a size.
    Scale,
}

impl StageToolType {
    /// The tool's word on the stage overlay.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Select => "SELECT",
            Self::Move => "MOVE",
            Self::Turn => "TURN",
            Self::Scale => "SCALE",
        }
    }

    /// What the legend calls this tool.
    pub(crate) fn hint(self) -> &'static str {
        match self {
            Self::Select => "tool: select",
            Self::Move => "tool: move",
            Self::Turn => "tool: turn",
            Self::Scale => "tool: scale",
        }
    }
}

/// Which way the handles point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GizmoSpaceType {
    /// Along the world axes, whatever the node faces.
    #[default]
    World,
    /// Along the primary node's own axes.
    Local,
}

/// The viewport's tool, and the axes its handles use.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StageTool {
    /// The tool in hand.
    pub(crate) tool: StageToolType,
    /// The axes its handles use.
    pub(crate) space: GizmoSpaceType,
    /// Whether gestures snap to their step without Ctrl held.
    pub(crate) snap: bool,
}

/// Whether Ctrl is down. It inverts the snap for a gesture, and it also starts
/// every chord, which is why [`snap_active`] is a separate question.
pub(crate) fn snapping(keys: Option<&ButtonInput<KeyCode>>) -> bool {
    keys.is_some_and(|keys| keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]))
}

/// Whether a gesture snaps: the toolbar's Snap, inverted while Ctrl is held.
pub(crate) fn snap_active(tool: &StageTool, keys: Option<&ButtonInput<KeyCode>>) -> bool {
    tool.snap != snapping(keys)
}

/// Pick a tool. The key and the toolbar button both land here.
pub(crate) fn pick_tool(tool: &mut StageTool, picked: StageToolType) {
    if tool.tool != picked {
        tool.tool = picked;
    }
}

/// Swap the handles' axes. X and the toolbar button both land here.
pub(crate) fn swap_handle_space(tool: &mut StageTool, says: &mut EditorSays) {
    let (space, line) = match tool.space {
        GizmoSpaceType::World => (GizmoSpaceType::Local, "handles: local axes"),
        GizmoSpaceType::Local => (GizmoSpaceType::World, "handles: world axes"),
    };
    tool.space = space;
    says.note(line);
}

/// The nearest multiple of `step` to `value`.
pub(crate) fn snapped(value: f32, step: f32) -> f32 {
    (value / step).round() * step
}

/// Q, W, E and R pick a tool, and X swaps the handles' axes.
///
/// Gated on `InputMode::Normal` where it is scheduled, so a typed letter never
/// reaches it. A chord is its own verb, RMB gives these letters to the rig, and
/// a part in hand keeps R for its roll. In Events the bar shows no tools, so a
/// press there must not change what the Scene tab comes back to.
pub(crate) fn tool_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    held: Res<HeldPart>,
    rail: Res<RailTab>,
    mut tool: ResMut<StageTool>,
    mut says: EditorSays,
) {
    let flying = mouse.is_some_and(|mouse| mouse.pressed(MouseButton::Right));
    if rail.is_events()
        || flying
        || *held != HeldPart(None)
        || snapping(Some(&keys))
        || alt_held(Some(&keys))
    {
        return;
    }
    let picked = [
        (KeyCode::KeyQ, StageToolType::Select),
        (KeyCode::KeyW, StageToolType::Move),
        (KeyCode::KeyE, StageToolType::Turn),
        (KeyCode::KeyR, StageToolType::Scale),
    ]
    .into_iter()
    .find_map(|(key, tool)| keys.just_pressed(key).then_some(tool));
    if let Some(picked) = picked {
        pick_tool(&mut tool, picked);
    }
    if keys.just_pressed(KeyCode::KeyX) {
        swap_handle_space(&mut tool, &mut says);
    }
}

/// Why the Scale tool has no handles for this selection, or `None` if it has.
fn scale_refusal(
    selected: &SelectedNode,
    ships: &Query<(), With<ShipNode>>,
    objects: &Query<&ObjectNode>,
) -> Option<&'static str> {
    let node = selected.primary()?;
    if selected.len() > 1 {
        return Some("scale works on one node at a time");
    }
    if ships.contains(node) {
        return Some("a ship is as big as its parts - it has no size to scale");
    }
    match objects.get(node) {
        Ok(object) if size_of(&object.kind).is_some() => None,
        Ok(_) => Some("this object has no size field to scale"),
        Err(_) => Some("a part is as big as it was built - it has no size to scale"),
    }
}

/// Say why the Scale tool shows no handles, once per tool or selection change.
pub(crate) fn sync_scale_refusal(
    tool: Res<StageTool>,
    selected: Res<SelectedNode>,
    ships: Query<(), With<ShipNode>>,
    objects: Query<&ObjectNode>,
    mut says: EditorSays,
) {
    if !(tool.is_changed() || selected.is_changed()) || tool.tool != StageToolType::Scale {
        return;
    }
    if let Some(reason) = scale_refusal(&selected, &ships, &objects) {
        says.flag(reason);
    }
}

/// The size field the Scale tool drives on this kind, if it has one.
pub(crate) fn size_of(kind: &ScenarioObjectKind) -> Option<Meters> {
    match kind {
        ScenarioObjectKind::Anchor(config) => Some(config.body_radius),
        ScenarioObjectKind::Asteroid(config) => Some(config.radius),
        ScenarioObjectKind::Beacon(config) => Some(config.radius),
        ScenarioObjectKind::Planet(config) => Some(config.radius),
        ScenarioObjectKind::Light(LightConfig::Point { range, .. }) => Some(*range),
        _ => None,
    }
}

/// Write the field [`size_of`] reads. A kind with no size is left alone.
pub(crate) fn set_size(kind: &mut ScenarioObjectKind, size: Meters) {
    match kind {
        ScenarioObjectKind::Anchor(config) => config.body_radius = size,
        ScenarioObjectKind::Asteroid(config) => config.radius = size,
        ScenarioObjectKind::Beacon(config) => config.radius = size,
        ScenarioObjectKind::Planet(config) => config.radius = size,
        ScenarioObjectKind::Light(LightConfig::Point { range, .. }) => *range = size,
        _ => {}
    }
}

#[cfg(test)]
mod tests;
