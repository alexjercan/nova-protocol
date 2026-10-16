//! The main-menu panel itself: its layout and the New Game / Sandbox / Exit
//! buttons, plus the shared `OnEnter(Playing)` scenario start they hand off to.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_gameplay::prelude::*;
use nova_info::prelude::*;
use nova_scenario::prelude::{OpenWorldSession, *};
use nova_training::prelude::{catalog_field_notes, FieldNoteRotation, TrainingCatalog};
use nova_ui::{
    screen::{
        details_pane, footer_back_slot, list_detail_screen, list_pane, overlay_root, scroll_bar,
        scroll_column, scroll_viewport,
    },
    theme,
    theme::UiColor,
    widget::{
        panel, panel_head, segmented_container, segmented_option_fit, ButtonVariant, Selected,
        ThemedFill, ThemedRadius, ThemedText, ThemedTextShadow, UiText,
    },
};
#[cfg(not(target_arch = "wasm32"))]
use nova_world_base::prelude::WorldSaveSession;

#[cfg(not(target_arch = "wasm32"))]
use crate::{
    leave::{begin_leave, LeaveTarget},
    load_screen::{on_load_back, on_load_screen, LoadPanel, LoadWorldDetails, LoadWorldList},
};
use crate::{
    mods::{
        on_mods, on_mods_back, on_mods_tab, ModDetailsPanel, ModsActiveTab, ModsList, ModsPanel,
        ModsTab, ModsTabKind, SelectedModId,
    },
    scenarios::{
        on_scenarios, on_scenarios_back, NewGameScenario, ScenarioDetailsPanel, ScenariosList,
        ScenariosPanel, SelectedScenarioId,
    },
    settings::{
        build_settings_tabs, on_settings, on_settings_back, SettingsActiveTab, SettingsPanel,
        SettingsTabBody, SETTINGS_PANEL_H, SETTINGS_PANEL_MAX_H_PCT, SETTINGS_PANEL_MAX_W,
        SETTINGS_PANEL_WIDTH_PCT,
    },
    training::{on_training, spawn_menu_aside, spawn_training_panel},
    widgets::{back_button, button, button_variant, MenuSfxButton},
    world_setup::on_new_game,
};

