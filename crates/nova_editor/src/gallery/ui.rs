//! The gallery overlay: the header (category row, filter box, paging), the tile
//! grid, and the focus card.
//!
//! The whole overlay is rebuilt from [`GalleryState`] whenever that state
//! changes, so there is one code path from state to screen and no partial
//! updates to keep in step. Change this module for gallery LAYOUT; what it
//! lists lives in `catalog`.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{Activate, Button},
};
use nova_ship::prelude::*;
use nova_ui::{
    prelude::{button, panel, ButtonSpec, UiSkin, UiText},
    theme,
};

use crate::{
    config::SectionChoice,
    gallery::{
        catalog::{self, GalleryCategory},
        scene::{spawn_tile, GalleryItem},
        GalleryState, COLS, PAGE, ROWS,
    },
    ExampleStates,
};

/// The overlay root, despawned and rebuilt on every state change.
#[derive(Component)]
pub(crate) struct GalleryRoot;

/// The editor's own rail + drawer root, hidden while the gallery is up.
///
/// The gallery is a MODE, not a panel: its tile grid has to stay transparent
/// for the 3D previews to show through, so the chrome behind it cannot simply
/// be covered - it has to go away.
#[derive(Component)]
pub(crate) struct EditorChrome;

/// Show the editor chrome only while the gallery is closed.
pub(crate) fn sync_editor_chrome(
    state: Res<GalleryState>,
    mut chrome: Query<&mut Visibility, With<EditorChrome>>,
) {
    let wanted = if state.open {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut chrome {
        // Compared before writing: an unconditional write dirties the whole UI
        // tree's visibility every frame.
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// One tile, carrying its position in the filtered list so the painter can tell
/// the selected cell from its neighbours.
#[derive(Component)]
pub(crate) struct GalleryCell {
    /// Index into the filtered list, not the catalog.
    pub(crate) listed: usize,
}

/// What pressing a gallery widget does. One component + one observer, so a new
/// control is a new variant rather than a new observer.
#[derive(Component, Clone, Debug)]
pub(crate) enum GalleryAction {
    /// Open the gallery (the editor rail's category row).
    Open,
    /// Close it, keeping the current selection for the next visit.
    Close,
    /// Leave the focus card for the grid.
    Back,
    /// Arm the selected prototype as the placement tool and close.
    Place,
    /// Select a tile and open its focus card.
    Focus(usize),
    /// Narrow the list to one section kind.
    Category(GalleryCategory),
    /// Open the gallery ALREADY narrowed to one kind - what an Add menu row
    /// that names a kind promises. [`GalleryAction::Open`] keeps whichever
    /// category the last browse left behind, which a named row must not.
    Browse(GalleryCategory),
    /// Step the selection by whole pages.
    Page(isize),
    /// Put the caret in the filter field.
    FocusFilter,
}

/// Height of the tile's label strip. The strip is opaque so the name stays
/// readable over the skybox; the preview area above it must NOT be, or it would
/// tint the 3D behind it.
const LABEL_H: f32 = 34.0;

/// Rebuild the overlay (and its previews) whenever the gallery state or the
/// catalog changes.
pub(crate) fn rebuild_gallery(
    mut commands: Commands,
    state: Res<GalleryState>,
    sections: Res<GameSections>,
    skin: Res<UiSkin>,
    existing: Query<Entity, Or<(With<GalleryRoot>, With<GalleryItem>)>>,
    mut last: Local<Option<GalleryState>>,
) {
    let dirty = last.as_ref() != Some(&*state) || sections.is_changed() || skin.is_changed();
    if !dirty {
        return;
    }
    *last = Some(state.clone());

    for entity in &existing {
        commands.entity(entity).despawn();
    }
    if !state.open {
        return;
    }

    let skin = *skin;
    let listed = catalog::browsable(&sections, state.category, &state.filter);
    let pages = listed.len().div_ceil(PAGE).max(1);
    let page = state.selected / PAGE;
    let start = page * PAGE;

    // Cells are spawned inside the layout closure but their previews are
    // spawned after it, so the closure can hand back the node each preview
    // centres on.
    let mut stages: Vec<(Entity, usize, bool)> = Vec::new();

    commands
        .spawn((
            GalleryRoot,
            DespawnOnExit(ExampleStates::Editor),
            Name::new("Parts Gallery"),
            // Over the rail and the drawer, and over the build area: while the
            // gallery is up nothing behind it may be clicked.
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Name::new("Gallery Header"),
                Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    padding: UiRect::axes(px(14), px(8)),
                    border: UiRect::bottom(px(theme::BORDER_W)),
                    ..default()
                },
                panel(skin),
            ))
            .with_children(|header| {
                header.spawn((
                    Name::new("Gallery Title"),
                    UiText,
                    Text::new("PARTS"),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR),
                ));

                for category in GalleryCategory::ROW {
                    header
                        .spawn((Node {
                            width: px(if matches!(category, GalleryCategory::All) {
                                54.0
                            } else {
                                96.0
                            }),
                            ..default()
                        },))
                        .with_children(|slot| {
                            let mut spec = ButtonSpec::new(category.label());
                            spec.min_height = 26.0;
                            spec.font_size = 12.0;
                            if category != state.category {
                                spec = spec.ghost();
                            }
                            slot.spawn((
                                Name::new(format!("Gallery Category {}", category.label())),
                                button(spec),
                                GalleryAction::Category(category),
                            ));
                        });
                }

                header.spawn(filter_box(&state.filter, state.filter_focused));

                for (label, name, step) in [
                    ("<", "Gallery Previous Page Button", -1),
                    (">", "Gallery Next Page Button", 1),
                ] {
                    header
                        .spawn(Node {
                            width: px(34),
                            ..default()
                        })
                        .with_children(|slot| {
                            let mut spec = ButtonSpec::new(label).ghost();
                            spec.min_height = 26.0;
                            spec.font_size = 12.0;
                            slot.spawn((Name::new(name), button(spec), GalleryAction::Page(step)));
                        });
                }

                header.spawn((
                    Name::new("Gallery Page"),
                    UiText,
                    Text::new(format!(
                        "{} parts   page {}/{}",
                        listed.len(),
                        page + 1,
                        pages
                    )),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                    Node {
                        margin: UiRect::left(px(6)),
                        ..default()
                    },
                ));

                header
                    .spawn(Node {
                        width: px(110),
                        margin: UiRect::left(px(10)),
                        ..default()
                    })
                    .with_children(|slot| {
                        slot.spawn((
                            Name::new("Gallery Close Button"),
                            button(ButtonSpec::new("Close").key("Esc")),
                            GalleryAction::Close,
                        ));
                    });
            });

            if state.focused {
                focus_body(root, &sections, &listed, state.selected, skin, &mut stages);
            } else {
                grid_body(root, &sections, &listed, start, state.selected, &mut stages);
            }

            root.spawn((
                Name::new("Gallery Hint"),
                UiText,
                // Contextual: the focus card's controls are not the grid's, and
                // a hint line that lists both is one nobody reads.
                // One grammar with the editor's key legend: the key as it is
                // typed, then a lowercase verb phrase. `Q` was "take it" here
                // and "pick a part" on the stage, which is two names for one
                // gesture.
                Text::new(if state.focused {
                    "drag turn it   wheel zoom   arrows next part   Enter place it   Esc back"
                } else if state.filter_focused {
                    "type filter   Enter the top hit   Esc leave the field"
                } else {
                    "Q pick the hovered part   arrows select   Enter focus   / filter   \
                     Esc/Tab close"
                }),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
                Node {
                    margin: UiRect::axes(px(16), px(8)),
                    ..default()
                },
            ));
        });

    for (stage, index, spin) in stages {
        if let Some(section) = sections.get(index) {
            spawn_tile(&mut commands, section, stage, spin);
        }
    }
}

