//! The main menu: the game's front door.
//!
//! `NovaMenuPlugin` owns [`GameStates::MainMenu`]: a small panel anchored to the
//! bottom-right of the screen with the game title and New Game / Sandbox /
//! Settings / Exit buttons, drawn over a live ambient scene - the
//! `menu_ambience` scenario (nova_assets), where an AI ship flies a real
//! thruster-driven orbit around a planetoid's gravity well (its
//! AIControllerConfig orbit directive engages the ORBIT autopilot), watched by a
//! fixed cinematic camera with the status bar hidden. The buttons write
//! [`GameMode`] and hand off to [`GameStates::Playing`]; the editor
//! (`nova_editor`) only comes up in `Sandbox` mode, and the menu's own
//! `OnEnter(Playing)` system loads the New Game scenario in `NewGame` mode.
//!
//! `nova_core`'s `AppBuilder` adds this plugin (and routes `Loading -> MainMenu`
//! instead of `Loading -> Playing`) only for the default editor app; examples
//! that supply their own game plugins never see the menu.
#![warn(missing_docs)]

use bevy::{
    prelude::*,
    state::state::{StateTransition, StateTransitionSystems},
};
use nova_assets::prelude::{GameAssets, ModQuarantine, ReloadContent};
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_os::prelude::NovaOsTerminal;
use nova_os_ui::prelude::NovaOsCloseTransition;
use nova_scenario::prelude::{CurrentOutcome, ScenarioStartFailure, UnloadScenario};
use nova_ui::{
    input_mode::prelude::{in_input_mode, InputMode},
    prelude::UiSkin,
    widget::button_on_setting,
};

/// Glob-import surface: `use nova_menu::prelude::*` brings [`NovaMenuPlugin`]
/// and [`NewGameScenario`] into scope.
pub mod prelude {
    pub use super::{
        ambience::MENU_BACKDROP_ENV,
        pause::FocusPause,
        settings::{
            FieldNoteSetting, TrainingPromptSetting, WindowModeSetting, SETTINGS_PANEL_H,
            SETTINGS_PANEL_MAX_H_PCT, SETTINGS_PANEL_MAX_W, SETTINGS_PANEL_WIDTH_PCT,
        },
        settings_store::{
            load_settings, save_settings, PersistedSettings, SettingsStoreAccess,
            SettingsStorePlugin, SettingsStoreRoot,
        },
        training_store::{
            allow_training_saves, load_training, save_training, PersistedTraining,
            TrainingProgressPlugin, TrainingStoreAccess,
        },
        widgets::MenuCueSystems,
        NewGameScenario, NovaMenuPlugin,
    };
}

mod ambience;
mod menu_ui;
mod mods;
mod outcome;
mod pause;
mod portal;
mod safe_mode;
mod scenarios;
mod settings;
mod settings_store;
mod training;
mod training_store;
mod widgets;

#[cfg(test)]
mod tests;

use ambience::{
    hide_hud_chrome, load_menu_ambience, restore_hud_chrome, stage_menu_camera,
    unload_menu_ambience,
};
use menu_ui::{setup_menu_ui, start_new_game_scenario};
use mods::{
    mod_details_dirty, mods_list_dirty, refresh_mod_details, refresh_mods_list,
    sync_mod_checkboxes, ModsActiveTab, SelectedModId,
};
use nova_training::prelude::{FieldNoteRotation, TrainingCatalog};
use outcome::{
    auto_advance_outcome, clear_start_failure, regrab_cursor_on_player_spawn, sync_outcome_cursor,
    sync_outcome_overlay, sync_outcome_pause, sync_start_failure_cursor,
    sync_start_failure_overlay,
};
use pause::{
    force_unpause, hold_clocks_for_pause_menu, hold_clocks_for_terminal,
    keep_frozen_cursor_released, open_command_shell, pause_on_focus_loss, reconcile_pause_overlay,
    release_clocks_for_pause_menu, release_clocks_for_terminal, release_cursor, restore_cursor,
    toggle_pause, FocusPause,
};
use portal::{drive_update_choreography, UpdateRequested};
pub use scenarios::NewGameScenario;
use scenarios::{
    poll_scenario_thumbnail, refresh_scenario_details, refresh_scenarios_list,
    scenario_details_dirty, scenarios_list_dirty, CollapsedCampaigns, PendingScenarioThumbnail,
    SelectedScenarioId,
};
use settings::{
    apply_settings_rebind, on_sensitivity_slider_change, on_volume_slider_change,
    refresh_settings_tab, settings_tab_dirty, sync_sensitivity_slider, sync_volume_slider,
    FieldNoteSetting, PendingRebind, SettingsActiveTab, SettingsControlsGroup,
    TrainingPromptSetting, WindowModeSetting,
};
use settings_store::SettingsStorePlugin;
use training::{
    advance_lesson_loops, poll_lesson_media, refresh_training_details, refresh_training_list,
    sync_menu_aside, training_details_dirty, training_list_dirty, PendingLessonMedia,
    SelectedLessonId,
};
use training_store::TrainingProgressPlugin;
use widgets::{on_menu_button_activate, play_menu_focus_cue, MenuCueSystems};

