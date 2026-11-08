//! The puzzle: picking a block up, what lights while it is held, and the drop.
//!
//! A press is only a candidate. Past [`DRAG_START`] pixels it becomes a drag;
//! a release before that is a click, which selects. Every question of what fits
//! where is [`refusal`]'s, so a slot that lights is a slot that takes the block.

use std::{borrow::Cow, collections::HashMap};

use bevy::{
    ecs::{
        hierarchy::ChildOf,
        system::{RunSystemOnce, SystemParam},
    },
    picking::{events::Pointer, Pickable},
    prelude::*,
    ui::UiGlobalTransform,
};
use nova_ui::prelude::TextField;

use super::{
    paint::{paint_stacked, Paint},
    palette::{BlockPalette, PaletteEntry},
    view::{BlockView, ViewBuilder},
    CanvasBoard, CanvasView, CanvasViewport, CanvasWorld, HandlerRoot, FLOW_GAP,
};
use crate::{
    camera::ViewportPointer,
    config::{EditorSays, SelectedNode},
    event::{
        delete_block, move_block, place_new, refusal, reveal, shape_of, BlockItemType,
        BlockShapeType, BlockSlotType, EventNode, ScriptNodes,
    },
    history::commit_edit_now,
    palette::{BLOCK_SLOT_GAP, BLOCK_SLOT_LIT},
    ui::{inspector::InspectorDrag, layer::GHOST_Z},
};

/// How far a press travels before it is a drag, in pixels.
const DRAG_START: f32 = 4.0;

/// How near a slot has to be to take the block, in pixels at zoom 1.
const REACH: f32 = 60.0;

/// How much nearer another slot has to be to take the aim away. Opening a gap
/// moves the slots below it, and without this the aim would flicker between two.
const AIM_STICK: f32 = 14.0;

/// A lit gap's thickness. It overlaps the block above rather than pushing it.
const LIT_H: f32 = 4.0;

/// The gap the aimed slot opens.
const GAP_H: f32 = 18.0;

/// A block on the canvas.
#[derive(Component, Debug)]
pub(crate) struct CanvasBlock {
    /// The node it draws.
    pub(crate) node: Entity,
    /// Whether a problem names it.
    pub(crate) faulted: bool,
}

/// An inline field's editable text.
#[derive(Component, Debug)]
pub(crate) struct CanvasChip {
    /// The whole value, which the chip may show cut.
    pub(crate) whole: String,
}

/// How a slot shows that it is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotLookType {
    /// A place between two blocks: a line that opens into a gap.
    Gap,
    /// A socket: an outline around the hole.
    Socket,
}

/// A place a block can land.
#[derive(Component, Debug)]
pub(crate) struct CanvasSlot {
    /// What landing here means.
    pub(crate) slot: BlockSlotType,
    /// How it lights.
    pub(crate) look: SlotLookType,
    /// Whether the block in hand fits. Set when the drag begins.
    pub(crate) legal: bool,
}

/// What a press took hold of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DragSourceType {
    /// A block already in the script.
    Block(Entity),
    /// A new block off the palette.
    Palette(BlockItemType),
}

/// A press that has not moved far enough to mean anything yet.
#[derive(Debug, Clone, Copy)]
pub(super) struct Candidate {
    pub(super) source: DragSourceType,
    pub(super) at: Vec2,
    /// Off on a number's scrub chip: the press belongs to the chip.
    pub(super) draggable: bool,
}

/// The block in hand.
#[derive(Debug, Clone)]
pub(crate) struct Held {
    /// Where it came from.
    pub(crate) source: DragSourceType,
    /// Its outline.
    pub(crate) shape: BlockShapeType,
    /// From the pointer to the ghost's top left, in pixels at zoom 1.
    pub(crate) grab: Vec2,
    /// The nodes it carries, which are hidden until it lands.
    pub(crate) run: Vec<Entity>,
    /// The slot it would land in.
    pub(crate) aim: Option<Entity>,
}

/// The press and the drag.
#[derive(Resource, Default)]
pub(crate) struct CanvasDrag {
    pub(super) candidate: Option<Candidate>,
    /// Where the last press landed: a click that travelled was a scrub.
    pub(crate) pressed_at: Option<Vec2>,
    /// The block in hand.
    pub(crate) held: Option<Held>,
    /// A block Esc took out of the hand, for the drag to put back.
    cancelled: Option<Held>,
}

