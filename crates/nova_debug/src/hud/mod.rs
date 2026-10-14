//! The Nova Debug window.
//!
//! F11 opens one egui window, "Nova Debug", over whatever is on screen. In
//! flight (`Playing` and [`PauseStates::Unpaused`]) it also enters
//! [`PauseStates::Debug`]: the clocks stop under [`FreezeOwner::DebugHud`],
//! every `is_frozen` guard takes flight input off, and the cursor is free.
//! Over the pause menu, the TAB interface, the command modal or the main menu
//! it opens without changing the state. F11 or Escape closes it; Escape is
//! consumed here, so the surface under the window does not also back out.
//!
//! The cursor is not this module's: `nova_menu`'s `SurfaceCursorPlugin` frees
//! it in every frozen state, `Debug` included, and gives flight its lock back
//! on the way out.
//!
//! The window's sections: the overlay switches ([`overlays`]), the cheat
//! buttons and a prompt over the game's command shell ([`console`]), the
//! world's top-level entities by sector and by type ([`entities`]), and the
//! raw world (every entity, resource and asset).

mod console;
mod entities;
pub mod overlays;

use bevy::{input::InputSystems, prelude::*};
use bevy_inspector_egui::{
    bevy_egui::{EguiContext, EguiPlugin, EguiPrimaryContextPass, PrimaryEguiContext},
    bevy_inspector, egui,
};
use nova_gameplay::{
    prelude::{Clocks, FreezeOwner},
    GameStates, PauseStates,
};
use nova_ui::prelude::{InputMode, InputModeSystems};

use crate::DEBUG_TOGGLE_KEYCODE;

/// Whether the Nova Debug window is open. Closed at boot.
#[derive(Resource, Debug, Default)]
pub struct DebugHud {
    pub(crate) open: bool,
    entities_tab: entities::EntitiesTab,
    /// The Cheats section's `<item> <n>` field.
    item_give: String,
}

impl DebugHud {
    /// Whether the window is open.
    pub fn is_open(&self) -> bool {
        self.open
    }
}

/// Where the debug window runs in a frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DebugHudSystems {
    /// Claim the keyboard while the window takes text. `PreUpdate`, before
    /// `nova_ui`'s `InputModeSystems`.
    Claim,
    /// Read F11 and Escape and keep [`PauseStates::Debug`] in step with the
    /// window. `PreUpdate`, after bevy's `InputSystems`: Escape must be
    /// consumed before the pause menu, the interface and the editor read it
    /// in `Update`, and none of them has a set to order against.
    Toggle,
    /// Draw the window. `EguiPrimaryContextPass`.
    Ui,
}

/// The Nova Debug window, its toggle and its freeze.
///
/// The window itself needs `bevy_egui`, so [`DebugHudPlugin`] draws it only
/// when `EguiPlugin` was added first ([`crate::inspector`]); the toggle and the
/// freeze run in every app.
pub struct DebugHudPlugin;

impl Plugin for DebugHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugHud>();
        app.configure_sets(
            PreUpdate,
            (
                DebugHudSystems::Claim.before(InputModeSystems),
                DebugHudSystems::Toggle.after(InputSystems),
            ),
        );
        app.add_systems(
            PreUpdate,
            (
                // An app without `nova_ui`'s arbiter has no input modes to
                // claim.
                console::claim_keyboard_for_debug_hud
                    .run_if(resource_exists::<InputMode>)
                    .in_set(DebugHudSystems::Claim),
                toggle_debug_hud.in_set(DebugHudSystems::Toggle),
            ),
        );
        app.add_systems(OnEnter(PauseStates::Debug), hold_clocks_for_debug_hud);
        app.add_systems(OnExit(PauseStates::Debug), release_clocks_for_debug_hud);
        if app.is_plugin_added::<EguiPlugin>() {
            app.add_systems(
                EguiPrimaryContextPass,
                debug_hud_ui
                    .run_if(|hud: Res<DebugHud>| hud.open)
                    .in_set(DebugHudSystems::Ui),
            );
        }
    }
}

