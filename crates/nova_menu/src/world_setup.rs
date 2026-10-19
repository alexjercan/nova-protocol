//! The New Game setup modal: the name a new open world is saved under, the
//! seed it is generated from, and the start role the player ship spawns as.
//!
//! New Game does not leave the menu. It opens this modal over it with a fresh
//! seed drawn from the game's entropy. The modal has two steps in one panel:
//! the world step (name and seed over the seed's chart preview, then Next)
//! and the role step (the start roles, the selected role's details with its
//! hull turning and its milestones, then Create). Both steps stay spawned
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
use nova_assets::prelude::LoadedSectionPacks;
use nova_events::prelude::{Meters, SectorCoord};
use nova_gameplay::prelude::*;
use nova_hud::prelude::{ChartSceneCell, ChartSceneMark, ChartSceneMarkType, ChartSceneView};
use nova_scenario::prelude::{
    resolve_ship_design, GameShipDesigns, GameStartRoles, SectionSource, ShipDesignId,
    ShipDesignSource, StartRoleDesignId,
};
use nova_ship::prelude::{DesignSection, GameSections, GameStyles, SectionConfig, SectionKind};
use nova_ui::{
    prelude::REPORT_Z,
    theme,
    theme::UiColor,
    widget::{
        list_row, panel, separator, text_field, ButtonVariant, Selected, TextFieldError,
        TextFieldSpec, TextFieldValue, ThemedBorder, ThemedRadius, ThemedText, UiText,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use nova_world_base::prelude::{create_world, world_slug, WorldSaveSession};
use nova_world_base::prelude::{
    start_preview, start_session, NovaLayeredWorld, START_PREVIEW_REACH,
};
use rand::Rng as _;

use crate::{
    preview::{preview_node, MenuChartPreview, MenuHullPreview},
    scenarios::NewGameScenario,
    widgets::{
        actions_row, back_button, button, button_slot, button_variant, detail_row, MenuSfxButton,
        MAIN_SLOT_W, SIDE_SLOT_W,
    },
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

/// The name the name field opens on when no world has it yet; see
/// [`unique_world_name`] for the one it opens on otherwise.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_WORLD_NAME: &str = "New World";

/// The Randomize slot beside the seed field.
const RANDOMIZE_SLOT_W: f32 = 130.0;

/// The one line over the fields: what a world is, in brief.
const PROFILE_TEXT: &str = "A seeded open world of asteroid clusters, planetoids and derelicts.";

/// The height of the seed's chart preview on the world step.
const SEED_PREVIEW_H: f32 = 200.0;

/// The height of the hull's view on the role step.
const HULL_PREVIEW_H: f32 = 150.0;

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

/// Marker for the seed's chart preview on the world step.
#[derive(Component)]
pub(crate) struct SeedPreview;

/// Marker for the line under the seed preview: what it shows, or why the
/// seed has no start.
#[derive(Component)]
pub(crate) struct SeedPreviewCaption;

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
    #[cfg(not(target_arch = "wasm32"))] root: Res<WorldsRoot>,
) {
    let seed = rng.next_u32().to_string();
    #[cfg(not(target_arch = "wasm32"))]
    let name = unique_world_name(DEFAULT_WORLD_NAME, |slug| {
        root.0
            .as_deref()
            .is_some_and(|root| root.join(slug).exists())
    });
    #[cfg(target_arch = "wasm32")]
    let name = String::new();
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
                        .with_children(spawn_world_step(seed, name));
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

/// The first free name in the series `base`, `base 2`, `base 3`, ...: the
/// first whose slug `taken` does not hold. A name that is no world name at
/// all ends the series as it is, and Create refuses it under the field.
///
/// A number, not `(1)`: a world name takes letters, digits, space, `_` and
/// `-` only, so a bracket would be refused by [`world_slug`].
#[cfg(not(target_arch = "wasm32"))]
fn unique_world_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut candidate = base.to_string();
    let mut n = 2u32;
    while world_slug(&candidate).is_ok_and(|slug| taken(&slug)) {
        candidate = format!("{base} {n}");
        n += 1;
    }
    candidate
}

/// The world step: one line on what a world is, the name (desktop) or the
/// web note, the seed with Randomize beside it, then Cancel and Next.
fn spawn_world_step(seed: String, name: String) -> impl FnOnce(&mut ChildSpawnerCommands) {
    #[cfg(target_arch = "wasm32")]
    let _ = name;
    move |parent| {
        parent.spawn(label("New Game Profile", PROFILE_TEXT));
        #[cfg(not(target_arch = "wasm32"))]
        {
            parent.spawn(label("World Name Label", "World name"));
            parent.spawn((
                Name::new("World Name Field"),
                WorldNameField,
                text_field(TextFieldSpec::new(name).max_chars(NAME_CHARS)),
            ));
        }
        #[cfg(target_arch = "wasm32")]
        parent.spawn(label(
            "Web Saves Note",
            "Saved worlds need the desktop build.",
        ));
        parent.spawn(label("World Seed Label", "World seed"));
        parent
            .spawn((
                Name::new("World Seed Row"),
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Name::new("World Seed Field Slot"),
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        ..default()
                    },
                ))
                .with_child((
                    Name::new("World Seed Field"),
                    WorldSeedField,
                    text_field(TextFieldSpec::new(seed).max_chars(SEED_DIGITS)),
                ));
                row.spawn(button_slot("Randomize Seed Button Slot", RANDOMIZE_SLOT_W))
                    .with_child((
                        Name::new("Randomize Seed Button"),
                        button("Randomize"),
                        observe(on_randomize_seed),
                    ));
            });
        // The seed's chart: `refresh_seed_preview` draws it from the typed
        // seed and writes the caption.
        parent.spawn((
            SeedPreview,
            preview_node(
                "Seed Preview",
                Node {
                    height: px(SEED_PREVIEW_H),
                    flex_shrink: 0.0,
                    margin: UiRect::top(px(4)),
                    border: UiRect::all(px(theme::BORDER_W)),
                    ..default()
                },
            ),
            ThemedBorder::new(UiColor::Label),
            ThemedRadius::control(),
        ));
        parent.spawn((SeedPreviewCaption, label("Seed Preview Caption", "")));
        parent
            .spawn(actions_row("World Step Actions", JustifyContent::FlexEnd))
            .with_children(|actions| {
                actions
                    .spawn(button_slot("Cancel New Game Button Slot", SIDE_SLOT_W))
                    .with_child((
                        Name::new("Cancel New Game Button"),
                        back_button("Cancel"),
                        observe(on_cancel_new_game),
                    ));
                actions
                    .spawn(button_slot("World Next Button Slot", MAIN_SLOT_W))
                    .with_child((
                        Name::new("World Next Button"),
                        WorldNextButton,
                        button_variant("Next", ButtonVariant::Primary, None),
                        observe(on_world_next),
                    ));
            });
    }
}

