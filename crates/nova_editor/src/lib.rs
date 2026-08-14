//! The spaceship editor: a scene where you build a ship out of sections and then
//! hand it off to a scenario simulation.
//!
//! Structure:
//! - `config`    - the build-state resources + preview markers
//! - `preview`   - the one place a section config becomes preview entities
//! - `placement` - creating a ship + the pointer place/preview/delete observers
//! - `keybind`   - section keybind chips + click-to-rebind
//! - `gallery`   - the full-screen parts browser that arms the placement tool
//! - `snap`      - where the armed prototype would land, and why not
//! - `scenario`  - the player-only asteroid+planetoid scene handed off on Play
//! - `ui`        - the wiki-style rail + component drawer + tooltip
#![warn(missing_docs)]

use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

mod config;
mod gallery;
mod keybind;
mod placement;
mod preview;
mod scenario;
mod snap;
mod ui;

use config::{PlacementPose, PlacementPreview, PlayerSpaceshipConfig, SectionChoice};
use keybind::{
    apply_section_rebind, hide_section_keybind_labels, position_section_keybind_labels,
    sync_section_keybind_labels, EditorRebind,
};
use nova_ui::widget::button_on_setting;
use placement::{
    cycle_placement_pose, draw_delete_target, draw_link_points, draw_ship_heading,
    on_click_spaceship_section, pick_section_under_pointer, rebuild_editor_preview_on_enter,
    sync_placement_ghost, sync_tool_selection, update_placement_preview, wheel_placement_pose,
};
use scenario::setup_scenario;
use ui::{setup_editor_scene, sync_key_legend};

/// Glob-import surface: `use nova_editor::prelude::*` brings [`NovaEditorPlugin`]
/// into scope.
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
    // Normally the gameplay plugin's; init'd here too so a menu-less rig (and
    // the tests below) still has one to write.
    app.init_resource::<EscapeOwner>();

    // Escape is a BACK key in the editor and a pause key only when there is
    // nothing to back out of. `PreUpdate` on purpose: `nova_menu`'s pause
    // toggle reads the answer in `Update`, and the two crates cannot order
    // against each other.
    app.add_systems(PreUpdate, declare_editor_escape_owner);
    app.add_systems(
        Update,
        escape_puts_down_the_armed_part
            // Same reason as the placement chain below: the Escape that closes
            // the gallery must not also put down the part it was holding.
            .before(gallery::gallery_keyboard)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
    );

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
            // The preview entities are DespawnOnExit(Editor) but the config
            // resource survives, so a second visit must rebuild the ship the
            // first visit built - Play spawns it either way.
            rebuild_editor_preview_on_enter,
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
    // The parts browser: its own state, overlay and 3D stage.
    gallery::register(app);
    // The rail tools set the placement tool via their ButtonValue.
    app.add_observer(button_on_setting::<SectionChoice>);

    app.add_observer(on_click_spaceship_section);

    // The placement ghost: solve once per frame, then show it. Both are gated
    // on the gallery being closed - it covers the build area, so nothing under
    // it is being pointed at.
    app.init_resource::<PlacementPose>();
    app.init_resource::<PlacementPreview>();
    app.add_systems(
        Update,
        (
            sync_tool_selection,
            sync_key_legend,
            pick_section_under_pointer,
            cycle_placement_pose,
            update_placement_preview,
            sync_placement_ghost,
            draw_link_points,
            draw_ship_heading,
            draw_delete_target,
        )
            .chain()
            // BEFORE the gallery's keyboard, which shares two keys with the
            // build view: Q takes a part in both, and Esc backs out of both. The
            // gallery answers first because it is on top, and running the build
            // view ahead of it means these read the gallery-open state the
            // gesture was AIMED at rather than the one it just changed - the
            // frame Q closes the gallery is not also a frame the pipette fires
            // on whatever the overlay was covering.
            .before(gallery::gallery_keyboard)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
    );
    // The wheel half of the pose control, split off so a headless rig with no
    // input plugin (and so no wheel message queue) still runs the rest.
    app.add_systems(
        Update,
        wheel_placement_pose
            .before(update_placement_preview)
            .run_if(resource_exists::<Messages<MouseWheel>>)
            .run_if(in_state(ExampleStates::Editor).and_then(not(gallery::gallery_open))),
    );
    // A fresh visit starts with the part's first socket, unrolled.
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut pose: ResMut<PlacementPose>| *pose = PlacementPose::default(),
    );

    // NOTE: a stale rebind must not survive a scene change, so clear it on
    // every state entry (like SectionChoice).
    app.add_systems(
        OnEnter(ExampleStates::Editor),
        |mut rebind: ResMut<EditorRebind>| *rebind = EditorRebind::default(),
    );
    app.add_systems(
        OnEnter(ExampleStates::Scenario),
        |mut rebind: ResMut<EditorRebind>| *rebind = EditorRebind::default(),
    );
    // NOTE: the rebind capture and the right-drag cursor grab are gated on the
    // gallery being CLOSED - while the overlay is up it owns the keyboard (a
    // filter keystroke would otherwise be captured as a section binding) and
    // the pointer.
    app.add_systems(
        Update,
        (
            sync_section_keybind_labels,
            apply_section_rebind.run_if(not(gallery::gallery_open)),
            position_section_keybind_labels.run_if(not(gallery::gallery_open)),
            // The gallery covers the ship the chips label, so they go off with
            // the rest of the editor's chrome while it is up.
            hide_section_keybind_labels.run_if(gallery::gallery_open),
        )
            .run_if(in_state(ExampleStates::Editor)),
    );

    app.add_systems(
        Update,
        lock_on_left_click
            .run_if(in_state(ExampleStates::Editor).and_then(in_state(PauseStates::Unpaused)))
            .run_if(not(gallery::gallery_open)),
    );
    app.add_systems(
        Update,
        // NOTE: F1-to-editor is demo/sandbox furniture - campaigns (NewGame)
        // must not offer an editor escape; the pause menu is the sanctioned way
        // out.
        switch_scene_editor
            .run_if(in_state(ExampleStates::Scenario).and_then(resource_equals(GameMode::Sandbox))),
    );

    // NOTE: the spaceship input/section system sets are deliberately NOT gated
    // here - nova_scenario's ScenarioLoaderPlugin gates them on
    // scenario-liveness. The editor's build-mode preview stays inert because the
    // Editor state never has a scenario loaded: initial entry loads nothing and
    // F1 triggers UnloadScenario.
}

