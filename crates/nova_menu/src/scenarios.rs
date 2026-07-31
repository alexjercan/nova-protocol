//! The Scenarios picker: a two-pane overlay in the mods-screen style listing
//! every `!hidden` scenario, grouped under collapsible campaign headers, that
//! plays the selected one through the New Game handoff.

use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{observe, Activate, Button},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ui::{
    prelude::UiSkin,
    theme,
    widget::{list_row, separator, themed_button, ListRow, Selected, ThemedButton, UiText},
};

/// Marker for the Scenarios panel root, toggled by the Scenarios button.
#[derive(Component)]
pub(crate) struct ScenariosPanel;

/// The scenario id the details pane renders. `None` until the list populates -
/// `refresh_scenarios_list` default-selects the first row (and repairs a stale
/// selection); `on_scenario_row_select` sets it from a row click.
#[derive(Resource, Default)]
pub(crate) struct SelectedScenarioId(pub(crate) Option<ScenarioId>);

/// Overrides which scenario the shared `start_new_game_scenario`
/// (OnEnter(Playing), gated `GameMode::NewGame`) loads. `None` -> the canned
/// New Game start; `Some(id)` -> that scenario. The Scenarios picker's Play
/// button sets it; `on_new_game` clears it so New Game always plays the story.
#[derive(Resource, Default)]
pub(crate) struct NewGameScenario(pub(crate) Option<ScenarioId>);

/// The selected scenario's thumbnail image, held while it finishes loading.
///
/// The thumbnail can only be validated (is it a plain 2D texture the UI can
/// bind?) once its image has loaded, so `refresh_scenario_details` defers the
/// `ImageNode` until then: it parks the still-loading handle here and
/// `poll_scenario_thumbnail` re-arms the refresh when the load lands. `None`
/// when nothing is pending (no thumbnail, already mounted, or skipped as
/// non-2D).
#[derive(Resource, Default)]
pub(crate) struct PendingScenarioThumbnail(pub(crate) Option<Handle<Image>>);

/// The scrollable container holding the scenario rows; `refresh_scenarios_list`
/// swaps its children when the registry or selection changes.
#[derive(Component)]
pub(crate) struct ScenariosList;

/// One clickable scenario row: clicking it selects the scenario for the details
/// pane.
#[derive(Component)]
pub(crate) struct ScenarioRow {
    pub(crate) id: ScenarioId,
}

/// The set of campaigns the player has COLLAPSED in the Scenarios picker.
///
/// Absent from the set = expanded (the default, so the initial view shows every
/// campaign's chapters and the default selection is visible). Clicking a
/// [`CampaignHeader`] toggles its id here, which re-arms `refresh_scenarios_list`
/// via [`scenarios_list_dirty`]. Purely view state - never persisted, never
/// touches the scenario registry.
#[derive(Resource, Default)]
pub(crate) struct CollapsedCampaigns(pub(crate) std::collections::HashSet<CampaignId>);

/// One clickable campaign header row: clicking it expands/collapses the
/// campaign's member rows. Carries the campaign id so the toggle handler knows
/// which entry of [`CollapsedCampaigns`] to flip.
#[derive(Component)]
pub(crate) struct CampaignHeader {
    pub(crate) id: CampaignId,
}

/// The scenario details side panel; `refresh_scenario_details` rebuilds its
/// children (name, description, source, thumbnail, Play button) from the
/// selected scenario.
#[derive(Component)]
pub(crate) struct ScenarioDetailsPanel;

/// The scenario details pane's action area (holds the Play button). Kept as a
/// stable marker so the container exists in every state.
#[derive(Component)]
pub(crate) struct ScenarioDetailsActions;

/// A details-pane Play button: the scenario id it launches. `on_scenario_play`
/// reads it on click.
#[derive(Component)]
pub(crate) struct ScenarioPlay {
    pub(crate) id: ScenarioId,
}

/// The scenarios the picker lists: every `!hidden` entry, in a stable order by display
/// name then id over the HashMap-backed registry.
///
/// This is the flat baseline. The collapsible campaign-header UI - which reads the
/// first-class `GameCampaigns` mapping to group and launch campaign members, hidden
/// ones included - is the follow-up UI; until it lands the picker lists the flat
/// `!hidden` set.
pub(crate) fn listed_scenarios(scenarios: &GameScenarios) -> Vec<ScenarioConfig> {
    let mut out: Vec<ScenarioConfig> = scenarios.values().filter(|s| !s.hidden).cloned().collect();
    out.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    out
}

