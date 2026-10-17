//! The gallery's copy of the game's pane chrome: the interface card with its
//! segmented tabs, the side panel paint, the footer with legend and key
//! chips, the filter chip, and the small table helpers every page shares.
//! The recipes come from `nova_interface::pane` and `inventory::app`, which
//! are crate-private, so the gallery draws them again rather than importing
//! them.

use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::observe};
use nova_ui::{
    prelude::*,
    theme::UiColor,
    widget::{ThemedBorder, ThemedFill, ThemedText, UiText},
};

use crate::{
    data::thousands,
    icons::{icon_node, GalleryIcons, IconType},
    on_click, Click, TOAST,
};

/// Width cap of the interface card, as the game's `INTERFACE_MAX_PX`.
const INTERFACE_MAX_PX: f32 = 1520.0;
/// Height of a pane footer, as the game's `PANE_FOOTER_PX`.
const PANE_FOOTER_PX: f32 = 96.0;
/// Least width of a side panel, as the game's `SIDE_PANEL_MIN_PX`.
const SIDE_PANEL_MIN_PX: f32 = 300.0;

/// A themed text bundle.
pub fn text(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        UiText,
        Text::new(value.to_string()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
    )
}

/// A themed text that never wraps and clips instead.
pub fn text_nowrap(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        text(value, size, color),
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        Node {
            overflow: Overflow::clip(),
            min_width: px(0),
            ..default()
        },
    )
}

/// A fixed-width column cell, so numbers line up down a table.
pub fn cell(width: f32, value: &str, size: f32, color: UiColor, right: bool) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            justify_content: if right {
                JustifyContent::FlexEnd
            } else {
                JustifyContent::FlexStart
            },
            ..default()
        },
        children![text_nowrap(value, size, color)],
    )
}

/// A cell that takes the rest of the row.
pub fn grow_cell(value: &str, size: f32, color: UiColor) -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            overflow: Overflow::clip(),
            ..default()
        },
        children![text_nowrap(value, size, color)],
    )
}

/// A bare display meter at a fixed width.
pub fn bare_meter(width: f32, fraction: f32) -> impl Bundle {
    (
        Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        },
        children![slider_track(fraction.clamp(0.0, 1.0))],
    )
}

/// A `have / need` meter: a display-only track with the counts beside it.
pub fn meter(width: f32, have: u32, need: u32) -> impl Bundle {
    let fraction = if need == 0 {
        0.0
    } else {
        have as f32 / need as f32
    };
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
            flex_shrink: 0.0,
            ..default()
        },
        children![
            bare_meter(width, fraction),
            text(
                &format!("{} / {}", thousands(have), thousands(need)),
                12.0,
                UiColor::Body
            ),
        ],
    )
}

/// A table row's node.
pub fn row_node() -> Node {
    Node {
        width: percent(100),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(12),
        flex_shrink: 0.0,
        ..default()
    }
}

/// A row of controls.
pub fn control_row(justify: JustifyContent) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: justify,
        column_gap: px(12),
        flex_shrink: 0.0,
        ..default()
    }
}

/// A column that grows to fill its parent and clips its overflow.
pub fn column() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        min_height: px(0),
        flex_direction: FlexDirection::Column,
        row_gap: px(10),
        ..default()
    }
}

/// A table's header line: `(width, label)` per column, width 0 for the
/// column that grows.
pub fn table_head(p: &mut ChildSpawnerCommands, columns: &[(f32, &str)]) {
    p.spawn(Node {
        padding: UiRect::axes(px(13), px(2)),
        ..row_node()
    })
    .with_children(|r| {
        for (width, label) in columns {
            if *width == 0.0 {
                r.spawn(grow_cell(label, 11.0, UiColor::Label));
            } else {
                r.spawn(cell(*width, label, 11.0, UiColor::Label, false));
            }
        }
    });
}

/// A button sized for a narrow side panel or inspector.
pub fn compact_button(spec: ButtonSpec) -> impl Bundle {
    button(ButtonSpec {
        min_height: 22.0,
        font_size: 12.0,
        ..spec
    })
}

/// The fill and border of a pane's panel.
pub fn panel_paint() -> impl Bundle {
    (
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.08),
        BorderColor::all(Color::NONE),
        ThemedBorder::new(UiColor::Secondary),
    )
}

/// A viewer pane's side panel: 20% of the row beside the view, never
/// narrower than the game's minimum.
pub fn side_panel(overflow: Overflow) -> impl Bundle {
    (
        Node {
            width: percent(20),
            min_width: px(SIDE_PANEL_MIN_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow,
            ..default()
        },
        panel_paint(),
    )
}

/// A side panel at a given width share.
pub fn side_panel_wide(share: f32, overflow: Overflow) -> impl Bundle {
    (
        Node {
            width: percent(share),
            min_width: px(SIDE_PANEL_MIN_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow,
            ..default()
        },
        panel_paint(),
    )
}

/// A column panel inside a pane: the inventory column's paint.
pub fn column_panel() -> impl Bundle {
    (
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.35),
    )
}

/// A one-pixel rule between groups of a panel.
pub fn divider() -> impl Bundle {
    (
        Node {
            height: px(1),
            flex_shrink: 0.0,
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Secondary, 0.35),
    )
}

/// One legend entry: an icon beside its word.
pub fn legend_entry() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(4),
        ..default()
    }
}

