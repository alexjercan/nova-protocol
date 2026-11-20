//! The New Game page: the main menu as the game draws it (the backdrop of
//! drifting rocks and the menu column at the bottom right) with the New
//! Game modal centred over it. The modal has the game's two steps: the
//! world step (name, seed with Randomize, the seed's 3D preview, Cancel and
//! Next) and the wider role step (the role list beside the details with the
//! spinning hull, then Back, Cancel and Create). The values live in
//! `NewGameState`, so Next and Back show what was committed.

use bevy::prelude::*;
use nova_ui::{
    prelude::*,
    theme::UiColorType,
    widget::{ThemedRadius, ThemedTextShadow},
};

// The gallery keeps its own fixed-size mock helpers; they shadow the kit's.
use crate::chrome::{cell, control_row, text};
use crate::{
    chart3d::{spawn_backdrop_scene, spawn_chart_scene, spawn_hull_scene, SceneAssets, SceneKind},
    chrome::*,
    data::{thousands, RoleCard, FIRST_MILESTONE, ROLES, RUNNING_VERSION},
    icons::{icon, GalleryIcons},
    load::seeded_view,
    Click, NewGameState, NewGameStep, CREATE, HULL_VIEWPORT, MENU_NEW_GAME, NEW_GAME_BACK,
    NEW_GAME_CANCEL, NEW_GAME_NEXT, NEW_GAME_PANEL, RANDOMIZE, ROLE_PREFIX, SEED_FIELD,
    SEED_PREVIEW, WORLD_NAME_FIELD,
};

/// The seed field: Next or a submit commits it.
#[derive(Component)]
pub struct SeedField;
/// The world name field: Next or a submit commits it.
#[derive(Component)]
pub struct WorldNameField;

/// The modal's width as a share of the window on both steps, as the game's
/// `WORLD_SETUP_WIDTH_PCT`.
const WORLD_SETUP_WIDTH_PCT: f32 = 92.0;
/// The modal's widest size in px, as the game's `WORLD_SETUP_MAX_W`.
const WORLD_SETUP_MAX_W: f32 = 640.0;
/// The role list's width, as the game's `ROLE_LIST_W`.
const ROLE_LIST_W: f32 = 240.0;
/// The main action's slot, as the game's `MAIN_SLOT_W`.
const MAIN_SLOT_W: f32 = 180.0;
/// A side action's slot, as the game's `SIDE_SLOT_W`.
const SIDE_SLOT_W: f32 = 140.0;
/// The Randomize slot beside the seed field, as the game's.
const RANDOMIZE_SLOT_W: f32 = 130.0;
/// The menu column's width, as `setup_menu_ui` spawns it.
const MENU_W: f32 = 280.0;
/// The label column of a details line, as the game's `DETAIL_LABEL_W`.
const DETAIL_LABEL_W: f32 = 110.0;
/// The gap from the menu card to the window edge, as the game's
/// `MENU_CARD_MARGIN`.
const MENU_CARD_MARGIN: f32 = 40.0;

pub fn new_game_page(
    body: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    new_game: &NewGameState,
) {
    body.spawn((
        Node {
            flex_grow: 1.0,
            min_height: px(0),
            position_type: PositionType::Relative,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::NONE),
        ThemedFill::new(UiColorType::Background),
    ))
    .with_children(|screen| {
        spawn_backdrop_scene(screen, assets);
        // The game hides the menu card while a window is open, so the window
        // stands alone over the backdrop.
        if new_game.step == NewGameStep::Closed {
            menu_column(screen);
        } else {
            modal(screen, icons, assets, new_game);
        }
    });
}

/// The main menu column at the bottom right, as `setup_menu_ui` spawns it:
/// the button groups stand a group gap apart, the buttons of one group a
/// button gap apart. Only New Game acts: it opens the modal on the world step.
fn menu_column(screen: &mut ChildSpawnerCommands) {
    screen
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(MENU_CARD_MARGIN),
                bottom: px(MENU_CARD_MARGIN),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                width: px(MENU_W),
                row_gap: px(metrics::GAP_L),
                padding: UiRect::all(px(metrics::PANEL_PAD_MENU)),
                border: UiRect::all(px(theme::BORDER_W)),
                ..default()
            },
            ThemedRadius::control(),
            panel(),
        ))
        .with_children(|menu| {
            menu.spawn((
                text("Nova Protocol", 28.0, UiColorType::TextBody),
                ThemedTextShadow::alpha(UiColorType::Text, 0.35),
                TextShadow {
                    color: Color::NONE,
                    offset: Vec2::ZERO,
                },
            ));
            menu.spawn((
                Node {
                    width: percent(80),
                    height: px(2),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColorType::TextMuted),
            ));
            menu.spawn(Node {
                align_self: AlignSelf::Stretch,
                ..group_stack()
            })
            .with_children(|groups| {
                groups.spawn(button_stack()).with_children(|group| {
                    group.spawn((
                        Name::new(MENU_NEW_GAME),
                        acts(
                            button(ButtonSpec::menu("New Game").primary().fill()),
                            Click::NewGameStep(NewGameStep::World),
                        ),
                    ));
                    group.spawn(button(ButtonSpec::menu("Load").fill()));
                });
                for labels in [
                    &["Lessons", "Scenarios", "Sandbox"][..],
                    &["Mods", "Settings"],
                ] {
                    groups.spawn(button_stack()).with_children(|group| {
                        for label in labels {
                            group.spawn(button(ButtonSpec::menu(*label).fill()));
                        }
                    });
                }
                groups
                    .spawn(button_stack())
                    .with_child(button(ButtonSpec::menu("Exit").danger().fill()));
            });
            menu.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|foot| {
                foot.spawn(text(RUNNING_VERSION, 10.0, UiColorType::TextMuted));
                foot.spawn(text("NOVA OS", 10.0, UiColorType::TextMuted));
            });
        });
}

