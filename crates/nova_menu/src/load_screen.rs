//! The Load picker: a two-pane overlay in the Scenarios style listing every
//! world folder under [`WorldsRoot`], that resumes the selected one through
//! the New Game handoff.
//!
//! The list reads only each world's header. Each world whose header reads is
//! then checked whole off the main thread with [`check_world`]: its state
//! file is read and every item it holds is looked up in the loaded catalog.
//! Until that check ends the row shows "Checking saved state" and Load is
//! greyed; a refused check replaces the row's header with the refusal. Load
//! checks again as it opens the world.

use std::{collections::HashMap, path::Path, sync::Arc};

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    tasks::{block_on, futures_lite::future, IoTaskPool, Task},
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate, Button},
};
use nova_assets::prelude::LoadedSectionPacks;
use nova_gameplay::prelude::*;
use nova_ui::{
    theme::UiColor,
    widget::{list_row, themed_button, ButtonVariant, Selected, ThemedText, UiText},
};
use nova_world_base::prelude::{
    check_world, delete_world, list_worlds, open_world, resume_world, WorldListing, WorldRefusal,
    WorldResumeRefused, WorldSaveSession,
};

use crate::{scenarios::NewGameScenario, widgets::button_variant, world_setup::WorldsRoot};

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

/// The state checks still running, by world slug, for the worlds of
/// [`WorldListings`] whose header read. Replaced on every re-read of the
/// list; dropping a task cancels it.
#[derive(Resource, Default)]
pub(crate) struct WorldChecks(pub(crate) HashMap<String, Task<Result<(), WorldRefusal>>>);

impl WorldChecks {
    /// Whether the world `slug` is still being checked.
    fn pending(&self, slug: &str) -> bool {
        self.0.contains_key(slug)
    }
}

/// The text a row and the details pane show while the world is checked.
const CHECKING_TEXT: &str = "Checking saved state";

/// The slug the details pane renders. `None` until the list populates;
/// `refresh_load_list` default-selects the first row, `on_load_world_row_select`
/// sets it from a row click.
#[derive(Resource, Default)]
pub(crate) struct SelectedWorldSlug(pub(crate) Option<String>);

/// The Load screen's delete flow for the selected world.
#[derive(Resource, Default)]
pub(crate) enum WorldDeleteStep {
    /// No delete is asked for or pending.
    #[default]
    Idle,
    /// Asking to confirm deleting this slug.
    Confirm(String),
    /// A delete refused for this slug, with the reason.
    Refused {
        /// The slug asked about.
        slug: String,
        /// Why the delete refused.
        reason: String,
    },
}

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

/// Read the worlds under `root` and start a [`check_world`] task for each
/// world whose header read, replacing every check still running.
fn read_listings(
    root: &Path,
    packs: &LoadedSectionPacks,
    items: &GameItems,
    checks: &mut WorldChecks,
) -> Result<Vec<WorldListing>, WorldRefusal> {
    let listings = list_worlds(root, packs);
    let packs = Arc::new(packs.clone());
    let items = Arc::new(items.clone());
    checks.0 = listings
        .iter()
        .flatten()
        .filter(|listing| listing.header.is_ok())
        .map(|listing| {
            let (root, slug) = (root.to_path_buf(), listing.folder.slug.clone());
            let (packs, items) = (Arc::clone(&packs), Arc::clone(&items));
            let task =
                IoTaskPool::get().spawn(async move { check_world(&root, &slug, &packs, &items) });
            (listing.folder.slug.clone(), task)
        })
        .collect();
    listings
}

/// Take every finished world check: a refusal replaces its row's header,
/// and either end redraws the list.
pub(crate) fn poll_world_checks(
    mut checks: ResMut<WorldChecks>,
    mut listings: ResMut<WorldListings>,
) {
    let mut finished = Vec::new();
    checks.0.retain(|slug, task| {
        let Some(result) = block_on(future::poll_once(task)) else {
            return true;
        };
        finished.push((slug.clone(), result));
        false
    });
    if finished.is_empty() {
        return;
    }
    if let Ok(rows) = &mut listings.0 {
        for (slug, result) in finished {
            if let (Err(refusal), Some(row)) =
                (result, rows.iter_mut().find(|row| row.folder.slug == slug))
            {
                row.header = Err(refusal);
            }
        }
    }
    listings.set_changed();
}

/// Open the Load panel: re-read the worlds root and show the panel the way
/// `on_scenarios` does.
pub(crate) fn on_load_screen(
    _activate: On<Activate>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
    mut checks: ResMut<WorldChecks>,
    mut listings: ResMut<WorldListings>,
    mut selected: ResMut<SelectedWorldSlug>,
    mut step: ResMut<WorldDeleteStep>,
    mut panel: Single<&mut Visibility, With<LoadPanel>>,
) {
    listings.0 = match root.0.as_deref() {
        Some(root) => read_listings(root, &packs, &items, &mut checks),
        None => Ok(Vec::new()),
    };
    selected.0 = None;
    *step = WorldDeleteStep::Idle;
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
    mut step: ResMut<WorldDeleteStep>,
    mut commands: Commands,
) {
    let Ok((entity, row)) = rows.get(activate.entity) else {
        return;
    };
    if selected.0.as_deref() == Some(row.slug.as_str()) {
        return;
    }
    *step = WorldDeleteStep::Idle;
    for previous in &selected_rows {
        commands.entity(previous).remove::<Selected>();
    }
    commands.entity(entity).insert(Selected);
    selected.0 = Some(row.slug.clone());
}

