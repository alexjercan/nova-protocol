//! The script canvas: a VIEW over [`ScriptNodes`], drawn as puzzle blocks.
//!
//! The node tree stays the document. Every gesture here ends in a verb of
//! [`crate::event`] and one undo step, so this module holds no script state of
//! its own: pan and zoom in [`CanvasView`], and in [`CanvasBoard`] what was last
//! drawn, kept to compare against.
//!
//! A handler is rebuilt when its shape changes and repainted when only a value
//! did, which keeps a chip alive under a scrub. See [`BlockView::same_shape`].

mod drag;
pub(crate) mod menu;
mod paint;
mod palette;
mod popup;
#[cfg(test)]
mod tests;
mod view;

use std::collections::HashMap;

use bevy::{
    ecs::{hierarchy::ChildOf, lifecycle::RemovedComponents},
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::{hover::Hovered, Pickable},
    prelude::*,
    ui::UiGlobalTransform,
    ui_widgets::Activate,
};
pub(crate) use drag::CanvasDrag;
use drag::{
    drive_canvas_drag, holding, on_canvas_press, paint_canvas_slots, rect_of, CanvasBlock,
    CanvasChip,
};
use nova_ui::prelude::*;
use paint::{paint_stacked, ChipText, Paint};
use palette::{spawn_block_palette, sync_palette_filter, sync_palette_rail, PaletteFolds};
pub(crate) use palette::{PaletteEntry, PaletteGroup};
pub(crate) use popup::{close_block_popup, BlockPopup, PopupLeaveType, POPUP_W};
use popup::{on_more_chip, place_block_popup, settle_block_popup, MoreChip};
use view::{BlockView, ViewBuilder};

use crate::{
    camera::ViewportPointer,
    config::{EditorSays, EditorViewType, SelectedNode},
    event::{
        delete_block, expr_choice, ActionNode, BlockShapeType, EventNode, ExpressionNode,
        FilterNode, GateNode, ScriptNodes, StepNode,
    },
    frame::FRAME_KEY,
    history::commit_edit,
    node::{EditContext, NodeId},
    palette::{BLOCK_FAULT, BLOCK_GROUND, BLOCK_MARKED, BLOCK_TEXT_DIM},
    problems::Problems,
    ui::inspector::{apply_inspector_edits, InspectorField},
    ExampleStates,
};

/// The closest the canvas zooms out.
const ZOOM_MIN: f32 = 0.4;
/// The closest it zooms in.
const ZOOM_MAX: f32 = 1.6;
/// One wheel line of zoom.
const ZOOM_STEP: f32 = 0.1;
/// One wheel line of scroll, in pixels.
const WHEEL_LINE: f32 = 40.0;
/// The margin the flow leaves around the handlers.
const FLOW_MARGIN: f32 = 24.0;
/// The gap between two flowed handlers.
const FLOW_GAP: f32 = 28.0;
/// How far down a flow column runs before the next one starts.
const FLOW_COLUMN_H: f32 = 1400.0;
/// The space kept round what F frames.
const FRAME_MARGIN: f32 = 32.0;

/// How far a pointer may travel over a chip and still be a click, in pixels.
const CHIP_TRAVEL: f32 = 3.0;

/// The canvas beside the palette, shown in Events only.
#[derive(Component)]
pub(crate) struct CanvasArea;

/// The clipped window onto the canvas.
#[derive(Component)]
pub(crate) struct CanvasViewport;

/// The node pan and zoom move. It has no size, so a scale pivots on its corner.
#[derive(Component)]
pub(crate) struct CanvasWorld;

/// One handler's absolute root and what it last drew.
#[derive(Component)]
pub(crate) struct HandlerRoot {
    view: BlockView,
    chips: Vec<Entity>,
}

/// Where the canvas is looking.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub(crate) struct CanvasView {
    /// How far the world has moved, in pixels.
    pub(crate) pan: Vec2,
    /// How large it is drawn.
    pub(crate) zoom: f32,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

