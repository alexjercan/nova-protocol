//! The main menu: the game's front door.
//!
//! `NovaMenuPlugin` owns [`GameStates::MainMenu`]: a small panel anchored to the
//! bottom-right of the screen with the game title and New Game / Sandbox /
//! Settings / Exit buttons, drawn over a live ambient scene - the
//! `menu_ambience` scenario (nova_assets), where a passive ship circles a
//! planetoid's gravity well on a ballistic orbit the menu seeds, watched by a
//! fixed cinematic camera with the status bar hidden. The buttons write
//! [`GameMode`] and hand off to [`GameStates::Playing`]; the editor
//! (`nova_editor`) only comes up in `Sandbox` mode, and the menu's own
//! `OnEnter(Playing)` system loads the New Game scenario in `NewGame` mode.
//!
//! `nova_core`'s `AppBuilder` adds this plugin (and routes `Loading -> MainMenu`
//! instead of `Loading -> Playing`) only for the default editor app; examples
//! that supply their own game plugins never see the menu.
//!
//! Design rationale: docs/spikes/20260711-180500-main-menu.md.

use avian3d::prelude::LinearVelocity;
use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::Pressed,
    ui_widgets::{observe, Activate, Button},
};
use nova_events::prelude::EntityId;
use nova_gameplay::prelude::*;
use nova_scenario::prelude::*;

pub mod prelude {
    pub use super::NovaMenuPlugin;
}

/// The scenario New Game drops the player into. `asteroid_field` is registered by
/// `nova_assets` and already contains a canned player ship, so the menu needs no
/// content of its own. Task 20260711-180506 swaps in a designed starter scenario.
const NEW_GAME_SCENARIO_ID: &str = "asteroid_field";

/// The backdrop scenario (nova_assets registers it; task 20260711-180455).
const MENU_AMBIENCE_SCENARIO_ID: &str = "menu_ambience";
/// EntityId of the ship the menu puts on a ballistic orbit.
const MENU_ORBITER_ID: &str = "menu_orbiter";
/// EntityId of the planetoid whose well anchors the orbit and the camera
/// framing. Selected by id (not "any well") so a second big rock in the
/// backdrop cannot silently retarget the camera or the orbit seed.
const MENU_PLANETOID_ID: &str = "menu_planetoid";
/// Clearance above the well's GEOMETRIC body radius for the orbit. The
/// planetoid's noise mesh reaches several times past its nominal 20u, and the
/// well's mu/SOI derive from that real radius at runtime (see
/// insert_asteroid_gravity_well) - orbiting a hardcoded radius put the ship
/// inside the collider, whose penetration impulse flung it and whose impact
/// damage destroyed the planetoid (observed: well vanished within a second).
const ORBIT_CLEARANCE: f32 = 40.0;

// Same palette as the editor sidebar (nova_editor keeps its constants private;
// the duplication is two colors, not worth a shared UI crate yet).
const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
const BACKGROUND_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

pub struct NovaMenuPlugin;

impl Plugin for NovaMenuPlugin {
    fn build(&self, app: &mut App) {
        // Owned by NovaHudPlugin in the assembled app; initialized here too so
        // the menu plugin stands alone (tests, future slim apps).
        app.init_resource::<HudVisibility>();
        app.add_systems(
            OnEnter(GameStates::MainMenu),
            (load_menu_ambience, setup_menu_ui, hide_hud_chrome),
        );
        // Uniform backdrop teardown: EVERY exit from the menu unloads the
        // ambience scenario, so future exits (e.g. the pause menu's Back path,
        // task 20260711-185156) cannot leak a simulating backdrop. OnExit runs
        // before OnEnter(Playing), so New Game's LoadScenario still lands after
        // this unload.
        app.add_systems(
            OnExit(GameStates::MainMenu),
            (restore_hud_chrome, unload_menu_ambience),
        );
        app.add_systems(
            Update,
            (
                update_button_colors,
                stage_menu_camera,
                seed_orbiter_velocity,
            )
                .run_if(in_state(GameStates::MainMenu)),
        );
        app.add_systems(
            OnEnter(GameStates::Playing),
            start_new_game_scenario.run_if(resource_equals(GameMode::NewGame)),
        );
    }
}