/// Ask to confirm deleting the selected world.
pub(crate) fn on_delete_world(
    _activate: On<Activate>,
    selected: Res<SelectedWorldSlug>,
    mut step: ResMut<WorldDeleteStep>,
) {
    if let Some(slug) = selected.0.clone() {
        *step = WorldDeleteStep::Confirm(slug);
    }
}

/// Cancel a pending delete confirmation.
pub(crate) fn on_delete_world_cancel(_activate: On<Activate>, mut step: ResMut<WorldDeleteStep>) {
    *step = WorldDeleteStep::Idle;
}

/// Delete the world asked about, then re-read the list from disk, so a
/// delete that removed only part of the folder (see [`delete_world`]) shows
/// truthfully instead of the stale list.
pub(crate) fn on_delete_world_confirm(
    _activate: On<Activate>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
    mut checks: ResMut<WorldChecks>,
    mut step: ResMut<WorldDeleteStep>,
    mut listings: ResMut<WorldListings>,
) {
    let WorldDeleteStep::Confirm(slug) = &*step else {
        return;
    };
    let Some(root) = root.0.as_deref() else {
        return;
    };
    let slug = slug.clone();
    let result = delete_world(root, &slug);
    listings.0 = read_listings(root, &packs, &items, &mut checks);
    *step = match result {
        Ok(()) => WorldDeleteStep::Idle,
        Err(refusal) => WorldDeleteStep::Refused {
            slug,
            reason: refusal.to_string(),
        },
    };
}

/// Load the selected world, checked again as it opens. On `Err` (a world
/// locked or refused since the list was read), the refusal replaces that
/// row's header so the list and details redraw with it; nothing starts. On
/// `Ok`, resume the world and hand off to Playing exactly like New Game.
pub(crate) fn on_load_world(
    _activate: On<Activate>,
    mut commands: Commands,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    items: Res<GameItems>,
    checks: Res<WorldChecks>,
    selected: Res<SelectedWorldSlug>,
    mut listings: ResMut<WorldListings>,
    mut step: ResMut<WorldDeleteStep>,
    mut pick: ResMut<NewGameScenario>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    let (Some(root), Some(slug)) = (root.0.as_deref(), selected.0.as_deref()) else {
        return;
    };
    if checks.pending(slug) {
        return;
    }
    // A Load drops a Delete asked about, so a refused resume does not bring
    // its prompt back.
    *step = WorldDeleteStep::Idle;
    match open_world(root, slug, &packs, &items) {
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
    items: Res<GameItems>,
    mut checks: ResMut<WorldChecks>,
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
        Some(root) => read_listings(root, &packs, &items, &mut checks),
        None => Ok(Vec::new()),
    };
    // The refusal stands over whatever the new check of the world finds.
    checks.0.remove(&refused.slug);
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
    checks: Res<WorldChecks>,
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
            let checking = checks.pending(&row.folder.slug);
            spawn_load_row(list, row, is_selected, checking);
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
/// read) over the refusal text, when it has one, or [`CHECKING_TEXT`] while
/// it is `checking`.
fn spawn_load_row(
    list: &mut ChildSpawnerCommands,
    listing: &WorldListing,
    selected: bool,
    checking: bool,
) {
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
        if checking {
            row.spawn((
                Name::new("Load World Row Checking"),
                UiText,
                Text::new(CHECKING_TEXT),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Label),
            ));
        }
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
/// button greyed on a refusal or while the world is checked, and a Delete
/// button (or its confirm prompt, or the reason a delete refused).
pub(crate) fn refresh_load_details(
    mut commands: Commands,
    listings: Res<WorldListings>,
    checks: Res<WorldChecks>,
    selected: Res<SelectedWorldSlug>,
    step: Res<WorldDeleteStep>,
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
        if let WorldDeleteStep::Refused { slug, reason } = &*step {
            details.spawn((
                Name::new("Load World Delete Refusal"),
                Text::new(format!("Cannot delete {slug}: {reason}")),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Danger),
            ));
        }

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

        let title = match &listing.header {
            Ok(header) => header.name.clone(),
            Err(_) => listing.folder.slug.clone(),
        };

        match &listing.header {
            Ok(header) => {
                details.spawn((
                    Name::new("Load World Details Name"),
                    Text::new(title.clone()),
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
                let mut load = details.spawn((
                    Name::new("Load World Button"),
                    themed_button("Load"),
                    LoadWorldButton,
                    observe(on_load_world),
                ));
                if checks.pending(&listing.folder.slug) {
                    load.insert(InteractionDisabled);
                    details.spawn((
                        Name::new("Load World Details Checking"),
                        Text::new(CHECKING_TEXT),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));
                }
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

        if matches!(&*step, WorldDeleteStep::Confirm(slug) if slug == &listing.folder.slug) {
            details.spawn((
                Name::new("Load World Delete Prompt"),
                Text::new(format!(
                    "Delete {title}? This removes its save files and cannot be undone."
                )),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Danger),
            ));
            details.spawn((
                Name::new("Load World Delete Confirm Button"),
                button_variant("Delete world", ButtonVariant::Danger, None),
                observe(on_delete_world_confirm),
            ));
            details.spawn((
                Name::new("Load World Delete Cancel Button"),
                themed_button("Cancel"),
                observe(on_delete_world_cancel),
            ));
        } else {
            details.spawn((
                Name::new("Load World Delete Button"),
                button_variant("Delete", ButtonVariant::Danger, None),
                observe(on_delete_world),
            ));
        }
    });
}
