//! The New Game page: the world panel with the name, the seed and its 3D
//! preview, then the role cards with their spinning hulls and the details
//! of the chosen role, and Create at the bottom.

use bevy::prelude::*;
use nova_ui::{prelude::*, theme::UiColor};

use crate::{
    chart3d::{spawn_chart_scene, spawn_hull_scene, SceneAssets, SceneKind},
    chrome::*,
    data::{thousands, RoleCard, FIRST_MILESTONE, ROLES},
    icons::{icon, icon_node, GalleryIcons, IconType},
    load::seeded_view,
    Click, NewGameState, CREATE, RANDOMIZE, ROLE_PREFIX, SEED_FIELD, SEED_PREVIEW,
    WORLD_NAME_FIELD,
};

/// The seed field: a submit regenerates the preview.
#[derive(Component)]
pub struct SeedField;
/// The world name field: a submit renames the world.
#[derive(Component)]
pub struct WorldNameField;

pub fn new_game_page(
    body: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    new_game: &NewGameState,
) {
    menu_card(body, "New Game", "WORLD", |body| {
        body.spawn(Node {
            column_gap: px(12),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            world_panel(row, icons, assets, new_game);
        });
        body.spawn(view_split()).with_children(|split| {
            split.spawn(column()).with_children(|col| {
                section(col, "Role");
                col.spawn(Node {
                    flex_grow: 1.0,
                    min_height: px(0),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|cards| {
                    for (i, role) in ROLES.iter().enumerate() {
                        role_card(cards, icons, assets, i, role, new_game.role == i);
                    }
                });
            });
            split
                .spawn(side_panel(Overflow::scroll_y()))
                .with_children(|side| role_details(side, icons, &ROLES[new_game.role]));
        });
        body.spawn(control_row(JustifyContent::SpaceBetween))
            .with_children(|footer| {
                footer.spawn(button(ButtonSpec::new("Back").fit().key("Esc")));
                footer.spawn((
                    Name::new(CREATE),
                    button(ButtonSpec::new("Create").fit().primary().key("Enter")),
                ));
            });
    });
}

fn world_panel(
    row: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    new_game: &NewGameState,
) {
    row.spawn((
        Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(8),
            padding: UiRect::all(px(12)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        panel_paint(),
    ))
    .with_children(|panel| {
        panel
            .spawn(Node {
                column_gap: px(10),
                ..control_row(JustifyContent::FlexStart)
            })
            .with_children(|line| {
                line.spawn(text("World name", 12.0, UiColor::Label));
                line.spawn(Node {
                    width: px(260),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn((
                        Name::new(WORLD_NAME_FIELD),
                        WorldNameField,
                        text_field(
                            TextFieldSpec::new(new_game.world_name.clone())
                                .placeholder("Unnamed world")
                                .max_chars(32)
                                .dense(),
                        ),
                    ));
                });
                line.spawn(text("Seed", 12.0, UiColor::Label));
                line.spawn(Node {
                    width: px(150),
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|c| {
                    c.spawn((
                        Name::new(SEED_FIELD),
                        SeedField,
                        text_field(
                            TextFieldSpec::new(new_game.seed.to_string())
                                .max_chars(10)
                                .dense(),
                        ),
                    ));
                });
                line.spawn((
                    Name::new(RANDOMIZE),
                    clickable(
                        (
                            Node {
                                width: px(26),
                                height: px(26),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            ThemedFill::alpha(UiColor::Secondary, 0.08),
                            BorderColor::all(Color::NONE),
                            ThemedBorder::alpha(UiColor::Secondary, 0.4),
                            Tooltip("Randomize the seed".into()),
                        ),
                        Click::Randomize,
                    ),
                    children![icon_node(icons, IconType::Randomize, UiColor::Accent, 16.0)],
                ));
                line.spawn(text(
                    "Enter commits a typed seed or name.",
                    11.0,
                    UiColor::Label,
                ));
            });
        spawn_chart_scene(
            panel,
            assets,
            SceneKind::NewGamePreview,
            &seeded_view(new_game.seed, 1, 9, 2, "START"),
            Node {
                height: px(230),
                flex_shrink: 0.0,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            SEED_PREVIEW,
        );
        panel.spawn(text(
            &format!(
                "The sectors around the start of seed {}; the start station sits at the centre. Right-drag turns, wheel zooms.",
                new_game.seed
            ),
            11.0,
            UiColor::Label,
        ));
    });
}

fn role_card(
    cards: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    i: usize,
    role: &RoleCard,
    selected: bool,
) {
    let (color, fill, border) = if selected {
        (UiColor::Accent, 0.1, 1.0)
    } else {
        (UiColor::Secondary, 0.08, 0.5)
    };
    cards
        .spawn((
            Name::new(format!("{ROLE_PREFIX} {i}")),
            clickable(
                (
                    Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        padding: UiRect::all(px(8)),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    ThemedFill::alpha(color, fill),
                    BorderColor::all(Color::NONE),
                    ThemedBorder::alpha(color, border),
                ),
                Click::Role(i),
            ),
        ))
        .with_children(|card| {
            spawn_hull_scene(
                card,
                assets,
                i,
                role.hull,
                Node {
                    height: px(96),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
            );
            card.spawn(Node {
                column_gap: px(6),
                ..control_row(JustifyContent::FlexStart)
            })
            .with_children(|head| {
                head.spawn(icon(icons, role.icon, 16.0));
                head.spawn((text(role.name, 14.0, UiColor::Primary), Pickable::IGNORE));
            });
            card.spawn((
                text_nowrap(&format!("1. {FIRST_MILESTONE}"), 12.0, UiColor::Body),
                Pickable::IGNORE,
            ));
            card.spawn((
                text_nowrap(&format!("2. {}", role.then), 12.0, UiColor::Body),
                Pickable::IGNORE,
            ));
            card.spawn((
                Node {
                    column_gap: px(4),
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(4),
                    ..control_row(JustifyContent::FlexStart)
                },
                Pickable::IGNORE,
            ))
            .with_children(|chips| {
                chips.spawn((
                    tag(BadgeKind::Amber, &format!("{} cr", thousands(role.credits))),
                    Pickable::IGNORE,
                ));
                for kit in role.kit {
                    chips.spawn((tag(BadgeKind::Mute, kit), Pickable::IGNORE));
                }
            });
        });
}

fn role_details(side: &mut ChildSpawnerCommands, icons: &GalleryIcons, role: &RoleCard) {
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|head| {
        head.spawn(icon(icons, role.icon, 24.0));
        head.spawn(text(role.name, 15.0, UiColor::Primary));
        head.spawn(tag(BadgeKind::Mute, role.id));
    });
    side.spawn(text(role.line, 12.0, UiColor::Body));
    side.spawn(divider());
    kv(side, "Hull", role.hull.name());
    kv(side, "Credits", &format!("{} cr", thousands(role.credits)));
    kv(side, "Kit", &role.kit.join(", "));
    side.spawn(divider());
    section(side, "Milestones");
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|r| {
        r.spawn(icon(icons, IconType::Milestone, 16.0));
        r.spawn(grow_cell(FIRST_MILESTONE, 13.0, UiColor::Primary));
    });
    side.spawn(Node {
        column_gap: px(8),
        ..control_row(JustifyContent::FlexStart)
    })
    .with_children(|r| {
        r.spawn(icon(icons, role.icon, 16.0));
        r.spawn(grow_cell(role.then, 13.0, UiColor::Primary));
    });
}
