//! Duplicate, copy and paste, within one document.
//!
//! Ships, objects and script blocks. A section's pose is its socket's, so a copy
//! of one has nowhere to stand until it is mated, and the pipette is how a
//! builder gets another.
//!
//! A copy keeps the ids under it: they are unique per parent, and the parent is
//! new. Only the top of each copy mints a fresh id. A script block lands beside
//! the block that is marked, through the same slots a drag uses, so a paste the
//! marked block cannot take is refused the way a drop is.
//!
//! Text goes out to the desktop through [`copy_lines`]: the Log and Problems
//! rows and a package's path. That is bevy's `Clipboard`, the OS clipboard on
//! desktop. A target with none refuses on the status line with the reason.

use avian3d::prelude::{ColliderAabb, Sensor};
use bevy::{ecs::system::RunSystemOnce, prelude::*, ui_widgets::Activate};
use nova_gameplay::prelude::subtree_collider_aabb;

use crate::{
    asset::on_bench,
    config::{EditorSays, SelectedNode},
    event::{paste_block, shape_of, slot_after, BlockShapeType, ScriptNodes},
    history::{commit_edit_now, revert_to_last_commit, NodeCopy},
    node::{EditContext, NodeId, ObjectNode, ShipNode},
    project::verbs::{note, pressed, refuse as refuse_line},
    tool::{snapping, MOVE_SNAP},
    ui::canvas::CanvasDrag,
};

/// The refusal for a part: where a second one comes from.
const PART_REFUSAL: &str = "a part mates to a socket - press P over it to arm another";

/// What copying says on the bench, which holds one item and no stage to land on.
const BENCH_REFUSAL: &str = "open a scenario to copy and paste";

/// What Ctrl+C took, and how far a paste lands from it.
#[derive(Resource, Debug, Default)]
pub(crate) struct EditorClipboard {
    copies: Vec<NodeCopy>,
    /// One paste's step clear of what was copied.
    step: Vec3,
    /// How many times this has been pasted, so a second paste does not land
    /// on the first.
    pasted: u32,
    /// The outline of each copy, where the copies are script blocks.
    blocks: Vec<BlockShapeType>,
}

impl EditorClipboard {
    /// Whether there is anything to paste.
    pub(crate) fn is_empty(&self) -> bool {
        self.copies.is_empty()
    }
}

/// Say `message` on the status line, from a world verb.
fn refuse(world: &mut World, message: &'static str) {
    let said = world.run_system_once(move |mut says: EditorSays| says.refuse(message));
    if let Err(fault) = said {
        error!("editor: a refusal could not be said: {fault}");
    }
}