/// The menu panel: title on top, buttons below, anchored bottom-right per the
/// spike's layout call (the center of the screen stays free for the background
/// scene).
pub(crate) fn setup_menu_ui(
    mut commands: Commands,
    mut active_tab: ResMut<ModsActiveTab>,
    mut selected: ResMut<SelectedModId>,
    mut selected_scenario: ResMut<SelectedScenarioId>,
    active_settings_tab: Res<SettingsActiveTab>,
    catalog: Res<TrainingCatalog>,
    mut rotation: ResMut<FieldNoteRotation>,
    time: Res<Time<Real>>,
) {
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Menu Panel"),
            Node {
                position_type: PositionType::Absolute,
                right: px(40),
                bottom: px(40),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                width: px(280),
                padding: UiRect::all(px(20)),
                border: UiRect::all(px(theme::BORDER_W)),
                ..default()
            },
            ThemedRadius::control(),
            panel(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Title"),
                UiText,
                Text::new("Nova Protocol"),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(Color::NONE),
                ThemedText::new(UiColor::Body),
                ThemedTextShadow::alpha(UiColor::Primary, 0.35),
                TextShadow {
                    color: Color::NONE,
                    offset: Vec2::ZERO,
                },
            ));
            parent.spawn((
                Name::new("Title Separator"),
                Node {
                    width: percent(80),
                    height: px(2),
                    margin: UiRect::all(px(10)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ThemedFill::new(UiColor::Label),
            ));
            parent.spawn((
                Name::new("New Game Button"),
                button_variant("New Game", ButtonVariant::Primary, None),
                observe(on_new_game),
            ));
            // Saved worlds are a desktop-build concept; the web build never
            // offers a folder to load from.
            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn((
                Name::new("Load Button"),
                button("Load"),
                observe(on_load_screen),
            ));
            parent.spawn((
                Name::new("Sandbox Button"),
                button("Sandbox"),
                observe(on_sandbox),
            ));
            parent.spawn((
                Name::new("Scenarios Button"),
                button("Scenarios"),
                observe(on_scenarios),
            ));
            parent.spawn((
                Name::new("Lessons Button"),
                button("Lessons"),
                observe(on_training),
            ));
            parent.spawn((Name::new("Mods Button"), button("Mods"), observe(on_mods)));
            parent.spawn((
                Name::new("Settings Button"),
                button("Settings"),
                observe(on_settings),
            ));
            // No process to quit on wasm; the browser tab owns the lifecycle.
            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn((
                Name::new("Exit Button"),
                button_variant("Exit", ButtonVariant::Danger, None),
                observe(on_exit),
            ));
            parent
                .spawn((
                    Name::new("Menu Footer"),
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::top(px(10)),
                        ..default()
                    },
                ))
                .with_children(|foot| {
                    for text in [format!("v{APP_VERSION}"), "NOVA OS".to_string()] {
                        foot.spawn((
                            UiText,
                            Text::new(text),
                            TextFont {
                                font_size: FontSize::Px(10.0),
                                ..default()
                            },
                            TextColor(Color::NONE),
                            ThemedText::new(UiColor::Label),
                        ));
                    }
                });
        });

    // The Settings overlay: hidden until the Settings button toggles it.
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Settings Panel Root"),
            SettingsPanel,
            overlay_root(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Settings Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        // A share of the window up to its own maximum, and a
                        // FIXED height so the panel does not resize under the
                        // pointer when a tab with fewer rows opens. Both caps
                        // keep it on screen on a small display; see
                        // `SETTINGS_PANEL_WIDTH_PCT`.
                        width: percent(SETTINGS_PANEL_WIDTH_PCT),
                        max_width: px(SETTINGS_PANEL_MAX_W),
                        height: px(SETTINGS_PANEL_H),
                        max_height: percent(SETTINGS_PANEL_MAX_H_PCT),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::panel(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Settings Title"),
                        panel_head("Settings", Some("SAVED ON CHANGE")),
                    ));
                    build_settings_tabs(parent, active_settings_tab.0);
                    parent
                        .spawn((
                            Name::new("Settings Body Row"),
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Stretch,
                                // Takes the panel's leftover height, so the
                                // scroll viewport clips instead of the fixed
                                // panel overflowing past the Back button.
                                flex_grow: 1.0,
                                min_height: px(0),
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            // Spawned EMPTY: `refresh_settings_tab` fills it on
                            // the first Update after entry, the same path a tab
                            // press and a rebind take.
                            row.spawn((
                                Name::new("Settings Body"),
                                SettingsTabBody,
                                scroll_column(),
                                scroll_viewport(),
                            ));
                            row.spawn((Name::new("Settings Scroll Bar"), scroll_bar()));
                        });
                    parent
                        .spawn((
                            Name::new("Settings Footer"),
                            footer_back_slot((
                                Name::new("Settings Back Button"),
                                back_button("Back"),
                                observe(on_settings_back),
                            )),
                        ))
                        .entry::<Node>()
                        .and_modify(|mut node| node.margin = UiRect::top(px(12)));
                });
        });

    // The Mods screen: hidden until the Mods button toggles it. Both panes spawn
    // EMPTY; writing the two resources below marks them changed, which re-arms
    // refresh_mods_list/refresh_mod_details to fill them on the first Update frame
    // after entry - one population path for entry, tab switches and live catalog
    // changes alike.
    *active_tab = ModsActiveTab::default();
    selected.0 = None;

    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Mods Panel Root"),
            ModsPanel,
            // The mods panel has its own Back button, so covering the menu card
            // loses nothing. Rendered z-order is only visually verifiable - the
            // component-presence test pins this.
            overlay_root(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Mods Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: percent(85),
                        height: percent(85),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::panel(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((Name::new("Mods Title"), panel_head("Mods", Some("DELTA-9"))));
                    parent.spawn((
                        Name::new("Mods Subtitle"),
                        Text::new("Enable installed mods. Base is always on."),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));

                    parent.spawn((
                        Name::new("Mods Content"),
                        list_detail_screen(
                            (
                                Name::new("Mods Left Pane"),
                                list_pane(),
                                children![
                                    (
                                        Name::new("Mods Tab Row"),
                                        // The same segmented bar the Settings
                                        // tabs use, with fit options: a
                                        // full-width option in a content-sized
                                        // bar wraps "Explore online" onto two
                                        // lines. setup resets ModsActiveTab to
                                        // Installed above, so the static
                                        // Selected marker matches it.
                                        segmented_container(),
                                        children![
                                            (
                                                Name::new("Installed Tab"),
                                                segmented_option_fit("Installed"),
                                                MenuSfxButton,
                                                ModsTab(ModsTabKind::Installed),
                                                Selected,
                                                observe(on_mods_tab),
                                            ),
                                            (
                                                Name::new("Explore Online Tab"),
                                                segmented_option_fit("Explore online"),
                                                MenuSfxButton,
                                                ModsTab(ModsTabKind::Explore),
                                                observe(on_mods_tab),
                                            ),
                                        ],
                                    ),
                                    (
                                        Name::new("Mods List"),
                                        ModsList,
                                        Node {
                                            margin: UiRect::top(px(8)),
                                            ..scroll_column()
                                        },
                                        scroll_viewport(),
                                    ),
                                ],
                            ),
                            (
                                Name::new("Mod Details Panel"),
                                ModDetailsPanel,
                                details_pane(),
                            ),
                        ),
                    ));

                    parent.spawn((
                        Name::new("Mods Footer"),
                        footer_back_slot((
                            Name::new("Mods Back Button"),
                            back_button("Back"),
                            observe(on_mods_back),
                        )),
                    ));
                });
        });

    // The Scenarios picker: hidden until the Scenarios button toggles it. Same
    // empty-panes-then-re-arm shape as the mods panel above, driven by resetting
    // SelectedScenarioId.
    selected_scenario.0 = None;

    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Scenarios Panel Root"),
            ScenariosPanel,
            overlay_root(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Scenarios Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: percent(85),
                        height: percent(85),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::panel(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Scenarios Title"),
                        panel_head("Scenarios", Some("PICK ONE")),
                    ));
                    parent.spawn((
                        Name::new("Scenarios Subtitle"),
                        Text::new("Pick a scenario to play."),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        ThemedText::new(UiColor::Label),
                    ));

                    parent.spawn((
                        Name::new("Scenarios Content"),
                        list_detail_screen(
                            (
                                Name::new("Scenarios List"),
                                ScenariosList,
                                Node {
                                    overflow: Overflow::scroll_y(),
                                    ..list_pane()
                                },
                                scroll_viewport(),
                            ),
                            (
                                Name::new("Scenario Details Panel"),
                                ScenarioDetailsPanel,
                                details_pane(),
                            ),
                        ),
                    ));

                    parent.spawn((
                        Name::new("Scenarios Footer"),
                        footer_back_slot((
                            Name::new("Scenarios Back Button"),
                            back_button("Back"),
                            observe(on_scenarios_back),
                        )),
                    ));
                });
        });

    // The Load picker: hidden until the Load button toggles it. Same
    // list-beside-details shape and panel head as the Mods panel; the Load
    // button itself re-reads the worlds root on each open, so nothing here
    // needs to seed its resources.
    #[cfg(not(target_arch = "wasm32"))]
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Load Panel Root"),
            LoadPanel,
            overlay_root(),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Load Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: percent(85),
                        height: percent(85),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        ..default()
                    },
                    ThemedRadius::panel(),
                    panel(),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Load Title"),
                        panel_head("Load", Some("SAVED WORLDS")),
                    ));

                    parent.spawn((
                        Name::new("Load Content"),
                        list_detail_screen(
                            (
                                Name::new("Load World List"),
                                LoadWorldList,
                                Node {
                                    overflow: Overflow::scroll_y(),
                                    ..list_pane()
                                },
                                scroll_viewport(),
                            ),
                            (
                                Name::new("Load World Details Panel"),
                                LoadWorldDetails,
                                details_pane(),
                            ),
                        ),
                    ));

                    parent.spawn((
                        Name::new("Load Footer"),
                        footer_back_slot((
                            Name::new("Load Back Button"),
                            back_button("Back"),
                            observe(on_load_back),
                        )),
                    ));
                });
        });

    // The Training handbook, reached from the `Lessons` row above, and the
    // bottom-left notice corner: the offer to fly Basic Training on a fresh
    // install, and a field note. The offer is a one-off - once it is answered
    // the row is the way in - and the note is picked ONCE here, on menu entry,
    // because the corner is never reconciled and a note must not change while
    // the player is reading it.
    spawn_training_panel(&mut commands);
    let notes = catalog_field_notes(&catalog);
    let note = rotation.pick(&notes, time.elapsed().subsec_nanos() as usize);
    spawn_menu_aside(&mut commands, note);
}

