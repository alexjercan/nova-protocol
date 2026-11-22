//! The Load picker: a two-pane overlay in the Mods style listing every
//! world folder under [`WorldsRoot`], that resumes the selected one through
//! the New Game handoff. A row reads the world's name over its start role,
//! its play time, how long ago it was saved and how many objectives stand
//! open; the details draw the save's chart, the fleet it holds and its open
//! objectives. The menu's Continue button opens the newest save the same
//! way the Load button does ([`open_world_task`]).
//!
//! The list reads only each world's header. Each world whose header reads is
//! then checked whole off the main thread with [`check_world`]: its state
//! file is read and every item it holds is looked up in the loaded catalog.
//! Until that check ends the row shows `CHECKING` and Load is greyed; a
//! refused check marks the row `INVALID` and the details tag the refusal's
//! kind, with the full reason in the log. Load
//! checks again as it opens the world, off the main thread as well
//! ([`WorldOpen`]); the world resumes on the frame the open lands.

use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    ecs::system::SystemParam,
    picking::hover::Hovered,
    prelude::*,
    tasks::{block_on, futures_lite::future, IoTaskPool, Task},
    ui::InteractionDisabled,
    ui_widgets::{observe, Activate, Button},
};
use nova_assets::prelude::LoadedSectionPacks;
use nova_gameplay::prelude::*;
use nova_hud::prelude::{ChartSceneCell, ChartSceneMark, ChartSceneMarkType, ChartSceneView};
use nova_interface::prelude::{icon_node, InterfaceIcons};
use nova_scenario::prelude::{GameScenarios, GameShipDesigns, GameStartRoles};
use nova_sound::prelude::*;
use nova_ui::{
    screen::{scroll_bar, scroll_column, scroll_row, scroll_viewport, EllipsisText},
    theme,
    theme::UiColorType,
    widget::{
        button, button_stack, divider, group_stack, list_row,
        metrics::{CONTROL_H, GAP_L, GAP_S},
        tag, ButtonCueType, ButtonSpec, ButtonVariant, Selected, ThemedBorder, ThemedRadius,
        ThemedText, UiText,
    },
};
use nova_world_base::prelude::{
    check_world, delete_world, list_worlds, open_world, resume_world, RegisteredScenarios,
    SavedChart, SavedChartMarkType, SavedObjectiveLine, WorldFolder, WorldListing, WorldLock,
    WorldRefusal, WorldResumeRefused, WorldSaveHeader, WorldSaveSession, WorldSaveState,
};

use crate::{
    preview::{preview_node, MenuChartPreview},
    scenarios::NewGameScenario,
    widgets::{
        button_variant, close_screen, danger_button, detail_row, fitted_action_row,
        fitted_button_slot, row_group, LABEL_LINE_GAP, LIST_ROW_INSET_X, MAIN_SLOT_W, SIDE_SLOT_W,
    },
    world_setup::{new_game_button, WorldsRoot, ROLE_ICON_PX},
};

/// How many cells the Load preview reaches out from the save's cell.
const LOAD_PREVIEW_REACH: i32 = 2;

/// The height of the save's chart preview in the details.
const LOAD_PREVIEW_H: f32 = 220.0;

/// The width of a row's play-time and saved-ago cells.
const ROW_TIME_W: f32 = 90.0;

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

/// What [`open_world`] hands back: the world to resume, or why not.
type OpenedWorld = Result<(WorldFolder, WorldLock, WorldSaveHeader, WorldSaveState), WorldRefusal>;

/// The world the Load button is opening off the main thread, by slug.
/// [`on_load_world`] starts it and ignores clicks until [`poll_world_open`]
/// takes the result; dropping the task cancels the open.
#[derive(Resource, Default)]
pub(crate) struct WorldOpen(pub(crate) Option<(String, Task<OpenedWorld>)>);

/// The text the details pane shows while the world is checked.
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

/// The crafting catalogs a save's craft job and consumable effects are
/// checked against.
#[derive(SystemParam)]
pub(crate) struct CraftCatalogs<'w> {
    recipes: Res<'w, GameRecipes>,
    consumables: Res<'w, GameConsumables>,
    encounters: Res<'w, GameEncounters>,
}

/// The details pane's Load button.
#[derive(Component)]
pub(crate) struct LoadWorldButton;