/// The tile grid: `ROWS` rows of `COLS` cells, padded with empty slots so the
/// last page keeps the same cell size as a full one.
fn grid_body(
    root: &mut ChildSpawnerCommands,
    sections: &GameSections,
    listed: &[usize],
    start: usize,
    selected: usize,
    stages: &mut Vec<(Entity, usize, bool)>,
) {
    root.spawn((
        Name::new("Gallery Grid"),
        Node {
            width: percent(100),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)),
            row_gap: px(10),
            ..default()
        },
    ))
    .with_children(|grid| {
        for row in 0..ROWS {
            grid.spawn((Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                column_gap: px(10),
                ..default()
            },))
                .with_children(|row_node| {
                    for column in 0..COLS {
                        let slot = start + row * COLS + column;
                        match listed
                            .get(slot)
                            .and_then(|index| sections.get(*index).map(|section| (*index, section)))
                        {
                            Some((index, section)) => {
                                let stage = spawn_cell(row_node, section, slot, slot == selected);
                                stages.push((stage, index, false));
                            }
                            // An empty slot still takes its share of the row, so a
                            // half-full page does not stretch its tiles.
                            None => {
                                row_node.spawn(Node {
                                    flex_grow: 1.0,
                                    flex_basis: px(0),
                                    ..default()
                                });
                            }
                        }
                    }
                });
        }
    });
}

