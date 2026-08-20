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
    window::{ExitCondition, PresentMode},
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
pub use nova_os_ui;
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

    pub use super::{editor_app, run_app, AppBuilder, StartupScenario};
}

/// Build the editor application - the exact app the `nova_protocol` binary runs.
///
/// The editor is [`AppBuilder`]'s default "game": `build()` adds `NovaEditorPlugin` when no
/// custom game plugins were supplied. Factoring it here lets the binary and the harnessed editor
/// example (`examples/systems/system_ship_editor.rs`) launch the identical app instead of each open-coding it, so
/// the example exercises the same editor the game ships.
///
/// `startup` is the binary's `--scenario` / `--scenario-file` flags: `Some(..)`
/// boots straight into that scenario instead of the main menu (see
/// [`AppBuilder::with_startup_scenario`]).
///
/// `render: false` is the binary's `--norender`. `render: true` still yields to
/// [`NORENDER_ENV`], because it goes through [`AppBuilder::new`].
pub fn editor_app(render: bool, startup: Option<StartupScenario>) -> App {
    let builder = if render {
        AppBuilder::new()
    } else {
        AppBuilder::headless()
    };
    builder.with_startup_scenario(startup).build()
}

/// What the app boots into instead of the main menu.
///
/// The id form resolves against the merged [`GameScenarios`] registry, so it
/// reaches shipped content, an enabled mod's content and the editor sandbox
/// alike. The file form reaches content that is not installed at all: a loose
/// `*.content.ron` a contributor is authoring or measuring, registered into the
/// same registry before anything resolves an id against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupScenario {
    /// A scenario id from the merged registry.
    Id(ScenarioId),
    /// A loose content file. Every scenario in it is registered; the run boots
    /// into `id` when one is named, else the file's FIRST scenario. Native
    /// only - the wasm bundle has neither a filesystem nor a command line.
    #[cfg(not(target_arch = "wasm32"))]
    File {
        /// Path to the `*.content.ron` file, resolved against the process cwd.
        path: std::path::PathBuf,
        /// Which of the file's scenarios to boot into.
        id: Option<ScenarioId>,
    },
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
/// [`with_startup_scenario`](Self::with_startup_scenario)) that [`build`](Self::build) resolves
/// into the concrete plugin set; with no game plugins it defaults to the editor app fronted by the
/// main menu.
///
/// Whether the app renders is fixed by the constructor - [`new`](Self::new) or
/// [`headless`](Self::headless) - not by a later setter, because the wgpu and
/// window settings are baked into [`DefaultPlugins`] the moment the builder
/// starts. [`NORENDER_ENV`] turns every `new()` in the process into a
/// `headless()`.
pub struct AppBuilder {
    app: App,
    use_default_plugins: bool,
    render: bool,
    startup_scenario: Option<StartupScenario>,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    /// Start a builder with [`DefaultPlugins`] already set up (windowing, logging,
    /// assets, and the `mods://` source registered before `AssetPlugin` lands).
    ///
    /// **[`NORENDER_ENV`] overrides this.** With that variable set in the
    /// process environment, `new()` assembles exactly what
    /// [`headless`](Self::headless) does - no device, no window, no winit - and
    /// the caller is not consulted. It is the transport for a harness that
    /// cannot pass an argument: an `examples/` range calls `new()` and takes no
    /// command line of its own. `headless()` stays unconditional, so the two
    /// read as "render unless told otherwise" and "never render".
    pub fn new() -> Self {
        Self::assemble(std::env::var_os(NORENDER_ENV).is_none())
    }

