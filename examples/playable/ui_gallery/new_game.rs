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
    theme::UiColor,
    widget::{ThemedRadius, ThemedTextShadow},
};

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

/// The modal's width on the world step, as the game's `WORLD_STEP_W`.
const WORLD_STEP_W: f32 = 420.0;
/// The modal's width on the role step, as the game's `ROLE_STEP_W`.
const ROLE_STEP_W: f32 = 760.0;
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
/// The line over the fields, as the game's `PROFILE_TEXT`.
const PROFILE_TEXT: &str = "A seeded open world of asteroid clusters, planetoids and derelicts.";

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
        ThemedFill::new(UiColor::Void),
    ))
    .with_children(|screen| {
        spawn_backdrop_scene(screen, assets);
        menu_column(screen);
        if new_game.step != NewGameStep::Closed {
            modal(screen, icons, assets, new_game);
        }
    });
}

/// The main menu column at the bottom right, as `setup_menu_ui` spawns it.
/// Only New Game acts: it reopens the modal on the world step.
fn menu_column(screen: &mut ChildSpawnerCommands) {
    screen
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(40),
                bottom: px(40),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                width: px(MENU_W),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(1)),
                ..default()
            },
            ThemedRadius::control(),
            panel(),
        ))
        .with_children(|menu| {
            menu.spawn((
                text("Nova Protocol", 28.0, UiColor::Body),
                ThemedTextShadow::alpha(UiColor::Primary, 0.35),
                TextShadow {
                    color: Color::NONE,
                    offset: Vec2::ZERO,
                },
            ));
            menu.spawn((
                Node {
                    width: percent(80),
                    height: px(2),
                    margin: UiRect::all(px(10)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColor::Label),
            ));
            menu.spawn((
                Name::new(MENU_NEW_GAME),
                acts(
                    button(ButtonSpec::new("New Game").menu().primary()),
                    Click::NewGameStep(NewGameStep::World),
                ),
            ));
            for label in [
                "Load",
                "Sandbox",
                "Scenarios",
                "Lessons",
                "Mods",
                "Settings",
            ] {
                menu.spawn(menu_button(label));
            }
            menu.spawn(button(ButtonSpec::new("Exit").menu().danger()));
            menu.spawn(Node {
                width: percent(100),
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::top(px(10)),
                ..default()
            })
            .with_children(|foot| {
                foot.spawn(text(RUNNING_VERSION, 10.0, UiColor::Label));
                foot.spawn(text("NOVA OS", 10.0, UiColor::Label));
            });
        });
}

/// The modal: the blocker over the menu and the centred panel with the
/// current step.
fn modal(
    screen: &mut ChildSpawnerCommands,
    icons: &GalleryIcons,
    assets: &mut SceneAssets,
    new_game: &NewGameState,
) {
    let width = match new_game.step {
        NewGameStep::Role => ROLE_STEP_W,
        NewGameStep::World | NewGameStep::Closed => WORLD_STEP_W,
    };
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
                        width: px(width),
                        padding: UiRect::all(px(20)),
                        row_gap: px(8),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|panel| {
                    panel.spawn(text("New Game", 24.0, UiColor::Body));
                    match new_game.step {
                        NewGameStep::Role => role_step(panel, icons, assets, new_game),
                        NewGameStep::World | NewGameStep::Closed => {
                            world_step(panel, assets, new_game)
                        }
                    }
                });
        });
}