/// `refresh_scenarios_list` / `refresh_scenario_details` re-run when the
/// scenario registry changed (an enabled mod added/removed scenarios) or the
/// selection changed. Both refreshers share the signals; the list writes the
/// selection (default/repair) and the chained details refresh sees that write
/// the same frame.
pub(crate) fn scenarios_list_dirty(
    skin: Res<UiSkin>,
    scenarios: Option<Res<GameScenarios>>,
    campaigns: Option<Res<GameCampaigns>>,
    collapsed: Res<CollapsedCampaigns>,
    selected: Res<SelectedScenarioId>,
) -> bool {
    skin.is_changed()
        || scenarios.is_some_and(|s| s.is_changed())
        || campaigns.is_some_and(|c| c.is_changed())
        || collapsed.is_changed()
        || selected.is_changed()
}

pub(crate) fn scenario_details_dirty(
    scenarios: Option<Res<GameScenarios>>,
    selected: Res<SelectedScenarioId>,
) -> bool {
    scenarios.is_some_and(|s| s.is_changed()) || selected.is_changed()
}

/// Campaigns in a stable display order: by display name, then id.
///
/// `GameCampaigns` is a HashMap, so a deterministic order is imposed here (the
/// picker's campaign groups must not shuffle between frames). Within a campaign,
/// member order comes from the campaign's own `scenarios` list, not from here.
pub(crate) fn ordered_campaigns(campaigns: &GameCampaigns) -> Vec<CampaignConfig> {
    let mut out: Vec<CampaignConfig> = campaigns.values().cloned().collect();
    out.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    out
}

/// Every scenario id that appears as a member of SOME campaign (hidden ones
/// included). Used to keep a campaigned scenario out of the uncampaigned tail.
pub(crate) fn campaign_member_ids(
    campaigns: &GameCampaigns,
) -> std::collections::HashSet<ScenarioId> {
    campaigns
        .values()
        .flat_map(|c| c.scenarios.iter().cloned())
        .collect()
}

/// Every scenario the picker can SELECT: the flat `!hidden` set plus every
/// campaign member that resolves to a real scenario (so a `hidden` member listed
/// under its campaign header is selectable/launchable even though the flat set
/// excludes it). Selection-repair keeps the current pick only if it is in here.
pub(crate) fn selectable_scenario_ids(
    scenarios: &GameScenarios,
    campaigns: &GameCampaigns,
) -> std::collections::HashSet<ScenarioId> {
    let mut ids: std::collections::HashSet<ScenarioId> = scenarios
        .values()
        .filter(|s| !s.hidden)
        .map(|s| s.id.clone())
        .collect();
    for member in campaign_member_ids(campaigns) {
        if scenarios.contains_key(&member) {
            ids.insert(member);
        }
    }
    ids
}

/// Rebuild the scenario list as collapsible campaign groups: one
/// [`CampaignHeader`] per campaign (in `ordered_campaigns` order), and - when the
/// campaign is expanded - one indented [`ScenarioRow`] per member in the
/// campaign's declared order, resolved against `GameScenarios` (hidden members
/// included, so a chained chapter is replayable from its header). Uncampaigned
/// `!hidden` scenarios list flat below the campaigns. A default/repaired
/// selection keeps the details pane fed.
pub(crate) fn refresh_scenarios_list(
    mut commands: Commands,
    skin: Res<UiSkin>,
    scenarios: Option<Res<GameScenarios>>,
    campaigns: Option<Res<GameCampaigns>>,
    collapsed: Res<CollapsedCampaigns>,
    mut selected: ResMut<SelectedScenarioId>,
    lists: Query<Entity, With<ScenariosList>>,
) {
    let Ok(list) = lists.single() else {
        return;
    };
    commands.entity(list).despawn_related::<Children>();

    let empty_scenarios = GameScenarios::default();
    let empty_campaigns = GameCampaigns::default();
    let scenarios = scenarios.as_deref().unwrap_or(&empty_scenarios);
    let campaigns = campaigns.as_deref().unwrap_or(&empty_campaigns);

    // The flat, non-hidden sequence still defines the default/fallback pick (a
    // hidden mid-campaign chapter is never a good default); selection-repair,
    // though, accepts any selectable id so a hidden member stays selected.
    let listed = listed_scenarios(scenarios);
    let selectable = selectable_scenario_ids(scenarios, campaigns);
    if !selected
        .0
        .as_deref()
        .is_some_and(|id| selectable.contains(id))
    {
        let first = listed.first().map(|s| s.id.clone());
        if selected.0 != first {
            selected.0 = first;
        }
    }

    let ordered = ordered_campaigns(campaigns);
    let members = campaign_member_ids(campaigns);
    // Uncampaigned tail: every !hidden scenario not claimed by a campaign, in the
    // same flat name order as before.
    let uncampaigned: Vec<&ScenarioConfig> =
        listed.iter().filter(|s| !members.contains(&s.id)).collect();

    commands.entity(list).with_children(|list| {
        if ordered.is_empty() && uncampaigned.is_empty() {
            list.spawn((
                Name::new("Scenarios Empty Note"),
                Text::new("No scenarios available."),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ));
        }
        for campaign in &ordered {
            let expanded = !collapsed.0.contains(&campaign.id);
            spawn_campaign_header(list, campaign, expanded);
            if !expanded {
                continue;
            }
            for member_id in &campaign.scenarios {
                let Some(member) = scenarios.get(member_id) else {
                    // A dangling member id (the content lint flags it) simply does
                    // not render - the header still lists its resolvable chapters.
                    continue;
                };
                let is_selected = selected.0.as_deref() == Some(member.id.as_str());
                spawn_scenario_row(list, member, is_selected, true, *skin);
            }
        }
        for s in &uncampaigned {
            let is_selected = selected.0.as_deref() == Some(s.id.as_str());
            spawn_scenario_row(list, s, is_selected, false, *skin);
        }
    });
}

