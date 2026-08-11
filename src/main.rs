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
    #[cfg(feature = "debug")]
    #[arg(long)]
    debugdump: bool,
    #[cfg(feature = "debug")]
    #[arg(long)]
    norender: bool,
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

    let mut app = editor_app(render);

    #[cfg(feature = "debug")]
    if cli.debugdump {
        debugdump(&mut app);
        return ExitCode::SUCCESS;
    }

    app.run();
    ExitCode::SUCCESS
}