/// The world step: the profile line, the name, the seed with Randomize, the
/// seed's preview, then Cancel and Next.
fn world_step(panel: &mut ChildSpawnerCommands, assets: &mut SceneAssets, new_game: &NewGameState) {
    panel.spawn(text(PROFILE_TEXT, 13.0, UiColor::Label));
    panel.spawn(text("World name", 13.0, UiColor::Label));
    panel.spawn((
        Name::new(WORLD_NAME_FIELD),
        WorldNameField,
        text_field(
            TextFieldSpec::new(new_game.world_name.clone())
                .placeholder("New World")
                .max_chars(32),
        ),
    ));
    panel.spawn(text("World seed", 13.0, UiColor::Label));
    panel
        .spawn(Node {
            column_gap: px(8),
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
                acts(menu_button("Randomize"), Click::Randomize),
            ));
        });
    spawn_chart_scene(
        panel,
        assets,
        SceneKind::NewGamePreview,
        &seeded_view(new_game.seed, 1, 9, 2, "START"),
        Node {
            height: px(200),
            flex_shrink: 0.0,
            margin: UiRect::top(px(4)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        SEED_PREVIEW,
    );
    panel.spawn(text(
        &format!(
            "The sectors around the start of seed {}. Right-drag turns, wheel zooms.",
            new_game.seed
        ),
        11.0,
        UiColor::Label,
    ));
    panel.spawn(actions_row()).with_children(|actions| {
        actions.spawn(button_slot(SIDE_SLOT_W)).with_child((
            Name::new(NEW_GAME_CANCEL),
            acts(
                menu_button("Cancel"),
                Click::NewGameStep(NewGameStep::Closed),
            ),
        ));
        actions.spawn(button_slot(MAIN_SLOT_W)).with_child((
            Name::new(NEW_GAME_NEXT),
            acts(
                button(ButtonSpec::new("Next").menu().primary()),
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
            column_gap: px(16),
            ..default()
        })
        .with_children(|body| {
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                width: px(ROLE_LIST_W),
                flex_shrink: 0.0,
                row_gap: px(4),
                ..default()
            })
            .with_children(|list| {
                list.spawn(text("Start role", 13.0, UiColor::Label));
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
                        r.spawn((text(role.name, 15.0, UiColor::Body), Pickable::IGNORE));
                    });
                }
            });
            body.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                row_gap: px(6),
                ..default()
            })
            .with_children(|details| {
                role_details(details, assets, new_game.role, &ROLES[new_game.role]);
            });
        });
    panel.spawn(actions_row()).with_children(|actions| {
        actions.spawn(button_slot(SIDE_SLOT_W)).with_child((
            Name::new(NEW_GAME_BACK),
            acts(menu_button("Back"), Click::NewGameStep(NewGameStep::World)),
        ));
        actions.spawn(button_slot(SIDE_SLOT_W)).with_child((
            Name::new(NEW_GAME_CANCEL),
            acts(
                menu_button("Cancel"),
                Click::NewGameStep(NewGameStep::Closed),
            ),
        ));
        actions.spawn(button_slot(MAIN_SLOT_W)).with_child((
            Name::new(CREATE),
            acts(
                button(ButtonSpec::new("Create").menu().primary()),
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
    details.spawn(text(role.name, 18.0, UiColor::Primary));
    details.spawn(text(role.line, 13.0, UiColor::Body));
    spawn_hull_scene(
        details,
        assets,
        index,
        role.hull,
        Node {
            height: px(150),
            flex_shrink: 0.0,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
            ..default()
        },
        HULL_VIEWPORT,
    );
    details.spawn(separator());
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
            column_gap: px(12),
            min_height: px(24),
            ..control_row(JustifyContent::FlexStart)
        })
        .with_children(|r| {
            r.spawn(cell(
                DETAIL_LABEL_W,
                &label.to_uppercase(),
                11.0,
                UiColor::Label,
                false,
            ));
            r.spawn(grow_cell(value, 13.0, UiColor::Body));
        });
}

/// The modal's button row, as the game's `actions_row`.
fn actions_row() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        column_gap: px(8),
        margin: UiRect::top(px(12)),
        ..default()
    }
}

/// A fixed-width slot for one button, as the game's `button_slot`.
fn button_slot(width: f32) -> Node {
    Node {
        width: px(width),
        flex_shrink: 0.0,
        ..default()
    }
}
