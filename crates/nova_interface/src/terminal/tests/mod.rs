//! Shared rig for the command modal and TAB toggle tests; each submodule
//! covers one concern.

mod chin;
mod commands;
mod crt;
mod editing;
mod flight_log;
mod sound;
mod structure;
mod toggle;

use bevy::{
    asset::{AssetApp, AssetPlugin},
    ecs::system::RunSystemOnce,
    input::{
        keyboard::{Key, KeyboardInput},
        touch::TouchPhase,
        ButtonState,
    },
    picking::{
        hover::{HoverMap, Hovered},
        pointer::PointerId,
    },
    prelude::*,
    state::app::StatesPlugin,
    ui_render::prelude::MaterialNode,
    ui_widgets::{Activate, Button},
};
use nova_command::prelude::*;
use nova_gameplay::{
    audio::prelude::{PlaySfx, SoundBank, UiSfx, NOVA_OS_BED_VOLUME},
    objectives::prelude::{GameObjectives, Objective},
    prelude::*,
    GameStates, PauseStates,
};
use nova_hud::prelude::*;
use nova_input::prelude::RegisterInputActions;
use nova_ui::theme;

use super::{
    components::*, content::*, crt::*, flight_log::*, input::*, shell::*, sound::*, spawn::*,
    style::*,
};
use crate::pane::{play_interface_toggle, toggle_interface};

/// A headless app with just the states and the TAB interface toggle, enough to
/// drive the open and close state machine.
fn toggle_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.insert_state(GameStates::Playing);
    app.init_state::<PauseStates>();
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.register_input_actions(crate::bindings::interface_bindings());
    app.init_resource::<NovaOsCloseTransition>();
    // The interface belongs to a ship: with none on the field the toggle is
    // inert (see `toggle_interface`), so the rig flies one.
    app.world_mut().spawn(PlayerSpaceshipMarker);
    app.add_systems(
        Update,
        toggle_interface.run_if(in_state(GameStates::Playing)),
    );
    // Enter Playing so the toggle runs.
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    app.update();
    app
}

fn press_tab(app: &mut App) {
    if let Some(mut keyboard) = app
        .world_mut()
        .get_resource_mut::<Messages<KeyboardInput>>()
    {
        keyboard.write(KeyboardInput {
            key_code: KeyCode::Tab,
            logical_key: Key::Tab,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Tab);
    app.update();
    // Clear the just_pressed edge like nova_menu's `press_escape` (no
    // InputPlugin in this rig, so nothing clears it automatically - a stale
    // edge would re-fire the toggle on the next update).
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(KeyCode::Tab);
    keys.clear();
    app.update();
}

fn press_key(app: &mut App, key_code: KeyCode, logical_key: Key, text: Option<&str>) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}

fn press_text(app: &mut App, text: &str) {
    press_key(app, KeyCode::KeyA, Key::Character(text.into()), Some(text));
}

fn type_text(terminal: &mut CommandTerminal, text: &str) {
    terminal.insert_text(text);
}

fn init_terminal_input_resources(app: &mut App) {
    app.init_resource::<CommandTerminal>();
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<GameObjectives>();
    // handle_terminal_keyboard reads the SND toggle.
    app.init_resource::<NovaOsMonitorSettings>();
    app.world_mut().init_resource::<Messages<KeyboardInput>>();
}

fn terminal_command_app() -> App {
    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.add_systems(
        Update,
        handle_terminal_keyboard.run_if(in_state(GameStates::Playing)),
    );
    open_commands(&mut app);
    app
}

fn pause_state(app: &App) -> PauseStates {
    *app.world().resource::<State<PauseStates>>().get()
}

// --- Modal sound ---

/// Records which modal cues were triggered (by handle identity), so tests
/// can assert WHICH sound played on each terminal event without an audio
/// device. Mirrors `objective_feedback`'s `sfx_app` capture.
#[derive(Resource, Default)]
struct SoundCapture(Vec<UiSfx>);

fn nova_os_sound_app() -> App {
    let mut app = toggle_app();
    init_terminal_input_resources(&mut app);
    app.init_resource::<NovaOsCloseTransition>();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<AudioSource>();
    let bank = SoundBank::load(
        app.world().resource::<AssetServer>(),
        nova_gameplay::audio::UI_SFX_FILES,
    );
    app.insert_resource(bank);
    app.init_resource::<SoundCapture>();
    app.add_observer(
        |sfx: On<PlaySfx>, bank: Res<SoundBank<UiSfx>>, mut cap: ResMut<SoundCapture>| {
            for (key, _) in nova_gameplay::audio::UI_SFX_FILES {
                if sfx.handle == bank.get(key) {
                    cap.0.push(key);
                    break;
                }
            }
        },
    );
    app.add_systems(
        Update,
        (
            handle_terminal_keyboard,
            play_nova_os_power_down.run_if(in_state(PauseStates::Commands)),
        )
            .run_if(in_state(GameStates::Playing)),
    );
    app.add_systems(OnEnter(PauseStates::Commands), start_nova_os_sound);
    app.add_systems(OnExit(PauseStates::Commands), stop_nova_os_bed);
    // The interface's back-out and toggle blip, wired as `InterfacePlugin` and
    // `CommandsPlugin` wire them.
    app.add_systems(
        Update,
        close_surface_from_menu_keys.before(toggle_interface),
    );
    app.add_systems(
        OnTransition {
            exited: PauseStates::Unpaused,
            entered: PauseStates::Interface,
        },
        play_interface_toggle,
    );
    app.add_systems(
        OnTransition {
            exited: PauseStates::Interface,
            entered: PauseStates::Unpaused,
        },
        play_interface_toggle,
    );
    app
}

/// Open the command modal over flight, the way `nova_menu::open_command_shell`
/// does after a typed `:`.
fn open_commands(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Commands);
    app.update();
}