/// Marker for the menu's buttons, so the color feedback system only touches ours.
#[derive(Component)]
struct MenuButton;

/// Marker for the Settings placeholder panel, toggled by the Settings button.
#[derive(Component)]
struct SettingsPanel;

/// The living backdrop: load the ambient scenario behind the menu. The loader
/// brings its own camera + skybox and tears down whatever was loaded before;
/// the uniform OnExit(MainMenu) teardown (unload_menu_ambience) tears this
/// down again on the way out, whatever the exit path.
fn load_menu_ambience(mut commands: Commands, scenarios: Res<GameScenarios>) {
    let scenario = scenarios
        .get(MENU_AMBIENCE_SCENARIO_ID)
        .unwrap_or_else(|| panic!("Scenario '{MENU_AMBIENCE_SCENARIO_ID}' not found"))
        .clone();
    commands.trigger(LoadScenario(scenario));
}

/// Turn the loader's flyable camera into a fixed cinematic viewpoint: strip the
/// WASD controller (the user must not be able to fly the menu backdrop), then
/// hold the framing pose every frame. The pose is written only AFTER the
/// controller is gone: the controller drives Transform from its own state each
/// frame, so a pose written in the same frame the removal is queued gets
/// overwritten before the removal applies (observed: camera stuck at the
/// loader's default inside the planetoid). The camera spawns a frame after
/// LoadScenario, so an OnEnter hook would miss it - this polls instead.
fn stage_menu_camera(
    mut commands: Commands,
    mut controlled: Query<(Entity, &mut Camera), (With<Camera3d>, With<WASDCameraController>)>,
    mut staged: Query<
        (&mut Transform, &mut Camera),
        (With<Camera3d>, Without<WASDCameraController>),
    >,
    wells: Query<(&Transform, &GravityWell, &EntityId), Without<Camera3d>>,
) {
    // Blank the frame while the controller is still attached: the loader
    // spawns the camera inside the planetoid's geometric radius, and staging
    // takes effect one frame later, so an active camera would flash the
    // inside of the rock on every menu entry.
    for (entity, mut camera) in &mut controlled {
        camera.is_active = false;
        commands.entity(entity).remove::<WASDCameraController>();
    }
    // Frame the planetoid + orbit from ITS well's real geometry (the body
    // radius is only known at runtime; see ORBIT_CLEARANCE).
    let Some((well_transform, well, _)) = wells.iter().find(|(_, _, id)| id.0 == MENU_PLANETOID_ID)
    else {
        return;
    };
    let r_orbit = well.body_radius + ORBIT_CLEARANCE;
    let pose = well_transform.translation + Vec3::new(0.0, r_orbit * 0.75, r_orbit * 2.5);
    for (mut transform, mut camera) in &mut staged {
        *transform =
            Transform::from_translation(pose).looking_at(well_transform.translation, Vec3::Y);
        camera.is_active = true;
    }
}

/// Marks the orbiter once its orbit velocity has been seeded.
#[derive(Component)]
struct OrbitSeeded;

/// Put the ambience ship on a ballistic circular orbit around the planetoid's
/// gravity well. The scenario spawn path has no velocity field, and thruster
/// flight is unavailable in MainMenu (the editor gates the spaceship
/// input/section sets on its own Scenario state), so the orbit is pure physics:
/// re-stage the ship onto the runtime orbit radius (body_radius +
/// ORBIT_CLEARANCE, along its spawn direction from the well), give it
/// tangential v_circ from the well's real mu, and gravity does the rest.
/// Polls until both the ship and the well exist, then never matches again.
fn seed_orbiter_velocity(
    mut commands: Commands,
    // Without<GravityWell> keeps the two Transform accesses disjoint (the
    // planetoid carries both an EntityId and the well).
    mut orbiters: Query<
        (Entity, &mut Transform, &EntityId),
        (Without<OrbitSeeded>, Without<GravityWell>),
    >,
    wells: Query<(&Transform, &GravityWell, &EntityId)>,
) {
    let Some((entity, mut ship_transform, _)) = orbiters
        .iter_mut()
        .find(|(_, _, id)| id.0 == MENU_ORBITER_ID)
    else {
        return;
    };
    let Some((well_transform, well, _)) = wells.iter().find(|(_, _, id)| id.0 == MENU_PLANETOID_ID)
    else {
        return;
    };

    let well_pos = well_transform.translation;
    let radial = (ship_transform.translation - well_pos)
        .with_y(0.0)
        .normalize_or(Vec3::X);
    let r_orbit = well.body_radius + ORBIT_CLEARANCE;
    ship_transform.translation = well_pos + radial * r_orbit;

    let velocity = orbit_insertion_velocity(well_pos, well.mu, ship_transform.translation);
    commands
        .entity(entity)
        .insert((LinearVelocity(velocity), OrbitSeeded));
}

