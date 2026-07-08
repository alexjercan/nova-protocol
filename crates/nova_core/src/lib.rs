use bevy::{
    app::Plugins,
    log::{Level, LogPlugin},
    prelude::*,
    window::PresentMode,
};
use nova_assets::prelude::*;
#[cfg(feature = "debug")]
use nova_debug::DebugPlugin;
pub use nova_editor;
use nova_editor::prelude::*;
pub use nova_events;
pub use nova_gameplay;
use nova_gameplay::prelude::*;
pub use nova_info;
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
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    pub fn new() -> Self {
        let mut app = App::new();
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

    pub fn build(mut self) -> App {
        // NOTE: UiWidgetsPlugins is part of Bevy's DefaultPlugins as of 0.19 (it was an
        // experimental, manually-added plugin group in 0.17). AppBuilder::new() already
        // adds DefaultPlugins, so adding it again here panics with "plugin was already
        // added". Do not re-add it.

        self.app.init_state::<GameStates>();

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

        #[cfg(feature = "debug")]
        self.app.add_plugins(DebugPlugin);

        // When we enter the Loaded state, switch to Playing state
        // Setup the status UI when entering the Playing state
        self.app.add_systems(
            OnEnter(GameAssetsStates::Loaded),
            (
                |mut state: ResMut<NextState<GameStates>>| {
                    state.set(GameStates::Playing);
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

fn assets_plugin() -> AssetPlugin {
    AssetPlugin {
        meta_check: bevy::asset::AssetMetaCheck::Never,
        ..default()
    }
}

fn setup_status_ui(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((status_bar(StatusBarRootConfig::default()),));

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