fn clear_capture(app: &mut App) {
    app.world_mut().resource_mut::<SoundCapture>().0.clear();
}

fn fired(app: &App, cue: UiSfx) -> bool {
    app.world().resource::<SoundCapture>().0.contains(&cue)
}

fn bed_count(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<NovaOsBedSfx>>()
        .iter(app.world())
        .count()
}

fn set_prompt(app: &mut App, command: &str) {
    let mut terminal = app.world_mut().resource_mut::<CommandTerminal>();
    terminal.reset_prompt();
    terminal.insert_text(command);
}

fn press_enter(app: &mut App) {
    press_key(app, KeyCode::Enter, Key::Enter, None);
}

/// One right-stick-click press: press + update (toggle sets NextState), then
/// release + clear + update (applies the transition; the clear stops the
/// stale edge re-firing next frame - same shape as `press_tab`).
///
/// Press the pad button on a CONNECTED pad. Bevy 0.19 keeps digital state on
/// the `Gamepad` component, so the rig spawns one on first use and reuses it -
/// a second pad would double every press.
fn press_pad(app: &mut App) {
    let existing = app
        .world_mut()
        .query::<(Entity, &Gamepad)>()
        .iter(app.world())
        .map(|(entity, _)| entity)
        .next();
    let entity = existing.unwrap_or_else(|| app.world_mut().spawn(Gamepad::default()).id());
    app.world_mut()
        .entity_mut(entity)
        .get_mut::<Gamepad>()
        .expect("just spawned")
        .digital_mut()
        .press(GamepadButton::RightThumb);
    app.update();
    let mut entity_mut = app.world_mut().entity_mut(entity);
    let mut pad = entity_mut.get_mut::<Gamepad>().expect("still connected");
    let digital = pad.digital_mut();
    digital.release(GamepadButton::RightThumb);
    digital.clear();
    app.update();
}

fn objectives_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<GameObjectives>();
    app.init_resource::<StoryFeed>();
    app.init_resource::<NovaOsFlightLog>();
    app.add_systems(
        Update,
        sync_nova_os_logs
            .run_if(resource_changed::<GameObjectives>.or_else(resource_changed::<StoryFeed>)),
    );
    app
}

fn spawn_nova_os_shell(app: &mut App) {
    // The keep-alive gate is the plugin's, not the system's: without it the
    // rig spawns a second monitor on every update.
    app.add_systems(
        Update,
        ensure_nova_os_spawned.run_if(not(any_with_component::<NovaOsRootMarker>)),
    );
    app.update();
}

