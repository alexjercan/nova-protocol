//! The spaceship editor: a scene where you build a ship out of sections and then
//! hand it off to a scenario simulation.
//!
//! Structure (task 20260714-204219 split the old single file):
//! - `config`    - the build-state resources + preview markers
//! - `placement` - creating a ship + the pointer place/preview/delete observers
//! - `keybind`   - section keybind chips + click-to-rebind
//! - `scenario`  - the player-only asteroid+planetoid scene handed off on Play
//! - `ui`        - the wiki-style rail + component drawer + tooltip

use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

mod config;
mod keybind;
mod placement;
mod scenario;
mod ui;

use config::{PlayerSpaceshipConfig, SectionChoice};
use keybind::{
    apply_section_rebind, position_section_keybind_labels, sync_section_keybind_labels,
    EditorRebind,
};
use nova_ui::widget::button_on_setting;
use placement::{
    on_click_spaceship_section, on_hover_spaceship_section, on_move_spaceship_section,
    on_out_spaceship_section,
};
use scenario::setup_scenario;
use ui::{scroll_editor_panel, setup_editor_scene};

pub mod prelude {
    pub use super::NovaEditorPlugin;
}

/// The spaceship editor plugin.
///
/// `nova_core` adds this as its default "game" plugin when no custom game plugins are
/// supplied (see `AppBuilder`). Examples that provide their own scenario opt out of it.
pub struct NovaEditorPlugin;

impl Plugin for NovaEditorPlugin {
    fn build(&self, app: &mut App) {
        editor_plugin(app);
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub(crate) enum ExampleStates {
    #[default]
    Loading,
    Editor,
    Scenario,
}

fn editor_plugin(app: &mut App) {
    app.init_state::<ExampleStates>();
    app.insert_resource(SectionChoice::None);
    app.insert_resource(PlayerSpaceshipConfig::default());
    app.init_resource::<EditorRebind>();

    // The editor is the Sandbox game. When the main menu fronts the app it hands
    // off to Playing with GameMode set: Sandbox enters the editor, NewGame goes
    // straight to the Scenario state. The menu owns the NewGame scenario load;
    // setup_scenario below stays Sandbox-only so the two do not both fire.
    // GameMode defaults to Sandbox (NovaGameplayPlugin), so menu-less apps
    // behave as before. (The spaceship input/section sets are gated on
    // scenario-liveness by nova_scenario, not on these states - see the note
    // at the end of this function.)
    app.add_systems(
        OnEnter(GameStates::Playing),
        (
            |mode: Res<GameMode>, mut game_state: ResMut<NextState<ExampleStates>>| {
                game_state.set(match *mode {
                    GameMode::Sandbox => ExampleStates::Editor,
                    GameMode::NewGame => ExampleStates::Scenario,
                });
            },
        ),
    );

    // Leaving Playing (the pause menu's Back to Main Menu) must tear the
    // editor scene down: DespawnOnExit(ExampleStates::...) entities only
    // despawn when the inner state actually changes, and a later Sandbox
    // entry must start fresh in Editor, not resume a stale Scenario.
    app.add_systems(
        OnExit(GameStates::Playing),
        |mut game_state: ResMut<NextState<ExampleStates>>| {
            game_state.set(ExampleStates::Loading);
        },
    );

    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (
            setup_grab_cursor_scenario,
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
    );
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        (
            setup_editor_scene,
            setup_grab_cursor_editor,
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        (
            // Sandbox-only: in NewGame the menu already loaded its scenario and a
            // second LoadScenario here would tear it straight back down.
            setup_scenario.run_if(resource_equals(GameMode::Sandbox)),
            |mut selection: ResMut<SectionChoice>| {
                *selection = SectionChoice::None;
            },
        ),
    );

    // Button colours, selection highlight, and the component tooltip.
    ui::register(app);
    // Component cards + rail tools set the placement tool via their ButtonValue.
    app.add_observer(button_on_setting::<SectionChoice>);

    app.add_observer(on_click_spaceship_section)
        .add_observer(on_hover_spaceship_section)
        .add_observer(on_move_spaceship_section)
        .add_observer(on_out_spaceship_section);

    // Editor section keybind labels + click-to-rebind (task 20260712-163912).
    // A stale rebind must not survive a scene change, so clear it on every
    // state entry (like SectionChoice).
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut rebind: ResMut<EditorRebind>| rebind.target = None,
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        |mut rebind: ResMut<EditorRebind>| rebind.target = None,
    );
    app.add_systems(
        Update,
        (
            sync_section_keybind_labels,
            apply_section_rebind,
            position_section_keybind_labels,
            scroll_editor_panel,
        )
            .run_if(in_state(ExampleStates::Editor)),
    );

    app.add_systems(
        Update,
        lock_on_left_click
            .run_if(in_state(ExampleStates::Editor).and_then(in_state(PauseStates::Unpaused))),
    );
    app.add_systems(
        Update,
        // F1-to-editor is demo/sandbox furniture: campaigns (NewGame) must
        // not offer an editor escape (task 20260711-203805); the pause menu
        // is the sanctioned way out.
        switch_scene_editor
            .run_if(in_state(ExampleStates::Scenario).and_then(resource_equals(GameMode::Sandbox))),
    );

    // The spaceship input/section system sets are deliberately NOT gated
    // here anymore: nova_scenario's ScenarioLoaderPlugin gates them on
    // scenario-liveness (task 20260711-212519). The editor's build-mode
    // preview stays inert because the Editor state never has a scenario
    // loaded - initial entry loads nothing and F1 triggers UnloadScenario.
}

fn switch_scene_editor(
    keys: Res<ButtonInput<KeyCode>>,
    gamepad: Option<Res<ButtonInput<GamepadButton>>>,
    mut state: ResMut<NextState<ExampleStates>>,
    mut commands: Commands,
) {
    let pad = gamepad
        .map(|g| g.just_pressed(GamepadButton::LeftThumb))
        .unwrap_or(false);
    if keys.just_pressed(KeyCode::F1) || pad {
        debug!("switch_scene_editor: F1/L3 pressed, switching to Editor state.");
        state.set(ExampleStates::Editor);
        commands.trigger(UnloadScenario);
    }
}

fn setup_grab_cursor_scenario(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if cfg!(not(feature = "debug")) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_cursor_options.visible = false;
    }
}

