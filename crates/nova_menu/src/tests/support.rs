//! The shared rig for the `nova_menu` tests: an app with the menu plugins, the
//! state helpers (`enter_playing`, `press_escape`, clock/pause readers), and
//! the dummy scenarios and load/unload observers the screens are driven with.

use avian3d::prelude::{Physics, PhysicsTime};
use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_rand::prelude::*;
use nova_assets::{
    mod_cache::InstalledModRecord,
    prelude::{
        DownloadedMod, DownloadedMods, EnabledMods, FetchPortalCatalog, InstallPortalMod,
        ModCatalog, ModInfo, ModMeta, RemoteCatalog, RemoteCatalogState, UninstallPortalMod,
    },
};
use nova_gameplay::prelude::*;
use nova_input::prelude::RegisterInputActions;
use nova_scenario::prelude::*;
use nova_ship::prelude::{camera_bindings, flight_bindings};

use crate::{
    mods::{ModEnableCheckbox, ModRow, ModToggle, SelectedModId},
    NovaMenuPlugin,
};

/// Fixture ids: the tests own their registry; production names no scenario ids.
pub(crate) const TEST_START_ID: &str = "story_start";
pub(crate) const TEST_BACKDROP_ID: &str = "test_backdrop";

/// Point the settings store at a scratch directory, once per test process.
///
/// `NovaMenuPlugin` loads the persisted settings at Startup. Without this the
/// fixture reads the DEVELOPER'S real `settings.ron`, so a keybind saved by
/// playing the game (or by a screenshot run) silently rewrites the table these
/// tests assert on - which is exactly how it was found.
fn isolate_the_config_store() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("NOVA_CONFIG_ROOT", shared_config_root());
    });
}

/// The scratch root [`isolate_the_config_store`] points the store at.
///
/// A test that needs a store of its OWN must put this back when it is done: the
/// `Once` above fires exactly once per process, so a test that removes
/// `NOVA_CONFIG_ROOT` instead of restoring it leaves every later fixture
/// reading the developer's real `settings.ron`.
pub(crate) fn shared_config_root() -> std::path::PathBuf {
    std::env::temp_dir().join("nova_menu_test_config")
}

/// A headless app with just enough for the menu's non-UI wiring: states, the mode
/// resource, and the plugin itself. Tests that enter MainMenu also run the OnEnter
/// systems (setup_menu_ui spawns plain components; the HUD level is a plain resource
/// write), so insert `dummy_scenarios()` first - load_menu_ambience reads GameScenarios.
pub(crate) fn app() -> App {
    isolate_the_config_store();
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    // Seeded so the backdrop draw is deterministic across runs.
    app.add_plugins(EntropyPlugin::<WyRand>::with_seed(42u64.to_ne_bytes()));
    app.init_state::<GameStates>();
    app.init_state::<PauseStates>();
    app.init_resource::<GameMode>();
    // Headless: no `InputPlugin`, so the surfaces the rebind capture reads
    // have to be here. A pad is a component, so there is nothing to init for
    // it - a fixture with no pad simply has none connected.
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    // The base bundle's declared New Game start (register_bundles writes
    // this in production).
    app.insert_resource(NewGameStart(Some(TEST_START_ID.to_string())));
    // Headless: no TimePlugin, so provide the clocks the pause systems
    // touch.
    app.insert_resource(Time::<Virtual>::default());
    app.insert_resource(Time::<Physics>::default());
    // The rig defaults the settings readout renders. In production the owning
    // plugins register these - `SpaceshipPlayerInputPlugin`,
    // `SpaceshipCameraControllerPlugin`, `NovaHudPlugin`, `NovaOsPlugin`,
    // `ScenarioLoaderPlugin` - and a menu-only harness adds none of them.
    app.register_input_actions(flight_bindings());
    app.register_input_actions(camera_bindings());
    app.register_input_actions(nova_hud::hud_bindings());
    app.register_input_actions(nova_os_ui::bindings::novaos_bindings());
    app.register_input_actions(nova_scenario::prelude::scenario_bindings());
    app.add_plugins(NovaMenuPlugin);
    app
}

pub(crate) fn enter_playing(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::Playing);
    app.update();
}