impl CanvasDrag {
    /// Whether a press or a drag owns a block right now.
    pub(crate) fn busy(&self) -> bool {
        self.candidate.is_some() || self.held.is_some()
    }

    /// Empty the hand, and answer whether it held anything.
    pub(crate) fn cancel(&mut self) -> bool {
        let busy = self.busy();
        self.candidate = None;
        self.cancelled = self.held.take().or(self.cancelled.take());
        busy
    }

    /// What the probe calls the block in hand.
    pub(crate) fn held_name(&self) -> Option<String> {
        self.held.as_ref().map(|held| match held.source {
            DragSourceType::Block(_) => format!("{:?}", held.shape),
            DragSourceType::Palette(item) => item.label(),
        })
    }
}

/// The ghost's root.
#[derive(Component)]
pub(crate) struct CanvasGhost;

/// `node`'s box in logical pixels, zoom included.
pub(crate) fn rect_of(node: &ComputedNode, transform: &UiGlobalTransform) -> Rect {
    let scale = node.inverse_scale_factor;
    let (stretch, _, centre) = transform.to_scale_angle_translation();
    Rect::from_center_size(centre * scale, node.size() * scale * stretch.abs())
}

/// The first press on a block or a palette entry becomes the candidate.
pub(crate) fn on_canvas_press(
    press: On<Pointer<Press>>,
    keys: Res<ButtonInput<KeyCode>>,
    blocks: Query<&CanvasBlock>,
    entries: Query<&PaletteEntry>,
    scrubs: Query<(), With<InspectorDrag>>,
    fields: Query<(), With<TextField>>,
    grounds: Query<(), Or<(With<CanvasViewport>, With<CanvasWorld>)>>,
    parents: Query<&ChildOf>,
    mut selected: ResMut<SelectedNode>,
    mut drag: ResMut<CanvasDrag>,
) {
    // A global observer hears a bubbling event once per entity it reaches.
    if press.entity != press.original_event_target()
        || press.button != PointerButton::Primary
        || keys.pressed(KeyCode::Space)
    {
        return;
    }
    let at = press.pointer_location.position;
    drag.pressed_at = Some(at);
    drag.candidate = None;
    let target = press.entity;
    if grounds.contains(target) {
        selected.set(None);
        return;
    }
    let mut draggable = true;
    for walked in std::iter::once(target).chain(parents.iter_ancestors(target)) {
        if fields.contains(walked) {
            return;
        }
        draggable &= !scrubs.contains(walked);
        if let Ok(entry) = entries.get(walked) {
            drag.candidate = Some(Candidate {
                source: DragSourceType::Palette(entry.0),
                at,
                draggable,
            });
            return;
        }
        if let Ok(block) = blocks.get(walked) {
            drag.candidate = Some(Candidate {
                source: DragSourceType::Block(block.node),
                at,
                draggable,
            });
            return;
        }
    }
}

/// What the drag reads and writes on the canvas.
#[derive(SystemParam)]
pub(crate) struct DragStage<'w, 's> {
    commands: Commands<'w, 's>,
    pointer: Res<'w, ViewportPointer>,
    buttons: Res<'w, ButtonInput<MouseButton>>,
    view: Res<'w, CanvasView>,
    board: Res<'w, CanvasBoard>,
    drag: ResMut<'w, CanvasDrag>,
    script: ScriptNodes<'w, 's>,
    slots: Query<
        'w,
        's,
        (
            Entity,
            &'static mut CanvasSlot,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    blocks: Query<
        'w,
        's,
        (
            Entity,
            &'static CanvasBlock,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    nodes: Query<'w, 's, &'static mut Node, Without<CanvasGhost>>,
    ghosts: Query<'w, 's, (Entity, &'static mut Node, &'static mut UiTransform), With<CanvasGhost>>,
    palette: Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform), With<BlockPalette>>,
    world_nodes: Query<'w, 's, &'static UiGlobalTransform, With<CanvasWorld>>,
    viewport:
        Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform), With<CanvasViewport>>,
    parents: Query<'w, 's, &'static ChildOf>,
    ghost_sizes: Query<'w, 's, &'static ComputedNode, With<CanvasGhost>>,
    handlers: Query<'w, 's, &'static ComputedNode, With<HandlerRoot>>,
}

impl DragStage<'_, '_> {
    /// The block in hand's size on the canvas, at zoom 1.
    fn ghost_size(&self) -> Vec2 {
        self.ghost_sizes
            .iter()
            .next()
            .map_or(Vec2::ZERO, |node| node.size() * node.inverse_scale_factor)
    }