/// What the canvas last drew.
#[derive(Resource, Default)]
pub(crate) struct CanvasBoard {
    /// Each handler's root.
    roots: HashMap<Entity, Entity>,
    /// What each block's problem says.
    pub(crate) faults: HashMap<Entity, String>,
    /// Where the flow put each handler with no stored position.
    pub(crate) flowed: Vec<(Entity, Vec2)>,
    /// Whether the script has moved since the last draw.
    stale: bool,
    /// How many blocks are drawn.
    pub(crate) blocks: usize,
    /// The block a Problems row asked to see, until it can be framed.
    pub(crate) frame: Option<Entity>,
}

/// The line over the canvas: how to start a script on an empty one, and why
/// the block in hand will not stand where it is.
#[derive(Component)]
pub(crate) struct CanvasHint;

/// What an empty scenario's canvas says.
pub(crate) const EMPTY_CANVAS_HINT: &str = "Drag an Events block here to start a script";

/// What the canvas says while a block that cannot stand alone is over bare
/// canvas.
pub(crate) const LOOSE_BLOCK_HINT: &str =
    "Only an Events block stands alone: drop this on a lit slot";

/// The inline editor a text chip opens.
#[derive(Component)]
pub(crate) struct ChipInput;

/// The area the mount holds as its first child.
pub(crate) fn spawn_canvas_area(mount: &mut ChildSpawnerCommands) {
    mount
        .spawn((
            Name::new("Canvas Area"),
            CanvasArea,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_width: px(0),
                min_height: px(0),
                ..default()
            },
        ))
        .with_children(|area| {
            spawn_block_palette(area);
            area.spawn((
                Name::new("Block Canvas"),
                CanvasViewport,
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    min_height: px(0),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(BLOCK_GROUND),
                // Marked a viewport, and clipped so nothing scrolls: with the
                // pointer here the wheel is the canvas's, and the shared driver
                // would otherwise roll every other list on screen.
                ScrollViewport,
                ScrollPosition::default(),
                Hovered::default(),
            ))
            .with_children(|viewport| {
                viewport.spawn((
                    Name::new("Canvas Hint"),
                    CanvasHint,
                    Pickable::IGNORE,
                    ZIndex(1),
                    Node {
                        display: Display::None,
                        position_type: PositionType::Absolute,
                        top: px(10),
                        width: percent(100),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    children![(
                        UiText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(BLOCK_TEXT_DIM),
                        Pickable::IGNORE,
                    )],
                ));
                viewport.spawn((
                    Name::new("Canvas World"),
                    CanvasWorld,
                    UiTransform::default(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: px(0),
                        height: px(0),
                        ..default()
                    },
                ));
            });
        });
}

/// What each block's problem says, first finding first.
fn block_faults(problems: &Problems) -> HashMap<Entity, String> {
    let mut faults = HashMap::new();
    for problem in &problems.list {
        if let Some(node) = problem.node {
            faults
                .entry(node)
                .or_insert_with(|| problem.message.clone());
        }
    }
    faults
}

/// Whether the script's nodes moved this frame.
type ScriptMoved = Or<(
    Changed<NodeId>,
    Changed<EventNode>,
    Changed<ActionNode>,
    Changed<StepNode>,
    Changed<GateNode>,
    Changed<FilterNode>,
    Changed<ExpressionNode>,
    Added<CanvasWorld>,
)>;

