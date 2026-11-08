//! Views to nodes: nested flex, with only a handler absolutely positioned.
//!
//! A stack is a column and a C-block a left bar beside an indented column, so
//! the layout engine does the sizing and a block grows with what is in it. The
//! puzzle pieces are small child nodes: the notch is painted in the colour of
//! the block above it, so that block's tab reads as plugged in, and the last
//! block of a stack keeps its tab out in the open.
//!
//! The ghost and the palette are painted by the same code with `live` off.

use std::f32::consts::FRAC_PI_4;

use bevy::{
    ecs::relationship::RelatedSpawnerCommands,
    picking::{hover::Hovered, Pickable},
    prelude::*,
    ui_widgets::{observe, Button},
};
use nova_ui::prelude::*;

use super::{
    drag::{CanvasBlock, CanvasChip, CanvasSlot, SlotLookType},
    on_chip_edit, on_more_chip,
    view::{BlockView, ChipType, ChipView, OutlineType, PieceType, StackView},
    MoreChip,
};
use crate::{
    palette::{
        BLOCK_CHIP, BLOCK_FAULT, BLOCK_LITERAL, BLOCK_LITERAL_TEXT, BLOCK_SOCKET, BLOCK_TEXT,
        BLOCK_TEXT_DIM,
    },
    ui::{
        inspector::{
            on_inspector_drag, on_inspector_drag_end, on_inspector_drag_start, on_inspector_flag,
            InspectorDrag, InspectorFlag, InspectorHint, InspectorPick, InspectorRef,
        },
        pick::{on_open_choice_list, on_open_ref_list},
    },
};

/// Corner radius of a stack block.
const RADIUS: f32 = 3.0;
/// Corner radius of a handler's top.
const HAT_RADIUS: f32 = 14.0;
/// How far a tab or a notch stands in from the block's left edge.
const TAB_LEFT: f32 = 14.0;
/// A tab's width.
const TAB_W: f32 = 20.0;
/// A tab's height.
const TAB_H: f32 = 4.0;
/// The width of a C-block's left bar.
const BAR_W: f32 = 12.0;
/// The height of a C-block's foot.
const FOOT_H: f32 = 10.0;
/// The tallest a stack's empty inside stands.
const EMPTY_INSIDE_H: f32 = 16.0;
/// The dark edge along a block's foot, which parts two blocks of one colour.
const EDGE_H: f32 = 1.0;
/// How much darker that edge is.
const EDGE_DARK: f32 = 0.12;
/// A block's text size.
const FONT: f32 = 13.0;
/// A field name's text size.
const SMALL_FONT: f32 = 10.0;

/// What one paint pass is drawing and what it made.
pub(crate) struct Paint {
    /// Off for a ghost or a palette preview: nothing is pressed, nothing
    /// is a target.
    pub(crate) live: bool,
    /// The chips made, in [`BlockView::chips`] order.
    pub(crate) chips: Vec<Entity>,
}

impl Paint {
    /// A pass that draws working blocks.
    pub(crate) fn live() -> Self {
        Self {
            live: true,
            chips: Vec::new(),
        }
    }

    /// A pass that draws a picture of blocks.
    pub(crate) fn picture() -> Self {
        Self {
            live: false,
            chips: Vec::new(),
        }
    }
}

type List<'a> = RelatedSpawnerCommands<'a, ChildOf>;

fn words(text: &str, size: f32, colour: Color) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_string()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextColor(colour),
        Pickable::IGNORE,
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    )
}

fn line() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(5),
        ..default()
    }
}

/// What every block node wears: a target when live, a picture when not.
fn mark_block(block: &mut EntityCommands, view: &BlockView, paint: &Paint) {
    match view.node.filter(|_| paint.live) {
        Some(node) => {
            block.insert((
                CanvasBlock {
                    node,
                    faulted: view.fault.is_some(),
                },
                Outline {
                    width: px(2),
                    offset: px(0),
                    color: Color::NONE,
                },
            ));
        }
        None => {
            block.insert(Pickable::IGNORE);
        }
    }
}

/// A gap in a stack. No height until a drag needs it.
fn paint_gap(list: &mut List, name: String, slot: crate::event::BlockSlotType, paint: &Paint) {
    if !paint.live {
        return;
    }
    list.spawn((
        Name::new(name),
        CanvasSlot {
            slot,
            look: SlotLookType::Gap,
            legal: false,
        },
        Node {
            height: px(0),
            align_self: AlignSelf::Stretch,
            ..default()
        },
        BackgroundColor(Color::NONE),
        Pickable::IGNORE,
    ));
}