/// Read the worlds under `root` and start a [`check_world`] task for each
/// world whose header read, replacing every check still running.
fn read_listings(
    root: &Path,
    packs: &LoadedSectionPacks,
    roles: &GameStartRoles,
    items: &GameItems,
    scenarios: &GameScenarios,
    designs: &GameShipDesigns,
    profiles: &GameTradeProfiles,
    recipes: &GameRecipes,
    consumables: &GameConsumables,
    encounters: &GameEncounters,
    checks: &mut WorldChecks,
) -> Result<Vec<WorldListing>, WorldRefusal> {
    let listings = list_worlds(root, packs, roles);
    let packs = Arc::new(packs.clone());
    let roles = Arc::new(roles.clone());
    let items = Arc::new(items.clone());
    let scenarios = Arc::new(RegisteredScenarios::from(scenarios));
    let designs = Arc::new(designs.clone());
    let profiles = Arc::new(profiles.clone());
    let recipes = Arc::new(recipes.clone());
    let consumables = Arc::new(consumables.clone());
    let encounters = Arc::new(encounters.clone());
    checks.0 = listings
        .iter()
        .flatten()
        .filter(|listing| listing.header.is_ok())
        .map(|listing| {
            let (root, slug) = (root.to_path_buf(), listing.folder.slug.clone());
            let (packs, roles, items) =
                (Arc::clone(&packs), Arc::clone(&roles), Arc::clone(&items));
            let scenarios = Arc::clone(&scenarios);
            let designs = Arc::clone(&designs);
            let profiles = Arc::clone(&profiles);
            let recipes = Arc::clone(&recipes);
            let consumables = Arc::clone(&consumables);
            let encounters = Arc::clone(&encounters);
            let task = IoTaskPool::get().spawn(async move {
                check_world(
                    &root,
                    &slug,
                    &packs,
                    &roles,
                    &items,
                    &scenarios,
                    &designs,
                    &profiles,
                    &recipes,
                    &consumables,
                    &encounters,
                )
            });
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
    roles: Res<GameStartRoles>,
    items: Res<GameItems>,
    scenarios: Res<GameScenarios>,
    designs: Res<GameShipDesigns>,
    profiles: Res<GameTradeProfiles>,
    crafting: CraftCatalogs,
    mut checks: ResMut<WorldChecks>,
    mut listings: ResMut<WorldListings>,
    mut selected: ResMut<SelectedWorldSlug>,
    mut step: ResMut<WorldDeleteStep>,
    mut panel: Single<&mut Visibility, With<LoadPanel>>,
) {
    listings.0 = match root.0.as_deref() {
        Some(root) => read_listings(
            root,
            &packs,
            &roles,
            &items,
            &scenarios,
            &designs,
            &profiles,
            &crafting.recipes,
            &crafting.consumables,
            &crafting.encounters,
            &mut checks,
        ),
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
    close_screen(&mut panel);
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
    mut commands: Commands,
    bank: Option<Res<SoundBank<UiSfx>>>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    roles: Res<GameStartRoles>,
    items: Res<GameItems>,
    scenarios: Res<GameScenarios>,
    designs: Res<GameShipDesigns>,
    profiles: Res<GameTradeProfiles>,
    crafting: CraftCatalogs,
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
    listings.0 = read_listings(
        root,
        &packs,
        &roles,
        &items,
        &scenarios,
        &designs,
        &profiles,
        &crafting.recipes,
        &crafting.consumables,
        &crafting.encounters,
        &mut checks,
    );
    // A refused delete leaves the row standing; the deny cue is the answer
    // and the reason goes to the log.
    match result {
        Ok(()) => play_menu_select(&mut commands, bank.as_deref()),
        Err(refusal) => {
            warn!("delete of world `{slug}` refused: {refusal}");
            play_menu_deny(&mut commands, bank.as_deref());
        }
    }
    *step = WorldDeleteStep::Idle;
}

/// Open the world `slug` under `root` off the main thread, checked again as
/// it opens; [`poll_world_open`] takes the result.
pub(crate) fn open_world_task(
    root: &Path,
    slug: &str,
    packs: &LoadedSectionPacks,
    roles: &GameStartRoles,
    items: &GameItems,
    scenarios: &GameScenarios,
    designs: &GameShipDesigns,
    profiles: &GameTradeProfiles,
    recipes: &GameRecipes,
    consumables: &GameConsumables,
    encounters: &GameEncounters,
) -> Task<OpenedWorld> {
    let (root, slug) = (root.to_path_buf(), slug.to_string());
    let (packs, roles, items) = (packs.clone(), roles.clone(), items.clone());
    let scenarios = RegisteredScenarios::from(scenarios);
    let (designs, profiles, recipes) = (designs.clone(), profiles.clone(), recipes.clone());
    let consumables = consumables.clone();
    let encounters = encounters.clone();
    IoTaskPool::get().spawn(async move {
        open_world(
            &root,
            &slug,
            &packs,
            &roles,
            &items,
            &scenarios,
            &designs,
            &profiles,
            &recipes,
            &consumables,
            &encounters,
        )
    })
}

/// Start opening the selected world off the main thread, checked again as
/// it opens; [`poll_world_open`] finishes the Load. A click while a check or
/// an open is still running does nothing.
pub(crate) fn on_load_world(
    _activate: On<Activate>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    roles: Res<GameStartRoles>,
    items: Res<GameItems>,
    scenarios: Res<GameScenarios>,
    designs: Res<GameShipDesigns>,
    profiles: Res<GameTradeProfiles>,
    crafting: CraftCatalogs,
    checks: Res<WorldChecks>,
    selected: Res<SelectedWorldSlug>,
    mut opening: ResMut<WorldOpen>,
    mut step: ResMut<WorldDeleteStep>,
) {
    let (Some(root), Some(slug)) = (root.0.as_deref(), selected.0.as_deref()) else {
        return;
    };
    if checks.pending(slug) || opening.0.is_some() {
        return;
    }
    // A Load drops a Delete asked about, so a refused resume does not bring
    // its prompt back.
    *step = WorldDeleteStep::Idle;
    let task = open_world_task(
        root,
        slug,
        &packs,
        &roles,
        &items,
        &scenarios,
        &designs,
        &profiles,
        &crafting.recipes,
        &crafting.consumables,
        &crafting.encounters,
    );
    opening.0 = Some((slug.to_string(), task));
}

/// The newest saved world of `listings`, by its save time: the one Continue
/// opens. A world whose header did not read has no save time and is not it.
pub(crate) fn latest_world(listings: &[WorldListing]) -> Option<&WorldListing> {
    listings
        .iter()
        .filter(|listing| listing.header.is_ok())
        .max_by_key(|listing| {
            listing
                .header
                .as_ref()
                .map_or(0, |header| header.saved_at_unix)
        })
}

/// The world the menu's Continue button opens, on the button.
#[derive(Component, Clone, Debug)]
pub(crate) struct ContinueWorld(pub(crate) String);

/// The menu panel's node that holds Continue, its caption and New Game.
/// [`refresh_continue`] redraws it when [`WorldListings`] changes, so a
/// deleted or refused save does not stay named there and New Game takes the
/// lit face back once the last save is gone.
#[derive(Component)]
pub(crate) struct ContinueSlot;

/// Draw the menu's head actions in `slot`: Continue and its caption for the
/// newest readable save of `listings` ([`latest_world`]), its age read against
/// `now`, then New Game. Continue is the only primary button when it is drawn;
/// with no readable save New Game is.
pub(crate) fn spawn_primary_actions(
    slot: &mut ChildSpawnerCommands,
    listings: &[WorldListing],
    now: u64,
) {
    let latest = latest_world(listings)
        .and_then(|latest| latest.header.as_ref().ok().map(|header| (latest, header)));
    if let Some((latest, header)) = latest {
        // The caption belongs to Continue, so the pair stands closer than
        // two buttons do.
        slot.spawn((
            Name::new("Continue Group"),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(GAP_S),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|group| {
            group.spawn((
                Name::new("Continue Button"),
                ContinueWorld(latest.folder.slug.clone()),
                button_variant("Continue", ButtonVariant::Primary, None),
                observe(on_continue_world),
            ));
            group.spawn((
                Name::new("Continue Caption"),
                UiText,
                Text::new(format!(
                    "{}, saved {}",
                    header.name,
                    saved_ago_text(now, header.saved_at_unix)
                )),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColorType::TextSecondary),
                Node {
                    align_self: AlignSelf::Center,
                    ..default()
                },
            ));
        });
    }
    slot.spawn(new_game_button(latest.is_none()));
}

/// The main menu's Load button, dead while no world is listed.
#[derive(Component)]
pub(crate) struct LoadMenuButton;

/// Redraw [`ContinueSlot`] from [`WorldListings`]. A refused root draws no
/// Continue, only New Game. The menu's Load button dies with the last world,
/// so a delete on the Load screen leaves no live way back to an empty list.
pub(crate) fn refresh_continue(
    mut commands: Commands,
    listings: Res<WorldListings>,
    slots: Query<Entity, With<ContinueSlot>>,
    loads: Query<Entity, With<LoadMenuButton>>,
) {
    let rows = listings.0.as_deref().unwrap_or_default();
    for load in &loads {
        if rows.is_empty() {
            commands.entity(load).insert(InteractionDisabled);
        } else {
            commands.entity(load).remove::<InteractionDisabled>();
        }
    }
    let Ok(slot) = slots.single() else {
        return;
    };
    commands.entity(slot).despawn_related::<Children>();
    let now = now_unix();
    commands
        .entity(slot)
        .with_children(|slot| spawn_primary_actions(slot, rows, now));
}

/// Open the newest saved world from the menu's Continue button: read the
/// list as the Load button does, so a refusal has a row to land on, select
/// the world and start its open. A press while an open runs does nothing.
#[expect(
    clippy::too_many_arguments,
    reason = "one observer reading the worlds root, the catalogs the open checks against and the picker's state"
)]
pub(crate) fn on_continue_world(
    activate: On<Activate>,
    buttons: Query<&ContinueWorld>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    roles: Res<GameStartRoles>,
    items: Res<GameItems>,
    scenarios: Res<GameScenarios>,
    designs: Res<GameShipDesigns>,
    profiles: Res<GameTradeProfiles>,
    crafting: CraftCatalogs,
    mut checks: ResMut<WorldChecks>,
    mut listings: ResMut<WorldListings>,
    mut selected: ResMut<SelectedWorldSlug>,
    mut opening: ResMut<WorldOpen>,
    mut step: ResMut<WorldDeleteStep>,
) {
    let (Ok(button), Some(root)) = (buttons.get(activate.entity), root.0.as_deref()) else {
        return;
    };
    if opening.0.is_some() {
        return;
    }
    listings.0 = read_listings(
        root,
        &packs,
        &roles,
        &items,
        &scenarios,
        &designs,
        &profiles,
        &crafting.recipes,
        &crafting.consumables,
        &crafting.encounters,
        &mut checks,
    );
    selected.0 = Some(button.0.clone());
    *step = WorldDeleteStep::Idle;
    let task = open_world_task(
        root,
        &button.0,
        &packs,
        &roles,
        &items,
        &scenarios,
        &designs,
        &profiles,
        &crafting.recipes,
        &crafting.consumables,
        &crafting.encounters,
    );
    opening.0 = Some((button.0.clone(), task));
}

/// Take a finished [`WorldOpen`]. On `Err` (a world locked or refused since
/// the list was read), the refusal replaces that row's header so the list
/// and details redraw with it, and the Load panel shows if it was not (a
/// Continue refused); nothing starts. On `Ok`, resume the world and hand off
/// to Playing exactly like New Game.
pub(crate) fn poll_world_open(
    mut commands: Commands,
    mut opening: ResMut<WorldOpen>,
    mut listings: ResMut<WorldListings>,
    mut pick: ResMut<NewGameScenario>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
    panel: Option<Single<&mut Visibility, With<LoadPanel>>>,
) {
    let Some((_, task)) = &mut opening.0 else {
        return;
    };
    let Some(result) = block_on(future::poll_once(task)) else {
        return;
    };
    let (slug, _) = opening.0.take().expect("the open polled above");
    match result {
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
            if let Some(mut panel) = panel {
                **panel = Visibility::Visible;
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
/// shows its own error. [`SelectedWorldSlug`] is set to the refused world, so
/// the details pane shows the reason too: a resume reached here off the Retry
/// path, which never ran [`on_load_world_row_select`] to point the selection
/// at it.
pub(crate) fn refuse_resumed_world(
    mut commands: Commands,
    refused: Res<WorldResumeRefused>,
    root: Res<WorldsRoot>,
    packs: Res<LoadedSectionPacks>,
    roles: Res<GameStartRoles>,
    items: Res<GameItems>,
    scenarios: Res<GameScenarios>,
    designs: Res<GameShipDesigns>,
    profiles: Res<GameTradeProfiles>,
    crafting: CraftCatalogs,
    mut checks: ResMut<WorldChecks>,
    mut listings: ResMut<WorldListings>,
    mut selected: ResMut<SelectedWorldSlug>,
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
        Some(root) => read_listings(
            root,
            &packs,
            &roles,
            &items,
            &scenarios,
            &designs,
            &profiles,
            &crafting.recipes,
            &crafting.consumables,
            &crafting.encounters,
            &mut checks,
        ),
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
    selected.0 = Some(refused.slug.clone());
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
    roles: Res<GameStartRoles>,
    icons: Res<InterfaceIcons>,
    mut selected: ResMut<SelectedWorldSlug>,
    lists: Query<Entity, With<LoadWorldList>>,
) {
    let Ok(list) = lists.single() else {
        return;
    };
    commands.entity(list).despawn_related::<Children>();

    // No folder, a refused one or an empty one lists nothing. The menu's
    // Load is dead in all three, so this is only reached by deleting the
    // last world: the empty list is the whole statement.
    let rows = match (&root.0, &listings.0) {
        (Some(_), Ok(rows)) if !rows.is_empty() => rows,
        (_, Err(refusal)) => {
            warn!("the worlds folder was refused: {refusal}");
            selected.0 = None;
            return;
        }
        _ => {
            selected.0 = None;
            return;
        }
    };

    if !selected
        .0
        .as_deref()
        .is_some_and(|slug| rows.iter().any(|row| row.folder.slug == slug))
    {
        selected.0 = rows.first().map(|row| row.folder.slug.clone());
    }

    let now = now_unix();
    commands.entity(list).with_children(|list| {
        spawn_load_head(list);
        for row in rows {
            let is_selected = selected.0.as_deref() == Some(row.folder.slug.as_str());
            let checking = checks.pending(&row.folder.slug);
            spawn_load_row(list, row, &roles, &icons, is_selected, checking, now);
        }
    });
}

/// Seconds since the Unix epoch now, the clock a save's `saved_at_unix` is
/// read against; zero on a clock set before the epoch.
pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// How long ago a save at `saved_at_unix` was, read against `now_unix`:
/// `just now` under a minute, else whole minutes, hours or days. A save
/// ahead of the clock reads `just now`.
pub(crate) fn saved_ago_text(now_unix: u64, saved_at_unix: u64) -> String {
    let elapsed = now_unix.saturating_sub(saved_at_unix);
    match elapsed {
        0..60 => "just now".to_string(),
        60..3_600 => format!("{} min ago", elapsed / 60),
        3_600..86_400 => format!("{} h ago", elapsed / 3_600),
        _ => format!("{} d ago", elapsed / 86_400),
    }
}

/// The column heads over the world rows: the name, the play time, the save
/// age and the open objectives.
fn spawn_load_head(list: &mut ChildSpawnerCommands) {
    list.spawn((
        Name::new("Load World List Head"),
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(GAP_L),
            padding: UiRect::axes(px(LIST_ROW_INSET_X), px(GAP_S)),
            ..default()
        },
    ))
    .with_children(|head| {
        head.spawn(head_cell("World", None));
        head.spawn(head_cell("Play time", Some(ROW_TIME_W)));
        head.spawn(head_cell("Saved", Some(ROW_TIME_W)));
        head.spawn(head_cell("Open", Some(ROW_BADGE_SLOT_W)));
    });
}

/// One column head: fixed at `width`, or growing.
fn head_cell(text: &str, width: Option<f32>) -> impl Bundle {
    (
        UiText,
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextLayout {
            linebreak: LineBreak::NoWrap,
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColorType::TextMuted),
        Node {
            width: width.map_or(Val::Auto, px),
            flex_grow: if width.is_some() { 0.0 } else { 1.0 },
            flex_shrink: 0.0,
            ..default()
        },
    )
}

/// One fixed cell of a world row, at `width`.
fn row_cell(name: &'static str, text: String, width: f32, color: UiColorType) -> impl Bundle {
    (
        Name::new(name),
        UiText,
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
        Node {
            width: px(width),
            flex_shrink: 0.0,
            ..default()
        },
    )
}

/// How many objectives a save holds open, as its row says it.
fn open_count_text(header: &WorldSaveHeader) -> String {
    match header.objectives.len() {
        0 => "none open".to_string(),
        n => format!("{n} open"),
    }
}

/// Every world row's height. Fixed, so a row never grows with its state:
/// the name sits over one status line, and the state reads as a badge in a
/// fixed trailing slot.
const ROW_H: f32 = 58.0;

/// The trailing badge slot width, sized for the longest badge (`CHECKING`).
const ROW_BADGE_SLOT_W: f32 = 84.0;

/// Spawn one clickable world row at [`ROW_H`]: the start role's icon, then
/// the world's name (or slug,
/// when the header did not read) over the start role's name when the header
/// read - a refused row has no second line, its badge says it - then, when
/// the header read, its play time, how long ago it was saved and how many
/// objectives it holds open, and a trailing badge: `CHECKING` while the
/// state check runs, `INVALID` when it refused.
fn spawn_load_row(
    list: &mut ChildSpawnerCommands,
    listing: &WorldListing,
    roles: &GameStartRoles,
    icons: &InterfaceIcons,
    selected: bool,
    checking: bool,
    now: u64,
) {
    let title = match &listing.header {
        Ok(header) => header.name.clone(),
        Err(_) => listing.folder.slug.clone(),
    };
    // The listing refused a role `roles` lacked; a header role resolves to
    // nothing only if a mod reload dropped the role since.
    let role = listing
        .header
        .as_ref()
        .ok()
        .and_then(|header| roles.get_role(&header.role));
    let status = match (&listing.header, role) {
        (Ok(_), Some(role)) => Some(role.name.clone()),
        (Ok(header), None) => Some(header.role.to_string()),
        (Err(_), _) => None,
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
    // Written onto the spawned `list_row` Node, so the row keeps one
    // definition of its box and the fixed height is the only local difference.
    row.entry::<Node>().and_modify(|mut node| {
        node.height = px(ROW_H);
        node.flex_shrink = 0.0;
    });
    if selected {
        row.insert(Selected);
    }
    row.with_children(|row| {
        // The slot keeps its width without an icon, so the names stay in one
        // column over a row whose role is unknown or whose header refused.
        let mut slot = row.spawn((
            Name::new("Load World Row Icon Slot"),
            Node {
                width: px(ROLE_ICON_PX),
                flex_shrink: 0.0,
                ..default()
            },
        ));
        if let Some(role) = role {
            slot.with_child((
                Name::new("Load World Row Icon"),
                icon_node(icons.role(role.icon), UiColorType::Accent, ROLE_ICON_PX),
            ));
        }
        row.spawn((
            Name::new("Load World Row Info"),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: px(0),
                row_gap: px(LABEL_LINE_GAP),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_children(|info| {
            info.spawn((
                Name::new("Load World Row Name"),
                UiText,
                // The info column is the clipped cell; the details pane names
                // the world in full.
                EllipsisText(title.clone()),
                Text::new(title),
                TextLayout::new(Justify::Left, LineBreak::NoWrap),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColorType::TextBody),
            ));
            if let Some(status) = status {
                info.spawn(row_line("Load World Row Status", status));
            }
        });
        if let Ok(header) = &listing.header {
            row.spawn(row_cell(
                "Load World Row Play Time",
                play_time_text(header.play_seconds),
                ROW_TIME_W,
                UiColorType::TextBody,
            ));
            row.spawn(row_cell(
                "Load World Row Saved",
                saved_ago_text(now, header.saved_at_unix),
                ROW_TIME_W,
                UiColorType::TextBody,
            ));
        }
        row.spawn((
            Name::new("Load World Row Badge Slot"),
            Node {
                width: px(ROW_BADGE_SLOT_W),
                flex_shrink: 0.0,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
        ))
        .with_children(|slot| {
            if checking {
                slot.spawn((
                    Name::new("Load World Row Checking"),
                    tag("Checking", UiColorType::TextSecondary),
                ));
            } else {
                match &listing.header {
                    Ok(header) => {
                        slot.spawn(row_cell(
                            "Load World Row Open",
                            open_count_text(header),
                            ROW_BADGE_SLOT_W,
                            UiColorType::TextBody,
                        ));
                    }
                    Err(_) => {
                        slot.spawn((
                            Name::new("Load World Row Invalid"),
                            tag("Invalid", UiColorType::Accent),
                        ));
                    }
                }
            }
        });
    });
}

/// The chart a save's details draw: every cell and mark the header records,
/// round the cell the player was saved in, with the player at its middle.
/// The header keeps each kind in the found or accept order, so counting each
/// kind gives the code the Map labels it with.
fn saved_chart_view(header: &WorldSaveHeader) -> ChartSceneView {
    let SavedChart { cells, marks } = &header.chart;
    let (mut stations, mut wrecks, mut jobs) = (0, 0, 0);
    ChartSceneView {
        centre: header.player_sector,
        reach: LOAD_PREVIEW_REACH,
        cells: cells
            .iter()
            .map(|cell| ChartSceneCell {
                at: cell.at,
                clusters: cell.clusters.clone(),
            })
            .collect(),
        marks: marks
            .iter()
            .map(|mark| {
                let (kind, code) = match mark.kind {
                    SavedChartMarkType::Station { dead, owned } => {
                        stations += 1;
                        (
                            ChartSceneMarkType::Station {
                                dead,
                                owned,
                                refusing: false,
                            },
                            format!("STN-{stations}"),
                        )
                    }
                    SavedChartMarkType::Wreck => {
                        wrecks += 1;
                        (ChartSceneMarkType::Wreck, format!("WRK-{wrecks}"))
                    }
                    SavedChartMarkType::Job => {
                        jobs += 1;
                        (ChartSceneMarkType::Job, format!("JOB-{jobs}"))
                    }
                };
                ChartSceneMark {
                    at: mark.at,
                    kind,
                    code,
                }
            })
            .collect(),
        selected: None,
        you: Some(Vec3::ZERO),
    }
}

/// A block heading in the details pane: FLEET, OPEN OBJECTIVES.
fn details_heading(name: &'static str, text: &str) -> impl Bundle {
    (
        Name::new(name),
        UiText,
        Text::new(text.to_uppercase()),
        TextFont {
            font_size: FontSize::Px(11.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColorType::TextMuted),
    )
}

/// One open objective's row: its title, then each count as `have / need`.
fn objective_row(details: &mut ChildSpawnerCommands, title: &str, counts: &str) {
    details
        .spawn((
            Name::new("Load World Details Objective"),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(GAP_L),
                min_height: px(CONTROL_H),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Name::new("Load World Details Objective Title"),
                UiText,
                Text::new(title.to_string()),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColorType::Text),
                Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                },
            ));
            if !counts.is_empty() {
                row.spawn((
                    Name::new("Load World Details Objective Counts"),
                    UiText,
                    Text::new(counts.to_string()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::NONE),
                    ThemedText::new(UiColorType::TextBody),
                ));
            }
        });
}

/// The `LABEL  have / need` text of an objective's counts, one per line.
fn objective_counts(header_lines: &[SavedObjectiveLine]) -> String {
    header_lines
        .iter()
        .map(|line| format!("{} {} / {}", line.label, line.have, line.need))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The one-line status under a world row's name, the size of the Mods row's
/// version line.
fn row_line(name: &'static str, text: String) -> impl Bundle {
    (
        Name::new(name),
        UiText,
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(UiColorType::TextSecondary),
    )
}

/// A one-line note in the details pane at `color`.
fn details_note(name: &'static str, text: String, color: UiColorType) -> impl Bundle {
    (
        Name::new(name),
        UiText,
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::NONE),
        ThemedText::new(color),
    )
}

/// `unix` seconds as `YYYY-MM-DD HH:MM UTC`, the date a save reads as in the
/// details pane. Civil-from-days after Howard Hinnant; the game has no
/// calendar crate and needs no zone.
pub(crate) fn utc_stamp(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        secs / 3_600,
        (secs % 3_600) / 60
    )
}

/// Rebuild the world details pane from the selected world. A scrolling body
/// holds the world's name and a rule, then the save's chart preview, one
/// labelled line each for the seed, sector, credits, play time, saved time
/// and game version, the FLEET block (the hull flown, then the owned ships,
/// escorts and owned stations the save holds) and the open objectives, or
/// the refusal's kind as a tag. Below the body sit a button row with Load,
/// greyed on a refusal or while the world is checked, and Delete; while a
/// delete is asked, Cancel takes Delete's box and a danger Confirm stands
/// beside it. With no world selected the pane stays empty.
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

    // The pane's parts - the scrolling body, the button row, the delete
    // prompt and its confirm row - stand a group gap apart in one stack.
    let stack = commands
        .spawn((
            Name::new("Load World Details Stack"),
            Node {
                flex_grow: 1.0,
                min_height: px(0),
                ..group_stack()
            },
            ChildOf(panel),
        ))
        .id();
    commands.entity(stack).with_children(|details| {
        // Only the information scrolls: the Load, Delete and delete-confirm
        // rows below stay in the pane, so a short window clips the rows
        // rather than pushing the buttons out of the panel.
        let mut body = None;
        details
            .spawn((
                Name::new("Load World Details Body Row"),
                Node {
                    // Short content keeps the buttons right under it.
                    flex_grow: 0.0,
                    ..scroll_row()
                },
            ))
            .with_children(|row| {
                body = Some(
                    row.spawn((
                        Name::new("Load World Details Body"),
                        Node {
                            // Headings, rules and row groups, not a list of
                            // rows: the heading gap takes the place of the
                            // list-row gap.
                            row_gap: button_stack().row_gap,
                            ..scroll_column()
                        },
                        scroll_viewport(),
                    ))
                    .id(),
                );
                // The bar says the rows go on under the pane's edge: at
                // 1280x720 the fleet and the objectives start below it.
                row.spawn((Name::new("Load World Details Scroll Bar"), scroll_bar()));
            });
        let Some(body) = body else {
            return;
        };
        let mut body_commands = details.commands();
        let mut body = body_commands.entity(body);

        let Some(listing) = listing else {
            return;
        };

        let title = match &listing.header {
            Ok(header) => header.name.clone(),
            Err(_) => listing.folder.slug.clone(),
        };
        let checking = checks.pending(&listing.folder.slug);

        body.with_children(|body| {
            body.spawn((
                Name::new("Load World Details Name"),
                UiText,
                Text::new(title.clone()),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColorType::Text),
            ));
            body.spawn((Name::new("Load World Details Separator"), divider()));

            match &listing.header {
                Ok(header) => {
                    body.spawn(row_group("Load World Details Preview Group"))
                        .with_children(|group| {
                            group.spawn((
                                MenuChartPreview(saved_chart_view(header)),
                                preview_node(
                                    "Load World Details Preview",
                                    Node {
                                        height: px(LOAD_PREVIEW_H),
                                        flex_shrink: 0.0,
                                        border: UiRect::all(px(theme::BORDER_W)),
                                        ..default()
                                    },
                                ),
                                ThemedBorder::new(UiColorType::TextMuted),
                                ThemedRadius::control(),
                            ));
                        });
                    let now = now_unix();
                    body.spawn(row_group("Load World Details Lines"))
                        .with_children(|lines| {
                            for (label, value) in [
                                ("Seed", header.seed.to_string()),
                                ("Sector", header.player_sector.to_string()),
                                ("Credits", cr_text(header.credits)),
                                ("Play time", play_time_text(header.play_seconds)),
                                (
                                    "Saved",
                                    format!(
                                        "{} ({})",
                                        utc_stamp(header.saved_at_unix),
                                        saved_ago_text(now, header.saved_at_unix)
                                    ),
                                ),
                                ("Version", header.game_version.to_string()),
                            ] {
                                lines.spawn(detail_row(
                                    "Load World Details Line",
                                    label,
                                    value,
                                    UiColorType::TextBody,
                                ));
                            }
                        });
                    body.spawn((Name::new("Load World Details Fleet Rule"), divider()));
                    body.spawn(details_heading("Load World Details Fleet", "Fleet"));
                    body.spawn(row_group("Load World Details Fleet Lines"))
                        .with_children(|lines| {
                            lines.spawn(detail_row(
                                "Load World Details Fleet Ship",
                                "Ship",
                                header.hull.clone(),
                                UiColorType::Text,
                            ));
                            for (name, label, count) in [
                                (
                                    "Load World Details Fleet Owned Ships",
                                    "Owned ships",
                                    header.fleet.owned_ships,
                                ),
                                (
                                    "Load World Details Fleet Escorts",
                                    "Escorts",
                                    header.fleet.escorts,
                                ),
                                (
                                    "Load World Details Fleet Owned Stations",
                                    "Owned stations",
                                    header.fleet.owned_stations,
                                ),
                            ] {
                                if count > 0 {
                                    lines.spawn(detail_row(
                                        name,
                                        label,
                                        count.to_string(),
                                        UiColorType::Text,
                                    ));
                                }
                            }
                        });
                    body.spawn((Name::new("Load World Details Objectives Rule"), divider()));
                    body.spawn(details_heading(
                        "Load World Details Objectives",
                        "Open objectives",
                    ));
                    body.spawn(row_group("Load World Details Objective Lines"))
                        .with_children(|lines| {
                            if header.objectives.is_empty() {
                                lines.spawn(details_note(
                                    "Load World Details No Objectives",
                                    "No open objectives".to_string(),
                                    UiColorType::TextMuted,
                                ));
                            }
                            for objective in &header.objectives {
                                objective_row(
                                    lines,
                                    &objective.title,
                                    &objective_counts(&objective.lines),
                                );
                            }
                            if checking {
                                lines.spawn(detail_row(
                                    "Load World Details Checking",
                                    "State",
                                    CHECKING_TEXT.to_string(),
                                    UiColorType::TextMuted,
                                ));
                            }
                        });
                }
                // The kind of refusal as a tag; the full reason is in the log.
                Err(refusal) => {
                    debug!("world `{}` cannot load: {refusal}", listing.folder.slug);
                    body.spawn((
                        Name::new("Load World Details Refusal"),
                        tag(refusal_tag(refusal), UiColorType::Accent),
                    ));
                }
            }
        });

        let confirming =
            matches!(&*step, WorldDeleteStep::Confirm(slug) if slug == &listing.folder.slug);
        let loadable = listing.header.is_ok() && !checking;

        details
            .spawn((Name::new("Load World Actions"), fitted_action_row()))
            .with_children(|actions| {
                actions
                    .spawn(fitted_button_slot("Load World Button Slot", MAIN_SLOT_W))
                    .with_children(|slot| {
                        let mut load = slot.spawn((
                            Name::new("Load World Button"),
                            nova_ui::widget::button(ButtonSpec::control("Load").primary().fill()),
                            LoadWorldButton,
                            observe(on_load_world),
                        ));
                        if !loadable {
                            load.insert(InteractionDisabled);
                        }
                    });
                // Armed, Cancel takes the box Delete had, so the second press
                // of a double click lands on it, and the danger Confirm stands
                // beside it: the shape of Reset Defaults and Uninstall.
                if !confirming {
                    actions
                        .spawn(fitted_button_slot(
                            "Load World Delete Button Slot",
                            SIDE_SLOT_W,
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                Name::new("Load World Delete Button"),
                                danger_button("Delete"),
                                observe(on_delete_world),
                            ));
                        });
                    return;
                }
                actions
                    .spawn(fitted_button_slot(
                        "Load World Delete Cancel Button Slot",
                        SIDE_SLOT_W,
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            Name::new("Load World Delete Cancel Button"),
                            button(ButtonSpec::control("Cancel").fill()),
                            ButtonCueType::Back,
                            observe(on_delete_world_cancel),
                        ));
                    });
                actions
                    .spawn(fitted_button_slot(
                        "Load World Delete Confirm Button Slot",
                        SIDE_SLOT_W,
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            Name::new("Load World Delete Confirm Button"),
                            danger_button("Confirm"),
                            ButtonCueType::Owned,
                            observe(on_delete_world_confirm),
                        ));
                    });
            });
    });
}