/// The modal: the scrim over the backdrop and the centred panel with the
/// current step.
fn modal(
    screen: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    new_game: &NewGameState,
) {
    screen
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ZIndex(10),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Name::new(NEW_GAME_PANEL),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        width: percent(WORLD_SETUP_WIDTH_PCT),
                        max_width: px(WORLD_SETUP_MAX_W),
                        padding: UiRect::all(px(metrics::PANEL_PAD_MENU)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|panel| {
                    let tag = if new_game.step == NewGameStep::Role {
                        "2 / 2 Role"
                    } else {
                        "1 / 2 World"
                    };
                    panel.spawn(panel_head("New Game", PanelHeadType::Inset, &[tag]));
                    // The step: its content over its actions row, a group gap
                    // apart, as the game's `step_node`.
                    panel
                        .spawn(group_stack())
                        .with_children(|step| match new_game.step {
                            NewGameStep::Role => role_step(step, icons, assets, new_game),
                            NewGameStep::World | NewGameStep::Closed => {
                                world_step(step, assets, new_game)
                            }
                        });
                });
        });
}

/// One label over its control, as the game's `field_group`.
fn field_group() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(metrics::GAP_S),
        ..default()
    }
}

/// The world step: the name, the seed with Randomize and the seed's preview,
/// each a group, then Cancel and Next.
fn world_step(panel: &mut ChildSpawnerCommands, assets: &mut SceneAssets, new_game: &NewGameState) {
    panel.spawn(group_stack()).with_children(|fields| {
        fields.spawn(field_group()).with_children(|group| {
            group.spawn(text("World name", 13.0, UiColorType::TextMuted));
            group.spawn((
                Name::new(WORLD_NAME_FIELD),
                WorldNameField,
                text_field(
                    TextFieldSpec::new(new_game.world_name.clone())
                        .placeholder("New World")
                        .max_chars(32),
                ),
            ));
        });
        fields.spawn(field_group()).with_children(|group| {
            group.spawn(text("World seed", 13.0, UiColorType::TextMuted));
            group
                .spawn(Node {
                    column_gap: px(metrics::GAP),
                    ..control_row(JustifyContent::FlexStart)
                })
                .with_children(|row| {
                    row.spawn(Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        ..default()
                    })
                    .with_child((
                        Name::new(SEED_FIELD),
                        SeedField,
                        text_field(TextFieldSpec::new(new_game.seed.to_string()).max_chars(10)),
                    ));
                    row.spawn(Node {
                        width: px(RANDOMIZE_SLOT_W),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_child((
                        Name::new(RANDOMIZE),
                        acts(
                            button(ButtonSpec::menu("Randomize").fill()),
                            Click::Randomize,
                        ),
                    ));
                });
        });
        fields.spawn(field_group()).with_children(|group| {
            // The control radius, as the game's `ThemedRadius::control()`.
            let radius = assets.theme.metric(theme::UiMetric::Radius);
            spawn_chart_scene(
                group,
                assets,
                SceneKind::NewGamePreview,
                &seeded_view(new_game.seed, 1, 9, 2, "START"),
                Node {
                    height: px(200),
                    flex_shrink: 0.0,
                    border: UiRect::all(px(theme::BORDER_W)),
                    border_radius: BorderRadius::all(px(radius)),
                    ..default()
                },
                SEED_PREVIEW,
            );
            group.spawn(text(
                &format!(
                    "The sectors around the start of seed {}. Right-drag turns, wheel zooms.",
                    new_game.seed
                ),
                11.0,
                UiColorType::TextMuted,
            ));
        });
    });
    panel
        .spawn(action_row(JustifyContent::FlexEnd))
        .with_children(|actions| {
            actions.spawn(button_slot(SIDE_SLOT_W)).with_child((
                Name::new(NEW_GAME_CANCEL),
                acts(
                    button(ButtonSpec::menu("Cancel").fill()),
                    Click::NewGameStep(NewGameStep::Closed),
                ),
            ));
            actions.spawn(button_slot(MAIN_SLOT_W)).with_child((
                Name::new(NEW_GAME_NEXT),
                acts(
                    button(ButtonSpec::menu("Next").primary().fill()),
                    Click::NewGameNext,
                ),
            ));
        });
}

/// The role step: the role list beside the selected role's details, then
/// Back, Cancel and Create.
fn role_step(
    panel: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    new_game: &NewGameState,
) {
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(metrics::GAP_XL),
            ..default()
        })
        .with_children(|body| {
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                width: px(ROLE_LIST_W),
                flex_shrink: 0.0,
                row_gap: px(metrics::GAP_S),
                ..default()
            })
            .with_children(|list| {
                list.spawn(text("Start role", 13.0, UiColorType::TextMuted));
                for (i, role) in ROLES.iter().enumerate() {
                    let mut row = list.spawn((
                        Name::new(format!("{ROLE_PREFIX} {i}")),
                        clickable(list_row(), Click::Role(i)),
                    ));
                    if new_game.role == i {
                        row.insert(Selected);
                    }
                    row.with_children(|r| {
                        r.spawn(icon(icons, role.icon, 16.0));
                        r.spawn((
                            text(role.name, 15.0, UiColorType::TextBody),
                            Pickable::IGNORE,
                        ));
                    });
                }
            });
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..button_stack()
            })
            .with_children(|details| {
                role_details(details, assets, new_game.role, &ROLES[new_game.role]);
            });
        });
    panel
        .spawn(action_row(JustifyContent::FlexEnd))
        .with_children(|actions| {
            actions.spawn(button_slot(SIDE_SLOT_W)).with_child((
                Name::new(NEW_GAME_BACK),
                acts(
                    button(ButtonSpec::menu("Back").fill()),
                    Click::NewGameStep(NewGameStep::World),
                ),
            ));
            actions.spawn(button_slot(SIDE_SLOT_W)).with_child((
                Name::new(NEW_GAME_CANCEL),
                acts(
                    button(ButtonSpec::menu("Cancel").fill()),
                    Click::NewGameStep(NewGameStep::Closed),
                ),
            ));
            actions.spawn(button_slot(MAIN_SLOT_W)).with_child((
                Name::new(CREATE),
                acts(
                    button(ButtonSpec::menu("Create").primary().fill()),
                    Click::Create,
                ),
            ));
        });
}

