//! The Load picker: a two-pane overlay in the Scenarios style listing every
//! world folder under [`WorldsRoot`], that resumes the selected one through
//! the New Game handoff.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate, Button},
};
use nova_assets::prelude::LoadedSectionPacks;
use nova_gameplay::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{list_row, themed_button, Selected, ThemedText, UiText},
};
use nova_world_base::prelude::{
    list_worlds, open_world, resume_world, WorldListing, WorldRefusal, WorldResumeRefused,
    WorldSaveSession,
};

use crate::{scenarios::NewGameScenario, world_setup::WorldsRoot};

/// Marker for the Load panel root, toggled by the Load button.
#[derive(Component)]
pub(crate) struct LoadPanel;

/// The worlds the Load screen lists, or why it could not read the root.
/// Written by [`on_load_screen`]; `None` (no root) is drawn by
/// [`refresh_load_list`] directly from [`WorldsRoot`], not from here.
#[derive(Resource)]
pub(crate) struct WorldListings(pub(crate) Result<Vec<WorldListing>, WorldRefusal>);

impl Default for WorldListings {
    fn default() -> Self {
        Self(Ok(Vec::new()))
    }
}

/// The slug the details pane renders. `None` until the list populates;
/// `refresh_load_list` default-selects the first row, `on_load_world_row_select`
/// sets it from a row click.
#[derive(Resource, Default)]
pub(crate) struct SelectedWorldSlug(pub(crate) Option<String>);

/// The scrollable container holding the world rows; `refresh_load_list` swaps
/// its children when [`WorldListings`] changes.
#[derive(Component)]
pub(crate) struct LoadWorldList;

/// One clickable world row: clicking it selects the world for the details
/// pane.
#[derive(Component)]
pub(crate) struct LoadWorldRow {
    pub(crate) slug: String,
}

/// The world details side panel; `refresh_load_details` rebuilds its children
/// from the selected world.
#[derive(Component)]
pub(crate) struct LoadWorldDetails;

/// The details pane's Load button.
#[derive(Component)]
pub(crate) struct LoadWorldButton;

/// Open the Load panel: re-read the worlds root and show the panel the way
/// `on_scenarios` does.
pub(crate) fn on_load_screen(
    _activate: On<Activate>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    mut listings: ResMut<WorldListings>,
    mut selected: ResMut<SelectedWorldSlug>,
    mut panel: Single<&mut Visibility, With<LoadPanel>>,
) {
    listings.0 = match root.0.as_deref() {
        Some(root) => list_worlds(root, &packs),
        None => Ok(Vec::new()),
    };
    selected.0 = None;
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

pub(crate) fn on_load_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<LoadPanel>>,
) {
    **panel = Visibility::Hidden;
}

/// Select the clicked row's world: write [`SelectedWorldSlug`] and move the
/// row [`Selected`] highlight, the same way `on_scenario_row_select` does.
pub(crate) fn on_load_world_row_select(
    activate: On<Activate>,
    rows: Query<(Entity, &LoadWorldRow)>,
    selected_rows: Query<Entity, (With<LoadWorldRow>, With<Selected>)>,
    mut selected: ResMut<SelectedWorldSlug>,
    mut commands: Commands,
) {
    let Ok((entity, row)) = rows.get(activate.entity) else {
        return;
    };
    if selected.0.as_deref() == Some(row.slug.as_str()) {
        return;
    }
    for previous in &selected_rows {
        commands.entity(previous).remove::<Selected>();
    }
    commands.entity(entity).insert(Selected);
    selected.0 = Some(row.slug.clone());
}