/// The role step: one row per role beside the selected role's details, then
/// Back, Cancel and Create in fixed slots. Create is greyed when there is no
/// role.
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
                        // `MenuSfxButton` gives the row the menu click and
                        // hover cues, as the Mods checkbox row has them.
                        let mut row = list.spawn((
                            Name::new(format!("Start Role Button: {id}")),
                            StartRoleButton(id.clone()),
                            list_row(),
                            Button,
                            Hovered::default(),
                            MenuSfxButton,
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
        parent
            .spawn(actions_row("Role Step Actions", JustifyContent::FlexEnd))
            .with_children(|actions| {
                actions
                    .spawn(button_slot("World Back Button Slot", SIDE_SLOT_W))
                    .with_child((
                        Name::new("World Back Button"),
                        back_button("Back"),
                        observe(on_world_back),
                    ));
                actions
                    .spawn(button_slot("Cancel Role Step Button Slot", SIDE_SLOT_W))
                    .with_child((
                        Name::new("Cancel Role Step Button"),
                        back_button("Cancel"),
                        observe(on_cancel_new_game),
                    ));
                actions
                    .spawn(button_slot("Create World Button Slot", MAIN_SLOT_W))
                    .with_children(|slot| {
                        let mut create = slot.spawn((
                            Name::new("Create World Button"),
                            button_variant("Create", ButtonVariant::Primary, None),
                            observe(on_create_world),
                        ));
                        if selected.is_none() {
                            create.insert(InteractionDisabled);
                        }
                    });
            });
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

/// What a role's hull adds up to, as `LABEL, value` lines: its catalog name,
/// its length, its sections, its weapons and its drives.
///
/// The lint refuses a role whose hull is not in the catalog; the id alone
/// stands in only if a mod reload dropped the hull since. A placed section
/// whose prototype the catalog lacks is skipped the way the spawn skips it.
fn hull_card(
    hull: &ShipDesignId,
    designs: &GameShipDesigns,
    sections: &GameSections,
) -> Vec<(&'static str, String)> {
    let Some(design) = designs.get_design(hull) else {
        return vec![("Hull", hull.to_string())];
    };
    let placed = &design.design.sections;
    let configs: Vec<&SectionConfig> = placed
        .iter()
        .filter_map(|section| match &section.source {
            SectionSource::Inline(config) => Some(config),
            SectionSource::Prototype { id, .. } => sections.get_section(id),
        })
        .collect();
    let (low, high) = placed.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(low, high), section| (low.min(section.position), high.max(section.position)),
    );
    // The span of the mount cells along the longest axis, plus the one cell
    // the last section fills; a cell is one engine unit.
    let span_cells = (high - low).max_element().max(0.0) + 1.0;
    let length = Meters::from_engine(span_cells).0;
    let count = |pick: fn(&SectionKind) -> bool| configs.iter().filter(|c| pick(&c.kind)).count();
    let turrets = count(|kind| matches!(kind, SectionKind::Turret(_)));
    let bays = count(|kind| matches!(kind, SectionKind::Torpedo(_)));
    let railguns = count(|kind| matches!(kind, SectionKind::Railgun(_)));
    let beams = count(|kind| matches!(kind, SectionKind::Mining(_)));
    let drives = count(|kind| matches!(kind, SectionKind::Thruster(_)));
    let mut arms: Vec<String> = Vec::new();
    for (n, one, many) in [
        (turrets, "turret", "turrets"),
        (bays, "torpedo bay", "torpedo bays"),
        (railguns, "railgun", "railguns"),
        (beams, "mining beam", "mining beams"),
    ] {
        match n {
            0 => {}
            1 => arms.push(format!("1 {one}")),
            n => arms.push(format!("{n} {many}")),
        }
    }
    let weapons = if arms.is_empty() {
        "unarmed".to_string()
    } else {
        arms.join(", ")
    };
    vec![
        ("Hull", design.name.clone()),
        ("Length", format!("{length:.0} m")),
        ("Sections", placed.len().to_string()),
        ("Weapons", weapons),
        ("Drives", drives.to_string()),
    ]
}

/// The sections a role's hull places, resolved against the catalogs, for
/// the hull view: `None` when the catalog lacks the hull. A placed section
/// whose prototype the catalog lacks is skipped the way the spawn skips it.
fn hull_sections(
    hull: &ShipDesignId,
    designs: &GameShipDesigns,
    sections: &GameSections,
) -> Option<Vec<DesignSection>> {
    designs.get_design(hull)?;
    let source = ShipDesignSource::Prototype {
        id: hull.clone(),
        section_patches: Default::default(),
    };
    let (resolved, _) = resolve_ship_design(&source, designs, sections);
    Some(
        resolved
            .sections
            .into_iter()
            .map(|section| DesignSection {
                id: section.id,
                position: section.position,
                rotation: section.rotation,
                config: section.config,
            })
            .collect(),
    )
}

/// The milestones a role step lists, in play order: the role's own goal.
fn milestones(goal_title: &str) -> [String; 1] {
    [format!("1. {goal_title}")]
}

/// Rebuild the details pane from the selected role: its name, how it plays,
/// its hull turning in its view, its hull's card ([`hull_card`]), its kit,
/// its milestones ([`milestones`]) and its lore.
pub(crate) fn refresh_start_role_details(
    mut commands: Commands,
    selections: Query<&SelectedStartRole, Changed<SelectedStartRole>>,
    panes: Query<Entity, With<StartRoleDetails>>,
    roles: Res<GameStartRoles>,
    designs: Res<GameShipDesigns>,
    sections: Res<GameSections>,
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
    let card = hull_card(&role.hull, &designs, &sections);
    let hull = hull_sections(&role.hull, &designs, &sections);
    let steps = milestones(&role.goal.title);
    // The lint refuses a kit item the catalog lacks; the id stands in only
    // if a mod reload dropped it since.
    let stock: Vec<String> = role
        .kit
        .stock
        .stacks()
        .map(|(item, count)| {
            let name = items
                .get(item)
                .map_or_else(|| item.to_string(), |design| design.name.clone());
            format!("{count} {name}")
        })
        .collect();
    let cargo = if stock.is_empty() {
        "empty hold".to_string()
    } else {
        stock.join(", ")
    };
    commands.entity(pane).with_children(|details| {
        details.spawn((
            Name::new("Start Role Details Name"),
            UiText,
            Text::new(role.name.clone()),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Primary),
        ));
        details.spawn((
            Name::new("Start Role Details About"),
            UiText,
            Text::new(role.about.clone()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Body),
        ));
        if let Some(hull) = hull {
            details.spawn((
                MenuHullPreview(hull),
                preview_node(
                    "Start Role Hull Preview",
                    Node {
                        height: px(HULL_PREVIEW_H),
                        flex_shrink: 0.0,
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                ),
                ThemedBorder::new(UiColor::Label),
                ThemedRadius::control(),
            ));
        }
        details.spawn((Name::new("Start Role Details Separator"), separator()));
        for (label, value) in card {
            details.spawn(detail_row(
                "Start Role Details Hull Line",
                label,
                value,
                UiColor::Body,
            ));
        }
        details.spawn(detail_row(
            "Start Role Details Credits",
            "Credits",
            format!("{} cr", role.kit.credits),
            UiColor::Body,
        ));
        details.spawn(detail_row(
            "Start Role Details Kit",
            "Cargo",
            cargo,
            UiColor::Body,
        ));
        for (index, step) in steps.into_iter().enumerate() {
            details.spawn(detail_row(
                "Start Role Details Milestone",
                if index == 0 { "Milestones" } else { "" },
                step,
                UiColor::Body,
            ));
        }
        details.spawn((
            Name::new("Start Role Details Lore"),
            UiText,
            Text::new(role.lore.clone()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Label),
            Node {
                margin: UiRect::top(px(8)),
                ..default()
            },
        ));
    });
}

/// Draw the typed seed's chart in the world step's preview: the cells round
/// the seed's starting station as the generator describes them, every
/// station in reach marked and the start where it stands in its cell. A
/// seed with no start draws an empty grid and says why under it. Only the
/// start is labelled, YOU: the gallery mock also labels the starting station
/// STN-1, but a chart code is the station's place in the found order, and
/// sensors may find another station first, so no code is known before play.
/// Runs when
/// the seed field changes or the preview is new; a seed that is not a number
/// is the field's refusal and leaves the last chart standing.
pub(crate) fn refresh_seed_preview(
    mut commands: Commands,
    seeds: Query<Ref<TextFieldValue>, With<WorldSeedField>>,
    previews: Query<(Entity, Has<MenuChartPreview>), With<SeedPreview>>,
    mut captions: Query<&mut Text, With<SeedPreviewCaption>>,
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
    items: Res<GameItems>,
) {
    let (Some(seed), Ok((preview, drawn))) = (seeds.iter().next(), previews.single()) else {
        return;
    };
    if drawn && !seed.is_changed() {
        return;
    }
    let Some(seed) = parse_world_seed(&seed.0) else {
        return;
    };
    let generator = NovaLayeredWorld::from_loaded(&loaded, &styles, &items).map_err(|faults| {
        faults
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    });
    let described = generator
        .and_then(|generator| start_preview(seed, &generator).map_err(|fault| fault.to_string()));
    let (view, caption) = match described {
        Ok(described) => {
            let stations = described.stations.len();
            let view = ChartSceneView {
                centre: described.cell(),
                reach: START_PREVIEW_REACH,
                cells: described
                    .cells
                    .iter()
                    .map(|(at, clusters)| ChartSceneCell {
                        at: *at,
                        clusters: clusters.clone(),
                    })
                    .collect(),
                marks: described
                    .stations
                    .iter()
                    .map(|(at, dead)| ChartSceneMark {
                        at: *at,
                        kind: ChartSceneMarkType::Station {
                            dead: *dead,
                            owned: false,
                            refusing: false,
                        },
                        code: String::new(),
                    })
                    .collect(),
                selected: None,
                you: Some(described.start_within_cell() * nova_hud::prelude::CHART_CELL),
            };
            let caption = format!(
                "The sectors round the start of seed {seed}: {stations} {} in reach. \
                 Right-drag turns, wheel zooms.",
                if stations == 1 { "station" } else { "stations" }
            );
            (view, caption)
        }
        Err(refusal) => (
            ChartSceneView {
                centre: SectorCoord::ORIGIN,
                reach: START_PREVIEW_REACH,
                cells: Vec::new(),
                marks: Vec::new(),
                selected: None,
                you: None,
            },
            format!("Seed {seed} has no start: {refusal}"),
        ),
    };
    commands.entity(preview).insert(MenuChartPreview(view));
    for mut text in &mut captions {
        text.0 = caption.clone();
    }
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
    field: Single<(Entity, &TextFieldValue), With<WorldSeedField>>,
    loaded: Res<LoadedSectionPacks>,
    styles: Res<GameStyles>,
    items: Res<GameItems>,
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
    let (seed_entity, seed_field) = *field;
    let Some(seed) = parse_world_seed(&seed_field.0) else {
        return;
    };
    let Some(role) = selection.0.clone() else {
        return;
    };
    // The seed's starting station, before anything is created for it: a
    // seed with no allied station near the origin is refused at the field.
    let session = match start_session(seed, role.clone(), &loaded, &styles, &items) {
        Ok(session) => session,
        Err(refusal) => {
            commands.entity(seed_entity).insert(TextFieldError(refusal));
            #[cfg(not(target_arch = "wasm32"))]
            show_step(&mut nodes, WorldSetupStepType::World);
            return;
        }
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
    commands.insert_resource(session);
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::unique_world_name;

    /// The default name counts up past the worlds that exist: a fresh root
    /// opens on the base name, a root holding it opens on `base 2`, and the
    /// first free number wins over a gap.
    #[test]
    fn the_default_world_name_counts_past_taken_names() {
        let free = |_: &str| false;
        assert_eq!(unique_world_name("New World", free), "New World");

        let one = |slug: &str| slug == "new-world";
        assert_eq!(unique_world_name("New World", one), "New World 2");

        let taken = ["new-world", "new-world-2", "new-world-4"];
        let several = |slug: &str| taken.contains(&slug);
        assert_eq!(unique_world_name("New World", several), "New World 3");
    }
}