    /// Start a builder that draws NOTHING: no wgpu device, no window, no winit
    /// event loop, and none of the visual game plugins. The main schedule still
    /// ticks, so the simulation runs and the probe still counts frames - CPU
    /// ones.
    ///
    /// Rendering is one switch, not two, because the halves cannot be taken
    /// apart: `bevy_hanabi` panics outright without a render sub-app
    /// (`bevy_hanabi-0.19.0/src/plugin.rs:361`), so dropping the device forces
    /// dropping the plugins that need it.
    ///
    /// Nothing in here ends the run. There is no window to close and no input,
    /// so a headless app needs a driver - `--scenario <id>` under
    /// `NOVA_AUTOPILOT`, or `probe scenario` - or it ticks until it is killed.
    ///
    /// Unconditional: unlike [`new`](Self::new), no environment variable can
    /// turn this back into a rendering app.
    pub fn headless() -> Self {
        Self::assemble(false)
    }

    fn assemble(render: bool) -> Self {
        let mut app = App::new();
        // NOTE: the `mods://` source must be registered BEFORE AssetPlugin lands
        // with DefaultPlugins below - bevy builds the registered sources at
        // AssetPlugin insertion, not lazily. It cannot live inside
        // `assets_plugin()`, which returns the AssetPlugin VALUE for `.set()`
        // while source registration needs the App.
        nova_assets::mod_cache::register_mods_source(&mut app);
        let plugins = DefaultPlugins
            .build()
            .set(assets_plugin())
            .set(log_plugin(render))
            .set(window_plugin(render))
            .set(render_plugin(render));

        if render {
            app.add_plugins(plugins);
        } else {
            // `WinitPlugin::build` constructs the event loop, which needs a
            // display server whether or not a window is ever opened - so a run
            // with no display has to replace the runner, not just ask winit for
            // zero windows. Without a replacement `App::run` would tick once
            // and return, because bevy's fallback runner is `run_once`.
            app.add_plugins(plugins.disable::<bevy::winit::WinitPlugin>());
            app.add_plugins(bevy::app::ScheduleRunnerPlugin::default());
            // Works around a bevy 0.19 hole, not a choice of ours.
            // `ExtractComponentPlugin::build` adds `SyncComponentPlugin`
            // unconditionally (bevy_render-0.19.0/src/extract_component.rs:85),
            // and its `on_remove` hook does an unguarded
            // `world.resource_mut::<PendingSyncEntity>()` (sync_component.rs:55)
            // - but the resource ships with `ExtractPlugin`, which a backendless
            // `RenderPlugin` never adds. So despawning any extracted component
            // (the loading screen's UI nodes, first thing after asset load)
            // panics. `SyncWorldPlugin` supplies the resource on its own.
            //
            // Its queue is then never drained, because draining is
            // `entity_sync_system` inside the render sub-app's extract. That
            // leaks one ~24-byte record per synced spawn and per synced
            // component removal - 2.4 MB across 100k of them, linear in run
            // length. Acceptable for probe-length runs, NOT for an indefinite
            // soak. Adding `ExtractPlugin` would drain it, and would also
            // rebuild the mirror render world this flag exists to avoid.
            app.add_plugins(bevy::render::sync_world::SyncWorldPlugin);
        }

        Self {
            app,
            use_default_plugins: true,
            render,
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

    /// Boot straight into one scenario instead of the main menu - the game
    /// binary's `--scenario <id>` / `--scenario-file <path>` flags.
    ///
    /// The menu plugin is still added (the pause overlay, the outcome screens
    /// and the New Game loader all live there); only the `Loaded` handoff
    /// changes target, and it writes the menu's own [`NewGameScenario`]
    /// override so the scenario comes up through the SAME OnEnter(Playing)
    /// loader - and the same non-blocking load screen - a click on Play uses.
    ///
    /// No effect on an app that supplied its own game plugins: those never had
    /// a menu to skip.
    pub fn with_startup_scenario(mut self, startup: Option<StartupScenario>) -> Self {
        self.startup_scenario = startup;
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

        // `--scenario <id>` / `--scenario-file <path>`: enter through the menu's
        // own New Game door rather than opening a second loader path. Only a
        // menu app has a menu to skip, so an app with its own game plugins
        // ignores the flags.
        let startup_scenario = if has_menu {
            self.startup_scenario.clone()
        } else {
            None
        };
        if startup_scenario.is_some() {
            self.app.insert_resource(GameMode::NewGame);
        }
        let boot_to_menu = has_menu && startup_scenario.is_none();

        // NOTE: only advance when still in Loading - the screenshot harness
        // (NOVA_SHOT) force-sets Playing on the first frame, and this hook firing
        // seconds later must not yank the app backwards into the menu.
        //
        // AFTER the editor's sandbox registration: the sandbox is the one
        // scenario with no content file behind it, and a membership check that
        // ran first would refuse the id the editor is about to publish.
        self.app.add_systems(
            OnEnter(GameAssetsStates::Loaded),
            (
                (move |state: Res<State<GameStates>>,
                       mut next: ResMut<NextState<GameStates>>,
                       mut scenarios: Option<ResMut<GameScenarios>>,
                       pick: Option<ResMut<NewGameScenario>>,
                       mut exit: MessageWriter<AppExit>| {
                    if *state.get() != GameStates::Loading {
                        return;
                    }
                    // The merged registry only exists here, once the bundle
                    // merge has run - which is why an unknown `--scenario` id
                    // cannot be refused before the window opens.
                    if let Some(startup) = &startup_scenario {
                        let mut empty = GameScenarios::default();
                        let scenarios = scenarios.as_deref_mut().unwrap_or(&mut empty);
                        match resolve_startup_scenario(startup, scenarios) {
                            Err(message) => {
                                eprintln!("error: {message}");
                                exit.write(AppExit::error());
                                return;
                            }
                            Ok(id) if !scenarios.contains_key(&id) => {
                                report_unknown_startup_scenario(&id, scenarios);
                                exit.write(AppExit::error());
                                return;
                            }
                            Ok(id) => {
                                if let Some(mut pick) = pick {
                                    pick.0 = Some(id);
                                }
                            }
                        }
                    }
                    next.set(if boot_to_menu {
                        GameStates::MainMenu
                    } else {
                        GameStates::Playing
                    });
                })
                .after(EditorSandboxSystems),
                setup_status_ui,
            ),
        );

        self.app
    }
}

/// Resolve a [`StartupScenario`] to the id the run boots into, registering a
/// loose file's scenarios into `scenarios` on the way.
///
/// Registration happens HERE rather than in its own system so the membership
/// check that follows cannot observe the registry without them.
fn resolve_startup_scenario(
    startup: &StartupScenario,
    scenarios: &mut GameScenarios,
) -> Result<ScenarioId, String> {
    // The loose-file variant is native-only, so wasm never reads the registry.
    #[cfg(target_arch = "wasm32")]
    let _ = scenarios;
    match startup {
        StartupScenario::Id(id) => Ok(id.clone()),
        #[cfg(not(target_arch = "wasm32"))]
        StartupScenario::File { path, id } => {
            let loaded = nova_assets::loose::read_loose_scenarios(path)?;
            let first = loaded[0].id.clone();
            for scenario in loaded {
                scenarios.insert(scenario.id.clone(), scenario);
            }
            Ok(id.clone().unwrap_or(first))
        }
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

/// Environment variable that makes [`AppBuilder::new`] assemble a headless app.
/// Any value, including empty, selects it; unset leaves `new()` rendering.
///
/// The game binary has `--norender` and needs no such thing. Examples do: all
/// 48 `AppBuilder::new()` sites take no arguments, so the only channel that
/// reaches them without editing each one - and without every future example
/// having to remember - is the process environment. That matches how the rest
/// of the harness arms itself ([`PERF_ENV`], `NOVA_AUTOPILOT`, `NOVA_CAPTURE`):
/// inert unless set.
///
/// `nova_probe` re-exports it so `probe run --render off` can push it into a
/// child, the same route [`PERF_ENV`] takes.
pub const NORENDER_ENV: &str = "NOVA_NORENDER";

/// Environment variable that arms frame-time capture. Read here only to give an
/// armed run a distinct window class; `nova_probe` owns what it MEANS and
/// re-exports this as `nova_probe::PERF_ENV`.
///
/// NOTE: the `NOVA_PERF_*` prefix predates that crate's rename to `nova_probe`
/// and is kept as-is so scripts and docs do not churn.
pub const PERF_ENV: &str = "NOVA_PERF";

/// `WM_CLASS` (X11) / app id (Wayland) worn by a run armed for measurement, and
/// by no other run.
///
/// A window manager can key on it to place captures somewhere other than the
/// desk the operator is working at - on i3, `for_window [class="nova-measure"]
/// move container to workspace 3`. Measuring under `xvfb-run` is NOT an
/// alternative: a software X server has no scanout, so presenting is a CPU copy
/// of every window pixel and adds about 13.7 ms a frame at 720p.
///
/// Distinct from the normal class on purpose - a placement rule must never
/// catch a hand-run someone is playing.
pub const MEASURE_WINDOW_CLASS: &str = "nova-measure";

fn window_plugin(render: bool) -> WindowPlugin {
    if !render {
        return WindowPlugin {
            primary_window: None,
            // With no window, the default `OnAllClosed` is satisfied on the
            // first frame and the app exits before anything runs. A headless
            // run ends when its driver sends `AppExit`, and only then.
            exit_condition: ExitCondition::DontExit,
            ..default()
        };
    }
    WindowPlugin {
        primary_window: Some(Window {
            title: format!("NovaProtocol - {}", env!("CARGO_PKG_VERSION")),
            // Set only when armed, so the class itself says "this window is
            // being measured" and a placement rule needs no other predicate.
            name: std::env::var_os(PERF_ENV).map(|_| MEASURE_WINDOW_CLASS.to_owned()),
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

/// Environment variable that asks the renderer for GPU timestamp queries, so
/// `nova_probe`'s frame-cost capability can time each render pass on the
/// device instead of inferring it from the wall clock.
///
/// It lives here because [`render_plugin`] is the only place a wgpu feature can
/// be requested and `nova_probe` is the only reader; `nova_core` is the lowest
/// crate both name.
///
/// Never a shipping default: the queries add a resolve pass and a buffer
/// readback to every frame, which is part of the thing being measured.
pub const RENDER_DIAG_ENV: &str = "NOVA_PERF_RENDER_DIAG";

fn render_plugin(render: bool) -> RenderPlugin {
    // Timestamp queries are Vulkan/DX12-only in wgpu, and asking for a feature
    // the adapter lacks fails device creation - so this is a request the probe
    // then verifies: `render/*/elapsed_gpu` is simply absent on a backend that
    // cannot serve it.
    let mut wgpu = bevy::render::settings::WgpuSettings::default();
    if !render {
        // No backend means `RenderPlugin` never asks wgpu for an adapter, so it
        // builds no device and no render sub-app at all - no extract, no
        // prepare, no queue (bevy_render-0.19.0/src/lib.rs:357). Everything
        // below it in `DefaultPlugins` still loads and guards on the sub-app
        // being absent, so the main schedule is untouched.
        wgpu.backends = None;
    }
    if std::env::var_os(RENDER_DIAG_ENV).is_some() {
        wgpu.features |= bevy::render::settings::WgpuFeatures::TIMESTAMP_QUERY
            | bevy::render::settings::WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS
            | bevy::render::settings::WgpuFeatures::TIMESTAMP_QUERY_INSIDE_PASSES;
    }
    RenderPlugin {
        render_creation: wgpu.into(),
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

fn log_plugin(render: bool) -> LogPlugin {
    LogPlugin {
        level: Level::INFO,
        filter: log_filter_str(render),
        ..default()
    }
}

/// Third-party targets clamped on every run, rendering or not. Each is a
/// library that talks at a level the game does not need: wgpu and naga narrate
/// device and shader setup at INFO, and `bevy_ecs`/`bevy_time` log per frame at
/// DEBUG, which buries everything else the moment the nova crates open up.
const THIRD_PARTY_FILTER: &str = "wgpu=error,bevy_ecs=warn,bevy_time=warn,naga=warn";

/// The bevy diagnostics a BACKENDLESS run provokes by construction: one ERROR
/// ("Render app did not exist when trying to add `extract_resource`"), the
/// `CompressedImageFormatSupport` warning, and gizmos noticing there is no
/// `RenderApp`. All three are correct reports of a state
/// [`AppBuilder::headless`] asks for on purpose, and all three are unreachable
/// when a render sub-app exists - so this is only ever added to a headless
/// filter, and a rendering run keeps every one of these targets at its normal
/// level.
///
/// The two `bevy_render` targets are MODULES, not the crate: at the pinned
/// version `bevy_render::extract_resource` logs nothing but the missing-render-
/// app pair and `bevy_render::texture` nothing but the compressed-format
/// warning, so silencing them cannot hide an unrelated error. Gizmos has no
/// such split - its remaining warnings are line-style complaints raised while
/// DRAWING, which a run with no renderer never reaches - so it is clamped to
/// `error` rather than off.
const HEADLESS_FILTER: &str =
    "bevy_render::extract_resource=off,bevy_render::texture=off,bevy_gizmos_render=error";

/// Build the `EnvFilter` string for a run. `render` is false for
/// [`AppBuilder::headless`], which earns the extra clamps above.
///
/// THE RULE FOR A NEW CRATE: there is nothing to do. `EnvFilter` matches a
/// directive against the target by PREFIX, not by equality
/// (`tracing-subscriber-0.3/src/filter/env/directive.rs:246`,
/// `meta.target().starts_with(target)`), and every workspace crate is named
/// `nova_*` - so the single `nova=` directive below covers all of them, and
/// covers a crate added tomorrow on the day it is added.
///
/// Do NOT go back to listing crates one at a time. The list this replaced named
/// nine of the twenty-two that exist, so thirteen - `nova_ship` and `nova_hud`,
/// the two busiest, among them - silently sat at the INFO default while their
/// neighbours were at DEBUG. That failure is invisible from the console: a line
/// that never prints looks exactly like a line that never ran.
fn log_filter_str(render: bool) -> String {
    let nova = if cfg!(feature = "debug") {
        if std::env::var("RUST_LOG")
            .unwrap_or_default()
            .contains("trace")
        {
            "nova=trace"
        } else {
            "nova=debug"
        }
    } else {
        // Release leaves the nova crates on the plugin's INFO default.
        ""
    };
    let bevy_render = if cfg!(feature = "debug") {
        "bevy_render=info"
    } else {
        "bevy_render=warn"
    };

    let mut filter = format!("{THIRD_PARTY_FILTER},{bevy_render}");
    if !nova.is_empty() {
        filter.push(',');
        filter.push_str(nova);
    }
    if !render {
        filter.push(',');
        filter.push_str(HEADLESS_FILTER);
    }
    filter
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_covers_every_nova_crate_with_one_prefix_directive() {
        let filter = log_filter_str(true);
        assert!(
            filter.contains("nova=") || !cfg!(feature = "debug"),
            "the nova prefix directive is what covers all 22 crates: {filter}"
        );
    }

    #[test]
    fn the_filter_never_names_a_single_nova_crate() {
        // A `nova_<crate>=` directive is the drift this design removed: it
        // covers the crates somebody remembered and silently leaves out the
        // rest. Prefix matching makes naming one both unnecessary and a bug.
        let filter = log_filter_str(true);
        assert!(
            !filter.contains("nova_"),
            "per-crate directives reintroduce the allowlist drift: {filter}"
        );
    }

    #[test]
    fn a_rendering_run_keeps_the_bevy_targets_a_headless_run_clamps() {
        let filter = log_filter_str(true);
        assert!(!filter.contains("bevy_render::extract_resource"));
        assert!(!filter.contains("bevy_gizmos_render"));
    }

    #[test]
    fn a_headless_run_clamps_the_three_diagnostics_it_provokes_by_construction() {
        let filter = log_filter_str(false);
        assert!(filter.contains("bevy_render::extract_resource=off"));
        assert!(filter.contains("bevy_render::texture=off"));
        assert!(filter.contains("bevy_gizmos_render=error"));
    }
}
