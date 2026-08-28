use std::process::ExitCode;

use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "nova_protocol")]
#[command(version = APP_VERSION)]
#[command(about = "Simple spaceship editor scene where you can build custom ships", long_about = None)]
struct Cli {
    // The dev-tool subcommands only exist on native builds; the wasm bundle
    // parses a bare game invocation.
    #[cfg(not(target_arch = "wasm32"))]
    #[command(subcommand)]
    command: Option<Command>,
    /// Boot straight into this scenario, past the main menu. An unknown id
    /// refuses the launch and lists every registered one.
    #[cfg(not(target_arch = "wasm32"))]
    #[arg(long, value_name = "ID")]
    scenario: Option<String>,
    /// Register the scenarios in this loose `*.content.ron` and boot into one,
    /// without installing it as a mod. `--scenario` picks which; without it,
    /// the file's first scenario.
    #[cfg(not(target_arch = "wasm32"))]
    #[arg(long, value_name = "PATH")]
    scenario_file: Option<std::path::PathBuf>,
    #[cfg(feature = "debug")]
    #[arg(long)]
    debugdump: bool,
    /// Run the simulation with no GPU device, no window and none of the visual
    /// plugins. Needs a driver to ever end: pair it with `--scenario` under
    /// `NOVA_AUTOPILOT`, or it ticks forever with nothing to close.
    #[cfg(feature = "debug")]
    #[arg(long)]
    norender: bool,
    /// Zero the audio output. The other half of the outputs-off pair: this
    /// silences the speakers as `--norender` drops the renderer. The volume
    /// SETTING is untouched, so persistence and the settings menu never see it.
    #[cfg(feature = "debug")]
    #[arg(long)]
    mute: bool,
    /// Speak the process channel: named inputs on stdin, world snapshots on
    /// stdout, one JSON object per line. `step` gates the clock on the
    /// driver's ticks; `free` lets the app run and applies lines as they
    /// arrive. Needs `--norender`: the channel's runner replaces the headless
    /// one, and a winit window loop cannot be stepped from stdin.
    #[cfg(feature = "debug")]
    #[arg(long, value_name = "MODE", requires = "norender")]
    channel: Option<nova_channel::ChannelMode>,
    /// With `--channel`: draw every tick offscreen and save it as
    /// `DIR/frame_%06d.png`. Still no window - but unlike plain `--norender`
    /// the GPU is armed, so the frames are the real render. Stitch them with
    /// `ffmpeg -framerate 60 -i DIR/frame_%06d.png -pix_fmt yuv420p out.mp4`
    /// for a real-time movie however slowly the driver stepped.
    #[cfg(feature = "debug")]
    #[arg(long, value_name = "DIR", requires = "channel")]
    record: Option<std::path::PathBuf>,
}

/// The dev tools behind the game binary. Each variant forwards its raw
/// arguments to the owning crate's CLI, which does its own parsing - so the
/// crates keep their help text and this dispatcher stays thin.
#[cfg(not(target_arch = "wasm32"))]
#[derive(clap::Subcommand)]
enum Command {
    /// Author and validate content (gen/lint); `content --help` for details.
    #[command(disable_help_flag = true)]
    Content {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
    /// The run-harness host: run cataloged examples, grade and report them.
    #[cfg(feature = "debug")]
    #[command(disable_help_flag = true)]
    Probe {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> ExitCode {
    #[allow(unused_variables)]
    let cli = Cli::parse();

    #[cfg(not(target_arch = "wasm32"))]
    match cli.command {
        Some(Command::Content { ref args }) => return nova_authoring::cli::main(args),
        #[cfg(feature = "debug")]
        Some(Command::Probe { ref args }) => return nova_probe_cli::native::main(args),
        None => {}
    }

    #[cfg(feature = "debug")]
    let render = !cli.norender;
    #[cfg(not(feature = "debug"))]
    let render = true;

    // The wasm bundle has no command line to carry a scenario id.
    #[cfg(not(target_arch = "wasm32"))]
    let startup_scenario = match (cli.scenario_file.clone(), cli.scenario.clone()) {
        (Some(path), id) => Some(StartupScenario::File { path, id }),
        (None, Some(id)) => Some(StartupScenario::Id(id)),
        (None, None) => None,
    };
    #[cfg(target_arch = "wasm32")]
    let startup_scenario = None;

    // `--record` swaps the headless assembly for the offscreen one: same
    // windowless shape, but with the GPU the frame capture reads.
    #[cfg(feature = "debug")]
    let mut app = if cli.record.is_some() {
        offscreen_app(startup_scenario)
    } else {
        editor_app(render, startup_scenario)
    };
    #[cfg(not(feature = "debug"))]
    let mut app = editor_app(render, startup_scenario);

    // AFTER the builder: `NovaSettingsPlugin` resolves `HarnessMute` from the
    // environment at its own build, and the flag has to win over that. Only
    // when asked - without it the env answer stands, so `NOVA_MUTE=0` still
    // forces sound through a harness run.
    #[cfg(feature = "debug")]
    if cli.mute {
        app.insert_resource(HarnessMute(true));
    }

    // The probe collectors ride the SHIPPED app, so `probe scenario <id|path>`
    // measures the same binary a player runs instead of an example standing in
    // for it. Each capability is env-gated; an unarmed run wires nothing. The
    // autopilot is what makes such a run terminate.
    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default());
        app.add_plugins(nova_protocol::nova_debug::harness::nova_autopilot());
    }

    #[cfg(feature = "debug")]
    if cli.debugdump {
        debugdump(&mut app);
        return ExitCode::SUCCESS;
    }

    // LAST, so its runner wins over the headless ScheduleRunner one and its
    // writer systems see every plugin's sets already registered.
    #[cfg(feature = "debug")]
    if let Some(mode) = cli.channel {
        app.add_plugins(nova_channel::NovaChannelPlugin {
            mode,
            record: cli.record.clone(),
        });
    }

    run_app(&mut app)
}