#[expect(
    clippy::too_many_arguments,
    reason = "one read per thing a block shows"
)]
fn sync_canvas(
    mut commands: Commands,
    rail: Res<EditorViewType>,
    context: Res<EditContext>,
    problems: Res<Problems>,
    drag: Res<CanvasDrag>,
    mut board: ResMut<CanvasBoard>,
    script: ScriptNodes,
    moved: Query<(), ScriptMoved>,
    mut gone: RemovedComponents<NodeId>,
    worlds: Query<Entity, With<CanvasWorld>>,
    mut roots: Query<&mut HandlerRoot>,
    mut chips: Query<(&mut CanvasChip, &Children)>,
    mut texts: Query<&mut Text, With<ChipText>>,
    inputs: Query<(), With<ChipInput>>,
) {
    let faults = block_faults(&problems);
    let removed = gone.read().count() > 0;
    if removed
        || !moved.is_empty()
        || rail.is_changed()
        || context.is_changed()
        || faults != board.faults
    {
        board.stale = true;
    }
    // A rebuild would take the open field and what is typed in it.
    if !board.stale || !rail.is_events() || holding(&drag) || !inputs.is_empty() {
        return;
    }
    let Ok(world) = worlds.single() else {
        return;
    };
    board.stale = false;
    board.faults = faults;
    let builder_faults = board.faults.clone();
    let builder = ViewBuilder {
        script: &script,
        faults: &builder_faults,
    };
    let hats = context
        .scenario()
        .map(|scenario| script.events_of(scenario))
        .unwrap_or_default();
    let mut next = HashMap::new();
    let mut blocks = 0;
    for hat in &hats {
        let Some(view) = builder.hat(*hat) else {
            continue;
        };
        blocks += view.count();
        let root = board.roots.get(hat).copied();
        let Some(root) = root.filter(|root| roots.contains(*root)) else {
            let made = spawn_root(&mut commands, world, view);
            next.insert(*hat, made);
            continue;
        };
        next.insert(*hat, root);
        let Ok(mut held) = roots.get_mut(root) else {
            continue;
        };
        if held.view == view {
            continue;
        }
        if held.view.same_shape(&view) {
            repaint(&held, &view, &mut chips, &mut texts);
            held.view = view;
        } else {
            commands.entity(root).despawn_related::<Children>();
            let mut paint = Paint::live();
            commands.entity(root).with_children(|list| {
                paint_stacked(list, &view, None, true, &mut paint);
            });
            held.chips = paint.chips;
            held.view = view;
        }
    }
    for (hat, root) in &board.roots {
        if !next.contains_key(hat) {
            commands.entity(*root).try_despawn();
        }
    }
    board.roots = next;
    board.blocks = blocks;
}

fn spawn_root(commands: &mut Commands, world: Entity, view: BlockView) -> Entity {
    let mut paint = Paint::live();
    let root = commands
        .spawn((
            Name::new(format!("Handler {}", view.path)),
            ChildOf(world),
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                ..default()
            },
        ))
        .with_children(|list| paint_stacked(list, &view, None, true, &mut paint))
        .id();
    commands.entity(root).insert(HandlerRoot {
        view,
        chips: paint.chips,
    });
    root
}

fn repaint(
    root: &HandlerRoot,
    view: &BlockView,
    chips: &mut Query<(&mut CanvasChip, &Children)>,
    texts: &mut Query<&mut Text, With<ChipText>>,
) {
    for (entity, chip) in root.chips.iter().zip(view.chips()) {
        let Ok((mut held, children)) = chips.get_mut(*entity) else {
            continue;
        };
        if held.whole != chip.whole {
            held.whole.clone_from(&chip.whole);
        }
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                let shown = chip.shown();
                if text.0 != shown {
                    text.0 = shown;
                }
            }
        }
    }
}

/// Put each handler where it stands, flowing the ones with no stored position.
fn layout_canvas(
    rail: Res<EditorViewType>,
    context: Res<EditContext>,
    script: ScriptNodes,
    mut board: ResMut<CanvasBoard>,
    mut roots: Query<(&HandlerRoot, &ComputedNode, &mut Node)>,
) {
    if !rail.is_events() {
        return;
    }
    let Some(scenario) = context.scenario() else {
        return;
    };
    let mut cursor = Vec2::splat(FLOW_MARGIN);
    let mut widest: f32 = 0.0;
    let mut flowed = Vec::new();
    for hat in script.events_of(scenario) {
        let Some(root) = board.roots.get(&hat) else {
            continue;
        };
        let Ok((_, computed, mut node)) = roots.get_mut(*root) else {
            continue;
        };
        let size = computed.size() * computed.inverse_scale_factor;
        let at = match script.event(hat).and_then(|event| event.position) {
            Some(at) => at,
            None => {
                if cursor.y > FLOW_MARGIN && cursor.y + size.y > FLOW_COLUMN_H {
                    cursor = Vec2::new(cursor.x + widest + FLOW_GAP, FLOW_MARGIN);
                    widest = 0.0;
                }
                let at = cursor;
                cursor.y += size.y + FLOW_GAP;
                widest = widest.max(size.x);
                flowed.push((hat, at));
                at
            }
        };
        let (left, top) = (px(at.x), px(at.y));
        if node.left != left {
            node.left = left;
        }
        if node.top != top {
            node.top = top;
        }
    }
    if board.flowed != flowed {
        board.flowed = flowed;
    }
}