/// The label a scenario row shows for its title: the scenario's display name.
///
/// The interim inline campaign prefix ("Nova Protocol 1 - Shakedown Run") is gone -
/// campaign grouping moves to the collapsible campaign-header UI reading
/// `GameCampaigns`.
pub(crate) fn scenario_row_label(s: &ScenarioConfig) -> String {
    s.name.clone()
}

/// How far a campaign member row is inset under its [`CampaignHeader`]. Sized
/// against the header's `[-] ` marker: the member rows start roughly where the
/// campaign NAME does, so the column of names reads as the nesting cue.
pub(crate) const CAMPAIGN_MEMBER_INDENT_PX: f32 = 24.0;

/// Spawn one clickable campaign header row: an `[-]`/`[+]` collapse affordance
/// and the campaign's display name. Clicking it toggles the campaign in
/// [`CollapsedCampaigns`], expanding or collapsing its member rows.
pub(crate) fn spawn_campaign_header(
    list: &mut ChildSpawnerCommands,
    campaign: &CampaignConfig,
    expanded: bool,
) {
    let marker = if expanded { "[-]" } else { "[+]" };
    list.spawn((
        Name::new(format!("Campaign Header: {}", campaign.id)),
        CampaignHeader {
            id: campaign.id.clone(),
        },
        Node {
            align_self: AlignSelf::Stretch,
            align_items: AlignItems::Center,
            column_gap: px(8),
            padding: UiRect::all(px(8)),
            margin: UiRect::bottom(px(4)),
            border: UiRect::all(px(theme::BORDER_W)),
            border_radius: BorderRadius::all(px(theme::RADIUS)),
            ..default()
        },
        ThemedButton,
        Button,
        Hovered::default(),
        BorderColor::all(theme::PHOSPHOR_MUTED),
        BackgroundColor(theme::SCREEN_0),
        observe(on_campaign_header_toggle),
    ))
    .with_children(|header| {
        header.spawn((
            Name::new("Campaign Header Label"),
            Text::new(format!("{marker} {}", campaign.name)),
            TextFont {
                font_size: FontSize::Px(15.0),
                ..default()
            },
            TextColor(theme::PHOSPHOR),
        ));
    });
}

