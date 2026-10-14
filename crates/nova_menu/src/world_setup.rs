//! The New Game setup modal: the name a new open world is saved under, the
//! seed it is generated from, and the start role the player ship spawns as.
//!
//! New Game does not leave the menu. It opens this modal over it with a fresh
//! seed drawn from the game's entropy. The modal has two steps in one panel:
//! the world step (name and seed, then Next) and the role step (the start
//! roles, the selected role's details, then Create). Both steps stay spawned
//! and only one is shown, so Back keeps what was typed. Only Create starts the
//! world. On the desktop build the name field opens on `New World`, and Create
//! first makes the world's folder under [`WorldsRoot`]; a name that is taken
//! or a folder that cannot be made is refused under the name field on the
//! world step and starts nothing. The world then saves itself as it is played,
//! and the Load screen brings it back. The web build saves nothing: the modal
//! says so, and the seed is the whole handle on a world.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate, Button},
};
use bevy_rand::prelude::*;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::{
    GameShipDesigns, GameStartRoles, OpenWorldSession, StartRoleDesignId,
};
use nova_ui::{
    prelude::REPORT_Z,
    theme,
    theme::UiColor,
    widget::{
        list_row, panel, text_field, ButtonVariant, Selected, TextFieldError, TextFieldSpec,
        TextFieldValue, ThemedRadius, ThemedText, UiText,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use nova_world_base::prelude::{create_world, world_slug, WorldSaveSession};
use rand::Rng as _;

use crate::{
    scenarios::NewGameScenario,
    widgets::{back_button, button, button_variant},
};

/// The panel width on the world step.
const WORLD_STEP_W: f32 = 420.0;

/// The panel width on the role step: the role list and the details side by
/// side.
const ROLE_STEP_W: f32 = 760.0;

/// The role list's width on the role step.
const ROLE_LIST_W: f32 = 240.0;

/// The widest seed: `u32::MAX` is ten digits.
const SEED_DIGITS: usize = 10;

/// The message under the seed field when what is typed is not a seed.
const SEED_REFUSAL: &str = "Enter a whole number from 0 to 4294967295";

/// The longest name the field takes: where `world_slug` refuses a name.
#[cfg(not(target_arch = "wasm32"))]
const NAME_CHARS: usize = 32;

/// The name the name field opens on. A taken name is refused like a typed
/// one: the player renames the world, nothing numbers it.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_WORLD_NAME: &str = "New World";

/// The refusal under the name field when there is no worlds folder.
#[cfg(not(target_arch = "wasm32"))]
const NO_WORLDS_ROOT: &str = "This system has no folder for saved worlds";

/// The folder saved worlds live in, or `None` when this system names none:
/// Create and Load then refuse visibly and start nothing.
///
/// `NovaMenuPlugin` inserts it from [`nova_assets::storage::worlds_root`]
/// unless the app already has one, so a test points it at a scratch folder
/// and never at the player's own worlds.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone, Debug)]
pub(crate) struct WorldsRoot(pub(crate) Option<std::path::PathBuf>);

/// Marker for the modal root.
#[derive(Component)]
pub(crate) struct WorldSetupOverlay;

/// Marker for the world name field. Desktop only: the web build saves no
/// world.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
pub(crate) struct WorldNameField;

/// Marker for the seed field.
#[derive(Component)]
pub(crate) struct WorldSeedField;

/// Marker for the panel, widened on the role step.
#[derive(Component)]
pub(crate) struct WorldSetupPanel;

/// Marker for the world step: the name, the seed and Next.
#[derive(Component)]
pub(crate) struct WorldStep;

/// Marker for the role step: the start roles, their details and Create.
#[derive(Component)]
pub(crate) struct RoleStep;

/// Which step of the modal is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldSetupStepType {
    /// The world name and seed.
    World,
    /// The start role.
    Role,
}

/// Marker for the Next button, greyed while the name or the seed is refused.
#[derive(Component)]
pub(crate) struct WorldNextButton;

/// The start role the role step has selected, on the modal root. `None` only
/// when the loaded content has no start role: Create is then greyed.
#[derive(Component, Debug)]
pub(crate) struct SelectedStartRole(pub(crate) Option<StartRoleDesignId>);

/// One start role's button in the role list.
#[derive(Component)]
pub(crate) struct StartRoleButton(pub(crate) StartRoleDesignId);

/// The selected role's details pane; [`refresh_start_role_details`] rebuilds
/// its children.
#[derive(Component)]
pub(crate) struct StartRoleDetails;

/// The seed `text` spells: decimal digits only, inside `u32`. Blank, signed,
/// and overflowing text is not a seed.
pub(crate) fn parse_world_seed(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// A small label line in the modal.
fn label(name: &'static str, text: &str) -> impl Bundle {
    (
        Name::new(name),
        Text::new(text.to_string()),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColor::Label),
    )
}

/// Open the modal on the world step with a fresh seed and the first start
/// role selected.
pub(crate) fn on_new_game(
    _activate: On<Activate>,
    mut commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    roles: Res<GameStartRoles>,
) {
    let seed = rng.next_u32().to_string();
    let first_role = roles.first().map(|role| role.id.clone());
    commands
        .spawn((
            WorldSetupOverlay,
            SelectedStartRole(first_role.clone()),
            DespawnOnExit(GameStates::MainMenu),
            Name::new("New Game Overlay"),
            // A modal blocker: the menu behind it takes no click while the
            // player is choosing a world.
            Pickable {
                should_block_lower: true,
                is_hoverable: false,
            },
            // ABSOLUTE for the same reason as the safe-mode report: the menu's
            // hidden screens still take layout space, and a modal in the flow
            // lays out below the viewport.
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
            GlobalZIndex(REPORT_Z),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("New Game Panel"),
                    WorldSetupPanel,
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        width: px(WORLD_STEP_W),
                        padding: UiRect::all(px(20)),
                        row_gap: px(8),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::control(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("New Game Title"),
                        Text::new("New Game"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Body),
                    ));
                    parent
                        .spawn((Name::new("World Step"), WorldStep, step_node(Display::Flex)))
                        .with_children(spawn_world_step(seed));
                    parent
                        .spawn((Name::new("Role Step"), RoleStep, step_node(Display::None)))
                        .with_children(spawn_role_step(&roles, first_role.as_ref()));
                });
        });
}