    /// Where every handler but the one in hand stands on the canvas.
    fn other_hats(&self, source: DragSourceType) -> Vec<Rect> {
        let moving = match source {
            DragSourceType::Block(node) => Some(node),
            DragSourceType::Palette(_) => None,
        };
        let flowed: HashMap<Entity, Vec2> = self.board.auto_placed().collect();
        self.board
            .roots
            .iter()
            .filter(|(hat, _)| Some(**hat) != moving)
            .filter_map(|(hat, root)| {
                let node = self.handlers.get(*root).ok()?;
                let at = self
                    .script
                    .event(*hat)
                    .and_then(|event| event.position)
                    .or_else(|| flowed.get(hat).copied())?;
                let size = node.size() * node.inverse_scale_factor;
                Some(Rect::from_corners(at, at + size))
            })
            .collect()
    }

    /// How far, in pixels, the canvas point `corner` stands past the viewport's
    /// bottom right: the pan that brings a nudged hat back into sight.
    fn past_the_view(&self, corner: Vec2) -> Vec2 {
        self.viewport
            .single()
            .map_or(Vec2::ZERO, |(node, transform)| {
                let edge = self.canvas_point(rect_of(node, transform).max);
                (corner - edge).max(Vec2::ZERO) * self.view.zoom
            })
    }

    /// Where `at` falls on the canvas.
    fn canvas_point(&self, at: Vec2) -> Vec2 {
        let origin = self
            .world_nodes
            .single()
            .map(|transform| transform.translation)
            .unwrap_or_default();
        let scale = self
            .viewport
            .single()
            .map(|(node, _)| node.inverse_scale_factor)
            .unwrap_or(1.0);
        (at - origin * scale) / self.view.zoom
    }

    fn over_palette(&self, at: Vec2) -> bool {
        self.palette
            .iter()
            .any(|(node, transform)| rect_of(node, transform).contains(at))
    }

    fn over_canvas(&self, at: Vec2) -> bool {
        self.viewport
            .iter()
            .any(|(node, transform)| rect_of(node, transform).contains(at))
    }

    fn set_run_shown(&mut self, run: &[Entity], shown: bool) {
        let display = if shown { Display::Flex } else { Display::None };
        let hidden: Vec<Entity> = self
            .blocks
            .iter()
            .filter(|(_, block, ..)| run.contains(&block.node))
            .map(|(entity, ..)| entity)
            .collect();
        for entity in hidden {
            if let Ok(mut node) = self.nodes.get_mut(entity) {
                node.display = display;
            }
        }
    }

    fn begin(&mut self, candidate: Candidate, at: Vec2) {
        let (shape, run, moving) = match candidate.source {
            DragSourceType::Palette(item) => (item.shape(), Vec::new(), None),
            DragSourceType::Block(node) => {
                let Some(shape) = shape_of(&self.script, node) else {
                    return;
                };
                let run = self.run_of(node, shape);
                (shape, run, Some(node))
            }
        };
        let hidden: Vec<Entity> = self
            .blocks
            .iter()
            .filter(|(_, block, ..)| run.contains(&block.node))
            .map(|(entity, ..)| entity)
            .collect();
        let legal: Vec<(Entity, bool)> = self
            .slots
            .iter()
            .map(|(entity, slot, ..)| {
                let inside_run = self
                    .parents
                    .iter_ancestors(entity)
                    .any(|above| hidden.contains(&above));
                let fits = !inside_run && refusal(&self.script, shape, moving, slot.slot).is_none();
                (entity, fits)
            })
            .collect();
        for (entity, fits) in legal {
            if let Ok((_, mut slot, ..)) = self.slots.get_mut(entity) {
                slot.legal = fits;
            }
        }
        let grab = self.grab_of(candidate.source, at);
        self.spawn_ghost(candidate.source, &run, at, grab);
        self.set_run_shown(&run, false);
        self.drag.held = Some(Held {
            source: candidate.source,
            shape,
            grab,
            run,
            aim: None,
        });
    }

    /// What a drag of `node` carries: the blocks below it in its own stack.
    fn run_of(&self, node: Entity, shape: BlockShapeType) -> Vec<Entity> {
        let list = match shape {
            BlockShapeType::Stack => self
                .script
                .owner(node)
                .map(|parent| self.script.actions_of(parent)),
            BlockShapeType::Step => self
                .script
                .owner(node)
                .map(|parent| self.script.steps_of(parent)),
            _ => None,
        };
        match list.and_then(|list| {
            list.iter()
                .position(|held| *held == node)
                .map(|at| list[at..].to_vec())
        }) {
            Some(run) => run,
            None => vec![node],
        }
    }