/// One tile: a transparent preview area (the 3D shows through it) over an
/// opaque name strip. Returns the preview area, which is what a preview
/// centres itself on.
fn spawn_cell(
    row: &mut ChildSpawnerCommands,
    section: &SectionConfig,
    listed: usize,
    selected: bool,
) -> Entity {
    let mut stage = Entity::PLACEHOLDER;
    row.spawn((
        Name::new(format!("Gallery Tile {}", section.base.name)),
        GalleryCell { listed },
        GalleryAction::Focus(listed),
        Button,
        Hovered::default(),
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(cell_border(selected, false)),
        BackgroundColor(cell_fill(selected, false)),
    ))
    .with_children(|cell| {
        stage = cell
            .spawn((
                // NOT prefixed "Gallery Tile": a harness counts tiles by that
                // prefix, and a stage per tile would double every count.
                Name::new("Gallery Grid Stage"),
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    ..default()
                },
                // The preview lives behind this node; it must not eat the
                // tile's own hover/press.
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
            ))
            .id();

        cell.spawn((
            Node {
                width: percent(100),
                height: px(LABEL_H),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(6)),
                ..default()
            },
            BackgroundColor(theme::SCREEN_0),
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            children![
                (
                    UiText,
                    Text::new(section.base.name.clone()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::SCREEN_TEXT),
                ),
                (
                    UiText,
                    Text::new(catalog::kind_label(&section.kind).to_string()),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                ),
            ],
        ));
    });
    stage
}

/// The focus card: a turntable stage beside the prototype's stat block.
fn focus_body(
    root: &mut ChildSpawnerCommands,
    sections: &GameSections,
    listed: &[usize],
    selected: usize,
    skin: UiSkin,
    stages: &mut Vec<(Entity, usize, bool)>,
) {
    let Some((index, section)) = listed
        .get(selected)
        .and_then(|index| sections.get(*index).map(|section| (*index, section)))
    else {
        return;
    };

    let mut stage = Entity::PLACEHOLDER;
    root.spawn((
        Name::new("Gallery Focus"),
        Node {
            width: percent(100),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Row,
            padding: UiRect::all(px(12)),
            column_gap: px(12),
            ..default()
        },
    ))
    .with_children(|focus| {
        stage = focus
            .spawn((
                Name::new("Gallery Focus Stage"),
                Node {
                    height: percent(100),
                    flex_grow: 2.0,
                    ..default()
                },
                Pickable {
                    should_block_lower: false,
                    is_hoverable: false,
                },
            ))
            .id();

        focus
            .spawn((
                Name::new("Gallery Focus Card"),
                Node {
                    width: px(320),
                    height: percent(100),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(12)),
                    row_gap: px(6),
                    border: UiRect::all(px(theme::BORDER_W)),
                    ..default()
                },
                panel(skin),
            ))
            .with_children(|card| {
                card.spawn((
                    UiText,
                    Text::new(section.base.name.clone()),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR),
                ));
                card.spawn((
                    UiText,
                    Text::new(section.base.description.clone()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_MUTED),
                    Node {
                        margin: UiRect::bottom(px(6)),
                        ..default()
                    },
                ));

                for (label, value) in catalog::stats(section) {
                    card.spawn((
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        },
                        children![
                            (
                                UiText,
                                Text::new(label),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(theme::PHOSPHOR_MUTED),
                            ),
                            (
                                UiText,
                                Text::new(value),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(theme::SCREEN_TEXT),
                            ),
                        ],
                    ));
                }

                card.spawn((
                    Name::new("Gallery Place Button"),
                    button(ButtonSpec::new("Place This Part").primary().key("Enter")),
                    GalleryAction::Place,
                ));
                card.spawn((
                    Name::new("Gallery Back Button"),
                    button(ButtonSpec::new("Back").ghost()),
                    GalleryAction::Back,
                ));
            });
    });

    stages.push((stage, index, true));
}