fn spawn_nova_os_shell_with_crt(app: &mut App) {
    app.init_asset::<NovaOsCrtMaterial>();
    app.init_asset::<Font>();
    // The chin's brand plate loads a logo image, so the render-capable rig
    // must register the `Image` asset too (production has it via DefaultPlugins).
    app.init_asset::<Image>();
    spawn_nova_os_shell(app);
}

fn assert_scrollable_viewport(app: &App, viewport: Entity, label: &str) {
    let node = app.world().entity(viewport).get::<Node>().expect(label);
    assert_eq!(
        node.overflow,
        Overflow::scroll_y(),
        "{label} clips overflowing rows on the y axis"
    );
    assert_eq!(
        node.flex_grow, 1.0,
        "{label} consumes the panel's remaining height instead of growing past it"
    );
    assert!(
        app.world().entity(viewport).contains::<ScrollPosition>(),
        "{label} carries ScrollPosition so wheel input can move it"
    );
}
fn set_objectives(app: &mut App, objectives: Vec<Objective>) {
    app.world_mut().resource_mut::<GameObjectives>().objectives = objectives;
}

fn push_story_line(app: &mut App, speaker: &str, text: &str) {
    app.world_mut()
        .resource_mut::<StoryFeed>()
        .0
        .push(StoryLine {
            accent: default_comms_accent(),
            speaker: speaker.to_string(),
            text: text.to_string(),
            dwell: None,
            icon: None,
        });
}

fn all_texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}

/// A headless app with the monitor's chin controls spawned and the command
/// modal open.
fn chin_controls_app() -> App {
    let mut app = App::new();
    // AssetPlugin so `init_asset` works and the AssetServer can hand back
    // (asynchronously-failing) handles for the font/logo loads; the Font +
    // Image asset types must be registered or those loads panic. This
    // mirrors `spawn_nova_os_shell_with_crt`'s callers.
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.add_plugins(StatesPlugin);
    app.insert_state(GameStates::Playing);
    app.init_state::<PauseStates>();
    app.init_resource::<NovaOsFlightLog>();
    app.init_resource::<CommandTerminal>();
    app.init_resource::<NovaOsCloseTransition>();
    app.init_resource::<NovaOsMonitorSettings>();
    app.init_resource::<Time<Real>>();
    app.init_asset::<Font>();
    app.init_asset::<Image>();
    app.init_asset::<NovaOsCrtMaterial>();
    app.add_systems(Update, ensure_nova_os_spawned);
    app.add_systems(
        Update,
        (animate_nova_os_crt, sync_nova_os_monitor_controls)
            .run_if(in_state(PauseStates::Commands)),
    );
    app.world_mut()
        .resource_mut::<NextState<PauseStates>>()
        .set(PauseStates::Commands);
    app.update();
    app
}

/// The `Button` entity (not the dial) for a chin knob.
fn knob_button(app: &mut App, which: NovaOsKnob) -> Entity {
    app.world_mut()
        .query_filtered::<(Entity, &NovaOsKnob), With<Button>>()
        .iter(app.world())
        .find(|(_, knob)| **knob == which)
        .map(|(entity, _)| entity)
        .expect("the knob button spawned")
}

/// A knob's dial-pointer rotation, in radians.
fn dial_rotation(app: &mut App, which: NovaOsKnob) -> f32 {
    app.world_mut()
        .query_filtered::<(&NovaOsKnob, &UiTransform), With<NovaOsKnobDialMarker>>()
        .iter(app.world())
        .find(|(knob, _)| **knob == which)
        .map(|(_, transform)| transform.rotation.as_radians())
        .expect("the dial spawned")
}

/// The base colour of a marked indicator bulb (the SND bulb or the PWR LED),
/// i.e. the state-reporting `BackgroundColor` under the fixed glassy cap.
fn bulb_color<M: Component>(app: &mut App) -> Color {
    app.world_mut()
        .query_filtered::<&BackgroundColor, With<M>>()
        .iter(app.world())
        .next()
        .map(|background| background.0)
        .expect("the bulb spawned")
}