pub(crate) fn press_escape(app: &mut App) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release(KeyCode::Escape);
    keys.clear();
    app.update();
}

pub(crate) fn pause_state(app: &App) -> PauseStates {
    app.world().resource::<State<PauseStates>>().get().clone()
}

pub(crate) fn clocks_paused(app: &App) -> (bool, bool) {
    (
        app.world().resource::<Time<Virtual>>().is_paused(),
        app.world().resource::<Time<Physics>>().is_paused(),
    )
}

#[derive(Resource, Default)]
pub(crate) struct LoadedScenario(pub(crate) Option<String>);

pub(crate) fn observe_load_scenario(app: &mut App) {
    app.init_resource::<LoadedScenario>();
    app.add_observer(
        |load: On<LoadScenario>, mut loaded: ResMut<LoadedScenario>| {
            loaded.0 = Some(load.0.id.clone());
        },
    );
}

pub(crate) fn dummy_scenario(id: &str) -> (String, ScenarioConfig) {
    (
        id.to_string(),
        ScenarioConfig {
            description: "Test".to_string(),
            events: vec![],
            ..ScenarioConfig::new(id.to_string(), "Test".to_string(), AssetRef::default())
        },
    )
}

pub(crate) fn dummy_backdrop(id: &str) -> (String, ScenarioConfig) {
    let (key, mut config) = dummy_scenario(id);
    config.menu_backdrop = true;
    (key, config)
}

pub(crate) fn dummy_scenarios() -> GameScenarios {
    GameScenarios(bevy::platform::collections::HashMap::from([
        dummy_scenario(TEST_START_ID),
        dummy_backdrop(TEST_BACKDROP_ID),
    ]))
}

#[derive(Resource, Default)]
pub(crate) struct Unloaded(pub(crate) bool);

pub(crate) fn observe_unload_scenario(app: &mut App) {
    app.init_resource::<Unloaded>();
    app.add_observer(|_: On<UnloadScenario>, mut unloaded: ResMut<Unloaded>| {
        unloaded.0 = true;
    });
}

/// Count of UI cues played, standing in for "sounds heard".
#[derive(Resource, Default)]
pub(crate) struct PlayedCues(pub(crate) usize);

/// A headless app with a loaded [`SoundBank`] and a `PlaySfx` counter, on
/// MinimalPlugins so the AssetPlugin task pools exist.
pub(crate) fn cue_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<AudioSource>();
    app.insert_resource(SoundBank::load(
        app.world().resource::<AssetServer>(),
        UI_SFX_FILES,
    ));
    app.init_resource::<PlayedCues>();
    app.add_observer(|_: On<PlaySfx>, mut cues: ResMut<PlayedCues>| cues.0 += 1);
    app
}

/// Entity lookup by Name, shared by the pause- and outcome-overlay tests.
pub(crate) fn find_named(app: &mut App, name: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Name)>();
    q.iter(app.world())
        .find(|(_, n)| n.as_str() == name)
        .map(|(e, _)| e)
}

/// Every Text value currently in the world, for banner/label asserts.
pub(crate) fn all_text(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

/// A menu rig with the outcome plumbing the real app gets from the
/// scenario loader: the CurrentOutcome + NovaEventWorld resources.
pub(crate) fn app_with_outcome() -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.init_resource::<CurrentOutcome>();
    app.init_resource::<NovaEventWorld>();
    app
}

/// Simulate the backdrop's own `SetCamera` landing: pin the scripted pose
/// on the scenario camera, exactly what the action inserts (the reference
/// backdrop pose (0, 90, 300) looking at the origin).
pub(crate) fn script_backdrop_pose(app: &mut App, camera: Entity) {
    app.world_mut()
        .entity_mut(camera)
        .insert(ScriptedCameraPose {
            position: Vec3::new(0.0, 90.0, 300.0),
            look_at: Vec3::ZERO,
        });
}