/// The filter field. Not an editable text widget: the gallery owns the keyboard
/// while it is up (see `input`), so the box only has to SHOW what was typed.
fn filter_box(filter: &str, focused: bool) -> impl Bundle {
    // The caret is the focus tell, as it is in every text field: without one
    // there is no way to know whether a keystroke is about to filter or to
    // trigger a shortcut.
    let (text, color) = match (focused, filter.is_empty()) {
        (true, _) => (format!("{filter}_"), theme::SCREEN_TEXT),
        (false, true) => ("/ to filter".to_string(), theme::PHOSPHOR_MUTED),
        (false, false) => (filter.to_string(), theme::SCREEN_TEXT),
    };
    (
        Name::new("Gallery Filter"),
        GalleryAction::FocusFilter,
        Button,
        Hovered::default(),
        Node {
            width: px(190),
            min_height: px(26),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(8)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        BorderColor::all(if focused {
            theme::PHOSPHOR
        } else {
            theme::PHOSPHOR_MUTED
        }),
        BackgroundColor(theme::SCREEN_0),
        children![(
            UiText,
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(color),
        )],
    )
}

/// Keep the tile borders in step with hover and selection. The tiles are not
/// [`ThemedButton`]s: a themed face paints an opaque fill, which would hide the
/// 3D preview behind it.
pub(crate) fn paint_gallery_cells(
    state: Res<GalleryState>,
    mut cells: Query<(
        &GalleryCell,
        &Hovered,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
) {
    for (cell, hovered, mut border, mut background) in &mut cells {
        let selected = cell.listed == state.selected;
        *border = BorderColor::all(cell_border(selected, hovered.get()));
        background.0 = cell_fill(selected, hovered.get());
    }
}

fn cell_border(selected: bool, hovered: bool) -> Color {
    match (selected, hovered) {
        (true, _) => theme::PHOSPHOR,
        (false, true) => theme::PHOSPHOR_DIM,
        (false, false) => theme::PHOSPHOR_MUTED,
    }
}

/// Tile fills stay nearly transparent on purpose: whatever alpha they carry
/// tints the 3D preview behind them.
fn cell_fill(selected: bool, hovered: bool) -> Color {
    match (selected, hovered) {
        (true, _) => theme::PHOSPHOR.with_alpha(0.05),
        (false, true) => theme::PHOSPHOR.with_alpha(0.05),
        (false, false) => Color::NONE,
    }
}

/// Apply a gallery control's action.
pub(crate) fn on_gallery_action(
    activate: On<Activate>,
    actions: Query<&GalleryAction>,
    sections: Res<GameSections>,
    mut state: ResMut<GalleryState>,
    mut choice: ResMut<SectionChoice>,
) {
    let Ok(action) = actions.get(activate.entity) else {
        return;
    };

    match action {
        GalleryAction::Open => {
            // A fresh browse starts unfiltered. A filter left over from the last
            // visit would open the gallery on a near-empty grid with no obvious
            // cause, since the field is the only thing that says why.
            state.filter.clear();
            state.filter_focused = false;
            state.selected = 0;
            state.focused = false;
            state.open = true;
        }
        GalleryAction::FocusFilter => state.filter_focused = true,
        GalleryAction::Close => {
            state.open = false;
            state.focused = false;
        }
        GalleryAction::Back => state.focused = false,
        GalleryAction::Focus(listed) => {
            state.selected = *listed;
            state.focused = true;
            state.filter_focused = false;
        }
        GalleryAction::Place => {
            if let Some(id) = state.selected_id(&sections) {
                *choice = SectionChoice::Section(id);
            }
            state.open = false;
            state.focused = false;
        }
        GalleryAction::Category(category) => {
            state.category = *category;
            state.selected = 0;
            state.focused = false;
            state.filter_focused = false;
        }
        GalleryAction::Browse(category) => {
            state.filter.clear();
            state.filter_focused = false;
            state.category = *category;
            state.selected = 0;
            state.focused = false;
            state.open = true;
        }
        GalleryAction::Page(step) => {
            let listed = catalog::browsable(&sections, state.category, &state.filter);
            state.step(step * PAGE as isize, listed.len());
        }
    }
}