/// Spawn one clickable scenario row: name over a muted description snippet. An
/// `indent`ed row (a campaign member under its header) gets a left margin so the
/// grouping reads visually.
pub(crate) fn spawn_scenario_row(
    list: &mut ChildSpawnerCommands,
    s: &ScenarioConfig,
    selected: bool,
    indent: bool,
    skin: UiSkin,
) {
    // The shared interactive `list_row` as a direct list child. A campaign
    // member is INDENTED under its header (owner feedback 2026-07-29: "the
    // lists should be indented such that you can easily see which scenarios are
    // part of a campaign") - the `[-]` header alone did not carry the grouping.
    // The indent is a left MARGIN, so it eats into the row's own box rather than
    // its padding: the row's border shifts right with the text, which is what
    // reads as nesting.
    let mut row = list.spawn((
        Name::new(format!("Scenario Row: {}", s.id)),
        ScenarioRow { id: s.id.clone() },
        list_row(selected, skin),
        ListRow,
        Button,
        Hovered::default(),
        observe(on_scenario_row_select),
    ));
    if selected {
        row.insert(Selected);
    }
    if indent {
        // Written onto the spawned `list_row` Node rather than rebuilding it
        // here, so the row keeps ONE definition of its box (padding, border,
        // radius) and the indent is the only local difference.
        row.entry::<Node>().and_modify(|mut node| {
            node.margin.left = px(CAMPAIGN_MEMBER_INDENT_PX);
            // The margin sits OUTSIDE the box, so a `list_row`'s `width:
            // percent(100)` would make the indented row 100% + 24px wide: it
            // would slide right, overhang the pane and cross the details
            // divider (the list only clips on y). `Auto` lets the column's
            // stretch alignment size it to the pane minus the margin, so the
            // row is INSET rather than shifted.
            node.width = Val::Auto;
        });
    }
    row.with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            row_gap: px(2),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Name::new("Scenario Name"),
                UiText,
                Text::new(scenario_row_label(s)),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(theme::SCREEN_TEXT),
            ));
            if !s.description.is_empty() {
                col.spawn((
                    Name::new("Scenario Row Blurb"),
                    UiText,
                    Text::new(s.description.clone()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme::PHOSPHOR_DIM),
                ));
            }
        });
    });
}

/// Rebuild the scenario details pane from the selected scenario: name,
/// separator, thumbnail (if authored and the asset server is available),
/// description, and a Play button. The empty fallback keeps
/// [`ScenarioDetailsActions`] present in every state.
pub(crate) fn refresh_scenario_details(
    mut commands: Commands,
    scenarios: Option<Res<GameScenarios>>,
    selected: Res<SelectedScenarioId>,
    asset_server: Option<Res<AssetServer>>,
    images: Option<Res<Assets<Image>>>,
    mut pending_thumb: ResMut<PendingScenarioThumbnail>,
    panels: Query<Entity, With<ScenarioDetailsPanel>>,
) {
    let Ok(panel) = panels.single() else {
        return;
    };
    commands.entity(panel).despawn_related::<Children>();
    // Default: nothing pending. The thumbnail branch below re-parks a handle if
    // the image is still loading.
    pending_thumb.0 = None;
    let scenario = selected
        .0
        .as_ref()
        .and_then(|id| scenarios.as_ref().and_then(|s| s.get(id)))
        .cloned();
    commands.entity(panel).with_children(|details| {
        let Some(scenario) = scenario else {
            details.spawn((
                Name::new("Scenario Details Empty"),
                Text::new("Select a scenario to see its details."),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(theme::PHOSPHOR_MUTED),
            ));
            details.spawn((
                Name::new("Scenario Details Actions"),
                ScenarioDetailsActions,
                Node::default(),
            ));
            return;
        };

        details.spawn((
            Name::new("Scenario Details Name"),
            // Same label as the list row (the scenario's display name).
            Text::new(scenario_row_label(&scenario)),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(theme::SCREEN_TEXT),
        ));
        details.spawn((Name::new("Scenario Details Separator"), separator()));
        // The thumbnail (authored + asset server present; headless test apps have
        // neither). It is only mounted once the image has LOADED and is a plain
        // 2D single-layer texture. Both guards matter: the UI pipeline binds a D2
        // texture, so a cube/array/3D image (e.g. a scenario that points its
        // thumbnail at a skybox cubemap, whose Image is reinterpreted to a Cube
        // view) makes wgpu reject the `ui_material_bind_group` - a hard render
        // crash, native AND web. A still-loading image is parked in
        // `PendingScenarioThumbnail` and `poll_scenario_thumbnail` re-arms this
        // refresh when it lands; a non-2D one is skipped with a warning. Fixed
        // 16:9 box so an odd source does not distort the pane.
        if let (Some(thumb), Some(server)) = (scenario.thumbnail.as_ref(), asset_server.as_ref()) {
            let handle = thumb.resolve(server);
            if server.is_loaded_with_dependencies(&handle) {
                // A cube/array texture carries >1 layers; a plain 2D image has 1.
                let is_2d = images
                    .as_ref()
                    .and_then(|imgs| imgs.get(&handle))
                    .is_some_and(|img| img.texture_descriptor.size.depth_or_array_layers == 1);
                if is_2d {
                    details.spawn((
                        Name::new("Scenario Details Thumbnail"),
                        ImageNode::new(handle),
                        Node {
                            width: percent(100),
                            max_width: px(320),
                            aspect_ratio: Some(16.0 / 9.0),
                            margin: UiRect::bottom(px(8)),
                            border: UiRect::all(px(theme::BORDER_W)),
                            ..default()
                        },
                        BorderColor::all(theme::PHOSPHOR_MUTED),
                    ));
                } else {
                    warn!(
                        "scenario '{}' thumbnail is not a 2D image (cube/array/3D texture); \
                         skipping it - a UI thumbnail must be a plain 2D image, not a skybox \
                         cubemap.",
                        scenario.id
                    );
                }
            } else {
                pending_thumb.0 = Some(handle);
            }
        }
        if !scenario.description.is_empty() {
            details.spawn((
                Name::new("Scenario Details Description"),
                Text::new(scenario.description.clone()),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(theme::SCREEN_TEXT),
                Node {
                    margin: UiRect::bottom(px(8)),
                    ..default()
                },
            ));
        }
        details
            .spawn((
                Name::new("Scenario Details Actions"),
                ScenarioDetailsActions,
                Node::default(),
            ))
            .with_children(|actions| {
                actions
                    .spawn((
                        Name::new("Scenario Play Slot"),
                        Node {
                            width: px(140),
                            ..default()
                        },
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            Name::new("Scenario Play Button"),
                            themed_button("Play"),
                            ScenarioPlay {
                                id: scenario.id.clone(),
                            },
                            observe(on_scenario_play),
                        ));
                    });
            });
    });
}