/// One step's column, shown or hidden by `display`.
fn step_node(display: Display) -> Node {
    Node {
        display,
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,
        row_gap: px(8),
        ..default()
    }
}

/// The world step: the profile, the name (desktop) or the web note, the seed,
/// Randomize, Next and Cancel.
fn spawn_world_step(seed: String) -> impl FnOnce(&mut ChildSpawnerCommands) {
    move |parent| {
        parent.spawn((label(
            "New Game Profile",
            "World: sparse asteroid clusters, planetoids and derelicts, generated from the \
                 seed and streamed in as you fly. The same seed gives the same world on the \
                 same build.",
        ),));
        #[cfg(not(target_arch = "wasm32"))]
        {
            parent.spawn(label("World Name Label", "World name"));
            parent.spawn((
                Name::new("World Name Field"),
                WorldNameField,
                text_field(TextFieldSpec::new(DEFAULT_WORLD_NAME).max_chars(NAME_CHARS)),
            ));
        }
        #[cfg(target_arch = "wasm32")]
        parent.spawn(label(
            "Web Saves Note",
            "Saved worlds need the desktop build.",
        ));
        parent.spawn(label("World Seed Label", "World seed"));
        parent.spawn((
            Name::new("World Seed Field"),
            WorldSeedField,
            text_field(TextFieldSpec::new(seed).max_chars(SEED_DIGITS)),
        ));
        parent.spawn((
            Name::new("Randomize Seed Button"),
            button("Randomize"),
            observe(on_randomize_seed),
        ));
        parent.spawn((
            Name::new("World Next Button"),
            WorldNextButton,
            button_variant("Next", ButtonVariant::Primary, None),
            observe(on_world_next),
        ));
        parent.spawn((
            Name::new("Cancel New Game Button"),
            back_button("Cancel"),
            observe(on_cancel_new_game),
        ));
    }
}