/// The main-menu plugin: owns [`GameStates::MainMenu`] and the settings/mods/
/// scenarios screens.
///
/// On `OnEnter(MainMenu)` it loads the ambient backdrop scenario and builds the
/// menu UI; `Update` runs the button/colour, settings-sync, mods-screen refresh
/// and update-choreography systems; the New Game / Sandbox buttons write
/// [`GameMode`] and hand off to [`GameStates::Playing`]. Added by `nova_core`'s
/// `AppBuilder` only for the default editor app.
pub struct NovaMenuPlugin;

impl Plugin for NovaMenuPlugin {
    fn build(&self, app: &mut App) {
        // HudVisibility is owned by another plugin in the assembled app.
        // init_resource is idempotent, so initing it here too lets the menu
        // plugin stand alone in slim and headless-test apps.
        app.init_resource::<HudVisibility>();
        app.init_resource::<ModsActiveTab>();
        app.init_resource::<SelectedModId>();
        app.init_resource::<SelectedScenarioId>();
        app.init_resource::<NewGameScenario>();
        app.init_resource::<PendingScenarioThumbnail>();
        app.init_resource::<ambience::MenuCameraMemory>();
        app.init_resource::<CollapsedCampaigns>();
        // The handbook's material and the player's record of it. The CATALOG is
        // merged content: `register_bundles` inserts the base bundle's lessons
        // plus every enabled mod's over this empty default, so a rig with no
        // asset stack (slim apps, headless tests) still has the resource the
        // screen reads. The RECORD belongs to `TrainingProgressPlugin`, which
        // owns the resource and both directions of its store; adding it under
        // a guard is what keeps the menu standing alone in a slim rig.
        app.init_resource::<TrainingCatalog>();
        // The session's field-note rotation, shared with the loading screens'
        // own slot so a note the player just read on a load does not come
        // straight back on the menu. `LoadingScreenPlugin` inits the same
        // resource; whoever gets there first wins, and init_resource is
        // idempotent, which is what lets the menu stand alone.
        app.init_resource::<FieldNoteRotation>();
        if !app.is_plugin_added::<TrainingProgressPlugin>() {
            app.add_plugins(TrainingProgressPlugin::default());
        }
        // The handbook is where a lesson is read, so the menu is what turns a
        // reading progress store into a writing one - the same bargain the
        // settings panel strikes with `allow_settings_saves`.
        training_store::allow_training_saves(app);
        app.init_resource::<SelectedLessonId>();
        app.init_resource::<PendingLessonMedia>();
        app.init_resource::<UpdateRequested>();
        // Ungated by menu state on purpose - an update started from the
        // menu must complete even if the player closes it mid-flight.
        app.add_systems(Update, drive_update_choreography);
        // The editor and gameplay want the same app-global UI wiring; whoever
        // gets there first adds it.
        if !app.is_plugin_added::<nova_ui::NovaUiPlugin>() {
            app.add_plugins(nova_ui::NovaUiPlugin);
        }

        // The store owns the settings resources and the load that fills them.
        // `AppBuilder` adds it to every app, menu or not, so it is normally
        // already here; adding it under a guard is what keeps the menu standing
        // alone in slim and headless-test rigs.
        if !app.is_plugin_added::<SettingsStorePlugin>() {
            app.add_plugins(SettingsStorePlugin::from_env());
        }
        // The panel below is what earns the store its write direction: this is
        // where a player asks for a setting to be KEPT. An app without one
        // reads the file and leaves it alone.
        settings_store::allow_settings_saves(app);
        app.init_resource::<SettingsActiveTab>();
        app.init_resource::<SettingsControlsGroup>();
        app.init_resource::<PendingRebind>();
        app.add_observer(on_volume_slider_change);
        app.add_observer(on_sensitivity_slider_change);
        app.add_observer(button_on_setting::<GraphicsQuality>);
        app.add_observer(button_on_setting::<UiSkin>);
        app.add_observer(button_on_setting::<WindowModeSetting>);
        app.add_observer(button_on_setting::<TrainingPromptSetting>);
        app.add_observer(button_on_setting::<FieldNoteSetting>);
        app.add_systems(Update, (sync_volume_slider, sync_sensitivity_slider));
        // Ungated by menu state: the SAME body is the pause overlay's, which
        // only exists while playing.
        app.add_systems(
            Update,
            (
                // The capture runs FIRST: a rebind taken this frame is what the
                // rows redraw from, so the chip stops prompting on the same
                // frame the key lands rather than a frame later.
                (
                    apply_settings_rebind,
                    refresh_settings_tab.run_if(settings_tab_dirty),
                )
                    .chain(),
            ),
        );

        app.add_systems(
            OnEnter(GameStates::MainMenu),
            (
                load_menu_ambience,
                ambience::spawn_menu_ui_camera,
                setup_menu_ui,
                hide_hud_chrome,
            ),
        );
        // EVERY exit from the menu unloads the backdrop, so no exit path
        // can leak a simulating scenario. OnExit runs before OnEnter(Playing),
        // so New Game's LoadScenario still lands after this unload.
        app.add_systems(
            OnExit(GameStates::MainMenu),
            (restore_hud_chrome, unload_menu_ambience),
        );
        app.add_systems(
            Update,
            (stage_menu_camera, sync_mod_checkboxes).run_if(in_state(GameStates::MainMenu)),
        );
        // Safe mode's one report, owed on the front door once per recovery.
        // `resource_exists`-gated: a menu rig without the content pipeline has
        // no quarantine to report.
        app.add_systems(
            Update,
            safe_mode::sync_mod_report_overlay
                .run_if(in_state(GameStates::MainMenu))
                .run_if(resource_exists::<ModQuarantine>),
        );
        // Chained so a default selection made while rebuilding the list is
        // rendered by the details refresh in the SAME frame.
        app.add_systems(
            Update,
            (
                refresh_mods_list.run_if(mods_list_dirty),
                refresh_mod_details.run_if(mod_details_dirty),
            )
                .chain()
                .run_if(in_state(GameStates::MainMenu)),
        );
        // Same same-frame chain rule as the mods screen above.
        app.add_systems(
            Update,
            (
                poll_scenario_thumbnail,
                refresh_scenarios_list.run_if(scenarios_list_dirty),
                refresh_scenario_details.run_if(scenario_details_dirty),
            )
                .chain()
                .run_if(in_state(GameStates::MainMenu)),
        );
        // Same same-frame chain rule again: the list writes the default
        // selection, the details pane renders it in the same frame.
        app.add_systems(
            Update,
            (
                poll_lesson_media,
                refresh_training_list.run_if(training_list_dirty),
                refresh_training_details.run_if(training_details_dirty),
                advance_lesson_loops,
                sync_menu_aside,
            )
                .chain()
                .run_if(in_state(GameStates::MainMenu)),
        );
        app.add_observer(on_menu_button_activate);
        app.add_systems(Update, play_menu_focus_cue.in_set(MenuCueSystems));
        app.add_systems(
            OnEnter(GameStates::Playing),
            start_new_game_scenario.run_if(resource_equals(GameMode::NewGame)),
        );

        // Update systems keep running while paused - pausing Time<Virtual>
        // zeroes deltas, it does not stop schedules - which is exactly what lets
        // the overlay stay interactive.
        // The pause axis, in the order its decisions are made: the player's own
        // gesture, then the window's, then the panel that mirrors whatever they
        // settled on. Ordered rather than left to chance because all three read
        // and write the same `NextState` in one frame - a panel reconciled
        // before the gesture that opened it would appear a frame late, and a
        // focus pause that landed after it would find it already built.
        app.add_systems(
            Update,
            (toggle_pause, pause_on_focus_loss, reconcile_pause_overlay)
                .chain()
                .run_if(in_state(GameStates::Playing)),
        );
        // The focus policy this process runs under. Inserted rather than
        // init'd so a range (or the editor) can state its own.
        app.init_resource::<FocusPause>();
        // The command shell opens over every surface, so its key is NOT gated
        // on Playing the way the pause overlay's is. It IS gated on the CRT
        // existing: a menu-only rig has no monitor to open. And on Normal input
        // mode: in a focused text field - the editor's inspector, a rename - a
        // `:` is a character the player is typing, not a gesture.
        app.add_systems(
            Update,
            open_command_shell.run_if(
                resource_exists::<NovaOsTerminal>
                    .and_then(resource_exists::<NovaOsCloseTransition>)
                    .and_then(in_input_mode(InputMode::Normal)),
            ),
        );
        app.add_systems(
            OnEnter(PauseStates::Paused),
            (hold_clocks_for_pause_menu, release_cursor),
        );
        app.add_systems(
            OnExit(PauseStates::Paused),
            (release_clocks_for_pause_menu, restore_cursor),
        );
        // The NOVA OS is a third variant on the same clock-freeze axis, but
        // WITHOUT `setup_pause_ui` - it draws its own surface in nova_gameplay's
        // HUD. `:` opens it over the pause menu too, so `Paused <-> NovaOs` is a
        // live transition: the `Paused` hooks run on the way in and again on the
        // way back, rebuilding the overlay the CRT covered.
        app.add_systems(
            OnEnter(PauseStates::NovaOs),
            (hold_clocks_for_terminal, release_cursor),
        );
        app.add_systems(
            OnExit(PauseStates::NovaOs),
            (release_clocks_for_terminal, restore_cursor),
        );
        // Leaving gameplay ENDS the scenario, whatever the exit path and
        // whichever screen comes next. The teardown used to ride the menu's own
        // backdrop load, which made it a property of where the player landed
        // rather than of what they left - and the content restart below does
        // not land in the menu at all.
        app.add_systems(
            OnExit(GameStates::Playing),
            (force_unpause, end_gameplay_scenario),
        );
        // The message this plugin writes below. `nova_assets` owns it and adds
        // it too, which is a no-op the second time - declared here so a rig
        // that stands the menu up without the content pipeline still runs.
        app.add_message::<ReloadContent>();
        // Coming back to the menu is where the game catches up with what is on
        // disk. A scenario just played, or a document just built and saved, may
        // have written content the merge has no reason to notice - and the menu
        // is where a player goes looking for it. The restart is what makes the
        // Scenarios picker, the campaign list and the ship catalog agree.
        app.add_systems(
            OnExit(GameStates::Playing),
            |mut reload: MessageWriter<ReloadContent>| {
                reload.write(ReloadContent);
            },
        );
        // ...and the restart goes THROUGH the loading screen, not through the
        // menu. See `restart_through_loading`.
        app.add_systems(
            StateTransition,
            restart_through_loading
                .before(StateTransitionSystems::DependentTransitions)
                .run_if(resource_exists::<GameAssets>),
        );
        app.add_systems(
            PostUpdate,
            keep_frozen_cursor_released.run_if(in_state(GameStates::Playing)),
        );

        // `resource_exists`-gated - headless rigs without the scenario
        // loader have no CurrentOutcome.
        app.add_systems(
            Update,
            (
                sync_outcome_overlay,
                sync_outcome_cursor,
                sync_outcome_pause,
                auto_advance_outcome,
            )
                .run_if(in_state(GameStates::Playing))
                .run_if(resource_exists::<CurrentOutcome>),
        );
        // Playing-only - the menu's backdrop draw filters broken scenarios
        // out of the pick instead of reporting them.
        app.add_systems(
            Update,
            (sync_start_failure_overlay, sync_start_failure_cursor)
                .run_if(in_state(GameStates::Playing))
                .run_if(resource_exists::<ScenarioStartFailure>),
        );
        // The loader plugin also inits this; repeated here so menu-only
        // rigs do not panic on the OnEnter clear.
        app.init_resource::<ScenarioStartFailure>();
        app.add_systems(OnEnter(GameStates::MainMenu), clear_start_failure);
        app.add_observer(regrab_cursor_on_player_spawn);
    }
}