/// While the selected scenario's thumbnail is still loading (parked in
/// [`PendingScenarioThumbnail`] by `refresh_scenario_details`), re-arm that
/// refresh the moment the image finishes loading, so it can validate the
/// texture and mount the `ImageNode` (or skip a non-2D one). Without this the
/// thumbnail would never appear after its first (still-loading) selection.
/// Fires `set_changed` at most once per load (the refresh clears the pending
/// handle when it mounts or skips the image).
pub(crate) fn poll_scenario_thumbnail(
    pending: Res<PendingScenarioThumbnail>,
    asset_server: Option<Res<AssetServer>>,
    mut selected: ResMut<SelectedScenarioId>,
) {
    let (Some(handle), Some(server)) = (pending.0.as_ref(), asset_server.as_ref()) else {
        return;
    };
    if server.is_loaded_with_dependencies(handle) {
        // `scenario_details_dirty` keys off `selected.is_changed()`; re-running
        // the refresh mounts the now-loaded image and clears `pending`.
        selected.set_changed();
    }
}

pub(crate) fn on_scenarios(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<ScenariosPanel>>,
) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

pub(crate) fn on_scenarios_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<ScenariosPanel>>,
) {
    **panel = Visibility::Hidden;
}

/// Select the clicked row's scenario: write [`SelectedScenarioId`] (which
/// re-arms `refresh_scenario_details`) and move the row `Selected` highlight.
pub(crate) fn on_scenario_row_select(
    activate: On<Activate>,
    rows: Query<(Entity, &ScenarioRow)>,
    selected_rows: Query<Entity, (With<ScenarioRow>, With<Selected>)>,
    mut selected: ResMut<SelectedScenarioId>,
    mut commands: Commands,
) {
    let Ok((entity, row)) = rows.get(activate.entity) else {
        return;
    };
    if selected.0.as_deref() == Some(row.id.as_str()) {
        return;
    }
    for previous in &selected_rows {
        commands.entity(previous).remove::<Selected>();
    }
    commands.entity(entity).insert(Selected);
    selected.0 = Some(row.id.clone());
}

/// Toggle a campaign's expand/collapse state: flip its id in
/// [`CollapsedCampaigns`], which re-arms `refresh_scenarios_list` (via
/// [`scenarios_list_dirty`]) to add or remove its member rows.
pub(crate) fn on_campaign_header_toggle(
    activate: On<Activate>,
    headers: Query<&CampaignHeader>,
    mut collapsed: ResMut<CollapsedCampaigns>,
) {
    let Ok(header) = headers.get(activate.entity) else {
        return;
    };
    if !collapsed.0.remove(&header.id) {
        collapsed.0.insert(header.id.clone());
    }
}

/// Play the selected scenario: record the override and hand off to Playing
/// exactly like New Game (so the same OnEnter loader, camera grab and backdrop
/// teardown apply). `start_new_game_scenario` reads [`NewGameScenario`].
pub(crate) fn on_scenario_play(
    activate: On<Activate>,
    plays: Query<&ScenarioPlay>,
    mut pick: ResMut<NewGameScenario>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    let Ok(play) = plays.get(activate.entity) else {
        return;
    };
    pick.0 = Some(play.id.clone());
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
}