fn setup_grab_cursor_editor(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut primary_cursor_options = primary_cursor_options.into_inner();
    primary_cursor_options.grab_mode = CursorGrabMode::None;
    primary_cursor_options.visible = true;
}

fn lock_on_left_click(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    if mouse.just_pressed(MouseButton::Right) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_cursor_options.visible = false;
    } else if mouse.just_released(MouseButton::Right) {
        let mut primary_cursor_options = primary_cursor_options.into_inner();
        primary_cursor_options.grab_mode = CursorGrabMode::None;
        primary_cursor_options.visible = true;
    }
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// Counts LoadScenario triggers so the NewGame test can prove the editor
    /// stayed out of the menu's scenario load (review R1.1).
    #[derive(Resource, Default)]
    struct EditorScenarioLoads(usize);

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_resource::<GameMode>();
        // switch_scene_editor polls the keyboard while in the Scenario state.
        app.init_resource::<ButtonInput<KeyCode>>();
        editor_plugin(&mut app);
        app.init_resource::<EditorScenarioLoads>();
        app.add_observer(
            |_: On<LoadScenario>, mut loads: ResMut<EditorScenarioLoads>| {
                loads.0 += 1;
            },
        );
        app
    }

    /// Regression for review R1.1: in NewGame mode the editor must still enter
    /// its Scenario state (cursor grab and the F1/despawn furniture key on it),
    /// while leaving the scenario load itself to the menu. (Flyability itself
    /// is no longer tied to this state: the spaceship sets are gated on
    /// scenario-liveness by nova_scenario, task 20260711-212519.)
    #[test]
    fn new_game_enters_scenario_state_without_loading_the_editor_scenario() {
        let mut app = app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();

        // Delivery guard: the handoff actually reached the Scenario state.
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        // The editor did not fire its own sandbox scenario on top of the menu's.
        assert_eq!(app.world().resource::<EditorScenarioLoads>().0, 0);
    }

    /// Leaving Playing (the pause menu's Back to Main Menu) resets the
    /// editor's inner state so DespawnOnExit scene entities are torn down
    /// and the next Sandbox entry starts fresh (task 20260711-185156).
    #[test]
    fn leaving_playing_resets_the_inner_state() {
        let mut app = app();
        // NewGame routes to Scenario, which applies safely headless (the
        // editor's own scenario load is Sandbox-gated).
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Loading,
            "inner state must reset when Playing is left"
        );
    }

    /// F1 back-to-editor is Sandbox-only (task 20260711-203805): in NewGame
    /// the same press must do nothing. Delivery guard: the identical press in
    /// Sandbox mode queues the Editor state and unloads the scenario, proving
    /// the stimulus path works.
    #[test]
    fn f1_returns_to_editor_only_in_sandbox_mode() {
        let make_app = app;
        // NewGame: F1 must be inert.
        let mut app = make_app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario,
            "F1 must not leave the scenario in NewGame"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "no editor scenario churn in NewGame"
        );

        // Sandbox: the same press flips to Editor. Enter Playing via NewGame
        // (going through Editor would run setup_editor_scene, which needs
        // GameAssets headless), then flip the mode - the gate reads the
        // resource at press time. Assert the queued target without applying
        // it, for the same reason.
        let mut app = make_app();
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(
            queued,
            Some(ExampleStates::Editor),
            "the same press must work in Sandbox (delivery guard)"
        );
    }

    /// The scenario-liveness gate (nova_scenario, task 20260711-212519)
    /// keeps the editor's build-mode preview inert only if the Editor state
    /// never has a live scenario. This exercises the one route that enters
    /// Editor FROM a live scenario - F1 - and asserts the same press
    /// unloads it, with the editor firing no scenario load of its own
    /// anywhere on the route.
    #[test]
    fn editor_state_never_keeps_a_scenario_live() {
        #[derive(Resource, Default)]
        struct Unloads(usize);

        let mut app = app();
        app.init_resource::<Unloads>();
        app.add_observer(|_: On<UnloadScenario>, mut unloads: ResMut<Unloads>| {
            unloads.0 += 1;
        });

        // Enter Playing via NewGame (Editor's OnEnter scene setup needs
        // GameAssets headless), then flip to Sandbox so F1 is armed - the
        // gate reads the resource at press time.
        app.insert_resource(GameMode::NewGame);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        app.update();
        app.update();
        assert_eq!(
            *app.world().resource::<State<ExampleStates>>().get(),
            ExampleStates::Scenario
        );
        assert_eq!(app.world().resource::<Unloads>().0, 0);

        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F1);
        app.update();
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(
            queued,
            Some(ExampleStates::Editor),
            "delivery guard: the press was seen and Editor is queued"
        );
        assert_eq!(
            app.world().resource::<Unloads>().0,
            1,
            "the same press must unload the scenario"
        );
        assert_eq!(
            app.world().resource::<EditorScenarioLoads>().0,
            0,
            "the editor fired no scenario load of its own on this route"
        );
    }

    /// Sandbox mode heads for the editor scene, exactly as before the menu. The
    /// full editor path (scene setup needs GameAssets) is covered end to end by
    /// the 09_editor smoke run; this pins just the state routing.
    #[test]
    fn sandbox_heads_to_editor_state() {
        let mut app = app();
        app.insert_resource(GameMode::Sandbox);
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::Playing);
        // A single transition step: entering Editor would run setup_editor_scene,
        // which needs GameAssets, so only assert the queued target.
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(queued, None, "nothing queued before Playing is applied");
        app.world_mut()
            .run_schedule(bevy::state::state::StateTransition);
        let queued = match app.world().resource::<NextState<ExampleStates>>() {
            NextState::Pending(s) => Some(s.clone()),
            _ => None,
        };
        assert_eq!(queued, Some(ExampleStates::Editor));
    }
}