/// The viewport a 3D scene fills beside a side panel.
pub fn viewport_node() -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        min_height: px(0),
        position_type: PositionType::Relative,
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The row a viewer pane splits into its view and its side panel.
pub fn view_split() -> Node {
    Node {
        flex_grow: 1.0,
        min_height: px(0),
        flex_direction: FlexDirection::Row,
        column_gap: px(12),
        ..default()
    }
}

/// A section heading inside a panel.
pub fn section(c: &mut ChildSpawnerCommands, label: &str) {
    c.spawn(text(&label.to_uppercase(), 11.0, UiColor::Label));
}

/// A `label  value` line.
pub fn kv(c: &mut ChildSpawnerCommands, label: &str, value: &str) {
    c.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|r| {
        r.spawn(cell(84.0, label, 12.0, UiColor::Label, false));
        r.spawn(grow_cell(value, 13.0, UiColor::Primary));
    });
}

/// A column fact: a small label over its value, as the inventory column's
/// Cargo and Credits.
pub fn fact(c: &mut ChildSpawnerCommands, label: &str, value: &str) {
    c.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(1),
        ..default()
    })
    .with_children(|f| {
        f.spawn(text(label, 11.0, UiColor::Label));
        f.spawn(text(value, 13.0, UiColor::Primary));
    });
}

/// A small tag chip, the badge family at the panel's smallest size.
pub fn tag(kind: BadgeKind, label: &str) -> impl Bundle {
    badge(kind, label)
}

/// A filter chip: the icon and the word, lit when current.
pub fn chip(
    bar: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    icon: Option<IconType>,
    label: &str,
    current: bool,
    name: String,
    click: Click,
) {
    let (color, fill, border) = if current {
        (UiColor::Accent, 0.25, 1.0)
    } else {
        (UiColor::Secondary, 0.08, 0.4)
    };
    bar.spawn((
        Name::new(name),
        clickable(
            (
                Node {
                    height: px(26),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    padding: UiRect::axes(px(8), px(0)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::alpha(color, fill),
                BorderColor::all(Color::NONE),
                ThemedBorder::alpha(color, border),
            ),
            click,
        ),
    ))
    .with_children(|chip| {
        if let Some(icon) = icon {
            chip.spawn(icon_node(icons, icon, icon.color(), 16.0));
        }
        chip.spawn((text(label, 12.0, UiColor::Body), Pickable::IGNORE));
    });
}

/// An interface card: the game's pane shell with `title` in its head and
/// the segmented tabs `tabs` at its right, `active` lit, and a clock before
/// them when given. The body is padded as the game's.
pub fn interface_card(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    tabs: &[&str],
    active: &str,
    clock: Option<&str>,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                max_width: px(INTERFACE_MAX_PX),
                align_self: AlignSelf::Center,
                ..panel_node()
            },
            panel(),
        ))
        .with_children(|card| {
            card.spawn((ZIndex(1), panel_head(title, None)))
                .with_children(|head| {
                    if let Some(clock) = clock {
                        head.spawn(Node {
                            column_gap: px(6),
                            margin: UiRect::right(px(16)),
                            ..control_row(JustifyContent::FlexEnd)
                        })
                        .with_children(|c| {
                            c.spawn(text("PLAY", 11.0, UiColor::Label));
                            c.spawn(text(clock, 13.0, UiColor::Primary));
                        });
                    }
                    head.spawn(segmented_container()).with_children(|seg| {
                        for label in tabs {
                            let mut option = seg.spawn((
                                segmented_option(label),
                                Name::new(format!("InterfaceTab{label}")),
                            ));
                            if *label == active {
                                option.insert(Selected);
                            }
                        }
                    });
                });
            card.spawn(Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                padding: UiRect::all(px(12)),
                ..default()
            })
            .with_children(body);
        });
}

/// A menu-shell card: the main menu's overlay panel with `panel_head(title,
/// tag)`, for the Load and New Game pages.
pub fn menu_card(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    tag: &str,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                max_width: px(INTERFACE_MAX_PX),
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(20)),
                row_gap: px(12),
                ..panel_node()
            },
            panel(),
        ))
        .with_children(|card| {
            card.spawn(panel_head(title, Some(tag)));
            card.spawn(Node {
                flex_grow: 1.0,
                min_height: px(0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                ..default()
            })
            .with_children(body);
        });
}