/// The selection as script blocks with their outlines, or `None` when it holds
/// anything else. A reporter is one field of its block, so it is not copied.
fn marked_blocks(world: &mut World) -> Option<Vec<(Entity, BlockShapeType)>> {
    let nodes: Vec<Entity> = world.resource::<SelectedNode>().iter().collect();
    if nodes.is_empty() {
        return None;
    }
    world
        .run_system_once(move |script: ScriptNodes| {
            nodes
                .iter()
                .map(|node| {
                    shape_of(&script, *node)
                        .filter(|shape| *shape != BlockShapeType::Reporter)
                        .map(|shape| (*node, shape))
                })
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_default()
}

/// Land each copy beside `anchor`, mark what landed and record one step.
fn land_blocks(
    world: &mut World,
    copies: &[(NodeCopy, BlockShapeType)],
    anchor: Entity,
    label: &'static str,
) {
    let mut fresh = Vec::new();
    let mut after = anchor;
    for (copy, shape) in copies {
        let shape = *shape;
        let slot = world
            .run_system_once(move |script: ScriptNodes| slot_after(&script, after, shape))
            .unwrap_or_default();
        let landed = slot
            .ok_or_else(|| "mark the block to paste beside".to_string())
            .and_then(|slot| paste_block(world, copy, shape, slot));
        match landed {
            Ok(made) => {
                fresh.push(made);
                after = made;
            }
            Err(reason) => {
                let said = world.run_system_once(move |mut says: EditorSays| {
                    says.refuse(reason.clone());
                });
                if let Err(fault) = said {
                    error!("editor: a refusal could not be said: {fault}");
                }
                revert_to_last_commit(world);
                return;
            }
        }
    }
    if !fresh.is_empty() {
        world.resource_mut::<SelectedNode>().set_all(fresh);
        commit_edit_now(world, label);
    }
}

/// The selection as what can be copied, or why it cannot be.
fn copyable(world: &mut World) -> Result<Vec<Entity>, &'static str> {
    if world.resource::<EditContext>().ship().is_some() {
        return Err(PART_REFUSAL);
    }
    if on_bench(world) {
        return Err(BENCH_REFUSAL);
    }
    let nodes: Vec<Entity> = world.resource::<SelectedNode>().iter().collect();
    let staged = |node: &Entity| {
        world.get::<ShipNode>(*node).is_some() || world.get::<ObjectNode>(*node).is_some()
    };
    if nodes.iter().all(staged) {
        Ok(nodes)
    } else {
        Err("only ships and objects can be copied")
    }
}

/// How far along +X to stand a copy of `nodes` so it clears them: their width,
/// plus a cell.
fn clear_step(world: &mut World, nodes: &[Entity]) -> Vec3 {
    let nodes = nodes.to_vec();
    let width = world
        .run_system_once(
            move |q_children: Query<&Children>, q_bounds: Query<&ColliderAabb, Without<Sensor>>| {
                let spans = nodes
                    .iter()
                    .filter_map(|node| subtree_collider_aabb(*node, &q_children, &q_bounds));
                let (low, high) = spans.fold((f32::MAX, f32::MIN), |(low, high), bounds| {
                    (low.min(bounds.min.x), high.max(bounds.max.x))
                });
                (high - low).max(0.0)
            },
        )
        .unwrap_or_default();
    Vec3::X * (width + MOVE_SNAP.to_engine())
}

/// Stand `copies` up beside where they came from, select them, and commit.
fn stand_up(world: &mut World, copies: &[NodeCopy], by: Vec3, label: &'static str) {
    let Some(scenario) = world.resource::<EditContext>().scenario() else {
        return;
    };
    let fresh: Vec<Entity> = copies
        .iter()
        .map(|copy| copy.stand_up(world, scenario, by))
        .collect();
    world.resource_mut::<SelectedNode>().set_all(fresh);
    commit_edit_now(world, label);
}

/// Ctrl+D: a copy of the selection, one step clear of it. Leaves the clipboard
/// alone.
pub(crate) fn duplicate_selection(world: &mut World) {
    if let Some(blocks) = marked_blocks(world) {
        let copies: Vec<(NodeCopy, BlockShapeType)> = blocks
            .iter()
            .filter_map(|(node, shape)| Some((NodeCopy::of(world, *node)?, *shape)))
            .collect();
        if let Some((anchor, _)) = blocks.last() {
            land_blocks(world, &copies, *anchor, "Duplicate");
        }
        return;
    }
    match copyable(world) {
        Ok(nodes) if nodes.is_empty() => {}
        Ok(nodes) => {
            let copies: Vec<NodeCopy> = nodes
                .iter()
                .filter_map(|node| NodeCopy::of(world, *node))
                .collect();
            let step = clear_step(world, &nodes);
            stand_up(world, &copies, step, "Duplicate");
        }
        Err(reason) => refuse(world, reason),
    }
}

/// Ctrl+C: hold a copy of the selection as it stands now.
pub(crate) fn copy_selection(world: &mut World) {
    if let Some(blocks) = marked_blocks(world) {
        let (copies, blocks) = blocks
            .iter()
            .filter_map(|(node, shape)| Some((NodeCopy::of(world, *node)?, *shape)))
            .unzip();
        *world.resource_mut::<EditorClipboard>() = EditorClipboard {
            copies,
            blocks,
            ..default()
        };
        return;
    }
    match copyable(world) {
        Ok(nodes) if nodes.is_empty() => {}
        Ok(nodes) => {
            let copies = nodes
                .iter()
                .filter_map(|node| NodeCopy::of(world, *node))
                .collect();
            let step = clear_step(world, &nodes);
            *world.resource_mut::<EditorClipboard>() = EditorClipboard {
                copies,
                step,
                ..default()
            };
        }
        Err(reason) => refuse(world, reason),
    }
}

/// Ctrl+V: stand up what was copied, one step further along each time.
pub(crate) fn paste_clipboard(world: &mut World) {
    if world.resource::<EditorClipboard>().is_empty() {
        return;
    }
    if world.resource::<EditContext>().ship().is_some() {
        refuse(world, "paste works on a scenario - go Back to one first");
        return;
    }
    if on_bench(world) {
        refuse(world, BENCH_REFUSAL);
        return;
    }
    let mut clipboard = std::mem::take(&mut *world.resource_mut::<EditorClipboard>());
    if !clipboard.blocks.is_empty() {
        let copies: Vec<(NodeCopy, BlockShapeType)> = clipboard
            .copies
            .iter()
            .cloned()
            .zip(clipboard.blocks.iter().copied())
            .collect();
        match world.resource::<SelectedNode>().primary() {
            Some(anchor) => land_blocks(world, &copies, anchor, "Paste"),
            None => refuse(world, "mark the block to paste beside"),
        }
        *world.resource_mut::<EditorClipboard>() = clipboard;
        return;
    }
    clipboard.pasted += 1;
    let by = clipboard.step * clipboard.pasted as f32;
    stand_up(world, &clipboard.copies, by, "Paste");
    *world.resource_mut::<EditorClipboard>() = clipboard;
}

/// Ctrl+D, Ctrl+C and Ctrl+V.
pub(crate) fn clipboard_keys(
    keys: Res<ButtonInput<KeyCode>>,
    drag: Res<CanvasDrag>,
    mut commands: Commands,
) {
    if !snapping(Some(&keys)) || drag.busy() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyD) {
        commands.queue(duplicate_selection);
    } else if keys.just_pressed(KeyCode::KeyC) {
        commands.queue(copy_selection);
    } else if keys.just_pressed(KeyCode::KeyV) {
        commands.queue(paste_clipboard);
    }
}