/// The role step: one button per role beside the selected role's details,
/// then Create, Back and Cancel. Create is greyed when there is no role.
fn spawn_role_step(
    roles: &GameStartRoles,
    selected: Option<&StartRoleDesignId>,
) -> impl FnOnce(&mut ChildSpawnerCommands) {
    let rows: Vec<(StartRoleDesignId, String)> = roles
        .iter()
        .map(|role| (role.id.clone(), role.name.clone()))
        .collect();
    let selected = selected.cloned();
    move |parent| {
        parent
            .spawn((
                Name::new("Start Role Body"),
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    ..default()
                },
            ))
            .with_children(|body| {
                body.spawn((
                    Name::new("Start Role List"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: px(ROLE_LIST_W),
                        flex_shrink: 0.0,
                        row_gap: px(4),
                        ..default()
                    },
                ))
                .with_children(|list| {
                    list.spawn(label("Start Role Label", "Start role"));
                    for (id, name) in rows {
                        let mut row = list.spawn((
                            Name::new(format!("Start Role Button: {id}")),
                            StartRoleButton(id.clone()),
                            list_row(),
                            Button,
                            Hovered::default(),
                            observe(on_start_role_select),
                        ));
                        if selected.as_ref() == Some(&id) {
                            row.insert(Selected);
                        }
                        row.with_child((
                            Name::new("Start Role Button Name"),
                            UiText,
                            Text::new(name),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(Color::NONE),
                            ThemedText::new(UiColor::Body),
                        ));
                    }
                });
                body.spawn((
                    Name::new("Start Role Details"),
                    StartRoleDetails,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        row_gap: px(6),
                        ..default()
                    },
                ));
            });
        let mut create = parent.spawn((
            Name::new("Create World Button"),
            button_variant("Create", ButtonVariant::Primary, None),
            observe(on_create_world),
        ));
        if selected.is_none() {
            create.insert(InteractionDisabled);
        }
        parent.spawn((
            Name::new("World Back Button"),
            back_button("Back"),
            observe(on_world_back),
        ));
        parent.spawn((
            Name::new("Cancel Role Step Button"),
            back_button("Cancel"),
            observe(on_cancel_new_game),
        ));
    }
}

/// Show `step` and hide the other, and size the panel for it. Both steps
/// stay spawned, so the fields keep what was typed.
fn show_step(nodes: &mut StepNodes, step: WorldSetupStepType) {
    for (mut node, world, role) in nodes {
        let shown = match step {
            WorldSetupStepType::World => world,
            WorldSetupStepType::Role => role,
        };
        if world || role {
            node.display = if shown { Display::Flex } else { Display::None };
        } else {
            node.width = px(match step {
                WorldSetupStepType::World => WORLD_STEP_W,
                WorldSetupStepType::Role => ROLE_STEP_W,
            });
        }
    }
}

/// The step nodes and the panel, for [`show_step`].
type StepNodes<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, Has<WorldStep>, Has<RoleStep>),
    Or<(With<WorldStep>, With<RoleStep>, With<WorldSetupPanel>)>,
>;