/// Tangential velocity for a circular orbit of a well with parameter `mu` at
/// `well_pos`, as seen from `ship_pos`: horizontal orbit plane (tangent =
/// Y x radial), speed from the shared `circular_orbit_speed` helper. Zero for
/// degenerate geometry (ship at the well center or directly above it).
fn orbit_insertion_velocity(well_pos: Vec3, mu: f32, ship_pos: Vec3) -> Vec3 {
    let radial = ship_pos - well_pos;
    let tangent = Vec3::Y.cross(radial).normalize_or_zero();
    tangent * circular_orbit_speed(mu, radial.length())
}

/// The menu is a cinematic shot: drive the HUD level to None while it is up
/// (this is the mechanism from task 20260711-180501; it owns the status bar
/// and every tagged HUD widget). Restoring to All on exit intentionally
/// resets any mid-game cycle the player had going - simple beats sticky.
fn hide_hud_chrome(mut level: ResMut<HudVisibility>) {
    *level = HudVisibility::None;
}

fn restore_hud_chrome(mut level: ResMut<HudVisibility>) {
    *level = HudVisibility::All;
}

/// The menu panel: title on top, buttons below, anchored bottom-right per the
/// spike's layout call (the center of the screen stays free for the background
/// scene).
fn setup_menu_ui(mut commands: Commands) {
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
                ..default()
            },
            BackgroundColor(BACKGROUND_COLOR),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Title"),
                Text::new("Nova Protocol"),
                TextFont {
                    font_size: FontSize::Px(28.0),
                    ..default()
                },
                TextColor(TEXT_COLOR),
            ));
            parent.spawn((
                Name::new("Title Separator"),
                Node {
                    width: percent(80),
                    height: px(2),
                    margin: UiRect::all(px(10)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
            ));
            parent.spawn((
                Name::new("New Game Button"),
                button("New Game"),
                observe(on_new_game),
            ));
            parent.spawn((
                Name::new("Sandbox Button"),
                button("Sandbox"),
                observe(on_sandbox),
            ));
            parent.spawn((
                Name::new("Settings Button"),
                button("Settings"),
                observe(on_settings),
            ));
            // No process to quit on wasm; the browser tab owns the lifecycle.
            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn((Name::new("Exit Button"), button("Exit"), observe(on_exit)));
        });

    // Settings placeholder: hidden until the Settings button toggles it. Real
    // content is task 20260711-180511 (v0.6.0).
    commands
        .spawn((
            DespawnOnExit(GameStates::MainMenu),
            Name::new("Settings Panel Root"),
            SettingsPanel,
            Visibility::Hidden,
            Pickable {
                should_block_lower: false,
                is_hoverable: false,
            },
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Settings Panel"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: px(360),
                        padding: UiRect::all(px(20)),
                        ..default()
                    },
                    BackgroundColor(BACKGROUND_COLOR),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Settings Title"),
                        Text::new("Settings"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(TEXT_COLOR),
                    ));
                    parent.spawn((
                        Name::new("Settings Placeholder"),
                        Text::new("Nothing to configure yet."),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(TEXT_COLOR),
                    ));
                    parent.spawn((
                        Name::new("Settings Back Button"),
                        button("Back"),
                        observe(on_settings_back),
                    ));
                });
        });
}