/// Say whether the editor answers Escape itself this frame (see
/// [`EscapeOwner`]).
///
/// It does while there is something to back OUT of: the parts gallery is up,
/// a part is armed, or a rebind is waiting for a key. With none of those the
/// key falls through to the pause menu, which stays the sanctioned way out of
/// the editor. Written every frame, including the `false`: whoever claims the
/// key also has to release it.
fn declare_editor_escape_owner(
    editor: Res<State<ExampleStates>>,
    gallery: Res<gallery::GalleryState>,
    choice: Res<SectionChoice>,
    rebind: Res<EditorRebind>,
    mut owner: ResMut<EscapeOwner>,
) {
    let owned = *editor.get() == ExampleStates::Editor
        && (gallery.open || rebind.target.is_some() || *choice != SectionChoice::None);
    if owner.0 != owned {
        owner.0 = owned;
    }
}

/// Escape puts the armed part down. The gallery answers its own Escape while it
/// is up (see `gallery::input`), and a pending rebind cancels on Escape in
/// `keybind`; this is the last step of the same gesture, and it leaves the
/// editor in select mode rather than raising the pause overlay.
fn escape_puts_down_the_armed_part(
    keys: Res<ButtonInput<KeyCode>>,
    mut choice: ResMut<SectionChoice>,
) {
    if keys.just_pressed(KeyCode::Escape) && *choice != SectionChoice::None {
        *choice = SectionChoice::None;
    }
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

/// Hide and lock the cursor for flight (owner decision): unconditional,
/// debug builds included. The F11 debug inspector is an egui
/// panel that needs a pointer, so it hands the cursor back while it is up via
/// nova_debug's `sync_inspector_cursor`; menus/pause/outcome free it through
/// their own transitions.
fn setup_grab_cursor_scenario(
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut primary_cursor_options = primary_cursor_options.into_inner();
    primary_cursor_options.grab_mode = CursorGrabMode::Locked;
    primary_cursor_options.visible = false;
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
    use bevy::{ecs::system::RunSystemOnce, state::app::StatesPlugin};

    use super::*;

    /// Escape is the editor's only while it HAS a back step. With nothing armed
    /// and no gallery up the key falls through to the pause menu, which is the
    /// sanctioned way out of the editor - and in flight the pause menu owns it
    /// outright.
    #[test]
    fn the_editor_claims_escape_only_when_it_has_a_back_step() {
        let cases = [
            (
                ExampleStates::Editor,
                false,
                SectionChoice::None,
                false,
                false,
            ),
            (
                ExampleStates::Editor,
                true,
                SectionChoice::None,
                false,
                true,
            ),
            (
                ExampleStates::Editor,
                false,
                SectionChoice::Section("hull".to_string()),
                false,
                true,
            ),
            (
                ExampleStates::Editor,
                false,
                SectionChoice::Delete,
                false,
                true,
            ),
            // A rebind waiting for a key: Escape cancels it (see `keybind`).
            (
                ExampleStates::Editor,
                false,
                SectionChoice::None,
                true,
                true,
            ),
            // Flying: the pause menu owns Escape whatever the editor left behind.
            (
                ExampleStates::Scenario,
                true,
                SectionChoice::Section("hull".to_string()),
                true,
                false,
            ),
        ];

        for (state, gallery_open, choice, rebinding, expected) in cases {
            let mut world = World::new();
            world.insert_resource(State::new(state.clone()));
            world.insert_resource(gallery::GalleryState {
                open: gallery_open,
                ..default()
            });
            world.insert_resource(choice.clone());
            world.insert_resource(EditorRebind {
                target: rebinding.then_some(Entity::PLACEHOLDER),
                awaiting_release: false,
            });
            world.init_resource::<EscapeOwner>();

            world
                .run_system_once(declare_editor_escape_owner)
                .expect("the claim system runs");

            assert_eq!(
                world.resource::<EscapeOwner>().0,
                expected,
                "{state:?} / gallery {gallery_open} / {choice:?} / rebinding {rebinding}"
            );
        }
    }

    /// The last step of the back gesture: Escape with a part in hand puts it
    /// down rather than raising the pause overlay.
    #[test]
    fn escape_puts_the_armed_part_down() {
        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Escape);
        world.insert_resource(keys);
        world.insert_resource(SectionChoice::Section("hull".to_string()));

        world
            .run_system_once(escape_puts_down_the_armed_part)
            .expect("the disarm system runs");

        assert_eq!(*world.resource::<SectionChoice>(), SectionChoice::None);
    }

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

    /// In NewGame mode the editor must still enter its Scenario state (cursor
    /// grab and the F1/despawn furniture key on it), while leaving the scenario
    /// load itself to the menu. (Flyability itself is not tied to this state:
    /// the spaceship sets are gated on scenario-liveness by nova_scenario.)
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
    /// and the next Sandbox entry starts fresh.
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

    /// F1 back-to-editor is Sandbox-only: in NewGame
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

    /// The scenario-liveness gate (nova_scenario)
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
    /// the editor smoke run; this pins just the state routing.
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

    /// Flight hides and locks the cursor. Before the fix
    /// `setup_grab_cursor_scenario` was wrapped in `cfg!(not(feature =
    /// "debug"))`, so a `--features dev` build (the standard playtest build)
    /// left the cursor visible the whole flight; the grab is unconditional now.
    #[test]
    fn scenario_grab_hides_and_locks_the_cursor() {
        let mut app = App::new();
        app.world_mut().spawn((
            PrimaryWindow,
            CursorOptions {
                visible: true,
                grab_mode: CursorGrabMode::None,
                ..default()
            },
        ));
        app.add_systems(Update, setup_grab_cursor_scenario);
        app.update();

        let cursor = app
            .world_mut()
            .query_filtered::<&CursorOptions, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        assert!(!cursor.visible, "cursor must be hidden while flying");
        assert_eq!(
            cursor.grab_mode,
            CursorGrabMode::Locked,
            "cursor must be locked (captured) while flying"
        );
    }
}