pub(crate) fn on_sandbox(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    *mode = GameMode::Sandbox;
    state.set(GameStates::Playing);
}

/// Quit the game. In a saved world (the pause menu's Exit) it waits on the
/// leave save first.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_exit(
    _activate: On<Activate>,
    mut commands: Commands,
    mut saved: Option<ResMut<WorldSaveSession>>,
    mut pause: ResMut<NextState<PauseStates>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(session) = saved.as_mut() {
        begin_leave(LeaveTarget::Exit, &mut commands, session, &mut pause);
        return;
    }
    exit.write(AppExit::Success);
}

/// Why the New Game hand-off could not resolve a scenario to load. Each is an
/// assembly fault in the base bundle or in the launch request, not a runtime
/// condition: the content lint refuses the declared-start faults offline, and
/// the picker, the handbook and `--scenario` only hand over registered ids.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NewGameStartFault {
    /// The base bundle declares no `new_game_scenario`.
    NoStartDeclared,
    /// The declared start is not in the registry.
    StartNotRegistered(ScenarioId),
    /// The declared start's world takes no session seed, so the seed New Game
    /// supplies would go unused (lint rule: the start must need the session).
    StartTakesNoSession {
        /// The declared start.
        id: ScenarioId,
        /// The world it declares instead.
        world: ScenarioWorldType,
    },
    /// The picked scenario is not in the registry.
    PickNotRegistered(ScenarioId),
    /// The resolved scenario takes its seed from the session and none exists:
    /// something other than New Game or Load reached the loader with it.
    NoSession(ScenarioId),
    /// The save Load brought in names a scenario the registry lacks. Load
    /// refuses such a save before it opens, so reaching this is a fault.
    ResumedNotRegistered(ScenarioId),
}