/// What the canvas hint says, if anything: the loose-block warning while one is
/// in hand over bare canvas, else the start line while there is no script.
pub(crate) fn canvas_hint_text(
    held: Option<(BlockShapeType, bool)>,
    over_canvas: bool,
    has_script: bool,
) -> Option<&'static str> {
    match held {
        Some((shape, aimed)) => {
            (shape != BlockShapeType::Hat && over_canvas && !aimed).then_some(LOOSE_BLOCK_HINT)
        }
        None => (!has_script).then_some(EMPTY_CANVAS_HINT),
    }
}

/// Show the canvas hint, and ring the ghost red while it would be refused.
fn sync_canvas_hint(
    rail: Res<EditorViewType>,
    board: Res<CanvasBoard>,
    drag: Res<CanvasDrag>,
    pointer: Res<ViewportPointer>,
    viewports: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasViewport>>,
    mut hints: Query<(&mut Node, &Children), With<CanvasHint>>,
    mut texts: Query<&mut Text>,
    mut ghosts: Query<&mut Outline, With<drag::CanvasGhost>>,
) {
    if !rail.is_events() {
        return;
    }
    let over_canvas = match (pointer.at, viewports.single()) {
        (Some(at), Ok((computed, transform))) => rect_of(computed, transform).contains(at),
        _ => false,
    };
    let held = drag
        .held
        .as_ref()
        .map(|held| (held.shape, held.aim.is_some()));
    let said = canvas_hint_text(held, over_canvas, !board.roots.is_empty());
    let refused = said == Some(LOOSE_BLOCK_HINT);
    for mut outline in &mut ghosts {
        let colour = if refused { BLOCK_FAULT } else { Color::NONE };
        if outline.color != colour {
            outline.color = colour;
        }
    }
    for (mut node, children) in &mut hints {
        let display = if said.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let Some(said) = said else {
            continue;
        };
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                if text.0 != said {
                    text.0 = said.to_string();
                }
            }
        }
    }
}

/// Mark what is selected and what is at fault.
fn paint_canvas_marks(
    selected: Res<SelectedNode>,
    mut blocks: Query<(&CanvasBlock, &mut Outline)>,
) {
    for (block, mut outline) in &mut blocks {
        let colour = if selected.contains(block.node) {
            BLOCK_MARKED
        } else if block.faulted {
            BLOCK_FAULT
        } else {
            Color::NONE
        };
        if outline.color != colour {
            outline.color = colour;
        }
    }
}

/// Pan, scroll, zoom.
#[expect(
    clippy::too_many_arguments,
    reason = "three devices and the node they move"
)]
fn drive_canvas_view(
    rail: Res<EditorViewType>,
    pointer: Res<ViewportPointer>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut view: ResMut<CanvasView>,
    viewports: Query<(&ComputedNode, &UiGlobalTransform, &Hovered), With<CanvasViewport>>,
    mut worlds: Query<&mut UiTransform, With<CanvasWorld>>,
    mut grabbed: Local<Option<Vec2>>,
    popup: Res<BlockPopup>,
) {
    let lines: f32 = wheel
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / WHEEL_LINE,
        })
        .sum();
    // An open pop-up stands on its chip, so the canvas holds still under it.
    if !rail.is_events() || popup.is_open() {
        *grabbed = None;
        return;
    }
    let Ok((computed, transform, hovered)) = viewports.single() else {
        return;
    };
    let rect = rect_of(computed, transform);
    let at = pointer.at;
    let panning = buttons.pressed(MouseButton::Middle)
        || buttons.pressed(MouseButton::Right)
        || (keys.pressed(KeyCode::Space) && buttons.pressed(MouseButton::Left));
    match (*grabbed, panning, at) {
        (Some(from), true, Some(at)) => {
            view.pan += at - from;
            *grabbed = Some(at);
        }
        (None, true, Some(at)) if hovered.get() && rect.contains(at) => *grabbed = Some(at),
        (_, false, _) => *grabbed = None,
        _ => {}
    }
    if lines != 0.0 && hovered.get() {
        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        if ctrl {
            let zoom = (view.zoom + lines * ZOOM_STEP).clamp(ZOOM_MIN, ZOOM_MAX);
            let anchor = at.unwrap_or(rect.center()) - rect.min;
            view.pan = anchor - (anchor - view.pan) * (zoom / view.zoom);
            view.zoom = zoom;
        } else {
            view.pan.y += lines * WHEEL_LINE;
        }
    }
    let shown = UiTransform {
        translation: Val2::new(px(view.pan.x), px(view.pan.y)),
        scale: Vec2::splat(view.zoom),
        ..default()
    };
    for mut transform in &mut worlds {
        if *transform != shown {
            *transform = shown;
        }
    }
}