/// F11 opens and closes the window, Escape closes it, and the pause state
/// follows the window.
///
/// The state rule is one invariant rather than a pair of transitions: an open
/// window over flight holds [`PauseStates::Debug`], and a closed one never
/// does. That also covers the ways out the window does not see - the command
/// modal opened over it and closed back to `Debug` after Escape shut the
/// window, or a Resume clicked on a pause menu the window was opened over.
fn toggle_debug_hud(
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut hud: ResMut<DebugHud>,
    game: Option<Res<State<GameStates>>>,
    pause: Option<Res<State<PauseStates>>>,
    next: Option<ResMut<NextState<PauseStates>>>,
) {
    let f11 = keys.just_pressed(DEBUG_TOGGLE_KEYCODE);
    let escape = hud.open && keys.just_pressed(KeyCode::Escape);
    if escape {
        keys.clear_just_pressed(KeyCode::Escape);
    }
    if f11 || escape {
        hud.open = !hud.open;
    }
    let (Some(pause), Some(mut next)) = (pause, next) else {
        return;
    };
    let playing = game.is_some_and(|game| *game.get() == GameStates::Playing);
    match (hud.open, pause.get()) {
        (true, PauseStates::Unpaused) if playing => next.set(PauseStates::Debug),
        (false, PauseStates::Debug) => next.set(PauseStates::Unpaused),
        _ => {}
    }
}

/// Close the window. The pause state follows on the next toggle pass.
pub(crate) fn close_debug_hud(world: &mut World) {
    if let Some(mut hud) = world.get_resource_mut::<DebugHud>() {
        hud.open = false;
    }
    if world
        .get_resource::<State<PauseStates>>()
        .is_some_and(|pause| *pause.get() == PauseStates::Debug)
    {
        if let Some(mut next) = world.get_resource_mut::<NextState<PauseStates>>() {
            next.set(PauseStates::Unpaused);
        }
    }
}

fn hold_clocks_for_debug_hud(mut clocks: Clocks) {
    clocks.hold(FreezeOwner::DebugHud);
}

fn release_clocks_for_debug_hud(mut clocks: Clocks) {
    clocks.release(FreezeOwner::DebugHud);
}

/// Draw the window on the primary egui context. A frame with no context (no
/// window camera yet) draws nothing.
fn debug_hud_ui(world: &mut World) {
    let Ok(context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut context = context.clone();
    egui::Window::new("Nova Debug")
        .default_width(440.0)
        .default_height(620.0)
        .show(context.get_mut(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::CollapsingHeader::new("Overlays")
                    .default_open(true)
                    .show(ui, |ui| overlays::overlays_ui(world, ui));
                egui::CollapsingHeader::new("Cheats").show(ui, |ui| {
                    let mut item = std::mem::take(&mut world.resource_mut::<DebugHud>().item_give);
                    console::cheats_ui(world, ui, &mut item);
                    world.resource_mut::<DebugHud>().item_give = item;
                });
                egui::CollapsingHeader::new("Command")
                    .show(ui, |ui| console::command_ui(world, ui));
                egui::CollapsingHeader::new("Entities").show(ui, |ui| {
                    let mut tab = world.resource::<DebugHud>().entities_tab;
                    entities::entities_ui(world, ui, &mut tab);
                    world.resource_mut::<DebugHud>().entities_tab = tab;
                });
                egui::CollapsingHeader::new("Raw").show(ui, |ui| raw_ui(world, ui));
            });
        });
}

/// Every entity, resource and asset, through `bevy-inspector-egui`. Each list
/// is built only while its header is open.
fn raw_ui(world: &mut World, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("All entities")
        .id_salt("nova_debug_raw_entities")
        .show(ui, |ui| bevy_inspector::ui_for_entities(world, ui));
    egui::CollapsingHeader::new("Resources")
        .id_salt("nova_debug_raw_resources")
        .show(ui, |ui| bevy_inspector::ui_for_resources(world, ui));
    egui::CollapsingHeader::new("Assets")
        .id_salt("nova_debug_raw_assets")
        .show(ui, |ui| bevy_inspector::ui_for_all_assets(world, ui));
}