impl std::fmt::Display for NewGameStartFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoStartDeclared => {
                write!(f, "the base bundle declares no new_game_scenario")
            }
            Self::StartNotRegistered(id) => {
                write!(f, "the base-declared start '{id}' is not registered")
            }
            Self::StartTakesNoSession { id, world } => write!(
                f,
                "the base-declared start '{id}' declares world {world:?}; New Game supplies \
                 the seed, so the start must declare `world: Generated((seed: None))`"
            ),
            Self::PickNotRegistered(id) => {
                write!(f, "the picked scenario '{id}' is not registered")
            }
            Self::NoSession(id) => write!(
                f,
                "'{id}' declares `world: Generated((seed: None))` and no OpenWorldSession \
                 exists; only New Game's Create and Load may start it"
            ),
            Self::ResumedNotRegistered(id) => {
                write!(f, "the saved world's scenario '{id}' is not registered")
            }
        }
    }
}

/// The scenario the New Game hand-off loads: the scenario a Load's save
/// names (`resumed`) when one is waiting, else the [`NewGameScenario`] pick
/// when the picker, the handbook or `--scenario` set one, else the base
/// bundle's declared start ([`NewGameStart`], written by the bundle merge
/// from `base.bundle.ron`'s `new_game_scenario`).
///
/// A Load loads what it saved, not today's declared start: the saved state
/// belongs to that scenario. The declared start must take its seed from the
/// session, and whichever scenario resolves may need the session only when
/// `has_session`. There is no fallback past a fault: a start the base got
/// wrong is refused, not replaced by the first listed scenario.
pub(crate) fn resolve_new_game_scenario(
    scenarios: &GameScenarios,
    resumed: Option<&ScenarioId>,
    start: &NewGameStart,
    pick: &NewGameScenario,
    has_session: bool,
) -> Result<ScenarioConfig, NewGameStartFault> {
    let scenario = match (resumed, &pick.0, &start.0) {
        (Some(id), ..) => scenarios
            .get(id)
            .ok_or_else(|| NewGameStartFault::ResumedNotRegistered(id.clone()))?,
        (None, Some(id), _) => scenarios
            .get(id)
            .ok_or_else(|| NewGameStartFault::PickNotRegistered(id.clone()))?,
        (None, None, Some(id)) => {
            let scenario = scenarios
                .get(id)
                .ok_or_else(|| NewGameStartFault::StartNotRegistered(id.clone()))?;
            if !scenario.world.needs_session() {
                return Err(NewGameStartFault::StartTakesNoSession {
                    id: id.clone(),
                    world: scenario.world.clone(),
                });
            }
            scenario
        }
        (None, None, None) => return Err(NewGameStartFault::NoStartDeclared),
    };
    if scenario.world.needs_session() && !has_session {
        return Err(NewGameStartFault::NoSession(scenario.id.clone()));
    }
    Ok(scenario.clone())
}

/// In `NewGame` mode the menu itself provides the game: load the scenario
/// [`resolve_new_game_scenario`] names the moment gameplay starts. `Sandbox`
/// mode does nothing here - the editor owns that path.
///
/// A fault is logged and ends the app with an error exit, loading nothing:
/// the same refusal `--scenario` gives a session-seeded id at boot, reached
/// from the menu's Create, Load, the picker and the handbook.
pub(crate) fn start_new_game_scenario(
    mut commands: Commands,
    scenarios: Res<GameScenarios>,
    resumed: Option<Res<ResumedScenario>>,
    start: Res<NewGameStart>,
    pick: Res<NewGameScenario>,
    session: Option<Res<OpenWorldSession>>,
    mut exit: MessageWriter<AppExit>,
) {
    let resumed = resumed.as_ref().map(|resumed| &resumed.0.scenario_id);
    match resolve_new_game_scenario(&scenarios, resumed, &start, &pick, session.is_some()) {
        Ok(scenario) => commands.trigger(LoadScenario(scenario)),
        Err(fault) => {
            error!("start_new_game_scenario: {fault}; New Game loads nothing");
            exit.write(AppExit::error());
        }
    }
}