/// F: bring the selected block, or else every handler, into view.
fn canvas_frame_key(
    keys: Res<ButtonInput<KeyCode>>,
    rail: Res<EditorViewType>,
    selected: Res<SelectedNode>,
    mut view: ResMut<CanvasView>,
    viewports: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasViewport>>,
    blocks: Query<(&CanvasBlock, &ComputedNode, &UiGlobalTransform)>,
    roots: Query<(&ComputedNode, &UiGlobalTransform), With<HandlerRoot>>,
) {
    if !rail.is_events() || !keys.just_pressed(FRAME_KEY) {
        return;
    }
    let Ok((computed, transform)) = viewports.single() else {
        return;
    };
    let target = selected
        .primary()
        .and_then(|node| block_rect(&blocks, node))
        .or_else(|| {
            roots
                .iter()
                .map(|(computed, transform)| rect_of(computed, transform))
                .reduce(|left, right| left.union(right))
        });
    if let Some(target) = target {
        frame_on(&mut view, rect_of(computed, transform), target);
    }
}

/// Where `node`'s block stands, once layout has given it a size.
fn block_rect(
    blocks: &Query<(&CanvasBlock, &ComputedNode, &UiGlobalTransform)>,
    node: Entity,
) -> Option<Rect> {
    blocks
        .iter()
        .find(|(block, ..)| block.node == node)
        .map(|(_, computed, transform)| rect_of(computed, transform))
        .filter(|rect| !rect.is_empty())
}

/// Move the view so `target` fills `window`, zooming in no further than 1:1.
fn frame_on(view: &mut CanvasView, window: Rect, target: Rect) {
    let room = (window.size() - Vec2::splat(FRAME_MARGIN * 2.0)).max(Vec2::ONE);
    let fit = (room / (target.size() / view.zoom).max(Vec2::ONE)).min_element();
    let zoom = fit.clamp(ZOOM_MIN, 1.0);
    let centre = (target.center() - window.min - view.pan) / view.zoom;
    view.zoom = zoom;
    view.pan = window.size() / 2.0 - centre * zoom;
}

/// Frame the block a Problems row named. Waits for the canvas: the row may have
/// switched to Events this frame, so the block has no place yet.
fn serve_canvas_frame(
    rail: Res<EditorViewType>,
    mut board: ResMut<CanvasBoard>,
    mut view: ResMut<CanvasView>,
    viewports: Query<(&ComputedNode, &UiGlobalTransform), With<CanvasViewport>>,
    blocks: Query<(&CanvasBlock, &ComputedNode, &UiGlobalTransform)>,
) {
    let Some(node) = board.frame else {
        return;
    };
    if !rail.is_events() {
        board.frame = None;
        return;
    }
    let Ok((computed, transform)) = viewports.single() else {
        return;
    };
    let window = rect_of(computed, transform);
    if let Some(target) = block_rect(&blocks, node).filter(|_| !window.is_empty()) {
        frame_on(&mut view, window, target);
        board.frame = None;
    }
}