/// The tag word a refused world shows in the details pane, one per kind of
/// [`WorldRefusal`].
fn refusal_tag(refusal: &WorldRefusal) -> &'static str {
    match refusal {
        WorldRefusal::Foreign(_) => "Foreign",
        WorldRefusal::InvalidName(_) => "Name",
        WorldRefusal::NameTaken => "Taken",
        WorldRefusal::Locked => "Locked",
        WorldRefusal::Unreadable(_) => "Unreadable",
        WorldRefusal::Format { .. } => "Format",
        WorldRefusal::Catalog { .. } => "Catalog",
        WorldRefusal::Io(_) => "Io",
        WorldRefusal::Unrestored(_) => "Unrestored",
        WorldRefusal::Scenario { .. } => "Scenario",
        WorldRefusal::Role { .. } => "Role",
    }
}

#[cfg(test)]
mod tests {
    use super::{saved_ago_text, utc_stamp};

    /// A save's age reads in the largest whole unit: seconds as `just now`,
    /// then minutes, hours and days; a save ahead of the clock is `just now`.
    #[test]
    fn a_save_s_age_reads_in_whole_units() {
        assert_eq!(saved_ago_text(1_000, 1_000), "just now");
        assert_eq!(saved_ago_text(1_000, 1_059), "just now");
        assert_eq!(saved_ago_text(1_059, 1_000), "just now");
        assert_eq!(saved_ago_text(1_000 + 60, 1_000), "1 min ago");
        assert_eq!(saved_ago_text(1_000 + 7_200, 1_000), "2 h ago");
        assert_eq!(saved_ago_text(1_000 + 3 * 86_400 + 5, 1_000), "3 d ago");
    }

    /// The saved time reads as a calendar date, not as seconds: the save
    /// fixture's `1_700_000_000` is 2023-11-14 22:13:20 UTC.
    #[test]
    fn a_save_time_reads_as_a_utc_date() {
        assert_eq!(utc_stamp(1_700_000_000), "2023-11-14 22:13 UTC");
        assert_eq!(utc_stamp(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_stamp(951_782_400), "2000-02-29 00:00 UTC");
    }
}