/// Edit > Duplicate.
pub(crate) fn duplicate_item(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, duplicate_selection));
}

/// Edit > Copy.
pub(crate) fn copy_item(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, copy_selection));
}

/// Edit > Paste.
pub(crate) fn paste_item(_activate: On<Activate>, mut commands: Commands) {
    commands.queue(|world: &mut World| pressed(world, paste_clipboard));
}

/// Put `text` on the desktop clipboard, or say why it did not go.
///
/// A rig with no clipboard plugin has no resource; a desktop session with no
/// clipboard to reach comes back from bevy as the reason.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn copy_out(world: &mut World, text: String) -> Result<(), String> {
    let mut clipboard = world
        .get_resource_mut::<bevy::clipboard::Clipboard>()
        .ok_or_else(|| "this build has no clipboard".to_string())?;
    clipboard.set_text(text).map_err(|fault| fault.to_string())
}

/// The web build does not copy out. The browser grants its clipboard to a page
/// late or never, and a copy must not say it worked before it knows.
#[cfg(target_arch = "wasm32")]
pub(crate) fn copy_out(_world: &mut World, _text: String) -> Result<(), String> {
    Err("the web build cannot reach the desktop clipboard".to_string())
}

/// What the status line says after `count` lines went out.
pub(crate) fn copied_line(count: usize) -> String {
    match count {
        1 => "Copied 1 line".to_string(),
        count => format!("Copied {count} lines"),
    }
}

/// Copy `lines` out, one to a line, and say how many went or why none did.
pub(crate) fn copy_lines(world: &mut World, lines: Vec<String>) {
    if lines.is_empty() {
        refuse_line(world, "nothing to copy: the list is empty");
        return;
    }
    let count = lines.len();
    match copy_out(world, lines.join("\n")) {
        Ok(()) => note(world, copied_line(count)),
        Err(reason) => refuse_line(world, format!("copy refused: {reason}")),
    }
}

/// A stem for a copy's id: the id with its ordinal off, so a copy of `rock_3`
/// mints another `rock_`.
pub(crate) fn id_stem(id: &NodeId) -> &str {
    id.0.rsplit_once('_')
        .filter(|(_, ordinal)| ordinal.chars().all(|digit| digit.is_ascii_digit()))
        .map_or(id.0.as_str(), |(stem, _)| stem)
}

#[cfg(test)]
mod tests;