fn on_new_game(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    *mode = GameMode::NewGame;
    state.set(GameStates::Playing);
}

fn on_sandbox(
    _activate: On<Activate>,
    mut mode: ResMut<GameMode>,
    mut state: ResMut<NextState<GameStates>>,
) {
    *mode = GameMode::Sandbox;
    state.set(GameStates::Playing);
}

/// Tear the backdrop down whenever the menu is left, no matter through which
/// button or future path. The editor does not unload scenarios on entry, and
/// a forgotten unload would leave the ambience simulating behind the game.
fn unload_menu_ambience(mut commands: Commands) {
    commands.trigger(UnloadScenario);
}

fn on_settings(_activate: On<Activate>, mut panel: Single<&mut Visibility, With<SettingsPanel>>) {
    **panel = match **panel {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
}

fn on_settings_back(
    _activate: On<Activate>,
    mut panel: Single<&mut Visibility, With<SettingsPanel>>,
) {
    **panel = Visibility::Hidden;
}

#[cfg(not(target_arch = "wasm32"))]
fn on_exit(_activate: On<Activate>, mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}

/// In `NewGame` mode the menu itself provides the game: load the canned scenario
/// (player ship included) the moment gameplay starts. `Sandbox` mode does nothing
/// here - the editor owns that path.
fn start_new_game_scenario(mut commands: Commands, scenarios: Res<GameScenarios>) {
    let scenario = scenarios
        .get(NEW_GAME_SCENARIO_ID)
        .unwrap_or_else(|| panic!("Scenario '{NEW_GAME_SCENARIO_ID}' not found"))
        .clone();
    commands.trigger(LoadScenario(scenario));
}

/// Hover/press feedback for the menu buttons. The editor drives the same feedback
/// through per-event observers; a single polling system is enough for four buttons
/// and keeps the menu self-contained.
fn update_button_colors(
    mut buttons: Query<
        (&Hovered, Has<Pressed>, &mut BackgroundColor),
        (With<MenuButton>, With<Button>),
    >,
) {
    for (hovered, pressed, mut color) in &mut buttons {
        let target = if pressed {
            PRESSED_BUTTON
        } else if hovered.get() {
            HOVERED_BUTTON
        } else {
            NORMAL_BUTTON
        };
        if color.0 != target {
            color.0 = target;
        }
    }
}

fn button(text: &str) -> impl Bundle {
    (
        Node {
            width: percent(100),
            min_height: px(40),
            margin: UiRect::all(px(8)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        MenuButton,
        Button,
        Hovered::default(),
        BackgroundColor(NORMAL_BUTTON),
        children![(
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(TEXT_COLOR),
            TextShadow::default(),
        )],
    )
}

#[cfg(test)]
mod tests {
    use bevy::state::app::StatesPlugin;

    use super::*;

    /// A headless app with just enough for the menu's non-UI wiring: states, the
    /// mode resource, and the plugin itself. Tests that enter MainMenu also run
    /// the OnEnter systems (setup_menu_ui spawns plain components; the HUD
    /// level is a plain resource write), so insert `dummy_scenarios()` first -
    /// load_menu_ambience reads GameScenarios.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<GameStates>();
        app.init_resource::<GameMode>();
        app.add_plugins(NovaMenuPlugin);
        app
    }

    #[derive(Resource, Default)]
    struct LoadedScenario(Option<String>);

    fn observe_load_scenario(app: &mut App) {
        app.init_resource::<LoadedScenario>();
        app.add_observer(
            |load: On<LoadScenario>, mut loaded: ResMut<LoadedScenario>| {
                loaded.0 = Some(load.0.id.clone());
            },
        );
    }

    fn dummy_scenario(id: &str) -> (String, ScenarioConfig) {
        (
            id.to_string(),
            ScenarioConfig {
                id: id.to_string(),
                name: "Test".to_string(),
                description: "Test".to_string(),
                cubemap: Handle::default(),
                events: vec![],
            },
        )
    }

    fn dummy_scenarios() -> GameScenarios {
        GameScenarios(bevy::platform::collections::HashMap::from([
            dummy_scenario(NEW_GAME_SCENARIO_ID),
            dummy_scenario(MENU_AMBIENCE_SCENARIO_ID),
        ]))
    }

    #[derive(Resource, Default)]
    struct Unloaded(bool);

    fn observe_unload_scenario(app: &mut App) {
        app.init_resource::<Unloaded>();
        app.add_observer(|_: On<UnloadScenario>, mut unloaded: ResMut<Unloaded>| {
            unloaded.0 = true;
        });
    }

    #[test]
    fn new_game_button_sets_mode_and_hands_off_to_playing() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        let button = app.world_mut().spawn(observe(on_new_game)).id();
        app.update();

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Playing
        );
        // Delivery guard: the handoff must actually load the scenario, not just
        // flip states.
        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(NEW_GAME_SCENARIO_ID)
        );
    }

    #[test]
    fn sandbox_button_sets_mode_and_loads_no_scenario() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        let button = app.world_mut().spawn(observe(on_sandbox)).id();
        // Enter the menu first (the real path), so leaving it exercises the
        // uniform OnExit teardown.
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        // Observers registered after entry, so the menu's own ambience load
        // does not count against the "loads nothing" assertion below.
        observe_load_scenario(&mut app);
        observe_unload_scenario(&mut app);

        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(*app.world().resource::<GameMode>(), GameMode::Sandbox);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Playing
        );
        // The editor owns the Sandbox path; the menu must not load anything,
        // and it must tear the ambience backdrop down (the editor does not).
        assert_eq!(app.world().resource::<LoadedScenario>().0, None);
        assert!(app.world().resource::<Unloaded>().0);
        // Leaving the menu restores the HUD level.
        assert_eq!(*app.world().resource::<HudVisibility>(), HudVisibility::All);
    }

    /// Entering MainMenu loads the ambience backdrop through the real
    /// OnEnter systems (task 20260711-180455).
    #[test]
    fn entering_main_menu_loads_the_ambience_scenario() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        app.update();

        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();
        app.update();

        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(MENU_AMBIENCE_SCENARIO_ID)
        );
        // The menu is a cinematic shot: entering drives the HUD level to None
        // (the absorbed status-bar hide, task 20260711-180501).
        assert_eq!(
            *app.world().resource::<HudVisibility>(),
            HudVisibility::None
        );
    }

    /// The seeded orbit velocity is tangential (perpendicular to the radial)
    /// with the shared v_circ magnitude, and degenerates to zero safely.
    #[test]
    fn orbit_insertion_velocity_is_tangential_v_circ() {
        let well_pos = Vec3::new(10.0, 0.0, -5.0);
        let mu = 2400.0;
        let ship_pos = well_pos + Vec3::new(50.0, 0.0, 0.0);

        let v = orbit_insertion_velocity(well_pos, mu, ship_pos);

        let radial = ship_pos - well_pos;
        assert!((v.length() - circular_orbit_speed(mu, 50.0)).abs() < 1e-4);
        assert!(v.dot(radial).abs() < 1e-4, "velocity must be tangential");
        // Degenerate: ship at the well center, or directly above it (radial
        // parallel to Y, so the horizontal tangent is undefined).
        assert_eq!(orbit_insertion_velocity(well_pos, mu, well_pos), Vec3::ZERO);
        assert_eq!(
            orbit_insertion_velocity(well_pos, mu, well_pos + Vec3::Y * 50.0),
            Vec3::ZERO
        );
    }

    /// Review R1.3: exercise the REAL New Game button, not just the handler fn.
    /// Builds the actual menu UI headless, finds the button by Name, and clicks
    /// it - so dropping the observe(on_new_game) wiring from setup_menu_ui fails
    /// this test.
    #[test]
    fn real_new_game_button_is_wired() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = app();
        app.insert_resource(dummy_scenarios());
        observe_load_scenario(&mut app);
        app.world_mut()
            .run_system_once(setup_menu_ui)
            .expect("setup_menu_ui runs headless");
        app.update();

        let button = {
            let mut names = app.world_mut().query::<(Entity, &Name)>();
            names
                .iter(app.world())
                .find(|(_, name)| name.as_str() == "New Game Button")
                .map(|(entity, _)| entity)
                .expect("the menu spawns a 'New Game Button'")
        };
        app.world_mut().trigger(Activate { entity: button });
        app.update();

        assert_eq!(*app.world().resource::<GameMode>(), GameMode::NewGame);
        assert_eq!(
            *app.world().resource::<State<GameStates>>().get(),
            GameStates::Playing
        );
        assert_eq!(
            app.world().resource::<LoadedScenario>().0.as_deref(),
            Some(NEW_GAME_SCENARIO_ID)
        );
    }

    fn spawn_planetoid_well(app: &mut App) {
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, 0.0),
            GravityWell {
                mu: 2400.0,
                body_radius: 80.0,
                soi_radius: 600.0,
            },
            EntityId::new(MENU_PLANETOID_ID),
        ));
    }

    /// Review R1.1: the camera staging that carried dev bug 1 (pose written in
    /// the same frame as the controller removal gets overwritten by the
    /// controller). Frame 1 must deactivate + strip the controller; frame 2
    /// must stage the runtime-derived pose and reactivate.
    #[test]
    fn menu_camera_is_staged_from_the_wells_geometry() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();

        let cam = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                WASDCameraController,
                Transform::from_xyz(0.0, 10.0, 20.0),
            ))
            .id();
        spawn_planetoid_well(&mut app);

        app.update();
        assert!(
            !app.world().get::<Camera>(cam).unwrap().is_active,
            "camera must be blanked while the controller is still attached"
        );
        app.update();
        assert!(
            app.world().get::<WASDCameraController>(cam).is_none(),
            "controller must be stripped"
        );
        // r_orbit = body_radius 80 + clearance 40 = 120 -> pose (0, 90, 300).
        let staged = app.world().get::<Transform>(cam).unwrap().translation;
        assert!(
            (staged - Vec3::new(0.0, 90.0, 300.0)).length() < 1e-3,
            "pose must derive from the well's runtime geometry, got {staged:?}"
        );
        assert!(app.world().get::<Camera>(cam).unwrap().is_active);
    }

    /// Review R1.1: the orbit seeding that carried dev bugs 2 and 3 (hardcoded
    /// radius inside the collider). The orbiter must be re-staged onto
    /// body_radius + ORBIT_CLEARANCE with a tangential v_circ velocity, seeded
    /// exactly once.
    #[test]
    fn orbiter_is_restaged_and_seeded_once() {
        let mut app = app();
        app.insert_resource(dummy_scenarios());
        app.world_mut()
            .resource_mut::<NextState<GameStates>>()
            .set(GameStates::MainMenu);
        app.update();

        spawn_planetoid_well(&mut app);
        let ship = app
            .world_mut()
            .spawn((
                Transform::from_xyz(140.0, 0.0, 0.0),
                EntityId::new(MENU_ORBITER_ID),
            ))
            .id();

        app.update();
        app.update();

        let pos = app.world().get::<Transform>(ship).unwrap().translation;
        assert!(
            (pos - Vec3::new(120.0, 0.0, 0.0)).length() < 1e-3,
            "orbiter must sit at body_radius + clearance, got {pos:?}"
        );
        let vel = app.world().get::<LinearVelocity>(ship).unwrap().0;
        let expected = orbit_insertion_velocity(Vec3::ZERO, 2400.0, pos);
        assert!(
            (vel - expected).length() < 1e-3,
            "tangential v_circ, got {vel:?}"
        );
        assert!(app.world().get::<OrbitSeeded>(ship).is_some());

        // Seeded exactly once: moving the ship afterwards must not re-stage it.
        app.world_mut()
            .get_mut::<Transform>(ship)
            .unwrap()
            .translation = Vec3::new(500.0, 0.0, 0.0);
        app.update();
        let moved = app.world().get::<Transform>(ship).unwrap().translation;
        assert_eq!(moved, Vec3::new(500.0, 0.0, 0.0));
    }
}
