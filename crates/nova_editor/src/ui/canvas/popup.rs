//! The pop-up behind a block's `...` chip: every field of the block.
//!
//! It is the Inspector's own column, floated over the canvas and fed the same
//! `InspectorRow` data, so there is one way to draw a field. While it is up a
//! transparent catcher takes the window: a press there keeps the edits as one
//! undo step, and Esc takes them back. A field that refused what was typed holds
//! the pop-up for one press, so the refusal is read before the value goes.

use bevy::{prelude::*, ui::UiGlobalTransform, ui_widgets::Activate};
use nova_ui::prelude::{take_keyboard_now, InputMode, TextFieldError};

use super::{drag::rect_of, CanvasDrag, CHIP_TRAVEL};
use crate::{
    camera::ViewportPointer,
    config::{EditorViewType, SelectedNode},
    event::ScriptNodes,
    history::{cancel_edits_since, fold_edits_since, EditHistory},
    project::verbs::refuse,
    ui::{
        inspector::{InspectorField, InspectorList},
        layer,
        picker::ColourWindow,
        shell::{ScriptDetail, ScriptMount},
    },
    ExampleStates,
};

/// How wide the pop-up stands.
pub(crate) const POPUP_W: f32 = 340.0;

/// How tall the pop-up grows before its list scrolls.
const POPUP_MAX_H: f32 = 440.0;

/// What the pop-up adds to its list's height: its head and its padding.
const POPUP_CHROME_H: f32 = 44.0;

/// How a pop-up is leaving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PopupLeaveType {
    /// Keep what was edited, as one step.
    Keep,
    /// Take it all back.
    Cancel,
}

/// The block whose pop-up is open.
#[derive(Resource, Debug, Default)]
pub(crate) struct BlockPopup {
    /// The block.
    node: Option<Entity>,
    /// The top left it opens at, in UI pixels.
    at: Vec2,
    /// The document's revision when it opened.
    since: Option<u64>,
    /// Set on the press or key that ends it. It closes after the frame's edits
    /// land: the press that leaves also commits a focused field.
    leaving: Option<PopupLeaveType>,
    /// Set when a press has already been held back for a refused field. The
    /// next press leaves and drops the value.
    warned: bool,
}

impl BlockPopup {
    /// Whether a pop-up is up.
    pub(crate) fn is_open(&self) -> bool {
        self.node.is_some()
    }

    /// The Esc rung: take back what the pop-up did. Answers whether one was up.
    pub(crate) fn cancel(&mut self) -> bool {
        let open = self.is_open();
        if open {
            self.leaving = Some(PopupLeaveType::Cancel);
        }
        open
    }

    pub(super) fn keep(&mut self) {
        self.leaving.get_or_insert(PopupLeaveType::Keep);
    }
}

/// A block's `...` chip: the node whose fields it opens.
#[derive(Component)]
pub(crate) struct MoreChip(pub(super) Entity);

/// The transparent node behind an open pop-up.
#[derive(Component)]
pub(crate) struct BlockPopupCatcher;

/// A press on `...`: select the block and float its fields beside the chip.
pub(crate) fn on_more_chip(
    activate: On<Activate>,
    mut commands: Commands,
    pointer: Res<ViewportPointer>,
    drag: Res<CanvasDrag>,
    history: Res<EditHistory>,
    chips: Query<(&MoreChip, &ComputedNode, &UiGlobalTransform)>,
    mut popup: ResMut<BlockPopup>,
    mut selected: ResMut<SelectedNode>,
    mut mode: ResMut<InputMode>,
) {
    let Ok((chip, node, transform)) = chips.get(activate.entity) else {
        return;
    };
    let travelled = match (drag.pressed_at, pointer.at) {
        (Some(from), Some(to)) => from.distance(to) > CHIP_TRAVEL,
        _ => false,
    };
    if travelled {
        return;
    }
    let rect = rect_of(node, transform);
    *popup = BlockPopup {
        node: Some(chip.0),
        at: Vec2::new(rect.min.x, rect.max.y),
        since: history.revision(),
        ..default()
    };
    take_keyboard_now(&mut mode, InputMode::Browse);
    selected.set(Some(chip.0));
    commands
        .spawn((
            Name::new("Block Popup Catcher"),
            BlockPopupCatcher,
            DespawnOnExit(ExampleStates::Editor),
            GlobalZIndex(layer::SCRIM_Z),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
        ))
        .observe(|_press: On<PointerPress>, mut popup: ResMut<BlockPopup>| popup.keep());
}