/// Go on to the role step when the typed seed and name are accepted.
///
/// Next is greyed while either is refused; the check here stops a press
/// that lands before [`read_world_setup`] greys it.
pub(crate) fn on_world_next(
    _activate: On<Activate>,
    seed: Single<&TextFieldValue, With<WorldSeedField>>,
    #[cfg(not(target_arch = "wasm32"))] name: Single<
        &TextFieldValue,
        (With<WorldNameField>, Without<WorldSeedField>),
    >,
    mut nodes: StepNodes,
) {
    if parse_world_seed(&seed.0).is_none() {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if world_slug(&name.0).is_err() {
        return;
    }
    show_step(&mut nodes, WorldSetupStepType::Role);
}

/// Go back to the world step, with the name and seed as they were typed.
pub(crate) fn on_world_back(_activate: On<Activate>, mut nodes: StepNodes) {
    show_step(&mut nodes, WorldSetupStepType::World);
}

/// Select the pressed role and move the row highlight to it.
pub(crate) fn on_start_role_select(
    activate: On<Activate>,
    mut commands: Commands,
    buttons: Query<&StartRoleButton>,
    highlighted: Query<Entity, (With<StartRoleButton>, With<Selected>)>,
    mut selection: Single<&mut SelectedStartRole>,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    if selection.0.as_ref() == Some(&button.0) {
        return;
    }
    for previous in &highlighted {
        commands.entity(previous).remove::<Selected>();
    }
    commands.entity(activate.entity).insert(Selected);
    selection.0 = Some(button.0.clone());
}

/// Rebuild the details pane from the selected role: its name, how it plays,
/// its hull's catalog name, its kit and its lore.
pub(crate) fn refresh_start_role_details(
    mut commands: Commands,
    selections: Query<&SelectedStartRole, Changed<SelectedStartRole>>,
    panes: Query<Entity, With<StartRoleDetails>>,
    roles: Res<GameStartRoles>,
    designs: Res<GameShipDesigns>,
    items: Res<GameItems>,
) {
    let Some(selection) = selections.iter().next() else {
        return;
    };
    let Ok(pane) = panes.single() else {
        return;
    };
    commands.entity(pane).despawn_related::<Children>();
    let Some(role) = selection.0.as_ref().and_then(|id| roles.get_role(id)) else {
        commands.entity(pane).with_child(label(
            "Start Role Details Empty",
            "No start role is loaded.",
        ));
        return;
    };
    // The lint refuses a role whose hull or kit item is not in the catalog;
    // the id stands in only if a mod reload dropped one since.
    let hull = designs
        .get_design(&role.hull)
        .map_or_else(|| role.hull.to_string(), |design| design.name.clone());
    let mut kit = vec![format!("{} cr", role.kit.credits)];
    kit.extend(role.kit.stock.stacks().map(|(item, count)| {
        let name = items
            .get(item)
            .map_or_else(|| item.to_string(), |design| design.name.clone());
        format!("{count} {name}")
    }));
    commands.entity(pane).with_children(|details| {
        details.spawn((
            Name::new("Start Role Details Name"),
            Text::new(role.name.clone()),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Body),
        ));
        for (name, text, color) in [
            (
                "Start Role Details About",
                role.about.clone(),
                UiColor::Body,
            ),
            (
                "Start Role Details Hull",
                format!("Hull: {hull}"),
                UiColor::Body,
            ),
            (
                "Start Role Details Kit",
                format!("Kit: {}", kit.join(", ")),
                UiColor::Body,
            ),
            ("Start Role Details Lore", role.lore.clone(), UiColor::Label),
        ] {
            details.spawn((
                Name::new(name),
                Text::new(text),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(color),
            ));
        }
    });
}