/// The selected role's details: its name and line, the spinning hull, the
/// hull line, the credits, the cargo kit and the milestones.
fn role_details(
    details: &mut ChildSpawnerCommands,
    assets: &mut SceneAssets,
    index: usize,
    role: &RoleCard,
) {
    details.spawn(text(role.name, 18.0, UiColorType::Text));
    details.spawn(text(role.line, 13.0, UiColorType::TextBody));
    let radius = assets.theme.metric(theme::UiMetric::Radius);
    spawn_hull_scene(
        details,
        assets,
        index,
        role.hull,
        Node {
            height: px(150),
            flex_shrink: 0.0,
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(radius)),
            ..default()
        },
        HULL_VIEWPORT,
    );
    details.spawn(nova_ui::widget::divider());
    details
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(metrics::GAP_S),
            ..default()
        })
        .with_children(|rows| role_rows(rows, role));
}

/// The lines of the role details, a list-row gap apart.
fn role_rows(details: &mut ChildSpawnerCommands, role: &RoleCard) {
    detail_row(
        details,
        "Hull",
        &format!("{}, {} m", role.hull.display(), role.hull.length_m()),
    );
    detail_row(
        details,
        "Credits",
        &format!("{} cr", thousands(role.credits)),
    );
    detail_row(details, "Cargo", &role.kit.join(", "));
    detail_row(details, "Milestones", &format!("1. {FIRST_MILESTONE}"));
    detail_row(details, "", &format!("2. {}", role.then));
}

/// One `LABEL  value` line of the details, as the game's `detail_row`.
fn detail_row(details: &mut ChildSpawnerCommands, label: &str, value: &str) {
    details
        .spawn(Node {
            column_gap: px(metrics::GAP_L),
            min_height: px(metrics::CONTROL_H),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(cell(
                DETAIL_LABEL_W,
                &label.to_uppercase(),
                11.0,
                UiColorType::TextMuted,
                false,
            ));
            r.spawn(grow_cell(value, 13.0, UiColorType::TextBody));
        });
}

/// A fixed-width slot for one button, as the game's `button_slot`.
fn button_slot(width: f32) -> Node {
    Node {
        width: px(width),
        flex_shrink: 0.0,
        ..default()
    }
}