/// A menu app with a two-mod catalog (locked base + toggleable demo, both
/// with full bundle meta), entered into MainMenu and updated once so the
/// mods screen's refresh systems have populated the list and details pane.
pub(crate) fn mods_app() -> App {
    let mut app = app();
    app.insert_resource(dummy_scenarios());
    app.insert_resource(ModCatalog(vec![
        ModInfo {
            id: "base".to_string(),
            base: true,
            meta: ModMeta {
                name: "Base Game".to_string(),
                description: "The core Nova Protocol content.".to_string(),
                author: "Nova".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
        },
        ModInfo {
            id: "demo".to_string(),
            base: false,
            meta: ModMeta {
                name: "Demo Mod".to_string(),
                description: "A demo mod for testing.".to_string(),
                author: "Alice".to_string(),
                version: "0.2.0".to_string(),
                dependencies: vec!["base".to_string()],
                ..Default::default()
            },
        },
    ]));
    app.insert_resource(EnabledMods(["base".to_string()].into_iter().collect()));
    app.world_mut()
        .resource_mut::<NextState<GameStates>>()
        .set(GameStates::MainMenu);
    app.update();
    app
}

pub(crate) fn entity_by_name(app: &mut App, name: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &Name)>();
    q.iter(app.world())
        .find(|(_, n)| n.as_str() == name)
        .map(|(e, _)| e)
}

pub(crate) fn all_texts(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

pub(crate) fn mod_row(app: &mut App, id: &str) -> Option<Entity> {
    let mut q = app.world_mut().query::<(Entity, &ModRow)>();
    q.iter(app.world())
        .find(|(_, r)| r.id == id)
        .map(|(e, _)| e)
}

pub(crate) fn checkbox_of(app: &mut App, id: &str) -> Option<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &ModToggle), With<ModEnableCheckbox>>();
    q.iter(app.world())
        .find(|(_, t)| t.id == id)
        .map(|(e, _)| e)
}

/// The single Text child's content (checkbox mark, themed button label).
/// The first `Text` in the subtree, in child order. Handles a direct text
/// child (checkbox glyph, campaign header) and a nested one (a `list_row`
/// whose name lives in an inner column). NOTE: assumes non-`.block()`
/// buttons - a block button's first child is the `> ` cursor span, so this
/// would return that, not the label. All buttons it inspects are plain.
pub(crate) fn label_of(app: &App, entity: Entity) -> String {
    fn first_text(app: &App, entity: Entity) -> Option<String> {
        if let Some(text) = app.world().get::<Text>(entity) {
            return Some(text.0.clone());
        }
        let kids: Vec<Entity> = app
            .world()
            .get::<Children>(entity)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        kids.into_iter().find_map(|child| first_text(app, child))
    }
    first_text(app, entity).expect("a Text somewhere in the subtree")
}

pub(crate) fn selected_mod(app: &App) -> Option<String> {
    app.world().resource::<SelectedModId>().0.clone()
}

/// Every portal trigger the menu can fire, captured with its id (and, for
/// the catalog fetch, whether the state was Idle at trigger time - the
/// retry's force-reset ordering).
#[derive(Resource, Default)]
pub(crate) struct PortalCaptures {
    pub(crate) installs: Vec<String>,
    pub(crate) uninstalls: Vec<String>,
    pub(crate) fetches: usize,
    pub(crate) fetch_seen_idle: bool,
}

pub(crate) fn observe_portal_events(app: &mut App) {
    app.init_resource::<PortalCaptures>();
    app.add_observer(|e: On<InstallPortalMod>, mut cap: ResMut<PortalCaptures>| {
        cap.installs.push(e.id.clone());
    });
    app.add_observer(
        |e: On<UninstallPortalMod>, mut cap: ResMut<PortalCaptures>| {
            cap.uninstalls.push(e.id.clone());
        },
    );
    app.add_observer(
        |_: On<FetchPortalCatalog>,
         remote: Option<Res<RemoteCatalog>>,
         mut cap: ResMut<PortalCaptures>| {
            cap.fetches += 1;
            cap.fetch_seen_idle =
                remote.is_some_and(|r| matches!(r.state, RemoteCatalogState::Idle));
        },
    );
}

pub(crate) fn downloaded_set(records: &[(&str, &str)]) -> DownloadedMods {
    DownloadedMods(
        records
            .iter()
            .map(|(id, version)| DownloadedMod {
                record: InstalledModRecord {
                    id: id.to_string(),
                    version: version.to_string(),
                    bundle: format!("{id}.bundle.ron"),
                },
                bundle: Handle::default(),
            })
            .collect(),
    )
}
