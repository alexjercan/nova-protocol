use clap::Parser;
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "nova_protocol")]
#[command(version = APP_VERSION)]
#[command(about = "Simple spaceship editor scene where you can build custom ships", long_about = None)]
struct Cli {
    #[cfg(feature = "debug")]
    #[arg(long)]
    debugdump: bool,
    #[cfg(feature = "debug")]
    #[arg(long)]
    norender: bool,
}

fn main() {
    #[allow(unused_variables)]
    let args = Cli::parse();

    // The editor is the default game; `editor_app` is shared with the harnessed editor example
    // (`examples/09_editor.rs`) so both launch the identical app.
    #[cfg(feature = "debug")]
    let render = !args.norender;
    #[cfg(not(feature = "debug"))]
    let render = true;

    let mut app = editor_app(render);

    #[cfg(feature = "debug")]
    if args.debugdump {
        debugdump(&mut app);
        std::process::exit(0);
    }

    app.run();
}