fn paint_chip(list: &mut List, chip: &ChipView, literal: bool, paint: &mut Paint, name: &str) {
    if !chip.label.is_empty() {
        list.spawn(words(&chip.label, SMALL_FONT, BLOCK_TEXT_DIM));
    }
    let fill = if literal { BLOCK_LITERAL } else { BLOCK_CHIP };
    let ink = if literal {
        BLOCK_LITERAL_TEXT
    } else {
        BLOCK_TEXT
    };
    let radius = if literal { 10.0 } else { 4.0 };
    let shown = chip.shown();
    let mut entity = list.spawn((
        Name::new(format!("Chip {name}")),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(3),
            padding: UiRect::horizontal(px(6)),
            min_height: px(20),
            min_width: px(if literal { 24 } else { 18 }),
            border_radius: BorderRadius::all(px(radius)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(fill),
    ));
    entity.with_children(|inner| {
        inner.spawn((ChipText, words(&shown, FONT - 1.0, ink)));
        if !chip.unit.is_empty() {
            inner.spawn(words(chip.unit, SMALL_FONT, BLOCK_TEXT_DIM));
        }
    });
    let Some(field) = chip.field.clone().filter(|_| paint.live) else {
        entity.insert(Pickable::IGNORE);
        return;
    };
    entity.insert((
        CanvasChip {
            whole: chip.whole.clone(),
        },
        field,
        Button,
    ));
    match &chip.kind {
        ChipType::Text => {
            entity.insert(observe(on_chip_edit));
        }
        ChipType::Number(rule) => {
            entity.insert((
                InspectorDrag::new(*rule),
                observe(on_chip_edit),
                observe(on_inspector_drag_start),
                observe(on_inspector_drag),
                observe(on_inspector_drag_end),
            ));
        }
        ChipType::Pick { options, hints } => {
            entity.insert((
                InspectorPick {
                    options: options.clone(),
                    hints: hints.clone(),
                },
                observe(on_open_choice_list),
            ));
        }
        ChipType::Flag => {
            entity.insert((InspectorFlag, observe(on_inspector_flag)));
        }
        ChipType::Ref(offers) => {
            entity.insert((InspectorRef { offers: *offers }, observe(on_open_ref_list)));
        }
    }
    paint.chips.push(entity.id());
}

/// The chip that opens a block's whole field list.
fn paint_more(list: &mut List, node: Entity, path: &str, paint: &Paint) {
    let mut more = list.spawn((
        Name::new(format!("More {path}")),
        Node {
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(px(6)),
            min_height: px(20),
            min_width: px(18),
            border_radius: BorderRadius::all(px(4)),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(BLOCK_CHIP),
        children![words("...", FONT - 1.0, BLOCK_TEXT)],
    ));
    if paint.live {
        more.insert((MoreChip(node), Button, observe(on_more_chip)));
    } else {
        more.insert(Pickable::IGNORE);
    }
}

/// The text inside a chip, for the repaint that keeps the chip.
#[derive(Component)]
pub(crate) struct ChipText;

/// The two pointed ends of a boolean: squares turned on their corners, half
/// out of the node. They ride the node, so the pointer's pick is the node's.
pub(crate) fn paint_points(list: &mut List, height: f32, colour: Color) {
    let side = height * std::f32::consts::FRAC_1_SQRT_2;
    for left in [true, false] {
        let mut node = Node {
            position_type: PositionType::Absolute,
            width: px(side),
            height: px(side),
            top: px((height - side) / 2.0),
            ..default()
        };
        if left {
            node.left = px(-side / 2.0);
        } else {
            node.right = px(-side / 2.0);
        }
        list.spawn((
            node,
            UiTransform {
                rotation: Rot2::radians(FRAC_PI_4),
                ..default()
            },
            BackgroundColor(colour),
            Pickable::IGNORE,
        ));
    }
}

fn paint_pieces(list: &mut List, view: &BlockView, paint: &mut Paint) {
    let mut chips = 0;
    for (place, piece) in view.head.iter().enumerate() {
        match piece {
            PieceType::Word(text) => {
                list.spawn(words(text, FONT, BLOCK_TEXT));
            }
            PieceType::Chip(chip) => {
                chips += 1;
                paint_chip(list, chip, false, paint, &format!("{} {chips}", view.path));
            }
            PieceType::Literal { slot, chip } => {
                chips += 1;
                let name = format!("{} {chips}", view.path);
                let mut holder = list.spawn((
                    Name::new(format!("Socket {name}")),
                    Node {
                        flex_shrink: 0.0,
                        border_radius: BorderRadius::all(px(10)),
                        ..default()
                    },
                ));
                if paint.live {
                    holder.insert((
                        CanvasSlot {
                            slot: *slot,
                            look: SlotLookType::Socket,
                            legal: false,
                        },
                        Outline {
                            width: px(2),
                            offset: px(1),
                            color: Color::NONE,
                        },
                    ));
                }
                holder.with_children(|inner| paint_chip(inner, chip, true, paint, &name));
            }
            PieceType::Empty { slot, wants } => {
                let height = 22.0;
                let mut socket = list.spawn((
                    Name::new(format!("Socket {} {wants} {place}", view.path)),
                    Node {
                        height: px(height),
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(px(4)),
                        margin: UiRect::horizontal(px(height / 2.0)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(BLOCK_SOCKET),
                ));
                if paint.live {
                    socket.insert((
                        CanvasSlot {
                            slot: *slot,
                            look: SlotLookType::Socket,
                            legal: false,
                        },
                        Outline {
                            width: px(2),
                            offset: px(1),
                            color: Color::NONE,
                        },
                    ));
                }
                socket.with_children(|inner| {
                    paint_points(inner, height, BLOCK_SOCKET);
                    inner.spawn(words(wants, SMALL_FONT, BLOCK_TEXT_DIM));
                });
            }
            PieceType::Block(block) => paint_pill(list, block, paint),
            PieceType::More(node) => paint_more(list, *node, &view.path, paint),
        }
    }
    if let Some(fault) = view.fault.as_ref().filter(|_| paint.live) {
        list.spawn((
            Name::new(format!("Fault {}", view.path)),
            Node {
                width: px(16),
                height: px(16),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(8)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(BLOCK_FAULT),
            Hovered::default(),
            InspectorHint {
                title: "Problem".to_string(),
                body: fault.clone(),
            },
            children![words("!", 12.0, BLOCK_TEXT)],
        ));
    }
}

/// A boolean or a reporter: a single pointed or rounded line.
fn paint_pill(list: &mut List, view: &BlockView, paint: &mut Paint) {
    let height = view.pill_height();
    let boolean = view.outline == OutlineType::Boolean;
    let mut node = Node {
        min_height: px(height),
        padding: UiRect::horizontal(px(if boolean { 2 } else { 8 })),
        flex_shrink: 0.0,
        ..line()
    };
    if boolean {
        node.margin = UiRect::horizontal(px(height / 2.0));
        node.height = px(height);
    } else {
        node.border_radius = BorderRadius::all(px(height / 2.0));
    }
    let mut block = list.spawn((
        Name::new(format!("Block {}", view.path)),
        node,
        BackgroundColor(view.colour),
    ));
    mark_block(&mut block, view, paint);
    block.with_children(|inner| {
        if boolean {
            paint_points(inner, height, view.colour);
        }
        paint_pieces(inner, view, paint);
    });
}

fn paint_stack(list: &mut List, stack: &StackView, above: Color, path: &str, paint: &mut Paint) {
    let mut notch = above;
    for (index, block) in stack.blocks.iter().enumerate() {
        paint_gap(
            list,
            format!("Slot {path} {index}"),
            stack.slot(index),
            paint,
        );
        let last = index + 1 == stack.blocks.len();
        paint_stacked(list, block, Some(notch), last, paint);
        notch = block.colour;
    }
    paint_gap(
        list,
        format!("Slot {path} {}", stack.blocks.len()),
        stack.slot(stack.blocks.len()),
        paint,
    );
}

/// The filters and the gate, as lines of the head.
fn paint_head_rows(list: &mut List, view: &BlockView, paint: &mut Paint) {
    let row = |colour: Color| {
        (
            Node {
                padding: UiRect::axes(px(10), px(2)),
                ..line()
            },
            BackgroundColor(colour),
        )
    };
    if let Some(step) = view.gate_slot {
        match &view.gate {
            Some(gate) => paint_stacked(list, gate, None, false, paint),
            None => paint_gap(
                list,
                format!("Slot {} gate", view.path),
                crate::event::BlockSlotType::Gate { step },
                paint,
            ),
        }
    }
    if let Some((parent, filters)) = &view.filters {
        for (index, filter) in filters.iter().enumerate() {
            paint_gap(
                list,
                format!("Slot {} filter {index}", view.path),
                crate::event::BlockSlotType::Filter {
                    parent: *parent,
                    index,
                },
                paint,
            );
            list.spawn(row(view.colour)).with_children(|line| {
                line.spawn(words("only if", SMALL_FONT, BLOCK_TEXT_DIM));
                paint_pill(line, filter, paint);
            });
        }
        paint_gap(
            list,
            format!("Slot {} filter {}", view.path, filters.len()),
            crate::event::BlockSlotType::Filter {
                parent: *parent,
                index: filters.len(),
            },
            paint,
        );
    }
}

/// A block that stands in a stack: a hat, a stack block, a C-block, a gate.
///
/// Each block is as wide as its own line, so a stack has the ragged right edge
/// that tells one block from the next. Only the parts of one block stretch to
/// each other.
pub(crate) fn paint_stacked(
    list: &mut List,
    view: &BlockView,
    notch: Option<Color>,
    last: bool,
    paint: &mut Paint,
) {
    if matches!(view.outline, OutlineType::Boolean | OutlineType::Reporter) {
        paint_pill(list, view, paint);
        return;
    }
    let hat = view.outline == OutlineType::Hat;
    let chain = view.outline == OutlineType::Chain;
    let top = if hat { HAT_RADIUS } else { RADIUS };
    let rows = view.gate.is_some()
        || view.gate_slot.is_some()
        || view
            .filters
            .as_ref()
            .is_some_and(|(_, list)| !list.is_empty());
    let corner = if chain || rows { 0.0 } else { RADIUS };
    let mut block = list.spawn((
        Name::new(format!("Block {}", view.path)),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: if hat {
                AlignItems::FlexStart
            } else {
                AlignItems::Stretch
            },
            flex_shrink: 0.0,
            ..default()
        },
    ));
    mark_block(&mut block, view, paint);
    block.with_children(|inner| {
        inner
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|top_part| {
                top_part
                    .spawn((
                        Node {
                            padding: UiRect::axes(px(10), px(if hat { 8 } else { 5 })),
                            min_height: px(30),
                            border: UiRect::bottom(px(EDGE_H)),
                            border_radius: BorderRadius {
                                top_left: px(top),
                                top_right: px(top),
                                bottom_left: px(corner),
                                bottom_right: px(corner),
                            },
                            ..line()
                        },
                        BackgroundColor(view.colour),
                        BorderColor::all(view.colour.darker(EDGE_DARK)),
                    ))
                    .with_children(|head| {
                        if let Some(colour) = notch {
                            head.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(TAB_LEFT),
                                    top: px(0),
                                    width: px(TAB_W),
                                    height: px(TAB_H),
                                    border_radius: BorderRadius::bottom(px(2)),
                                    ..default()
                                },
                                BackgroundColor(colour),
                                Pickable::IGNORE,
                            ));
                        }
                        paint_pieces(head, view, paint);
                    });
                paint_head_rows(top_part, view, paint);
            });
        match (&view.stack, chain) {
            (Some(stack), true) => {
                inner
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Stretch,
                        ..default()
                    })
                    .with_children(|body| {
                        body.spawn((
                            Node {
                                width: px(BAR_W),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            BackgroundColor(view.colour),
                        ));
                        body.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexStart,
                            flex_grow: 1.0,
                            min_height: px(EMPTY_INSIDE_H),
                            ..default()
                        })
                        .with_children(|column| {
                            paint_stack(column, stack, view.colour, &view.path, paint);
                        });
                    });
                inner.spawn((
                    Node {
                        height: px(FOOT_H),
                        border_radius: BorderRadius {
                            bottom_left: px(RADIUS),
                            bottom_right: px(RADIUS),
                            top_right: px(RADIUS),
                            top_left: px(0.0),
                        },
                        ..default()
                    },
                    BackgroundColor(view.colour),
                ));
                if last {
                    paint_tab(inner, view.colour);
                }
            }
            (Some(stack), false) => {
                if stack.blocks.is_empty() {
                    paint_tab(inner, view.colour);
                }
                paint_stack(inner, stack, view.colour, &view.path, paint);
            }
            (None, _) => {
                if last {
                    paint_tab(inner, view.colour);
                }
            }
        }
    });
}

fn paint_tab(list: &mut List, colour: Color) {
    list.spawn((
        Node {
            width: px(TAB_W),
            height: px(TAB_H),
            margin: UiRect::left(px(TAB_LEFT)),
            align_self: AlignSelf::FlexStart,
            border_radius: BorderRadius::bottom(px(2)),
            ..default()
        },
        BackgroundColor(colour),
        Pickable::IGNORE,
    ));
}