/// Del on a reporter: its socket goes back to a stock `0`.
fn delete_reporter_key(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    selected: Res<SelectedNode>,
    drag: Res<CanvasDrag>,
    script: ScriptNodes,
) {
    if !keys.just_pressed(KeyCode::Delete) || drag.busy() {
        return;
    }
    // A comparison is its filter's whole body: it goes with the filter.
    let Some(node) = selected.primary().filter(|node| {
        script
            .expression(*node)
            .is_some_and(|expression| !expr_choice(&expression.kind).compares())
    }) else {
        return;
    };
    commands.queue(move |world: &mut World| {
        delete_block(world, node);
        world.resource_mut::<SelectedNode>().set(None);
    });
    commit_edit(&mut commands, "Delete Block");
}

/// A click on a text chip: open its field in place.
///
/// A click the pointer travelled through was a scrub, not a request to type.
pub(crate) fn on_chip_edit(
    activate: On<Activate>,
    mut commands: Commands,
    pointer: Res<ViewportPointer>,
    drag: Res<CanvasDrag>,
    chips: Query<(&CanvasChip, &InspectorField, Option<&Children>)>,
    inputs: Query<(), With<ChipInput>>,
) {
    let Ok((chip, field, children)) = chips.get(activate.entity) else {
        return;
    };
    let open = children.is_some_and(|list| list.iter().any(|child| inputs.contains(child)));
    let travelled = match (drag.pressed_at, pointer.at) {
        (Some(from), Some(to)) => from.distance(to) > CHIP_TRAVEL,
        _ => false,
    };
    if open || travelled {
        return;
    }
    let holder = commands
        .spawn((
            Name::new("Chip Input"),
            ChipInput,
            ChildOf(activate.entity),
            ZIndex(5),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(-3),
                width: px(180),
                ..default()
            },
        ))
        .id();
    commands.spawn((
        Name::new("Chip Input Field"),
        ChildOf(holder),
        field.clone(),
        text_field(TextFieldSpec::new(chip.whole.clone()).tool()),
        TextFieldFocused::at_end(&chip.whole),
    ));
}

/// Put an inline field away once it has lost the focus, saying why if the edit
/// was turned down: the block has nowhere to wear the error.
fn retire_chip_inputs(
    mut commands: Commands,
    inputs: Query<(Entity, &Children), With<ChipInput>>,
    fields: Query<Option<&TextFieldError>, (With<InspectorField>, Without<TextFieldFocused>)>,
    mut says: EditorSays,
) {
    for (holder, children) in &inputs {
        for child in children {
            let Ok(error) = fields.get(*child) else {
                continue;
            };
            if let Some(error) = error {
                says.flag(error.0.clone());
            }
            commands.entity(holder).despawn();
        }
    }
}

/// Wire the canvas into the app.
pub(crate) fn register(app: &mut App) {
    app.init_resource::<CanvasView>()
        .init_resource::<CanvasBoard>()
        .init_resource::<CanvasDrag>()
        .init_resource::<BlockPopup>()
        .init_resource::<PaletteFolds>()
        .init_resource::<PaletteGroup>()
        .init_resource::<menu::CanvasRightPress>()
        .add_observer(on_canvas_press)
        .add_systems(
            OnEnter(ExampleStates::Editor),
            |mut board: ResMut<CanvasBoard>,
             mut drag: ResMut<CanvasDrag>,
             mut popup: ResMut<BlockPopup>,
             mut folds: ResMut<PaletteFolds>,
             mut group: ResMut<PaletteGroup>| {
                *board = CanvasBoard::default();
                folds.0.clear();
                *group = PaletteGroup::default();
                *drag = CanvasDrag::default();
                *popup = BlockPopup::default();
            },
        )
        .add_systems(
            Update,
            (
                sync_canvas,
                layout_canvas,
                drive_canvas_drag,
                paint_canvas_slots,
                sync_canvas_hint,
                paint_canvas_marks,
                drive_canvas_view,
                menu::canvas_context_menu,
                canvas_frame_key.run_if(in_input_mode(InputMode::Normal)),
                delete_reporter_key.run_if(in_input_mode(InputMode::Normal)),
                serve_canvas_frame,
                sync_palette_filter,
                sync_palette_rail,
                retire_chip_inputs.after(apply_inspector_edits),
                settle_block_popup.after(apply_inspector_edits),
                place_block_popup,
            )
                .chain()
                .run_if(in_state(ExampleStates::Editor)),
        );
}