/// Refuse a typed name or seed inline, and grey Next while either stands.
///
/// Reads the fields, not the press: a bad name or seed shows as refused while
/// it is typed, and Next cannot look like it works on it. A name is checked
/// for its form here; a taken name is refused by Create itself, which is the
/// one check that cannot go stale.
pub(crate) fn read_world_setup(
    mut commands: Commands,
    #[cfg(not(target_arch = "wasm32"))] names: Query<
        (Entity, Ref<TextFieldValue>),
        (With<WorldNameField>, Without<WorldSeedField>),
    >,
    seeds: Query<(Entity, Ref<TextFieldValue>), With<WorldSeedField>>,
    buttons: Query<Entity, With<WorldNextButton>>,
) {
    let Some((seed_field, seed)) = seeds.iter().next() else {
        return;
    };
    #[cfg_attr(
        target_arch = "wasm32",
        expect(unused_mut, reason = "only the native build adds the name check")
    )]
    let mut checks = vec![(
        seed_field,
        seed.is_changed(),
        parse_world_seed(&seed.0)
            .map(|_| ())
            .ok_or_else(|| SEED_REFUSAL.to_string()),
    )];
    #[cfg(not(target_arch = "wasm32"))]
    if let Some((name_field, name)) = names.iter().next() {
        checks.push((
            name_field,
            name.is_changed(),
            world_slug(&name.0)
                .map(|_| ())
                .map_err(|refusal| refusal.to_string()),
        ));
    }
    if !checks.iter().any(|(_, changed, _)| *changed) {
        return;
    }
    for (field, changed, check) in &checks {
        if !changed {
            continue;
        }
        match check {
            Ok(()) => {
                commands.entity(*field).remove::<TextFieldError>();
            }
            Err(refusal) => {
                commands
                    .entity(*field)
                    .insert(TextFieldError(refusal.clone()));
            }
        }
    }
    let valid = checks.iter().all(|(_, _, check)| check.is_ok());
    for button in &buttons {
        if valid {
            commands.entity(button).remove::<InteractionDisabled>();
        } else {
            commands.entity(button).insert(InteractionDisabled);
        }
    }
}

/// Write a fresh seed into the field.
pub(crate) fn on_randomize_seed(
    _activate: On<Activate>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut fields: Query<&mut TextFieldValue, With<WorldSeedField>>,
) {
    for mut value in &mut fields {
        value.0 = rng.next_u32().to_string();
    }
}

/// Start the open world on the typed seed as the selected start role.
///
/// On the desktop build the world's folder is made first, under the typed
/// name: a taken name, an invalid one, a missing worlds folder or a folder
/// that cannot be made is refused under the name field, the modal goes back
/// to the world step, and nothing starts. The new folder's session then
/// saves the world from its first frame.
///
/// Clears the Scenarios picker's override, so `start_new_game_scenario` loads
/// the base bundle's declared start. The modal goes with the press, so a
/// second press in the same frame has nothing to land on.
pub(crate) fn on_create_world(
    _activate: On<Activate>,
    mut commands: Commands,
    field: Single<&TextFieldValue, With<WorldSeedField>>,
    #[cfg(not(target_arch = "wasm32"))] name_field: Single<
        (Entity, &TextFieldValue),
        (With<WorldNameField>, Without<WorldSeedField>),
    >,
    #[cfg(not(target_arch = "wasm32"))] root: Res<WorldsRoot>,
    #[cfg(not(target_arch = "wasm32"))] mut nodes: StepNodes,
    selection: Single<&SelectedStartRole>,
    overlays: Query<Entity, With<WorldSetupOverlay>>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
    mut pick: ResMut<NewGameScenario>,
) {
    let Some(seed) = parse_world_seed(&field.0) else {
        return;
    };
    let Some(role) = selection.0.clone() else {
        return;
    };
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (name_entity, name) = *name_field;
        let created = match &root.0 {
            Some(root) => create_world(root, &name.0).map_err(|refusal| refusal.to_string()),
            None => Err(NO_WORLDS_ROOT.to_string()),
        };
        let (folder, lock) = match created {
            Ok(created) => created,
            Err(refusal) => {
                commands.entity(name_entity).insert(TextFieldError(refusal));
                show_step(&mut nodes, WorldSetupStepType::World);
                return;
            }
        };
        commands.insert_resource(WorldSaveSession::created(
            folder,
            lock,
            name.0.trim().to_string(),
            seed,
            role.clone(),
        ));
    }
    commands.insert_resource(OpenWorldSession { seed, role });
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
    for overlay in &overlays {
        commands.entity(overlay).despawn();
    }
}

/// Close the modal and nothing else.
pub(crate) fn on_cancel_new_game(
    _activate: On<Activate>,
    mut commands: Commands,
    overlays: Query<Entity, With<WorldSetupOverlay>>,
) {
    for overlay in &overlays {
        commands.entity(overlay).despawn();
    }
}