#[cfg(test)]
mod tests {
    use bevy::{
        state::app::StatesPlugin,
        window::{CursorGrabMode, CursorOptions, PrimaryWindow},
    };
    use nova_gameplay::prelude::{ClockFreeze, PlayerSpaceshipMarker};
    use nova_menu::prelude::SurfaceCursorPlugin;

    use super::*;

    /// The debug window beside the game's one surface cursor rule, over a
    /// flying player ship whose cursor flight has locked.
    fn rig(game: GameStates) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(game);
        app.init_state::<PauseStates>();
        app.init_resource::<ClockFreeze>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_plugins((DebugHudPlugin, SurfaceCursorPlugin));
        app.world_mut().spawn((
            PrimaryWindow,
            CursorOptions {
                visible: false,
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut().spawn(PlayerSpaceshipMarker);
        app.update();
        app
    }

    /// Whether the cursor is free and visible.
    fn cursor_free(app: &mut App) -> bool {
        let cursor = app
            .world_mut()
            .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        cursor.visible && cursor.grab_mode == CursorGrabMode::None
    }

    /// Tap `key` for one frame. No `InputPlugin` clears the edge, so the tap
    /// resets the input itself.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
    }

    fn pause(app: &App) -> PauseStates {
        *app.world().resource::<State<PauseStates>>().get()
    }

    fn open(app: &App) -> bool {
        app.world().resource::<DebugHud>().is_open()
    }

    fn clocks_paused(app: &App) -> bool {
        app.world().resource::<Time<Virtual>>().is_paused()
    }

    /// F11 in flight opens the window and freezes the world behind it - the
    /// clocks stop and the state is a frozen one, which is what every flight
    /// input guard reads - and F11 again gives the world back.
    #[test]
    fn f11_in_flight_freezes_the_world_and_f11_again_resumes_it() {
        let mut app = rig(GameStates::Playing);

        tap(&mut app, DEBUG_TOGGLE_KEYCODE);
        assert!(open(&app));
        assert_eq!(pause(&app), PauseStates::Debug);
        assert!(pause(&app).is_frozen(), "flight input reads is_frozen");
        assert!(clocks_paused(&app));
        assert!(app
            .world()
            .resource::<ClockFreeze>()
            .is_held_by(FreezeOwner::DebugHud));
        assert!(cursor_free(&mut app), "the window owns the pointer");

        tap(&mut app, DEBUG_TOGGLE_KEYCODE);
        assert!(!open(&app));
        assert_eq!(pause(&app), PauseStates::Unpaused);
        assert!(!clocks_paused(&app));
        assert!(!cursor_free(&mut app), "flight takes its lock back");
    }

    /// TAB over flight in a debug build frees the cursor and nothing in the
    /// debug layer locks it again: the surface rule is the one owner of the
    /// cursor in every frozen state.
    #[test]
    fn tab_over_flight_leaves_the_cursor_free_and_visible() {
        let mut app = rig(GameStates::Playing);
        assert!(!cursor_free(&mut app), "flight starts locked");
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Interface);
        for _ in 0..3 {
            app.update();
            assert!(cursor_free(&mut app));
        }
    }

    /// Over the TAB interface the window opens without taking the state, and
    /// Escape closes the window alone: the key is consumed before the
    /// interface's own back-out reads it.
    #[test]
    fn f11_over_the_interface_keeps_the_interface_and_escape_closes_only_the_window() {
        let mut app = rig(GameStates::Playing);
        app.world_mut()
            .resource_mut::<NextState<PauseStates>>()
            .set(PauseStates::Interface);
        app.update();

        tap(&mut app, DEBUG_TOGGLE_KEYCODE);
        assert!(open(&app));
        assert_eq!(pause(&app), PauseStates::Interface);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(!open(&app));
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Escape),
            "the window consumed the Escape edge"
        );
        assert_eq!(pause(&app), PauseStates::Interface);
    }
}