/// End the scenario the player is leaving.
///
/// `OnExit(GameStates::Playing)` is the one gate every way out passes through -
/// the pause menu's Back, the editor's File menu, the outcome frame's Main
/// Menu, the scenario-advance key - so the teardown belongs here rather than on
/// whatever screen happens to be next. Before this, the menu's backdrop load
/// was the de-facto owner, and the two paths that do not load a backdrop (the
/// no-clean-backdrop fallback, and the content restart that goes straight to
/// the loading screen) left gameplay simulating behind the next screen.
fn end_gameplay_scenario(mut commands: Commands) {
    commands.trigger(UnloadScenario);
}

/// Send a departure from gameplay through the LOADING screen instead of through
/// the menu.
///
/// The reload itself is unconditional and stays that way (see the
/// `OnExit(Playing)` writer above): leaving the editor or a scenario is where
/// the game catches up with what is on disk. What this removes is the disposable
/// menu in the middle of it. `Playing -> MainMenu` used to build the whole front
/// door - the menu panel, its UI camera, a randomly drawn ambience backdrop and
/// the scenario load behind it - one frame before `restart_for_content` threw it
/// all away and started the boot load. The player saw the flash; the machine
/// paid for a scenario nobody watched.
///
/// Rewriting the pending state rather than changing the four call sites keeps
/// "back to the front door" as what each of them means, and keeps the reload
/// policy in the one crate that owns it. `boot_into_the_game` (nova_core) puts
/// the player in the menu when the load finishes, so the destination is
/// unchanged - only the route is.
///
/// Gated on [`GameAssets`]: an app with no content pipeline has no restart to
/// come back from, and redirecting it into `Loading` would strand it there.
fn restart_through_loading(
    current: Res<State<GameStates>>,
    mut next: ResMut<NextState<GameStates>>,
) {
    if *current.get() != GameStates::Playing {
        return;
    }
    if !matches!(*next, NextState::Pending(GameStates::MainMenu)) {
        return;
    }
    *next = NextState::Pending(GameStates::Loading);
}