    fn grab_of(&self, source: DragSourceType, at: Vec2) -> Vec2 {
        let DragSourceType::Block(node) = source else {
            return Vec2::new(-16.0, -14.0);
        };
        self.blocks
            .iter()
            .find(|(_, block, ..)| block.node == node)
            .map(|(_, _, computed, transform)| {
                (rect_of(computed, transform).min - at) / self.view.zoom
            })
            .unwrap_or_default()
    }

    /// Spawn the ghost already under the pointer. Spawned with an `Auto`
    /// corner, layout put it at the window's top left for its first frame.
    fn spawn_ghost(&mut self, source: DragSourceType, run: &[Entity], at: Vec2, grab: Vec2) {
        let (left, top, scale) = ghost_place(at, grab, self.view.zoom);
        let views: Vec<BlockView> = match source {
            DragSourceType::Palette(item) => vec![BlockView::preview(item)],
            DragSourceType::Block(_) => {
                let builder = ViewBuilder {
                    script: &self.script,
                    faults: &self.board.faults,
                };
                run.iter().filter_map(|node| builder.any(*node)).collect()
            }
        };
        self.commands
            .spawn((
                Name::new("Canvas Ghost"),
                CanvasGhost,
                GlobalZIndex(GHOST_Z),
                Pickable::IGNORE,
                UiTransform { scale, ..default() },
                Outline {
                    width: px(2),
                    offset: px(2),
                    color: Color::NONE,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left,
                    top,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
            ))
            .with_children(|ghost| {
                let mut paint = Paint::picture();
                let last = views.len().saturating_sub(1);
                let mut notch = None;
                for (index, view) in views.iter().enumerate() {
                    paint_stacked(ghost, view, notch, index == last, &mut paint);
                    notch = Some(view.colour);
                }
            });
    }

    /// The slot the block would take, from the pointer.
    fn aim(&mut self, at: Vec2, current: Option<Entity>) -> Option<Entity> {
        if !self.over_canvas(at) {
            return None;
        }
        let reach = REACH * self.view.zoom;
        let distance = |rect: Rect| {
            let nearest = at.clamp(rect.min, rect.max);
            nearest.distance(at)
        };
        let mut best: Option<(Entity, f32)> = None;
        for (entity, slot, computed, transform) in &self.slots {
            if !slot.legal {
                continue;
            }
            let mut away = distance(rect_of(computed, transform));
            if Some(entity) == current {
                away -= AIM_STICK;
            }
            if away < reach && best.is_none_or(|(_, held)| away < held) {
                best = Some((entity, away));
            }
        }
        best.map(|(entity, _)| entity)
    }

    /// Why the slot nearest `at` turned the block away.
    fn nearest_refusal(&self, held: &Held, at: Vec2) -> Option<String> {
        let moving = match held.source {
            DragSourceType::Block(node) => Some(node),
            DragSourceType::Palette(_) => None,
        };
        let reach = REACH * self.view.zoom;
        self.slots
            .iter()
            .filter_map(|(_, slot, computed, transform)| {
                let rect = rect_of(computed, transform);
                let away = at.clamp(rect.min, rect.max).distance(at);
                (away < reach).then_some((slot.slot, away))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .and_then(|(slot, _)| refusal(&self.script, held.shape, moving, slot))
    }

    /// Take the ghost down and show the blocks it carried where they stand.
    fn put_back(&mut self, held: &Held) {
        for (ghost, ..) in &self.ghosts {
            self.commands.entity(ghost).despawn();
        }
        self.set_run_shown(&held.run, true);
    }

    fn finish(&mut self, held: Held, at: Vec2) {
        self.put_back(&held);
        let aimed = held
            .aim
            .and_then(|aim| self.slots.get(aim).ok())
            .map(|(_, slot, ..)| slot.slot);
        let landing = if let Some(slot) = aimed {
            Landing::Slot(slot)
        } else if self.over_palette(at) {
            Landing::Bin
        } else if self.over_canvas(at) && held.shape == BlockShapeType::Hat {
            let dropped = self.canvas_point(at + held.grab * self.view.zoom);
            let size = self.ghost_size();
            let spot = clear_of(dropped, size, &self.other_hats(held.source));
            if spot != dropped {
                let shift = self.past_the_view(spot + size);
                self.commands.queue(move |world: &mut World| {
                    world.resource_mut::<CanvasView>().pan -= shift;
                });
            }
            Landing::Slot(BlockSlotType::Canvas(spot))
        } else {
            let moving = match held.source {
                DragSourceType::Block(node) => Some(node),
                DragSourceType::Palette(_) => None,
            };
            let reason = self.nearest_refusal(&held, at).or_else(|| {
                refusal(
                    &self.script,
                    held.shape,
                    moving,
                    BlockSlotType::Canvas(Vec2::ZERO),
                )
            });
            Landing::Refused(reason.unwrap_or_else(|| "nothing here takes this block".to_string()))
        };
        let frozen: Vec<(Entity, Vec2)> = self
            .board
            .auto_placed()
            .filter(|_| held.shape == BlockShapeType::Hat)
            .collect();
        let source = held.source;
        self.commands
            .queue(move |world: &mut World| land(world, source, landing, &frozen));
    }
}

/// The ghost's left, top and scale for the pointer at `at`: `grab` away from
/// it, at the canvas's zoom.
fn ghost_place(at: Vec2, grab: Vec2, zoom: f32) -> (Val, Val, Vec2) {
    let corner = at + grab * zoom;
    (px(corner.x), px(corner.y), Vec2::splat(zoom))
}

/// `at`, or the nearer of the spots down and right of it where a block of `size`
/// overlaps none of `others`.
pub(super) fn clear_of(at: Vec2, size: Vec2, others: &[Rect]) -> Vec2 {
    let down = walked_clear(at, size, others, |at, other| {
        Vec2::new(at.x, other.max.y + FLOW_GAP)
    });
    let right = walked_clear(at, size, others, |at, other| {
        Vec2::new(other.max.x + FLOW_GAP, at.y)
    });
    if down.distance_squared(at) <= right.distance_squared(at) {
        down
    } else {
        right
    }
}

/// Step `at` past each rect it overlaps. `past` only ever moves one way, so no
/// rect is met twice and the walk ends.
fn walked_clear(
    mut at: Vec2,
    size: Vec2,
    others: &[Rect],
    past: impl Fn(Vec2, &Rect) -> Vec2,
) -> Vec2 {
    while let Some(other) = others.iter().find(|other| {
        !other
            .intersect(Rect::from_corners(at, at + size))
            .is_empty()
    }) {
        at = past(at, other);
    }
    at
}

/// How a drop ended.
#[derive(Debug, Clone)]
pub(super) enum Landing {
    /// In a slot, or on the bare canvas.
    Slot(BlockSlotType),
    /// On the palette: the block is thrown away.
    Bin,
    /// Nowhere it fits.
    Refused(String),
}

fn say_refused(world: &mut World, reason: String) {
    let said = world.run_system_once(move |mut says: EditorSays| {
        says.refuse(reason.clone());
    });
    if let Err(fault) = said {
        error!("editor: a refusal could not be said: {fault}");
    }
}

/// Whether what a drop names still stands. An undo with the block in hand can
/// take away either end of it.
fn drop_stands(world: &World, source: DragSourceType, landing: &Landing) -> bool {
    let block = match source {
        DragSourceType::Block(node) => Some(node),
        DragSourceType::Palette(_) => None,
    };
    let slot = match landing {
        Landing::Slot(slot) => slot.parent(),
        Landing::Bin | Landing::Refused(_) => None,
    };
    block
        .into_iter()
        .chain(slot)
        .all(|node| world.get_entity(node).is_ok())
}

/// Apply a drop. One undo step, or nothing at all.
pub(super) fn land(
    world: &mut World,
    source: DragSourceType,
    landing: Landing,
    frozen: &[(Entity, Vec2)],
) {
    if !drop_stands(world, source, &landing) {
        say_refused(world, "this block is gone: nothing was dropped".to_string());
        return;
    }
    let step: Result<Option<(Entity, Cow<'static, str>)>, String> = match (landing, source) {
        (Landing::Refused(reason), _) => Err(reason),
        (Landing::Bin, DragSourceType::Palette(_)) => Ok(None),
        (Landing::Bin, DragSourceType::Block(node)) => {
            delete_block(world, node);
            world.resource_mut::<SelectedNode>().set(None);
            commit_edit_now(world, "Delete Block");
            return;
        }
        (Landing::Slot(slot), DragSourceType::Block(node)) => {
            move_block(world, node, slot).map(|()| Some((node, "Move Block".into())))
        }
        (Landing::Slot(slot), DragSourceType::Palette(item)) => place_new(world, item, slot)
            .map(|made| Some((made, format!("Add {}", item.label()).into()))),
    };
    match step {
        Err(reason) => say_refused(world, reason),
        Ok(None) => {}
        Ok(Some((node, label))) => {
            // The hats still laid out by the flow keep where they were seen:
            // one that moved has left the flow, and the rest must not close up
            // underneath it.
            for (hat, at) in frozen {
                if let Some(mut event) = world.get_mut::<EventNode>(*hat) {
                    if event.position.is_none() && *hat != node {
                        event.position = Some(*at);
                    }
                }
            }
            reveal(world, node);
            world.resource_mut::<SelectedNode>().set(Some(node));
            commit_edit_now(world, label);
        }
    }
}

/// Follow the pointer: begin, aim, drop.
pub(crate) fn drive_canvas_drag(mut stage: DragStage) {
    if let Some(held) = stage.drag.cancelled.take() {
        stage.put_back(&held);
    }
    let Some(at) = stage.pointer.at else {
        return;
    };
    let released = !stage.buttons.pressed(MouseButton::Left);
    if let Some(mut held) = stage.drag.held.take() {
        if released {
            stage.finish(held, at);
            return;
        }
        held.aim = stage.aim(at, held.aim);
        let (left, top, scale) = ghost_place(at, held.grab, stage.view.zoom);
        for (_, mut node, mut transform) in &mut stage.ghosts {
            node.left = left;
            node.top = top;
            transform.scale = scale;
        }
        stage.drag.held = Some(held);
        return;
    }
    let Some(candidate) = stage.drag.candidate else {
        return;
    };
    if released {
        stage.drag.candidate = None;
        if let DragSourceType::Block(node) = candidate.source {
            stage.commands.queue(move |world: &mut World| {
                if world.get_entity(node).is_ok() {
                    reveal(world, node);
                    world.resource_mut::<SelectedNode>().set(Some(node));
                }
            });
        }
        return;
    }
    if candidate.draggable && candidate.at.distance(at) >= DRAG_START {
        stage.drag.candidate = None;
        stage.begin(candidate, at);
    }
}

/// Light what fits. Illegal slots stay dark.
pub(crate) fn paint_canvas_slots(
    drag: Res<CanvasDrag>,
    mut slots: Query<(
        Entity,
        &CanvasSlot,
        &mut Node,
        &mut BackgroundColor,
        Option<&mut Outline>,
    )>,
    mut was_held: Local<bool>,
) {
    let held = drag.held.as_ref();
    if held.is_none() && !*was_held {
        return;
    }
    *was_held = held.is_some();
    for (entity, slot, mut node, mut fill, outline) in &mut slots {
        let lit = held.is_some() && slot.legal;
        let aimed = held.is_some_and(|held| held.aim == Some(entity));
        match slot.look {
            SlotLookType::Gap => {
                let (height, overlap, colour) = match (aimed, lit) {
                    (true, _) => (GAP_H, 0.0, BLOCK_SLOT_GAP),
                    (false, true) => (LIT_H, -LIT_H, BLOCK_SLOT_LIT),
                    (false, false) => (0.0, 0.0, Color::NONE),
                };
                let (height, margin) = (px(height), UiRect::top(px(overlap)));
                if node.height != height {
                    node.height = height;
                }
                if node.margin != margin {
                    node.margin = margin;
                }
                if fill.0 != colour {
                    fill.0 = colour;
                }
            }
            SlotLookType::Socket => {
                let colour = if lit { BLOCK_SLOT_LIT } else { Color::NONE };
                if let Some(mut outline) = outline {
                    let width = px(if aimed { 3 } else { 2 });
                    if outline.color != colour {
                        outline.color = colour;
                    }
                    if outline.width != width {
                        outline.width = width;
                    }
                }
            }
        }
    }
}

/// Whether anything is in hand. The canvas does not rebuild under a drag.
pub(crate) fn holding(drag: &CanvasDrag) -> bool {
    drag.held.is_some()
}

/// The handlers still placed by the flow, for a drop to freeze.
impl CanvasBoard {
    fn auto_placed(&self) -> impl Iterator<Item = (Entity, Vec2)> + '_ {
        self.flowed.iter().copied()
    }
}