/// Load the selected world. On `Err` (a world locked or refused since the
/// list was read), the refusal replaces that row's header so the list and
/// details redraw with it; nothing starts. On `Ok`, resume the world and hand
/// off to Playing exactly like New Game.
pub(crate) fn on_load_world(
    _activate: On<Activate>,
    mut commands: Commands,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    selected: Res<SelectedWorldSlug>,
    mut listings: ResMut<WorldListings>,
    mut pick: ResMut<NewGameScenario>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    let (Some(root), Some(slug)) = (root.0.as_deref(), selected.0.as_deref()) else {
        return;
    };
    match open_world(root, slug, &packs) {
        Ok((folder, lock, header, world_state)) => {
            commands.queue(move |world: &mut World| {
                resume_world(world, folder, lock, &header, world_state);
            });
            pick.0 = None;
            *mode = GameMode::NewGame;
            state.set(GameStates::Playing);
        }
        Err(refusal) => {
            if let Ok(rows) = &mut listings.0 {
                if let Some(row) = rows.iter_mut().find(|row| row.folder.slug == slug) {
                    row.header = Err(refusal);
                }
            }
        }
    }
}

/// Answer a refused Load ([`WorldResumeRefused`]): the teardown of "Leave
/// without saving" to the menu, then the Load panel with the reason on the
/// world's row.
///
/// The teardown drops [`WorldSaveSession`], and with it the lock, and asks
/// for `MainMenu` unpaused. The panel exists only from `OnEnter(MainMenu)`,
/// so the resource stays until the next frame finds it. That frame re-reads
/// the list from disk, as [`on_load_screen`] does, so a world never listed
/// before (New Game, then "Load last save") has its row. A failed re-read
/// shows its own error. [`SelectedWorldSlug`] is kept, so the details pane
/// shows the reason too.
pub(crate) fn refuse_resumed_world(
    mut commands: Commands,
    refused: Res<WorldResumeRefused>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    mut listings: ResMut<WorldListings>,
    mut state: ResMut<NextState<GameStates>>,
    mut pause: ResMut<NextState<PauseStates>>,
    panel: Option<Single<&mut Visibility, With<LoadPanel>>>,
) {
    commands.remove_resource::<WorldSaveSession>();
    state.set(GameStates::MainMenu);
    pause.set(PauseStates::Unpaused);
    let Some(mut panel) = panel else {
        return;
    };
    listings.0 = match root.0.as_deref() {
        Some(root) => list_worlds(root, &packs),
        None => Ok(Vec::new()),
    };
    if let Ok(rows) = &mut listings.0 {
        match rows.iter_mut().find(|row| row.folder.slug == refused.slug) {
            Some(row) => row.header = Err(WorldRefusal::Unrestored(refused.reason.clone())),
            None => error!(
                "refuse_resumed_world: {} is not in its own fresh listing ({})",
                refused.slug, refused.reason
            ),
        }
    }
    **panel = Visibility::Visible;
    commands.remove_resource::<WorldResumeRefused>();
}

/// Rebuild the world list from [`WorldListings`]: one line when the platform
/// names no worlds root, one line for a refused root, one line for an empty
/// root, otherwise one row per world. A default/repaired selection keeps the
/// details pane fed.
pub(crate) fn refresh_load_list(
    mut commands: Commands,
    root: Res<WorldsRoot>,
    listings: Res<WorldListings>,
    mut selected: ResMut<SelectedWorldSlug>,
    lists: Query<Entity, With<LoadWorldList>>,
) {
    let Ok(list) = lists.single() else {
        return;
    };
    commands.entity(list).despawn_related::<Children>();

    if root.0.is_none() {
        selected.0 = None;
        spawn_load_note(
            &mut commands,
            list,
            "This system has no folder for saved worlds",
        );
        return;
    }
    let rows = match &listings.0 {
        Ok(rows) => rows,
        Err(refusal) => {
            selected.0 = None;
            spawn_load_note(&mut commands, list, &refusal.to_string());
            return;
        }
    };
    if rows.is_empty() {
        selected.0 = None;
        spawn_load_note(&mut commands, list, "No saved worlds yet");
        return;
    }

    if !selected
        .0
        .as_deref()
        .is_some_and(|slug| rows.iter().any(|row| row.folder.slug == slug))
    {
        selected.0 = rows.first().map(|row| row.folder.slug.clone());
    }

    commands.entity(list).with_children(|list| {
        for row in rows {
            let is_selected = selected.0.as_deref() == Some(row.folder.slug.as_str());
            spawn_load_row(list, row, is_selected);
        }
    });
}

