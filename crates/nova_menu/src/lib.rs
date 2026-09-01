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

use bevy::prelude::*;
use nova_assets::prelude::ReloadContent;
use nova_gameplay::prelude::*;
use nova_hud::prelude::HudVisibility;
use nova_os_ui::prelude::NovaOsMonitorSettings;
use nova_scenario::prelude::{CurrentOutcome, ScenarioStartFailure};
use nova_ui::{prelude::UiSkin, widget::button_on_setting};

/// Glob-import surface: `use nova_menu::prelude::*` brings [`NovaMenuPlugin`]
/// and [`NewGameScenario`] into scope.
pub mod prelude {
    pub use super::{ambience::MENU_BACKDROP_ENV, NewGameScenario, NovaMenuPlugin};
}

mod ambience;
mod menu_ui;
mod mods;
mod outcome;
mod pause;
mod portal;
mod scenarios;
mod settings;
mod settings_store;
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
use outcome::{
    auto_advance_outcome, clear_start_failure, regrab_cursor_on_player_spawn, sync_outcome_cursor,
    sync_outcome_overlay, sync_outcome_pause, sync_start_failure_overlay,
};
use pause::{
    force_unpause, keep_frozen_cursor_released, pause_clocks, release_cursor, restore_cursor,
    setup_pause_ui, toggle_pause, unpause_clocks,
};
use portal::{drive_update_choreography, UpdateRequested};
pub use scenarios::NewGameScenario;
use scenarios::{
    poll_scenario_thumbnail, refresh_scenario_details, refresh_scenarios_list,
    scenario_details_dirty, scenarios_list_dirty, CollapsedCampaigns, PendingScenarioThumbnail,
    SelectedScenarioId,
};
use settings::{
    apply_settings_rebind, flush_settings_on_exit, load_persisted_settings,
    on_volume_slider_change, persist_settings_on_change, refresh_settings_tab, settings_tab_dirty,
    sync_volume_slider, PendingRebind, PendingSettingsSave, SettingsActiveTab,
    SettingsControlsGroup, WindowModeSetting,
};
use widgets::on_menu_button_activate;

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
        // HudVisibility, the settings resources and NovaOsMonitorSettings
        // are owned by other plugins in the assembled app. init_resource is
        // idempotent, so initing them here too lets the menu plugin stand alone
        // in slim and headless-test apps.
        app.init_resource::<HudVisibility>();
        app.init_resource::<ModsActiveTab>();
        app.init_resource::<SelectedModId>();
        app.init_resource::<SelectedScenarioId>();
        app.init_resource::<NewGameScenario>();
        app.init_resource::<PendingScenarioThumbnail>();
        app.init_resource::<ambience::MenuCameraMemory>();
        app.init_resource::<CollapsedCampaigns>();
        app.init_resource::<UpdateRequested>();
        // Ungated by menu state on purpose - an update started from the
        // menu must complete even if the player closes it mid-flight.
        app.add_systems(Update, drive_update_choreography);
        // The editor and gameplay want the same app-global UI wiring; whoever
        // gets there first adds it.
        if !app.is_plugin_added::<nova_ui::NovaUiPlugin>() {
            app.add_plugins(nova_ui::NovaUiPlugin);
        }

        app.init_resource::<MasterVolume>();
        app.init_resource::<InterfaceVolume>();
        app.init_resource::<WorldVolume>();
        app.init_resource::<MusicVolume>();
        app.init_resource::<GraphicsQuality>();
        // `NovaUiPlugin` above inits `UiSkin` transitively; repeat it here
        // so the invariant survives a future reorder.
        app.init_resource::<UiSkin>();
        app.init_resource::<NovaOsMonitorSettings>();
        app.init_resource::<WindowModeSetting>();
        app.init_resource::<SettingsActiveTab>();
        app.init_resource::<SettingsControlsGroup>();
        app.init_resource::<PendingRebind>();
        app.add_observer(on_volume_slider_change);
        app.add_observer(button_on_setting::<GraphicsQuality>);
        app.add_observer(button_on_setting::<UiSkin>);
        app.add_observer(button_on_setting::<WindowModeSetting>);
        app.add_systems(Update, sync_volume_slider);
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
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, settings::apply_window_mode);
        app.add_systems(Startup, load_persisted_settings);
        app.init_resource::<PendingSettingsSave>();
        app.add_systems(Update, persist_settings_on_change);
        app.add_systems(Last, flush_settings_on_exit);

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
        app.add_observer(on_menu_button_activate);
        app.add_systems(
            OnEnter(GameStates::Playing),
            start_new_game_scenario.run_if(resource_equals(GameMode::NewGame)),
        );

        // Update systems keep running while paused - pausing Time<Virtual>
        // zeroes deltas, it does not stop schedules - which is exactly what lets
        // the overlay stay interactive.
        app.add_systems(Update, toggle_pause.run_if(in_state(GameStates::Playing)));
        app.add_systems(
            OnEnter(PauseStates::Paused),
            (pause_clocks, release_cursor, setup_pause_ui),
        );
        app.add_systems(
            OnExit(PauseStates::Paused),
            (unpause_clocks, restore_cursor),
        );
        // The NOVA OS is a third variant on the same clock-freeze axis, but
        // WITHOUT `setup_pause_ui` - it draws its own surface in nova_gameplay's
        // HUD. Only ever entered from / exited to `Unpaused`, so these never race
        // the `Paused` hooks.
        app.add_systems(OnEnter(PauseStates::NovaOs), (pause_clocks, release_cursor));
        app.add_systems(
            OnExit(PauseStates::NovaOs),
            (unpause_clocks, restore_cursor),
        );
        app.add_systems(OnExit(GameStates::Playing), force_unpause);
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
            sync_start_failure_overlay
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
