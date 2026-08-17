//! `nova_core` is the composition root: [`AppBuilder`] wires the whole plugin
//! stack - window and logging setup, assets, gameplay, the scenario/modding
//! engine, menus, the editor, and (under the `debug` feature) the debug
//! tooling - into a runnable app. It owns no gameplay logic itself; every
//! subsystem lives in its own `nova_*` crate and this crate assembles them in
//! the right order. Start here to see how the pieces fit together; the
//! architecture wiki page carries the dependency graph.
#![warn(missing_docs)]

use std::process::ExitCode;

use bevy::{
    app::Plugins,
    log::{Level, LogPlugin},
    prelude::*,
    // NOTE: RenderPlugin is not in bevy's prelude.
    render::RenderPlugin,
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
pub use nova_hud;
use nova_hud::prelude::*;
pub use nova_info;
pub use nova_menu;
use nova_menu::prelude::*;
pub use nova_scenario;
use nova_scenario::prelude::*;
pub use nova_ship;
use nova_ship::prelude::*;
use nova_ui::status_bar::{
    status_bar, status_bar_item, status_fps_color_fn, status_fps_value_fn, status_version_color_fn,
    status_version_value_fn, StatusBarItemConfig, StatusBarRootConfig,
};

mod loading_screen;
use loading_screen::LoadingScreenPlugin;

/// Glob-import surface: `use nova_core::prelude::*` re-exports every subsystem
/// crate's prelude plus [`AppBuilder`], [`editor_app`] and [`run_app`], so a
/// binary or example wires the whole stack from one import.
pub mod prelude {
    pub use nova_assets::prelude::*;
    #[cfg(feature = "debug")]
    pub use nova_debug::prelude::*;
    pub use nova_editor::prelude::*;
    pub use nova_events::prelude::*;
    pub use nova_gameplay::prelude::*;
    pub use nova_hud::prelude::*;
    pub use nova_info::prelude::*;
    pub use nova_menu::prelude::*;
    pub use nova_scenario::prelude::*;
    pub use nova_ship::prelude::*;

    pub use super::{editor_app, run_app, AppBuilder};
}

/// Build the editor application - the exact app the `nova_protocol` binary runs.
///
/// The editor is [`AppBuilder`]'s default "game": `build()` adds `NovaEditorPlugin` when no
/// custom game plugins were supplied. Factoring it here lets the binary and the harnessed editor
/// example (`examples/systems/ship_editor.rs`) launch the identical app instead of each open-coding it, so
/// the example exercises the same editor the game ships.
///
/// `startup_scenario` is the binary's `--scenario <id>` flag: `Some(id)` boots
/// straight into that scenario instead of the main menu (see
/// [`AppBuilder::with_startup_scenario`]).
pub fn editor_app(render: bool, startup_scenario: Option<ScenarioId>) -> App {
    AppBuilder::new()
        .with_rendering(render)
        .with_startup_scenario(startup_scenario)
        .build()
}

/// Run `app` and translate bevy's [`AppExit`] into a process exit code.
///
/// The binary hands its app here rather than calling `App::run` and discarding
/// the result: a refusal raised INSIDE the app - an unknown `--scenario` id is
/// the only one today - must leave a non-zero status for the shell.
pub fn run_app(app: &mut App) -> ExitCode {
    match app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(code) => ExitCode::from(code.get()),
    }
}

/// Composition root that assembles the full plugin stack into a runnable [`App`].
///
/// Holds the in-progress [`App`] plus the choices ([`with_game_plugins`](Self::with_game_plugins),
/// [`with_rendering`](Self::with_rendering),
/// [`with_startup_scenario`](Self::with_startup_scenario)) that [`build`](Self::build) resolves
/// into the concrete plugin set; with no game plugins it defaults to the editor app fronted by the
/// main menu.
pub struct AppBuilder {
    app: App,
    use_default_plugins: bool,
    render: bool,
    startup_scenario: Option<ScenarioId>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    /// Start a builder with [`DefaultPlugins`] already set up (windowing, logging,
    /// assets, and the `mods://` source registered before `AssetPlugin` lands).
    pub fn new() -> Self {
        let mut app = App::new();
        // NOTE: the `mods://` source must be registered BEFORE AssetPlugin lands
        // with DefaultPlugins below - bevy builds the registered sources at
        // AssetPlugin insertion, not lazily. It cannot live inside
        // `assets_plugin()`, which returns the AssetPlugin VALUE for `.set()`
        // while source registration needs the App.
        nova_assets::mod_cache::register_mods_source(&mut app);
        app.add_plugins(
            DefaultPlugins
                .build()
                .set(assets_plugin())
                .set(log_plugin())
                .set(window_plugin())
                .set(render_plugin()),
        );

        Self {
            app,
            use_default_plugins: true,
            render: true,
            startup_scenario: None,
        }
    }

