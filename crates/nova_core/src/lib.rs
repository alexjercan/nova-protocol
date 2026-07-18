use bevy::{
    app::Plugins,
    log::{Level, LogPlugin},
    prelude::*,
    window::PresentMode,
};
use nova_assets::prelude::*;
#[cfg(feature = "debug")]
pub use nova_debug;
#[cfg(feature = "debug")]
use nova_debug::DebugPlugin;
pub use nova_editor;
use nova_editor::prelude::*;
pub use nova_events;
pub use nova_gameplay;
use nova_gameplay::prelude::*;
pub use nova_info;
pub use nova_menu;
use nova_menu::prelude::*;
pub use nova_scenario;
use nova_scenario::prelude::*;

pub mod prelude {
    pub use nova_assets::prelude::*;
    #[cfg(feature = "debug")]
    pub use nova_debug::prelude::*;
    pub use nova_editor::prelude::*;
    pub use nova_events::prelude::*;
    pub use nova_gameplay::prelude::*;
    pub use nova_info::prelude::*;
    pub use nova_menu::prelude::*;
    pub use nova_scenario::prelude::*;

    pub use super::{editor_app, AppBuilder};
}

/// Build the editor application - the exact app the `nova_protocol` binary runs.
///
/// The editor is [`AppBuilder`]'s default "game": `build()` adds `NovaEditorPlugin` when no
/// custom game plugins were supplied. Factoring it here lets the binary and the harnessed editor
/// example (`examples/09_editor.rs`) launch the identical app instead of each open-coding it, so
/// the example exercises the same editor the game ships.
pub fn editor_app(render: bool) -> App {
    AppBuilder::new().with_rendering(render).build()
}

pub struct AppBuilder {
    app: App,
    use_default_plugins: bool,
    render: bool,
    main_menu: Option<bool>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        let mut app = App::new();
        // The `mods://` source (the downloaded-mods cache, task 20260715-142906)
        // must be registered BEFORE AssetPlugin lands with DefaultPlugins below:
        // bevy builds the registered sources at AssetPlugin insertion, not
        // lazily. It cannot live inside `assets_plugin()` (that returns the
        // AssetPlugin VALUE for `.set()`; source registration needs the App), so
        // the registration helper lives next to the cache in
        // `nova_assets::mod_cache` and is called here - which also lets the
        // nova_assets integration tests build their rigs on the exact production
        // registration.
        nova_assets::mod_cache::register_mods_source(&mut app);
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(assets_plugin())
                .set(log_plugin())
                .set(window_plugin()),
        );

        Self {
            app,
            use_default_plugins: true,
            render: true,
            main_menu: None,
        }
    }

    pub fn with_game_plugins<M>(mut self, plugins: impl Plugins<M>) -> Self {
        self.app.add_plugins(plugins);
        self.use_default_plugins = false;
        self
    }

    pub fn with_rendering(mut self, render: bool) -> Self {
        self.render = render;
        self
    }

    /// Override whether the app boots into the main menu.
    ///
    /// By default the menu comes up only for the default (editor) app: examples that
    /// supply their own game plugins via [`with_game_plugins`](Self::with_game_plugins)
    /// go straight `Loading -> Playing` as before, so they need no changes.
    pub fn with_main_menu(mut self, main_menu: bool) -> Self {
        self.main_menu = Some(main_menu);
        self
    }

    pub fn build(mut self) -> App {
        // NOTE: UiWidgetsPlugins is part of Bevy's DefaultPlugins as of 0.19 (it was an
        // experimental, manually-added plugin group in 0.17). AppBuilder::new() already
        // adds DefaultPlugins, so adding it again here panics with "plugin was already
        // added". Do not re-add it.

        self.app.init_state::<GameStates>();
        self.app.init_state::<PauseStates>();

        self.app
            .add_plugins(bevy_enhanced_input::EnhancedInputPlugin);
        self.app.add_plugins(GameAssetsPlugin);
        self.app.add_plugins(NovaGameplayPlugin {
            render: self.render,
        });
        self.app.add_plugins(NovaScenarioPlugin {
            render: self.render,
        });

        // Add the editor (the default "game") if no custom game plugins were provided
        if self.use_default_plugins {
            self.app.add_plugins(NovaEditorPlugin);
        }

        // The main menu fronts the default app unless explicitly overridden; custom
        // game plugins (the examples) skip it and keep the direct Loading -> Playing
        // lifecycle.
        let main_menu = self.main_menu.unwrap_or(self.use_default_plugins);
        if main_menu {
            self.app.add_plugins(NovaMenuPlugin);
        }

        #[cfg(feature = "debug")]
        self.app.add_plugins(DebugPlugin);

        // When assets are loaded, hand off to the main menu (when it fronts the app)
        // or straight to gameplay. The status UI comes up either way. Only advance
        // when still in Loading: the screenshot harness (BCS_SHOT) force-sets
        // Playing on the first frame, and this hook firing seconds later must not
        // yank the app backwards into the menu (review R1.2).
        self.app.add_systems(
            OnEnter(GameAssetsStates::Loaded),
            (
                move |state: Res<State<GameStates>>, mut next: ResMut<NextState<GameStates>>| {
                    if *state.get() != GameStates::Loading {
                        return;
                    }
                    next.set(if main_menu {
                        GameStates::MainMenu
                    } else {
                        GameStates::Playing
                    });
                },
                setup_status_ui,
            ),
        );

        self.app
    }
}