/// End the pop-up a press or a key asked to leave, and one whose block went.
pub(crate) fn settle_block_popup(
    mut commands: Commands,
    mut popup: ResMut<BlockPopup>,
    rail: Res<EditorViewType>,
    script: ScriptNodes,
    refused: Query<(), (With<InspectorField>, With<TextFieldError>)>,
) {
    let Some(node) = popup.node else {
        return;
    };
    if popup.warned && refused.is_empty() {
        popup.warned = false;
    }
    let gone = !rail.is_events() || !script.holds(node);
    if gone {
        popup.keep();
    }
    let Some(leaving) = popup.leaving else {
        return;
    };
    let held = leaving == PopupLeaveType::Keep && !gone && !popup.warned && !refused.is_empty();
    if held {
        popup.leaving = None;
        popup.warned = true;
        commands.queue(|world: &mut World| {
            refuse(world, "a field refused its value: press again to drop it");
        });
        return;
    }
    commands.queue(move |world: &mut World| close_block_popup(world, leaving));
}

/// Close the pop-up now, with all that stands on it: its catcher, a colour
/// window opened from one of its rows, and any refusal. A field keeps its red
/// text until the refusal goes, so it would still show it on the next open.
pub(crate) fn close_block_popup(world: &mut World, leaving: PopupLeaveType) {
    let popup = std::mem::take(&mut *world.resource_mut::<BlockPopup>());
    if !popup.is_open() {
        return;
    }
    let standing: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<BlockPopupCatcher>, With<ColourWindow>)>>()
        .iter(world)
        .collect();
    for entity in standing {
        world.despawn(entity);
    }
    let refused: Vec<Entity> = world
        .query_filtered::<Entity, (With<InspectorField>, With<TextFieldError>)>()
        .iter(world)
        .collect();
    for entity in refused {
        world.entity_mut(entity).remove::<TextFieldError>();
    }
    let Some(since) = popup.since else {
        return;
    };
    match leaving {
        PopupLeaveType::Keep => fold_edits_since(world, since, "Edit Block"),
        PopupLeaveType::Cancel => cancel_edits_since(world, since),
    }
}

/// Keep the open pop-up inside the canvas, as tall as its rows.
pub(crate) fn place_block_popup(
    popup: Res<BlockPopup>,
    mounts: Query<(&ComputedNode, &UiGlobalTransform), With<ScriptMount>>,
    lists: Query<&ComputedNode, With<InspectorList>>,
    mut details: Query<&mut Node, With<ScriptDetail>>,
) {
    if !popup.is_open() {
        return;
    }
    let Ok((computed, transform)) = mounts.single() else {
        return;
    };
    let mount = rect_of(computed, transform);
    let rows = lists
        .iter()
        .next()
        .map_or(0.0, |list| list.content_size.y * list.inverse_scale_factor);
    let height = (rows + POPUP_CHROME_H).min(POPUP_MAX_H).min(mount.height());
    let left = (popup.at.x - mount.min.x).clamp(0.0, (mount.width() - POPUP_W).max(0.0));
    let top = (popup.at.y - mount.min.y).clamp(0.0, (mount.height() - height).max(0.0));
    for mut node in &mut details {
        if node.left != px(left) {
            node.left = px(left);
        }
        if node.top != px(top) {
            node.top = px(top);
        }
        if node.height != px(height) {
            node.height = px(height);
        }
    }
}