    /// Supply custom game plugins in place of the default editor app; this also
    /// suppresses the main menu, so the examples boot straight into gameplay.
    pub fn with_game_plugins<M>(mut self, plugins: impl Plugins<M>) -> Self {
        self.app.add_plugins(plugins);
        self.use_default_plugins = false;
        self
    }

    /// Set whether the app renders; passed down to the gameplay and scenario
    /// plugins so headless runs (tests, the probe harness) skip GPU work.
    pub fn with_rendering(mut self, render: bool) -> Self {
        self.render = render;
        self
    }

    /// Boot straight into one scenario instead of the main menu - the game
    /// binary's `--scenario <id>` flag.
    ///
    /// The menu plugin is still added (the pause overlay, the outcome screens
    /// and the New Game loader all live there); only the `Loaded` handoff
    /// changes target, and `build` writes the menu's own [`NewGameScenario`]
    /// override so the scenario comes up through the SAME OnEnter(Playing)
    /// loader - and the same non-blocking load screen - a click on Play uses.
    ///
    /// No effect on an app that supplied its own game plugins: those never had
    /// a menu to skip.
    pub fn with_startup_scenario(mut self, id: Option<ScenarioId>) -> Self {
        self.startup_scenario = id;
        self
    }

    /// Resolve the builder into a runnable [`App`]: adds the gameplay, scenario,
    /// asset, editor (when no game plugins were given), menu, and debug plugins,
    /// and installs the `Loaded -> MainMenu`/`Playing` handoff.
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
        self.app.add_plugins(LoadingScreenPlugin);
        self.app.add_plugins(NovaGameplayPlugin {
            render: self.render,
        });
        // The ship sits above the shared gameplay layer and orders its sets
        // inside gameplay's `SpaceshipSystems` brackets, so it is added after.
        self.app.add_plugins(NovaShipPlugin {
            render: self.render,
        });
        self.app.add_plugins(NovaScenarioPlugin {
            render: self.render,
        });

        // The flight HUD and the NOVA OS cockpit monitor are peers, each its own
        // crate above gameplay - so the crate that orders them adds them. Both
        // are render-gated: a headless harness run draws neither. The HUD goes
        // first, because the monitor orders its own sets against
        // `NovaHudSystems`.
        if self.render {
            self.app.add_plugins(nova_hud::NovaHudPlugin);
            self.app.add_plugins(nova_os_ui::NovaOsUiPlugin);
        }

        if self.use_default_plugins {
            self.app.add_plugins(NovaEditorPlugin);
        }

        // The menu fronts the default (editor) app only: an example that supplies
        // its own game plugins goes straight `Loading -> Playing`.
        let has_menu = self.use_default_plugins;
        if has_menu {
            self.app.add_plugins(NovaMenuPlugin);
        }

        #[cfg(feature = "debug")]
        self.app.add_plugins(DebugPlugin);

        // `--scenario <id>`: enter through the menu's own New Game door rather
        // than opening a second loader path. Only a menu app has a menu to
        // skip, so an app with its own game plugins ignores the flag.
        let startup_scenario = if has_menu {
            self.startup_scenario.clone()
        } else {
            None
        };
        if let Some(id) = &startup_scenario {
            self.app.insert_resource(GameMode::NewGame);
            self.app.insert_resource(NewGameScenario(Some(id.clone())));
        }
        let boot_to_menu = has_menu && startup_scenario.is_none();