/// A pane footer: the legend at the left, the key hints centred, the
/// controls at the right. Each hint is a key chip and its verb.
pub fn pane_footer(
    body: &mut ChildSpawnerCommands,
    legend: impl FnOnce(&mut ChildSpawnerCommands),
    controls: impl FnOnce(&mut ChildSpawnerCommands),
    keys: &[(&str, &str)],
    pointer: &[&str],
) {
    let side = || Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        height: percent(100),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        align_items: AlignItems::Center,
        align_content: AlignContent::Center,
        column_gap: px(12),
        row_gap: px(6),
        overflow: Overflow::clip(),
        ..default()
    };
    body.spawn((
        Node {
            height: px(PANE_FOOTER_PX),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(16),
            padding: UiRect::axes(px(12), px(8)),
            border: UiRect::top(px(1)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Secondary, 0.5),
    ))
    .with_children(|footer| {
        footer.spawn(side()).with_children(legend);
        footer
            .spawn(Node {
                flex_shrink: 1.0,
                min_width: px(0),
                max_width: percent(50),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(4),
                ..default()
            })
            .with_children(|centre| {
                centre
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        column_gap: px(12),
                        row_gap: px(4),
                        ..default()
                    })
                    .with_children(|hints| {
                        for (key, verb) in keys {
                            hints.spawn(legend_entry()).with_children(|h| {
                                h.spawn(key_chip(key, 11.0));
                                h.spawn(text(verb, 12.0, UiColor::Body));
                            });
                        }
                        for hint in pointer {
                            hints.spawn(text(hint, 12.0, UiColor::Label));
                        }
                    });
            });
        footer
            .spawn(Node {
                justify_content: JustifyContent::FlexEnd,
                ..side()
            })
            .with_children(|right| {
                right
                    .spawn(control_row(JustifyContent::FlexEnd))
                    .with_children(controls);
            });
    });
}

/// Wrap a visual node so it receives clicks and carries what the click does.
pub fn clickable(widget: impl Bundle, click: Click) -> impl Bundle {
    (
        widget,
        bevy::ui_widgets::Button,
        Hovered::default(),
        click,
        observe(on_click),
    )
}

/// Give a `button(...)`, which already carries `Button` and `Hovered`, its
/// click.
pub fn acts(widget: impl Bundle, click: Click) -> impl Bundle {
    (widget, click, observe(on_click))
}

/// A clickable, named table row: a `list_row` at the table's tighter
/// vertical padding, so a board of ten rows fits beside its inspector.
pub fn spawn_row<'a>(
    p: &'a mut ChildSpawnerCommands,
    click: Click,
    name: String,
    selected: bool,
) -> EntityCommands<'a> {
    let mut row = p.spawn((clickable(list_row(), click), Name::new(name)));
    row.insert(Node {
        padding: UiRect::axes(px(13), px(3)),
        border: UiRect::all(px(1)),
        ..row_node()
    });
    if selected {
        row.insert(Selected);
    }
    row
}

/// A notice that fades out: the hand-in toast and the trade note.
#[derive(Component)]
pub struct Toast {
    ttl: f32,
    life: f32,
}

const TOAST_SECS: f32 = 4.0;

/// The hand-in toast: the credit delta and the new balance, bottom right.
pub fn spawn_toast(commands: &mut Commands, headline: &str, detail: &str) {
    clear_toasts(commands);
    commands.spawn((
        Name::new(TOAST),
        Toast {
            ttl: TOAST_SECS,
            life: TOAST_SECS,
        },
        Node {
            position_type: PositionType::Absolute,
            right: px(32),
            bottom: px(72),
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            padding: UiRect::axes(px(14), px(10)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::alpha(UiColor::Surface, 1.0),
        BorderColor::all(Color::NONE),
        ThemedBorder::alpha(UiColor::Nominal, 1.0),
        GlobalZIndex(50),
        Pickable::IGNORE,
        children![
            text(headline, 16.0, UiColor::Nominal),
            text(detail, 12.0, UiColor::Body),
        ],
    ));
}

/// Drop every toast: a new one replaces the old, and a capture starts clean.
pub fn clear_toasts(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let toasts: Vec<Entity> = world
            .query_filtered::<Entity, With<Toast>>()
            .iter(world)
            .collect();
        for toast in toasts {
            world.despawn(toast);
        }
    });
}

/// Fade every toast's fill and text with its remaining life, then drop it.
pub fn fade_toasts(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<(
        Entity,
        &mut Toast,
        &mut ThemedFill,
        &mut ThemedBorder,
        &Children,
    )>,
    mut texts: Query<&mut ThemedText>,
) {
    for (entity, mut toast, mut fill, mut border, children) in &mut toasts {
        toast.ttl -= time.delta_secs();
        if toast.ttl <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = (toast.ttl / toast.life).clamp(0.0, 1.0);
        fill.alpha = alpha;
        border.alpha = alpha;
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.alpha = alpha;
            }
        }
    }
}
