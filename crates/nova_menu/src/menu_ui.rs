//! The main-menu panel itself: its layout and the New Game / Sandbox / Exit
//! buttons, plus the shared `OnEnter(Playing)` scenario start they hand off to.

use bevy::{
    prelude::*,
    ui_widgets::{observe, Activate},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;
use nova_ui::{
    prelude::UiSkin,
    screen::{
        details_pane, footer_back_slot, list_detail_screen, list_pane, overlay_root, scroll_bar,
        scroll_column, scroll_viewport,
    },
    theme,
    widget::{panel, panel_head, themed_button, ButtonVariant, Selected, UiText},
};

use crate::{
    mods::{
        on_mods, on_mods_back, on_mods_tab, ModDetailsPanel, ModsActiveTab, ModsList, ModsPanel,
        ModsTab, ModsTabKind, SelectedModId,
    },
    scenarios::{
        listed_scenarios, on_scenarios, on_scenarios_back, NewGameScenario, ScenarioDetailsPanel,
        ScenariosList, ScenariosPanel, SelectedScenarioId,
    },
    settings::{
        build_settings_tabs, on_settings, on_settings_back, SettingsActiveTab, SettingsPanel,
        SettingsTabBody,
    },
    widgets::{button, button_variant},
};

/// The menu panel: title on top, buttons below, anchored bottom-right per the
/// spike's layout call (the center of the screen stays free for the background
/// scene).
pub(crate) fn setup_menu_ui(
    mut commands: Commands,
    mut active_tab: ResMut<ModsActiveTab>,
    mut selected: ResMut<SelectedModId>,
    mut selected_scenario: ResMut<SelectedScenarioId>,
    skin: Res<UiSkin>,
    active_settings_tab: Res<SettingsActiveTab>,
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
                border_radius: BorderRadius::all(px(theme::RADIUS)),
                ..default()
            },
            panel(*skin),
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
                TextColor(theme::SCREEN_TEXT),
                TextShadow {
                    color: theme::PHOSPHOR.with_alpha(0.35),
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
                BackgroundColor(theme::PHOSPHOR_MUTED),
            ));
            parent.spawn((
                Name::new("New Game Button"),
                button_variant("New Game", ButtonVariant::Primary, None),
                observe(on_new_game),
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
                    for text in [
                        format!("v{}", env!("CARGO_PKG_VERSION")),
                        "NOVA OS".to_string(),
                    ] {
                        foot.spawn((
                            UiText,
                            Text::new(text),
                            TextFont {
                                font_size: FontSize::Px(10.0),
                                ..default()
                            },
                            TextColor(theme::PHOSPHOR_MUTED),
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
                        width: px(460),
                        max_height: percent(92),
                        padding: UiRect::all(px(20)),
                        border: UiRect::all(px(theme::BORDER_W)),
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Settings Title"),
                        Text::new("Settings"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                        Node {
                            margin: UiRect::bottom(px(12)),
                            ..default()
                        },
                    ));
                    build_settings_tabs(parent, *skin, active_settings_tab.0);
                    parent
                        .spawn((
                            Name::new("Settings Body Row"),
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Stretch,
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
                            row.spawn((Name::new("Settings Scroll Bar"), scroll_bar(*skin)));
                        });
                    parent.spawn((
                        Name::new("Settings Back Button"),
                        button("Back"),
                        observe(on_settings_back),
                    ));
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
                        border_radius: BorderRadius::all(px(theme::PANEL_RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Mods Title"),
                        panel_head("Mods", Some("DELTA-9"), *skin),
                    ));
                    parent.spawn((
                        Name::new("Mods Subtitle"),
                        Text::new("Enable installed mods. Base is always on."),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(theme::PHOSPHOR_MUTED),
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
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            align_self: AlignSelf::Stretch,
                                            column_gap: px(8),
                                            ..default()
                                        },
                                        // setup resets ModsActiveTab to
                                        // Installed above, so the static
                                        // Selected marker matches it.
                                        children![
                                            (
                                                Name::new("Installed Tab"),
                                                themed_button("Installed"),
                                                ModsTab(ModsTabKind::Installed),
                                                Selected,
                                                observe(on_mods_tab),
                                            ),
                                            (
                                                Name::new("Explore Online Tab"),
                                                themed_button("Explore online"),
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
                            button("Back"),
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
                        border_radius: BorderRadius::all(px(theme::RADIUS)),
                        ..default()
                    },
                    panel(*skin),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Scenarios Title"),
                        Text::new("Scenarios"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(theme::SCREEN_TEXT),
                    ));
                    parent.spawn((
                        Name::new("Scenarios Subtitle"),
                        Text::new("Pick a scenario to play. New Game plays the main story."),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(theme::PHOSPHOR_MUTED),
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
                            button("Back"),
                            observe(on_scenarios_back),
                        )),
                    ));
                });
        });
}

pub(crate) fn on_new_game(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
    mut pick: ResMut<NewGameScenario>,
) {
    // New Game always plays the main story from the top: clear any override the
    // Scenarios picker left, so `start_new_game_scenario` loads the canned start.
    pick.0 = None;
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
}

pub(crate) fn on_sandbox(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    *mode = GameMode::Sandbox;
    state.set(GameStates::Playing);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_exit(_activate: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

/// In `NewGame` mode the menu itself provides the game: load a scenario (player
/// ship included) the moment gameplay starts. `Sandbox` mode does nothing here -
/// the editor owns that path.
///
/// Which scenario, in fallback order (each miss warns and falls through):
/// 1. the [`NewGameScenario`] override, if the Scenarios picker set one and it
///    is still registered (a mod can get disabled between pick and play);
/// 2. the base bundle's declared start ([`NewGameStart`], written by the
///    bundle merge from `base.bundle.ron`'s `new_game_scenario` - base-owned,
///    not moddable);
/// 3. the first LISTED scenario (the picker's own order), so a base bundle
///    that forgot to declare a start still launches something;
/// 4. nothing registered at all: log an error and load nothing.
pub(crate) fn start_new_game_scenario(
    mut commands: Commands,
    scenarios: Res<GameScenarios>,
    start: Res<NewGameStart>,
    pick: Res<NewGameScenario>,
) {
    let picked = pick.0.as_ref().filter(|id| {
        let registered = scenarios.contains_key(*id);
        if !registered {
            warn!(
                "start_new_game_scenario: picked scenario '{id}' not in GameScenarios; \
                 falling back to the base-declared start"
            );
        }
        registered
    });
    let declared = picked.is_none().then(|| {
        start.0.as_ref().filter(|id| {
            let registered = scenarios.contains_key(*id);
            if !registered {
                warn!(
                    "start_new_game_scenario: the base-declared start '{id}' is not \
                     registered; falling back to the first listed scenario"
                );
            }
            registered
        })
    });

    let id = match (picked, declared) {
        (Some(id), _) => id.clone(),
        (None, Some(Some(id))) => id.clone(),
        _ => {
            if start.0.is_none() {
                warn!(
                    "start_new_game_scenario: the base bundle declares no \
                     new_game_scenario; falling back to the first listed scenario"
                );
            }
            match listed_scenarios(&scenarios).into_iter().next() {
                Some(first) => first.id,
                None => {
                    error!(
                        "start_new_game_scenario: no scenario is registered at all; \
                         New Game loads nothing"
                    );
                    return;
                }
            }
        }
    };
    let scenario = scenarios
        .get(&id)
        .expect("the fallback chain only yields registered ids")
        .clone();
    commands.trigger(LoadScenario(scenario));
}
