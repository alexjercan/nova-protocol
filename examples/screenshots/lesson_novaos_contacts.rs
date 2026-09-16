//! lesson_novaos_contacts: the handbook's "Reading contacts" still - the NOVA
//! OS map app with one contact picked and its range and bearing under the plot.
//!
//! Its own producer, and its own range, because the map plots EVERYTHING the
//! scenario carries. The rest of the NOVA OS set (`lesson_novaos`) is shot in
//! the rock hollow, where the forty-eight asteroids are forty-eight contacts,
//! each drawn at its own projected size - the two ships the lesson is about
//! disappear under a field of white discs, and no amount of zoom helps, because
//! the shell surrounds the pocket. `computer::nova_os_plot_range` is the same
//! NOVA OS range the wiki shots use with traffic parked on it: one hostile
//! close in, a friendly tender to port, a second hostile further out.
//!
//! ## Why the walk cycles TWICE
//!
//! The map's contact list is the own ship first, then the ships in spawn order
//! (`crates/nova_os_ui/src/map/contacts.rs`), and `cycle_index` picks index 0
//! when nothing is selected - so the first press lands on SELF, whose readout
//! line is "That is you". The second steps onto the raider, which is the
//! contact the lesson is about, and the plot range spawns the raider first for
//! exactly that reason.
//!
//! Two run modes, both under the autopilot (`NOVA_AUTOPILOT`):
//! - `NOVA_AUTOPILOT=1` alone: the smoke path - drive the whole walk, exit
//!   clean, writing nothing.
//! - `NOVA_AUTOPILOT=1 NOVA_CAPTURE=1`: also write the still (staged under
//!   `NOVA_CAPTURE_DIR`).
//!
//! Capture (windowed, real GPU):
//! ```text
//! NOVA_CAPTURE_DIR=target/lesson-shots NOVA_AUTOPILOT=1 NOVA_CAPTURE=1 \
//!   cargo run --example lesson_novaos_contacts --features debug
//! ```

#[path = "shared/computer.rs"]
mod computer;

use bevy::prelude::*;
use clap::Parser;
use computer::nova_os_plot_range;
#[cfg(feature = "debug")]
use computer::{press_enter, press_tab, type_word};
use nova_protocol::prelude::*;

#[derive(Parser)]
#[command(name = "lesson_novaos_contacts")]
#[command(version = "1.0.0")]
#[command(about = "Record the handbook's contacts still", long_about = None)]
struct Cli;

/// The still this shoots for: "Reading contacts".
#[cfg(feature = "debug")]
const CONTACTS_SHOT: &str = "novaos_contacts.png";

fn main() -> bevy::app::AppExit {
    let _ = Cli::parse();
    let mut app = AppBuilder::new().with_game_plugins(custom_plugin).build();

    #[cfg(feature = "debug")]
    {
        app.add_plugins(nova_probe::NovaProbePlugin::default().without_frametime());
        app.add_systems(Startup, (force_capture_resolution, hide_dev_overlays));
        app.add_plugins(contacts_script());
        // The traffic is parked and the shot is posed, so nothing in the range
        // may drift while the map is being read - but only on a capture run, so
        // a plain `cargo run` is still the range as it flies.
        app.add_systems(Update, freeze_bodies.run_if(capturing));
    }

    app.run()
}

fn custom_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameAssetsStates::Loaded), setup_range);
}

fn setup_range(mut commands: Commands, game_assets: Res<GameAssets>, sections: Res<GameSections>) {
    commands.trigger(LoadScenario(nova_os_plot_range(&game_assets, &sections)));
}

/// Raise the monitor, open the map, pick the hostile, shoot the plot.
#[cfg(feature = "debug")]
fn contacts_script() -> nova_protocol::nova_debug::harness::AutopilotPlugin<GameStates> {
    nova_protocol::nova_debug::harness::AutopilotPlugin::<GameStates>::new()
        .step("load the plot range")
        .enter(GameStates::Loading)
        .until(player_ship_present())
        .deadline(30.0)
        .add()
        .step("settle the range")
        .until(elapsed(2.0))
        .add()
        .step("raise the monitor")
        .on_enter(press_tab)
        .until(nova_os_raster_open())
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("type the map command")
        .on_enter(|world: &mut World| type_word(world, "map"))
        .until(nova_os_command_line_reads("map"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        .step("launch the map app")
        .on_enter(press_enter)
        .until(nova_os_app_owns_the_screen("map"))
        .deadline(STEP_DEADLINE_SECS)
        .add()
        // Owning the screen is not being STILL: the plot's offscreen scene
        // builds and settles over frames, which is render work.
        .step("settle the map")
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("cycle past the own ship")
        .on_enter(press_action("novaos_next"))
        .until(frames(1))
        .add()
        .step("let the cycle key up")
        .on_enter(release_action("novaos_next"))
        .until(frames(1))
        .add()
        .step("cycle onto the hostile")
        .on_enter(press_action("novaos_next"))
        .until(frames(1))
        .add()
        // The map EASES its framing onto a new selection
        // (`map_focus_follow`), so the plot is still sliding for a moment
        // after the key goes up.
        .step("let the cycle key up again and let the plot settle")
        .on_enter(release_action("novaos_next"))
        .until(frames(SETTLE_FRAMES))
        .add()
        .step("capture the contacts screen")
        .on_enter(|world: &mut World| shoot(world, CONTACTS_SHOT))
        .until(shot_written(CONTACTS_SHOT))
        .deadline(SHOT_DEADLINE_SECS)
        .add()
}