/// One explanatory line in place of the row list: no root, a refused root, or
/// an empty one.
fn spawn_load_note(commands: &mut Commands, list: Entity, text: &str) {
    commands.entity(list).with_children(|list| {
        list.spawn((
            Name::new("Load Worlds Note"),
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Label),
        ));
    });
}

/// Spawn one clickable world row: its name (or slug, when the header did not
/// read) over the refusal text, when it has one.
fn spawn_load_row(list: &mut ChildSpawnerCommands, listing: &WorldListing, selected: bool) {
    let title = match &listing.header {
        Ok(header) => header.name.clone(),
        Err(_) => listing.folder.slug.clone(),
    };
    let mut row = list.spawn((
        Name::new(format!("Load World Row: {}", listing.folder.slug)),
        LoadWorldRow {
            slug: listing.folder.slug.clone(),
        },
        list_row(),
        Button,
        Hovered::default(),
        observe(on_load_world_row_select),
    ));
    if selected {
        row.insert(Selected);
    }
    row.with_children(|row| {
        row.spawn((
            Name::new("Load World Row Name"),
            UiText,
            Text::new(title),
            TextFont {
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(Color::NONE),
            ThemedText::new(UiColor::Body),
        ));
        if let Err(refusal) = &listing.header {
            row.spawn((
                Name::new("Load World Row Refusal"),
                UiText,
                Text::new(refusal.to_string()),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Danger),
            ));
        }
    });
}

/// Rebuild the world details pane from the selected world: name, seed,
/// sector, credits, game version and saved time, or the refusal, and a Load
/// button greyed on a refusal.
pub(crate) fn refresh_load_details(
    mut commands: Commands,
    listings: Res<WorldListings>,
    selected: Res<SelectedWorldSlug>,
    panels: Query<Entity, With<LoadWorldDetails>>,
) {
    let Ok(panel) = panels.single() else {
        return;
    };
    commands.entity(panel).despawn_related::<Children>();

    let listing = selected.0.as_deref().and_then(|slug| {
        listings
            .0
            .as_ref()
            .ok()
            .and_then(|rows| rows.iter().find(|row| row.folder.slug == slug))
    });

    commands.entity(panel).with_children(|details| {
        let Some(listing) = listing else {
            details.spawn((
                Name::new("Load World Details Empty"),
                Text::new("Select a saved world to see its details."),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
            ));
            return;
        };

        match &listing.header {
            Ok(header) => {
                details.spawn((
                    Name::new("Load World Details Name"),
                    Text::new(header.name.clone()),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    ThemedText::new(UiColor::Body),
                ));
                for line in [
                    format!("Seed: {}", header.seed),
                    format!("Sector: {}", header.player_sector),
                    format!("Credits: {}", header.credits),
                    format!("Game version: {}", header.game_version),
                    format!("Saved: {} (unix seconds)", header.saved_at_unix),
                ] {
                    details.spawn((
                        Name::new("Load World Details Line"),
                        Text::new(line),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Body),
                    ));
                }
                details.spawn((
                    Name::new("Load World Button"),
                    themed_button("Load"),
                    LoadWorldButton,
                    observe(on_load_world),
                ));
            }
            Err(refusal) => {
                details.spawn((
                    Name::new("Load World Details Refusal"),
                    Text::new(refusal.to_string()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    ThemedText::new(UiColor::Danger),
                ));
                details.spawn((
                    Name::new("Load World Button"),
                    themed_button("Load"),
                    LoadWorldButton,
                    InteractionDisabled,
                    observe(on_load_world),
                ));
            }
        }
    });
}