// pub fn new_headless_app() -> App {
//     let mut app = App::new();
//     app.add_plugins((
//         DefaultPlugins
//             .build()
//             .set(AssetPlugin {
//                 meta_check: bevy::asset::AssetMetaCheck::Never,
//                 ..default()
//             })
//             .set(log_plugin())
//             .disable::<WinitPlugin>(),
//         ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 64.0)),
//     ));
//
//     app
// }

fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: format!("NovaProtocol - {}", env!("CARGO_PKG_VERSION")),
            resolution: (1024, 768).into(),
            present_mode: PresentMode::AutoVsync,
            // Bind to canvas included in `index.html`
            canvas: Some("#bevy".to_owned()),
            fit_canvas_to_parent: true,
            // set to true if we want to capture tab etc in wasm
            prevent_default_event_handling: true,
            ..Default::default()
        }),
        ..default()
    }
}

fn log_plugin() -> LogPlugin {
    LogPlugin {
        level: Level::INFO,
        filter: log_filter_str().to_string(),
        ..default()
    }
}

fn log_filter_str<'a>() -> &'a str {
    if cfg!(feature = "debug") {
        if std::env::var("RUST_LOG")
            .unwrap_or_default()
            .contains("trace")
        {
            "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn,bevy_common_systems=trace,nova_assets=trace,nova_core=trace,nova_debug=trace,nova_events=trace,nova_gameplay=trace,nova_info=trace,nova_scenario=trace"
        } else {
            "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn,bevy_common_systems=debug,nova_assets=debug,nova_core=debug,nova_debug=debug,nova_events=debug,nova_gameplay=debug,nova_info=debug,nova_scenario=debug"
        }
    } else {
        "wgpu=error,bevy_render=warn,bevy_ecs=warn,bevy_time=warn,naga=warn"
    }
}

/// The app's asset configuration. Public so tests can load assets through the
/// exact config the game ships (a hand-rolled `AssetPlugin` once masked a bug
/// here: the cubemap meta fix was verified against default settings while the
/// app ignored metas entirely, task 20260713-175416).
///
/// `AssetMetaCheck::Always` reads a `.meta` sidecar for EVERY asset, whatever
/// its source. This is what makes the shipped `.meta` files actually take
/// effect. `Never` silently defeated `cubemap.png.meta`'s `array_layout` and
/// resurrected the skybox upload race (tasks/20260710-143138/NOTES.md); the
/// `Paths` set that followed (task 20260717-013440) fixed the two BASE cubemaps
/// but could not cover mod-shipped skyboxes, whose `mods://`/`self://` paths are
/// dynamic and never appear in a set fixed at App build - so a mod's own
/// cubemap loaded single-layer and rode the same teardown race (task
/// 20260717-111558). `Always` honors every mod's sidecar with no per-path
/// bookkeeping and closes that class for good.
///
/// The cost is web-only and non-fatal: on wasm the asset reader `fetch()`es
/// `<path>.meta` for every asset, and the ones without a sidecar (most of
/// them) come back HTTP 404, which bevy handles by falling back to the
/// loader's default meta (bevy_asset 0.19 `server/mod.rs:1564-1644` and
/// `io/wasm.rs:100-124`, verified at the pinned rev). So the price is one
/// extra request per asset and some 404 console noise on web; native pays
/// only a filesystem stat. We take that over shipping a class of skybox that
/// crashes on WebGL2-class GPUs.
///
/// A cubemap whose `.meta` `array_layout` applied arrives already 6-layer, which
/// SKIPS the bcs SkyboxPlugin fallback branch that also set the Cube texture
/// view - the swap applier (`nova_scenario::apply_pending_skybox_swaps`) sets the
/// view for that case; keep the two in sync. That fallback (a single-layer
/// stacked image reinterpreted only when the observer runs) is exactly the
/// teardown race `Always` avoids by loading the array up front.
///
/// The `mods://` source for downloaded mods is NOT configured here - it must be
/// registered on the App before this plugin is added; `AppBuilder::new` calls
/// `nova_assets::mod_cache::register_mods_source` for that.
pub fn assets_plugin() -> AssetPlugin {
    AssetPlugin {
        meta_check: bevy::asset::AssetMetaCheck::Always,
        ..default()
    }
}

fn setup_status_ui(mut commands: Commands, game_assets: Res<GameAssets>) {
    // Chrome tier: the fps/version bar disappears at HudVisibility::Minimal
    // and below (and in the main menu, which drives the level to None).
    commands.spawn((HudTier::Chrome, status_bar(StatusBarRootConfig::default())));

    commands.spawn((status_bar_item(StatusBarItemConfig {
        icon: Some(game_assets.fps_icon.clone()),
        value_fn: status_fps_value_fn(),
        color_fn: status_fps_color_fn(),
        prefix: "".to_string(),
        suffix: "fps".to_string(),
    }),));
    commands.spawn((status_bar_item(StatusBarItemConfig {
        icon: None,
        value_fn: status_version_value_fn(nova_info::APP_VERSION),
        color_fn: status_version_color_fn(),
        prefix: "v".to_string(),
        suffix: "".to_string(),
    }),));
}