        // NOTE: only advance when still in Loading - the screenshot harness
        // (NOVA_SHOT) force-sets Playing on the first frame, and this hook firing
        // seconds later must not yank the app backwards into the menu.
        self.app.add_systems(
            OnEnter(GameAssetsStates::Loaded),
            (
                move |state: Res<State<GameStates>>,
                      mut next: ResMut<NextState<GameStates>>,
                      scenarios: Option<Res<GameScenarios>>,
                      mut exit: MessageWriter<AppExit>| {
                    if *state.get() != GameStates::Loading {
                        return;
                    }
                    // The merged registry only exists here, once the bundle
                    // merge has run - which is why an unknown `--scenario` id
                    // cannot be refused before the window opens.
                    if let Some(id) = startup_scenario.as_deref() {
                        let empty = GameScenarios::default();
                        let scenarios = scenarios.as_deref().unwrap_or(&empty);
                        if !scenarios.contains_key(id) {
                            report_unknown_startup_scenario(id, scenarios);
                            exit.write(AppExit::error());
                            return;
                        }
                    }
                    next.set(if boot_to_menu {
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

/// Refuse an unknown `--scenario <id>` in words, and list what the player could
/// have asked for.
///
/// The list comes from the MERGED registry the Scenarios picker itself reads, so
/// an enabled mod's ids are in it. It is the full registry rather than the
/// picker's visible rows: the flag can also launch a `hidden` chapter or a menu
/// backdrop, which is most of the point of having it.
///
/// Printed to stderr rather than logged. This is a command-line refusal and it
/// must reach the terminal whatever `RUST_LOG` and the release log filter say.
fn report_unknown_startup_scenario(id: &str, scenarios: &GameScenarios) {
    eprintln!("error: --scenario '{id}' matches no registered scenario.");
    let mut ids: Vec<&str> = scenarios.keys().map(String::as_str).collect();
    if ids.is_empty() {
        eprintln!("no scenarios are registered at all - the content merge found none.");
        return;
    }
    ids.sort_unstable();
    eprintln!("available scenario ids ({}):", ids.len());
    for available in ids {
        eprintln!("  {available}");
    }
}

fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        primary_window: Some(Window {
            title: format!("NovaProtocol - {}", env!("CARGO_PKG_VERSION")),
            resolution: (1024, 768).into(),
            present_mode: PresentMode::AutoVsync,
            // NOTE: selector of the canvas shipped in the repo-root `index.html`.
            canvas: Some("#bevy".to_owned()),
            fit_canvas_to_parent: true,
            // NOTE: true lets the canvas capture tab and other browser keys on wasm.
            prevent_default_event_handling: true,
            ..Default::default()
        }),
        ..default()
    }
}

fn render_plugin() -> RenderPlugin {
    RenderPlugin {
        // NOTE: do not flip this back to bevy's async default (task
        // 20260805-111329). An async pipeline-compile task still in flight at
        // exit drops the last `Arc<Device>` from an `AsyncComputeTaskPool`
        // thread while the main thread tears the same Vulkan device down,
        // which SIGSEGVs inside the driver - one run in five for the
        // self-ending `menu_scenarios` example. Compiling synchronously means
        // no compile task ever owns a device reference, so the race cannot
        // occur.
        synchronous_pipeline_compilation: true,
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
            "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn,nova_assets=trace,nova_autopilot=trace,nova_core=trace,nova_debug=trace,nova_events=trace,nova_gameplay=trace,nova_info=trace,nova_scenario=trace,nova_ui=trace"
        } else {
            "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn,nova_assets=debug,nova_autopilot=debug,nova_core=debug,nova_debug=debug,nova_events=debug,nova_gameplay=debug,nova_info=debug,nova_scenario=debug,nova_ui=debug"
        }
    } else {
        "wgpu=error,bevy_render=warn,bevy_ecs=warn,bevy_time=warn,naga=warn"
    }
}

/// The app's asset configuration. Public so tests can load assets through the
/// exact config the game ships, rather than a hand-rolled `AssetPlugin` that
/// can mask a bug by differing from it.
///
/// `AssetMetaCheck::Always` reads a `.meta` sidecar for EVERY asset, whatever
/// its source, which is what makes the shipped `.meta` files take effect.
/// Do not narrow it: `Never` defeats `cubemap.png.meta`'s `array_layout` and
/// resurrects the skybox upload race, and a fixed `Paths` set cannot cover
/// mod-shipped skyboxes, whose `mods://`/`self://` paths are dynamic and never
/// appear in a set fixed at App build.
///
/// The cost is web-only and non-fatal: on wasm the asset reader `fetch()`es
/// `<path>.meta` for every asset and the sidecar-less majority come back HTTP
/// 404, which bevy handles by falling back to the loader's default meta
/// (bevy_asset 0.19 `server/mod.rs:1564-1644` and `io/wasm.rs:100-124`, at the
/// pinned rev). Native pays only a filesystem stat.
///
/// A cubemap whose `.meta` `array_layout` applied arrives already 6-layer, which
/// SKIPS the SkyboxPlugin fallback branch that also set the Cube texture
/// view - the swap applier (`nova_scenario::apply_pending_skybox_swaps`) sets the
/// view for that case; keep the two in sync.
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
    // NOTE: the bar is deliberately NOT `HudNovaOsExempt`. While the NOVA OS
    // computer is open the whole flight status bar hides, and the one item that
    // matters there - FPS - is rehomed onto the NOVA OS terminal topbar (see
    // `drive_nova_os_topbar_fps` in nova_os_ui/src/terminal/shell.rs).
    // Without the exemption `apply_hud_visibility` hides the bar in
    // `PauseStates::NovaOs` and its pause-change restore branch un-hides it on
    // close. The base GlobalZIndex keeps a stable z at the HUD layer.
    commands.spawn((
        HudTier::Status,
        GlobalZIndex::default(),
        status_bar(StatusBarRootConfig::default()),
    ));

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
